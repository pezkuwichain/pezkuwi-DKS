#!/usr/bin/env python3
"""No source file may name an upstream crate this fork has renamed.

The compiler catches a wrong crate name in code and says nothing about one in a doc
comment: `///` examples compile only under `cargo test --doc`, which runs in the heavy
suite rather than the pull-request gate. So a stale name can sit in a doc example for
weeks and surface as a release blocker -- `staging_xcm_builder` did exactly that, in a
crate whose own manifest reads `pezstaging-xcm-builder`.

`rebrand_gate.sh` does not cover this and should not: it looks for chain identities --
rococo, westend, kusama -- because its question is which network a node would talk to.
This one's question is different: does a name in this tree still refer to the crate
upstream published rather than the one we build?

The list is derived, not written. Every workspace manifest is read, every `pez`-prefixed
package name is stripped back to what upstream calls it, and the tree is searched for that
original. A crate renamed tomorrow is covered without anybody remembering to add it.

`vendor/` is excluded: those are upstream's own sources, vendored unmodified, and their
examples correctly name upstream's crates.

Usage: check-crate-names-renamed.py [--verbose]
Exit 1 on any stale reference, or if no renamed crates are found at all.
"""

import re
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]

# `use foo_bar::`, `foo_bar::Thing`, `extern crate foo_bar`. Underscore form only: the
# hyphenated form is the package name and appears legitimately in manifests when declaring
# the dependency with a `package = ` rename.
REF = re.compile(r"\b(?<!pez)([a-z][a-z0-9_]{2,})::")

SKIP_DIRS = (
    "vendor/",  # upstream's own sources, vendored unmodified
    "target/",
    ".git/",
    # Separate workspaces with their own manifests and their own dependencies. They pull
    # `sp-core` and friends from crates.io by their published names, so a reference there is
    # correct -- and this crate's `cargo metadata` cannot see it to know that.
    "tools/usdt-bridge/",
)


def renamed_crates():
    """upstream name -> our name, read from the manifests rather than a list.

    A `pez` prefix is not proof of a rename. `frame-metadata` is a crate we depend on from
    crates.io *and* the name a workspace crate of ours strips to, so a reference to
    `frame_metadata::` is correct and naming it a defect would be nineteen false reports --
    enough to make the gate unreadable on its first run. Anything still resolvable as a real
    dependency is therefore excluded: if cargo can find it, the reference is to it.
    """
    import json

    out = subprocess.run(
        ["cargo", "metadata", "--format-version", "1"],
        cwd=REPO, capture_output=True, text=True, timeout=600,
    )
    if out.returncode != 0:
        sys.exit(f"cargo metadata failed:\n{out.stderr[-800:]}")
    meta = json.loads(out.stdout)

    # Everything cargo resolves, ours and external alike.
    all_names = {p["name"] for p in meta["packages"]}

    pairs = {}
    for p in meta["packages"]:
        name = p["name"]
        if not name.startswith("pez") or name == "pez":
            continue
        upstream = name[3:].lstrip("-")
        if len(upstream) < 4 or upstream in all_names:
            continue
        pairs[upstream.replace("-", "_")] = name.replace("-", "_")
    return pairs


def main():
    verbose = "--verbose" in sys.argv
    pairs = renamed_crates()
    if len(pairs) < 20:
        sys.exit(f"only {len(pairs)} renamed crates found -- that is not this workspace")

    hits = []
    for f in REPO.rglob("*.rs"):
        rel = str(f.relative_to(REPO))
        if any(rel.startswith(d) or f"/{d}" in rel for d in SKIP_DIRS):
            continue
        try:
            text = f.read_text(errors="replace")
        except OSError:
            continue
        for i, line in enumerate(text.splitlines(), 1):
            for m in REF.finditer(line):
                up = m.group(1)
                if up in pairs:
                    hits.append((rel, i, up, pairs[up], line.strip()[:90]))

    for rel, i, up, ours, line in hits:
        print(f"  {rel}:{i}")
        print(f"      {up} -> {ours}")
        print(f"      {line}")

    if hits:
        print()
        print("Bir crate adı yeniden adlandırıldığında derleyici koddaki kullanımı yakalar,")
        print("doc örneğindekini yakalamaz -- `cargo test --doc` ağır suitte koşar. Orada")
        print("kalan eski ad, haftalar sonra lansmanı bloke eden bir kırmızı olarak çıkar.")
        return 1

    print(f"{len(pairs)} renamed crates; no source file names an upstream original")
    if verbose:
        for up, ours in sorted(pairs.items())[:8]:
            print(f"  {up} -> {ours}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
