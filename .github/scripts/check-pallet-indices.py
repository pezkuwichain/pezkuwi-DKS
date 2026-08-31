#!/usr/bin/env python3
"""No runtime may give two pallets the same index, and the twins must agree on the map.

`construct_runtime!` already refuses a duplicate -- "Pezpallet indices are conflicting" -- so
this proves nothing the compiler cannot. What it changes is when you find out. A runtime build
does not run on the development box (rocksdb, wasm, an hour of CPU), so the compiler's answer
arrives from CI, and on 2026-08-31 that cost a full round trip: the airdrop pot was put at 66
on both hubs without counting, and `pezpallet_revive` already held 66 on the Zagros hub. Five
jobs went red and reported the cascade -- every type `construct_runtime!` generates went
missing at once -- rather than the one line underneath.

The twin half is the same argument pointed sideways. An index that is free on one hub and
taken on the other is not a free index: the twins keep one map, so a pallet can only take a
number both hubs can give it. Counting one hub is how 66 looked free.

Usage: check-pallet-indices.py [--verbose]
Exit 1 on a duplicate within a runtime, or if no runtimes are found.
"""

import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
RUNTIME_GLOBS = ["pezkuwi/runtime/*/src/lib.rs", "pezcumulus/teyrchains/runtimes/*/*/src/lib.rs"]

# `Name: pallet_path = 12,` inside construct_runtime!. The path may carry an instance
# (`pezpallet_treasury::<Instance2>`) or a module path (`pezpallet_assets_precompiles::permit::pezpallet`).
ENTRY = re.compile(r"^\s*(\w+)\s*:\s*[\w:<>]+\s*=\s*(\d+)\s*,", re.M)
# The same line without an index, to tell "numbered by order" from "parser went blind".
ANY_ENTRY = re.compile(r"^\s*\w+\s*:\s*[\w:<>]+\s*(?:=\s*\d+\s*)?,", re.M)


# The macro's own line, at column zero. Anchored, because both relays open with a *comment*
# about `construct_runtime!` a thousand lines above the macro, and a plain substring search
# lands on the comment, ends at the next `\n}` -- some unrelated function -- and reports zero
# pallets. Zero from a parser reads exactly like zero from a clean runtime.
MACRO = re.compile(r"^construct_runtime!\s*[{(]", re.M)


def runtime_map(lib: Path):
    """Pallet name -> index for one runtime, read from its construct_runtime! block."""
    text = lib.read_text(errors="replace")
    m = MACRO.search(text)
    if not m:
        return None
    # The macro body ends at the first line that closes at column zero.
    end = text.find("\n}", m.end())
    body = text[m.end() : end if end != -1 else len(text)]
    found = {name: int(idx) for name, idx in ENTRY.findall(body)}
    if found:
        return found
    # No explicit indices. Two very different reasons, and they must not be conflated: a
    # runtime may number its pallets by declaration order (`test-runtime` does), which is
    # legitimate and has nothing for this check to hold; or the syntax moved and the parser
    # went blind, which returns the same empty map and passes every check below. Telling them
    # apart needs a second question -- did any pallet line parse at all.
    if ANY_ENTRY.search(body):
        return {}
    sys.exit(f"{lib.relative_to(REPO)}: construct_runtime! found but not one pallet line "
             f"parsed out of it -- the syntax moved, and an empty map is a silent pass")


def main():
    verbose = "--verbose" in sys.argv
    runtimes = {}
    for pattern in RUNTIME_GLOBS:
        for lib in sorted(REPO.glob(pattern)):
            m = runtime_map(lib)
            if m:
                runtimes[lib.parts[-3]] = m
            elif m == {} and verbose:
                print(f"  --  {lib.parts[-3]}: pallets numbered by declaration order")

    if not runtimes:
        print("  no construct_runtime! blocks found -- the parser stopped seeing them, which")
        print("  is not the same as there being none")
        return 1

    bad = False
    for runtime, pallets in sorted(runtimes.items()):
        by_index = {}
        for name, idx in pallets.items():
            by_index.setdefault(idx, []).append(name)
        for idx, names in sorted(by_index.items()):
            if len(names) > 1:
                bad = True
                print(f"  {runtime}: index {idx} is held by {' and '.join(sorted(names))}")
        if verbose and not bad:
            print(f"  ok  {runtime}: {len(pallets)} pallets, no index held twice")

    # Free indices common to the two hubs, so the next pallet can be placed by reading rather
    # than by guessing and waiting for CI.
    hubs = {k: v for k, v in runtimes.items() if k.startswith("asset-hub-")}
    if verbose and len(hubs) >= 2:
        taken = set().union(*(set(v.values()) for v in hubs.values()))
        free = [i for i in range(60, 100) if i not in taken]
        print(f"  free on every asset hub (60-99): {free[:12]}")

    if bad:
        print()
        print("Aynı indekste iki pallet: `construct_runtime!` genişleyemez ve ürettiği HER tip")
        print("birden kaybolur, yani CI beş işte artçıyı raporlar, altındaki tek satırı değil.")
        return 1

    total = sum(len(v) for v in runtimes.values())
    print(f"{len(runtimes)} runtimes, {total} pallet indices, none held twice")
    return 0


if __name__ == "__main__":
    sys.exit(main())
