#!/usr/bin/env python3
"""Refuse a People mainnet spec whose interim state is not seated.

The mainnet preset carries the whole interim state from the first block: the Serok and the
cabinet in `tiki.founding_government`, the house, its Speaker and both halves of the court in
`welati`. Every one of them is a person, so those lists stay empty in the tree until the
wallets are given before launch -- and an empty list builds a perfectly valid spec. A chain
started from it has a Serok and nobody else, and nothing at runtime would say so.

This is the check that says so, at the one moment it matters: when the launch spec is built.

    check-founding-cadre.py <people-plain.json>

`FOUNDING_CADRE_PENDING=1` lets a rehearsal build the spec anyway and prints why it is not
a launch spec. It is never set for the real launch.
"""
import json
import os
import sys


def norm(key):
    # The genesis patch is serde-renamed to camelCase; accept snake_case too, so a rename on
    # either side cannot turn this check into one that finds nothing and passes.
    return key.replace("_", "").lower()


def find(node, name):
    """The first object stored under `name` anywhere in the spec's genesis."""
    if isinstance(node, dict):
        for key, value in node.items():
            if norm(key) == norm(name) and isinstance(value, dict):
                return value
            found = find(value, name)
            if found is not None:
                return found
    elif isinstance(node, list):
        for value in node:
            found = find(value, name)
            if found is not None:
                return found
    return None


def field(obj, name):
    for key, value in (obj or {}).items():
        if norm(key) == norm(name):
            return value
    return None


def main(path):
    with open(path) as f:
        genesis = json.load(f).get("genesis", {})
    tiki, welati = find(genesis, "tiki"), find(genesis, "welati")

    faults = []
    if tiki is None:
        faults.append("no tiki genesis section")
    if welati is None:
        faults.append("no welati genesis section -- the house, Speaker and court are unset")

    founder = field(tiki, "founding_citizen")
    government = field(tiki, "founding_government") or []
    holders = {}
    for who, office in government:
        holders.setdefault(office, []).append(who)

    serok = holders.get("Serok", [])
    if len(serok) != 1:
        faults.append(f"founding_government seats {len(serok)} Serok, expected exactly one")
    elif serok[0] == founder:
        faults.append("the Serok is the founding citizen's wallet; Founder and Serok are separate wallets")
    if not holders.get("SerokWeziran"):
        faults.append("founding_government has no SerokWeziran -- the cabinet is not seated")

    house = field(welati, "founding_parliament") or []
    speaker = field(welati, "founding_speaker")
    if not house:
        faults.append("founding_parliament is empty")
    if not speaker:
        faults.append("founding_speaker is unset")
    elif speaker not in house:
        faults.append("founding_speaker does not sit in founding_parliament")
    if not field(welati, "founding_court_elected"):
        faults.append("founding_court_elected is empty")
    if not field(welati, "founding_court_appointed"):
        faults.append("founding_court_appointed is empty")

    if not faults:
        print(f"  ✓ People Chain: interim state seated -- {len(government)} offices, "
              f"{len(house)} in the house, Speaker set, court on both halves")
        return 0
    for fault in faults:
        print(f"  ✗ People Chain: {fault}")
    if os.environ.get("FOUNDING_CADRE_PENDING") == "1":
        print("  ! FOUNDING_CADRE_PENDING=1: built anyway. This is NOT a launch spec.")
        return 0
    print("  The interim cadre's wallets go into the mainnet preset before launch "
          "(people-pezkuwichain genesis_config_presets.rs).")
    return 1


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    sys.exit(main(sys.argv[1]))
