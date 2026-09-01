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

Two things it is careful about, because the first draft of this measurement got both wrong:

  - It reads the *resolved graph* from `cargo metadata`, not `Cargo.lock`. A lock file keeps
    entries nothing depends on any more -- nine of the thirty a raw lock scan reported were
    orphans, including the `ring 0.16.20` and `jsonwebtoken 9.3.1` that Dependabot keeps
    failing to update. Reporting those would be reporting work that does not exist.
  - It compares against a recorded baseline rather than a threshold. The list can only shrink:
    a new advisory fails, and an entry that stops applying fails too, so the record cannot
    quietly outlive what it describes.

Usage: check-osv-advisories.py [--verbose] [--update-baseline]
Exit 1 on an advisory outside the baseline, on a stale baseline entry, or if the scan cannot
run at all -- a query that fails is not the same as a graph that is clean.
"""

import json
import subprocess
import sys
import urllib.error
import urllib.request
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
BASELINE = Path(__file__).with_name("osv-baseline.json")
OSV_BATCH = "https://api.osv.dev/v1/querybatch"
CHUNK = 500


def resolved_packages():
    """(name, version) for everything in the resolved graph, features and all."""
    out = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--all-features"],
        cwd=REPO, capture_output=True, text=True, timeout=900,
    )
    if out.returncode != 0:
        sys.exit(f"cargo metadata failed:\n{out.stderr[-2000:]}")
    meta = json.loads(out.stdout)
    nodes = (meta.get("resolve") or {}).get("nodes") or []
    if not nodes:
        sys.exit("cargo metadata returned no resolve graph -- nothing was audited")

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
    return sorted(found)


def query_osv(pkgs):
    """Ask OSV about every package. Raises rather than returning a hopeful empty set."""
    hits = {}
    for i in range(0, len(pkgs), CHUNK):
        chunk = pkgs[i : i + CHUNK]
        body = json.dumps(
            {"queries": [
                {"package": {"name": n, "ecosystem": "crates.io"}, "version": v}
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
                hits[f"{name}@{version}"] = sorted(v["id"] for v in vulns)
    return hits


def main():
    verbose = "--verbose" in sys.argv
    pkgs = resolved_packages()
    if len(pkgs) < 100:
        sys.exit(f"only {len(pkgs)} packages resolved -- that is not this workspace")

    hits = query_osv(pkgs)
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
    print(f"{len(pkgs)} resolved packages queried against OSV; "
          f"{len(hits)} carry an advisory, {total} advisories, all in the recorded baseline")
    if verbose:
        for k, ids in sorted(hits.items()):
            print(f"  {k:34s} {', '.join(ids)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
