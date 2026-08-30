#!/usr/bin/env python3
"""The two tokens have opposite supply rules. Nothing may claim otherwise.

PEZ is an asset on the Asset Hub: five billion, fixed, its rewards pool released on a halving
schedule. HEZ is the native token of the relay, the Asset Hub and People alike, and it
inflates -- `MAX_INFLATION_RATE` caps it at ten per cent a year.

`CLAUDE.md` used to state the rule without naming a token -- "the supply is fixed and halving"
-- and that sentence travelled into eleven comments as the stated reason for not burning HEZ.
The behaviour those comments guard is right; the reason written next to it was false, and a
rule whose reason is false is a rule somebody later measures and discards.

So this checks two things:

  1. The ground truth is still what this script assumes. If HEZ ever stops inflating or PEZ
     stops being fixed, the premise below is wrong and the script says so rather than going on
     enforcing yesterday's arrangement.
  2. No unqualified claim of a fixed or halving supply appears anywhere. A line may say it
     about PEZ, and then it has to name PEZ.

Usage: check-token-claims.py [--verbose]
Exit 1 on a claim that does not name its token, or if the ground truth has moved.
"""

import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]

# Where the ground truth is written. Read, not assumed: this script's whole point is that a
# sentence about a token has to be checked against the token.
INFLATION = REPO / "pezcumulus/teyrchains/runtimes/assets/asset-hub-pezkuwichain/src/staking.rs"
PEZ_SUPPLY = (REPO / "pezcumulus/teyrchains/runtimes/assets/asset-hub-pezkuwichain/src"
              / "genesis_config_presets.rs")
# The halving is not a description, it is a pallet. Checked because the claim that it exists
# was made, and then denied, in the same conversation -- by a grep cut short with `head -5`
# that returned five comment lines and no implementation. A search that is truncated is not a
# measurement, and this file exists so the answer does not depend on one.
HALVING = REPO / "pezcumulus/teyrchains/pezpallets/pez-treasury/src/lib.rs"

# The claim, in the shapes it has actually been written in this tree.
CLAIM = re.compile(r"supply is fixed|fixed and halving|fixed supply|arz(ı)? sabit", re.I)

# What makes a claim qualified: the token it is true of, named close enough to read together.
NAMES_PEZ = re.compile(r"\bPEZ\b|PEZ_TOTAL_SUPPLY|PEZ_REWARDS_POOL")

# Ours only. `bizinikiwi` is upstream and says "fixed supply" about assets in general,
# which is not a claim about either of our tokens and is not ours to edit.
SEARCH = ["CLAUDE.md", "docs", "pezkuwi", "pezcumulus", "pezbridges"]
SKIP = ("/target/", "/.git/")

# How many lines around the claim may carry the token's name. A doc comment often names the
# subject in its first line and makes the claim in its third.
WINDOW = 4


def ground_truth():
    """Confirm HEZ inflates and PEZ is capped, or say the premise has moved."""
    problems = []
    inflation = INFLATION.read_text() if INFLATION.exists() else ""
    if "MAX_INFLATION_RATE" not in inflation:
        problems.append(f"{INFLATION.name}: no MAX_INFLATION_RATE -- does HEZ still inflate?")
    supply = PEZ_SUPPLY.read_text() if PEZ_SUPPLY.exists() else ""
    if "PEZ_TOTAL_SUPPLY == 5_000_000_000" not in supply:
        problems.append(f"{PEZ_SUPPLY.name}: PEZ's total is no longer pinned at five billion")

    halving = HALVING.read_text() if HALVING.exists() else ""
    if "HALVING_PERIOD_MONTHS: u32 = 48" not in halving:
        problems.append("pez-treasury: no 48-month halving period -- the schedule the docs "
                        "describe is not in the pallet")
    wired = [q for q in (REPO / "pezcumulus/teyrchains/runtimes/assets").glob("asset-hub-*")
             if "PezTreasury: pezpallet_pez_treasury" in (q / "src/lib.rs").read_text()]
    if len(wired) != 2:
        problems.append(f"pez-treasury is in {len(wired)} of the two asset hubs -- a halving "
                        f"one twin runs and the other does not is worse than neither")
    return problems


def files():
    for entry in SEARCH:
        path = REPO / entry
        if path.is_file():
            yield path
        elif path.is_dir():
            for f in path.rglob("*"):
                if f.suffix in (".rs", ".md") and not any(s in str(f) for s in SKIP):
                    yield f


def main():
    verbose = "--verbose" in sys.argv

    moved = ground_truth()
    if moved:
        print("  the premise this script rests on has moved:")
        for m in moved:
            print(f"    {m}")
        print("  fix the script's premise before trusting anything below")
        return 1

    bad, qualified = [], 0
    for f in files():
        lines = f.read_text(errors="replace").splitlines()
        for i, line in enumerate(lines):
            if not CLAIM.search(line):
                continue
            near = "\n".join(lines[max(0, i - WINDOW):i + WINDOW + 1])
            if NAMES_PEZ.search(near):
                qualified += 1
                if verbose:
                    print(f"  ok       {f.relative_to(REPO)}:{i + 1}")
            else:
                bad.append((f.relative_to(REPO), i + 1, line.strip()))

    for path, num, text in bad:
        print(f"  {path}:{num}")
        print(f"           {text[:96]}")

    if bad:
        print()
        print("Sabit arz ve halving PEZ'in özelliği; HEZ enflasyonlu ve tavanı %10. Jetonu")
        print("adlandırmayan bir cümle ikisini birbirine karıştırır -- bu satırlar tam olarak")
        print("öyle doğdu ve on bir yere yayıldı. İddiayı ya PEZ'e bağla ya da HEZ için doğru")
        print("gerekçeyi yaz: enflasyonlu bir jetonu yakmak, müsadereyi elinde tutanlara dağıtır.")
        return 1

    print(f"no unqualified supply claim; {qualified} name the token they are true of")
    return 0


if __name__ == "__main__":
    sys.exit(main())
