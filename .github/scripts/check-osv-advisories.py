#!/usr/bin/env python3
"""Audit the resolved dependency graph against OSV, which `cargo-deny` does not cover.

Two gates already touch dependencies and neither answers this question.
`dependency-review` uses GitHub's database but only looks at what a pull request *adds*, so
nothing it says applies to the tree that is already here. `cargo-deny` audits the whole tree
but against RustSec alone, and on 2026-09-01 it reported "advisories ok" while OSV found
twenty-one packages in the graph carrying one. Two of those gaps have different causes and both
matter:

  - RustSec does not carry every advisory. `yamux 0.12.1` has GHSA-vxx9-2994-q338, a remote
    panic on a malformed frame, and no RustSec entry at all -- so no configuration of
    cargo-deny could have found it.
  - cargo-deny was also silent on advisories it *does* have. `hickory-proto 0.24.4` is in the
    graph for a configured target, RUSTSEC-2026-0119 was in the database it fetched that same
    minute, the advisory is a plain denial-of-service with no `informational` marker, and it is
    not in the ignore list. Removing an unrelated entry from that list proved the tool still
    reports -- it emitted `error[unmaintained]` and exited 1 -- so it is reporting the
    unmaintained class and not the vulnerability class. Why is not established, and that is
    recorded as its own item rather than guessed at here.

So this asks OSV directly, which aggregates GHSA and RustSec both.

Two things it is careful about, each of which reads as a detail and is not:

  - It reads the *resolved graph* from `cargo metadata --all-features`, not `Cargo.lock`, and
    it takes the whole feature space rather than the default one. Both halves were learned the
    hard way. A raw lock scan reports entries by text; the graph is what cargo can actually
    build. But `cargo tree` with default features could not reach nine of the thirty, and the
    first reading of that was "orphans, ignore them" -- wrong. `ring 0.16.20` comes in behind
    `rcgen`, `jsonwebtoken 9.3.1` behind `alloy-rpc-types-engine`, `h2 0.3.27` behind an old
    `hyper`. Reachable under a feature is reachable, so the wide scope is the honest one and
    matches what `deny.toml` already uses.
  - It compares against a recorded baseline rather than a threshold. The list can only shrink:
    a new advisory fails, and an entry that stops applying fails too, so the record cannot
    quietly outlive what it describes.

It also reads every committed npm lockfile (`package-lock.json`, `bun.lock`). Those are test
and tooling harnesses, not the node, but they are in the tree, OpenSSF Scorecard scores them, and
on 2026-10-16 Scorecard reported seventeen advisories in `staking-async/runtimes/papi-tests`
while this gate, reading Cargo only, said zero. A gate narrower than what is scored is a gate
that reports clean on a tree that is not.

Usage: check-osv-advisories.py [--verbose] [--update-baseline]
Exit 1 on an advisory outside the baseline, on a stale baseline entry, or if the scan cannot
run at all -- a query that fails is not the same as a graph that is clean.
"""

import json
import re
import subprocess
import sys
import urllib.error
import urllib.request
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
BASELINE = Path(__file__).with_name("osv-baseline.json")
OSV_BATCH = "https://api.osv.dev/v1/querybatch"
CHUNK = 500


def lock_roots():
    """Every directory in the tree that carries its own `Cargo.lock`.

    Read from git rather than listed, because a list is what went wrong: until 2026-10-12 this
    audited the root workspace only, while `tools/usdt-bridge` -- a service that runs on the
    fleet -- resolved nine packages with advisories against a lockfile of its own, among
    them `libsecp256k1` and `paste` that the root graph had already closed, and only two of
    which Dependabot reported. The bridge is a workspace member now; a tool that still keeps
    its own lockfile builds its own graph, so it is covered the moment it is committed.
    """
    out = subprocess.run(
        ["git", "ls-files", "--", "Cargo.lock", "*/Cargo.lock"],
        cwd=REPO, capture_output=True, text=True, check=True,
    )
    roots = sorted({(REPO / f).parent for f in out.stdout.split()})
    if REPO not in roots:
        sys.exit("the root Cargo.lock is not tracked -- that is not this workspace")
    return roots


def resolved_packages():
    """(name, version) for everything in every resolved graph, features and all."""
    found = set()
    for root in lock_roots():
        found |= resolved_packages_in(root)
    return sorted(found)


def resolved_packages_in(root):
    out = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--all-features"],
        cwd=root, capture_output=True, text=True, timeout=900,
    )
    if out.returncode != 0:
        sys.exit(f"cargo metadata failed in {root.relative_to(REPO)}:\n{out.stderr[-2000:]}")
    meta = json.loads(out.stdout)
    nodes = (meta.get("resolve") or {}).get("nodes") or []
    if not nodes:
        sys.exit(f"cargo metadata returned no resolve graph in {root.relative_to(REPO)} -- "
                 "nothing was audited")

    found = set()
    for n in nodes:
        pid = n["id"]
        # `registry+https://...#name@version`, or `path+file:///...#name@version`
        if "#" not in pid:
            continue
        tail = pid.rsplit("#", 1)[1]
        if "@" not in tail:
            continue
        name, version = tail.rsplit("@", 1)
        # Workspace members are ours; OSV has nothing to say about them.
        if pid.startswith("path+"):
            continue
        found.add((name, version))
    return found


