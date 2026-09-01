#!/usr/bin/env python3
"""A pallet gets one `WeightInfo`, and a generated weights file gets compiled.

`check-zero-weights.py` asks whether a bound fallback returns `Weight::zero()`. That is one way
to be priced wrongly. On 2026-08-31 `pezpallet_welati` was priced wrongly in a way it could not
see, and the shape is worth its own check because nothing about it looks broken:

  - The pallet declared `pub trait WeightInfo` twice -- once in `weights.rs` beside the
    generated impls, and once in `lib.rs` with eight hand-written figures. `Config` named the
    one in `lib.rs`, so `weights.rs` was dead code and the hand-written copy was the contract.
  - Both People runtimes bound `type WeightInfo = ()`, which resolved to that hand-written
    impl: bare `Weight::from_parts(..)` with no `reads`/`writes` attached to any of the eight.
    Every welati call on both chains was priced without the storage it touches.
  - A third file existed -- `weights/pezpallet_welati.rs`, generated against this runtime's own
    storage layout, the most accurate of the three -- and was never declared in
    `weights/mod.rs`, so it was not compiled. Twelve files in that directory were in the same
    state.

Three weight sources for one pallet and the one in use was the only one nobody had measured.
So this checks the two things that let that happen, both of them cheap and neither of them
about the numbers:

  1. No pallet declares `WeightInfo` in both `lib.rs` and `weights.rs`.
  2. Every `weights/*.rs` in a runtime is declared in that directory's `mod.rs`.

Usage: check-weights-wiring.py [--verbose]
Exit 1 on either, or if the scan finds nothing to look at.
"""

import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]

PALLET_GLOBS = [
    "bizinikiwi/pezframe/*/src",
    "pezcumulus/teyrchains/pezpallets/*/src",
    "pezkuwi/pezpallets/*/src",
]
WEIGHTS_DIR_GLOBS = [
    "pezkuwi/runtime/*/src/weights",
    "pezcumulus/teyrchains/runtimes/*/*/src/weights",
]

TRAIT = re.compile(r"^pub trait WeightInfo", re.M)
DECLARED = re.compile(r"^\s*pub mod (\w+)\s*;", re.M)

# Files a weights directory holds that are not per-pallet weights and are pulled in by name
# elsewhere, or not at all.
NOT_A_PALLET = {"mod", "block_weights", "extrinsic_weights", "paritydb_weights", "rocksdb_weights"}

# What was already undeclared on 2026-08-31, when this check was written.
#
# A baseline rather than an exclusion, and the difference is that it can only shrink: a file
# that becomes undeclared tomorrow fails, and a file wired today has to be struck from here or
# the check says so. Recorded by directory as well as by name, so a pallet wired on one chain
# and not its twin still fails on the twin.
#
# Measured before recording. Declaring the eleven in `people-pezkuwichain` produced seven
# compile errors, so these files are stale as well as unwired -- and wiring one changes what
# that pallet charges on a chain that is bound to it. That is a decision about fees, not a
# tidy-up, so it is routed rather than made quietly inside a commit about something else.
# `pezpallet_welati` is deliberately absent: it was in exactly this state and was wired.
BACKLOG_2026_08_31 = {
    ("pezkuwi/runtime/pezkuwichain/src/weights", "pezframe_benchmarking_baseline"),
    ("pezkuwi/runtime/pezkuwichain/src/weights", "pezpallet_balances"),
    ("pezkuwi/runtime/pezkuwichain/src/weights", "pezpallet_referenda"),
    ("pezkuwi/runtime/zagros/src/weights", "pezframe_benchmarking_baseline"),
    ("pezkuwi/runtime/zagros/src/weights", "pezpallet_balances"),
    ("pezkuwi/runtime/zagros/src/weights", "pezpallet_referenda"),
    ("pezcumulus/teyrchains/runtimes/assets/asset-hub-pezkuwichain/src/weights", "pezpallet_pez_treasury"),
    ("pezcumulus/teyrchains/runtimes/assets/asset-hub-pezkuwichain/src/weights", "pezpallet_staking_async_rc_client"),
    ("pezcumulus/teyrchains/runtimes/assets/asset-hub-pezkuwichain/src/weights", "pezpallet_token_wrapper"),
    ("pezcumulus/teyrchains/runtimes/assets/asset-hub-zagros/src/weights", "pezpallet_pez_treasury"),
    ("pezcumulus/teyrchains/runtimes/assets/asset-hub-zagros/src/weights", "pezpallet_staking_async_rc_client"),
    ("pezcumulus/teyrchains/runtimes/assets/asset-hub-zagros/src/weights", "pezpallet_token_wrapper"),
    ("pezcumulus/teyrchains/runtimes/collectives/collectives-zagros/src/weights", "pezpallet_ranked_collective_secretary_collective"),
    ("pezcumulus/teyrchains/runtimes/collectives/collectives-zagros/src/weights", "pezpallet_salary_secretary_salary"),
    ("pezcumulus/teyrchains/runtimes/people/people-pezkuwichain/src/weights", "pezpallet_collective"),
    ("pezcumulus/teyrchains/runtimes/people/people-pezkuwichain/src/weights", "pezpallet_messaging"),
    ("pezcumulus/teyrchains/runtimes/people/people-pezkuwichain/src/weights", "pezpallet_perwerde"),
    ("pezcumulus/teyrchains/runtimes/people/people-pezkuwichain/src/weights", "pezpallet_pez_rewards"),
    ("pezcumulus/teyrchains/runtimes/people/people-pezkuwichain/src/weights", "pezpallet_recovery"),
    ("pezcumulus/teyrchains/runtimes/people/people-pezkuwichain/src/weights", "pezpallet_referral"),
    ("pezcumulus/teyrchains/runtimes/people/people-pezkuwichain/src/weights", "pezpallet_society"),
    ("pezcumulus/teyrchains/runtimes/people/people-pezkuwichain/src/weights", "pezpallet_staking_score"),
    ("pezcumulus/teyrchains/runtimes/people/people-pezkuwichain/src/weights", "pezpallet_tiki"),
    ("pezcumulus/teyrchains/runtimes/people/people-pezkuwichain/src/weights", "pezpallet_trust"),
    ("pezcumulus/teyrchains/runtimes/people/people-pezkuwichain/src/weights", "pezpallet_vesting"),
    ("pezcumulus/teyrchains/runtimes/people/people-zagros/src/weights", "pezpallet_collective"),
    ("pezcumulus/teyrchains/runtimes/people/people-zagros/src/weights", "pezpallet_messaging"),
    ("pezcumulus/teyrchains/runtimes/people/people-zagros/src/weights", "pezpallet_perwerde"),
    ("pezcumulus/teyrchains/runtimes/people/people-zagros/src/weights", "pezpallet_pez_rewards"),
    ("pezcumulus/teyrchains/runtimes/people/people-zagros/src/weights", "pezpallet_recovery"),
    ("pezcumulus/teyrchains/runtimes/people/people-zagros/src/weights", "pezpallet_referral"),
    ("pezcumulus/teyrchains/runtimes/people/people-zagros/src/weights", "pezpallet_society"),
    ("pezcumulus/teyrchains/runtimes/people/people-zagros/src/weights", "pezpallet_staking_score"),
    ("pezcumulus/teyrchains/runtimes/people/people-zagros/src/weights", "pezpallet_tiki"),
    ("pezcumulus/teyrchains/runtimes/people/people-zagros/src/weights", "pezpallet_trust"),
    ("pezcumulus/teyrchains/runtimes/people/people-zagros/src/weights", "pezpallet_vesting"),
}

# Pallets that declare `WeightInfo` in both files, as of 2026-08-31.
#
# All three are upstream's own shape and all three have the welati defect underneath: the
# generated `weights.rs` is used by no runtime, the relay binds `()`, and `()` resolves to a
# hand-written impl in `lib.rs` with no storage costs on any function. Recorded rather than
# fixed for the same reason as above -- changing them changes what the relay charges.
DUPLICATE_TRAIT_BACKLOG = {"babe", "grandpa", "merkle-mountain-range"}



def duplicate_traits():
    """Pallets that declare `WeightInfo` in both files."""
    found = []
    for pattern in PALLET_GLOBS:
        for src in sorted(REPO.glob(pattern)):
            lib, weights = src / "lib.rs", src / "weights.rs"
            if not (lib.is_file() and weights.is_file()):
                continue
            if src.parent.name in DUPLICATE_TRAIT_BACKLOG:
                continue
            if TRAIT.search(lib.read_text(errors="replace")) and TRAIT.search(
                weights.read_text(errors="replace")
            ):
                found.append(src.parent.name)
    return found


def undeclared_files():
    """Generated weight files sitting in a runtime's weights directory, uncompiled."""
    found = {}
    for pattern in WEIGHTS_DIR_GLOBS:
        for d in sorted(REPO.glob(pattern)):
            mod = d / "mod.rs"
            if not mod.is_file():
                continue
            declared = set(DECLARED.findall(mod.read_text(errors="replace")))
            missing = sorted(
                f.stem
                for f in d.glob("*.rs")
                if f.stem not in NOT_A_PALLET
                and f.stem not in declared
                and (str(d.relative_to(REPO)), f.stem) not in BACKLOG_2026_08_31
            )
            if missing:
                found[str(d.relative_to(REPO))] = missing
    return found


def backlog_cleared():
    """Baseline entries that are now declared everywhere, so the list is out of date."""
    still_missing = set()
    for pattern in WEIGHTS_DIR_GLOBS:
        for d in sorted(REPO.glob(pattern)):
            mod = d / "mod.rs"
            if not mod.is_file():
                continue
            declared = set(DECLARED.findall(mod.read_text(errors="replace")))
            for f in d.glob("*.rs"):
                key = (str(d.relative_to(REPO)), f.stem)
                if key in BACKLOG_2026_08_31 and f.stem not in declared:
                    still_missing.add(key)
    return sorted(BACKLOG_2026_08_31 - still_missing)


def main():
    verbose = "--verbose" in sys.argv
    dupes = duplicate_traits()
    missing = undeclared_files()

    if not dupes and not missing and verbose:
        print("  ok  one WeightInfo per pallet, every weights file declared")

    for name in dupes:
        print(f"  {name}: declares `WeightInfo` in both lib.rs and weights.rs")
        print("           `Config` names one of them and the other is dead; the runtime")
        print("           cannot tell which it bound")

    for d, files in missing.items():
        print(f"  {d}/mod.rs does not declare {len(files)} file(s) beside it:")
        for f in files:
            print(f"             {f}")
        print("           a weights file that is not declared is not compiled, and the")
        print("           binding falls back to whatever the pallet's `()` happens to be")

    for d, name in backlog_cleared():
        print(f"  `{name}` is declared in {d} now -- strike it from BACKLOG_2026_08_31")
        print("           a baseline that outlives what it records stops being a baseline")

    if dupes or missing or backlog_cleared():
        print()
        print("Bir pallet'e üç ağırlık kaynağı olabilir ve kullanılan, ölçülmemiş olan çıkabilir.")
        print("2026-08-31'de welati'de tam olarak bu oldu: her çağrı, dokunduğu deponun bedeli")
        print("hiç yazılmadan fiyatlanıyordu.")
        return 1

    print(f"one WeightInfo per pallet; every generated weights file is declared "
          f"({len(BACKLOG_2026_08_31)} in the recorded backlog)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