def npm_lockfiles():
    """Every committed npm or bun lockfile, read from git for the same reason as `lock_roots`."""
    out = subprocess.run(
        ["git", "ls-files", "--", "package-lock.json", "*/package-lock.json", "bun.lock", "*/bun.lock"],
        cwd=REPO, capture_output=True, text=True, check=True,
    )
    return sorted(REPO / f for f in out.stdout.split())


# bun.lock package entries: `"key": ["name@version", ...` where name may be scoped. Local
# `file:`/`workspace:` entries have no registry version and are ours, like path+ crates.
BUN_ENTRY = re.compile(r'^\s*"[^"]+":\s*\["((?:@[^@"/]+/)?[^@"]+)@([^"]+)"', re.M)


def npm_packages_in(lock):
    found = set()
    if lock.name == "package-lock.json":
        for key, meta in (json.loads(lock.read_text()).get("packages") or {}).items():
            if not key or meta.get("link") or "version" not in meta:
                continue
            found.add((key.rsplit("node_modules/", 1)[-1], meta["version"]))
    else:
        for name, version in BUN_ENTRY.findall(lock.read_text()):
            if not re.match(r"^\d", version):
                continue
            found.add((name, version))
    if not found:
        sys.exit(f"{lock.relative_to(REPO)} resolved no packages -- nothing was audited")
    return found


def npm_packages():
    found = set()
    for lock in npm_lockfiles():
        found |= npm_packages_in(lock)
    return sorted(found)


def query_osv(pkgs, ecosystem="crates.io", prefix=""):
    """Ask OSV about every package. Raises rather than returning a hopeful empty set."""
    hits = {}
    for i in range(0, len(pkgs), CHUNK):
        chunk = pkgs[i : i + CHUNK]
        body = json.dumps(
            {"queries": [
                {"package": {"name": n, "ecosystem": ecosystem}, "version": v}
                for n, v in chunk
            ]}
        ).encode()
        req = urllib.request.Request(
            OSV_BATCH, data=body, headers={"Content-Type": "application/json"}
        )
        try:
            res = json.load(urllib.request.urlopen(req, timeout=180))
        except (urllib.error.URLError, TimeoutError, json.JSONDecodeError) as e:
            sys.exit(f"OSV query failed: {e}. A failed query is not a clean graph.")
        for j, r in enumerate(res.get("results", [])):
            vulns = r.get("vulns") or []
            if vulns:
                name, version = chunk[j]
                hits[f"{prefix}{name}@{version}"] = sorted(v["id"] for v in vulns)
    return hits


def main():
    verbose = "--verbose" in sys.argv
    pkgs = resolved_packages()
    if len(pkgs) < 100:
        sys.exit(f"only {len(pkgs)} packages resolved -- that is not this workspace")

    npm = npm_packages()
    hits = query_osv(pkgs)
    hits.update(query_osv(npm, ecosystem="npm", prefix="npm:"))
    baseline = json.loads(BASELINE.read_text()) if BASELINE.is_file() else {}

    if "--update-baseline" in sys.argv:
        BASELINE.write_text(json.dumps(hits, indent=2, sort_keys=True) + "\n")
        print(f"baseline written: {len(hits)} packages, {sum(len(v) for v in hits.values())} advisories")
        return 0

    new = {k: v for k, v in hits.items() if k not in baseline}
    grown = {
        k: sorted(set(v) - set(baseline[k]))
        for k, v in hits.items()
        if k in baseline and set(v) - set(baseline[k])
    }
    gone = [k for k in baseline if k not in hits]

    for k, ids in sorted(new.items()):
        print(f"  NEW   {k}")
        for i in ids:
            print(f"          {i}")
    for k, ids in sorted(grown.items()):
        print(f"  MORE  {k} gained {', '.join(ids)}")
    for k in sorted(gone):
        print(f"  GONE  {k} is clean now -- drop it from osv-baseline.json")

    if new or grown or gone:
        print()
        print("OSV, GHSA ile RustSec'i birleştirir; `cargo-deny` yalnız RustSec'e bakar ve")
        print("2026-09-01'de gördüğü açıkları da raporlamıyordu. Taban listesi yalnız küçülür:")
        print("yeni bir açık da, artık geçerli olmayan bir kayıt da kırmızı verir.")
        return 1

    total = sum(len(v) for v in hits.values())
    print(f"{len(pkgs)} resolved crates and {len(npm)} locked npm packages queried against OSV; "
          f"{len(hits)} carry an advisory, {total} advisories, all in the recorded baseline")
    if verbose:
        for k, ids in sorted(hits.items()):
            print(f"  {k:34s} {', '.join(ids)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
