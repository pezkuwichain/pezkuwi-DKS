#!/usr/bin/env python3
"""Hold every hand-built cross-chain call address against the code it is sent to.

`pezpallet-welati` reaches the Asset Hub and the relay by encoding a call *by number*:

    (T::TreasuryPalletIndex::get(), ACTIVATE_DISTRIBUTION_CALL_INDEX).encode()

Both halves of that address are written down on the sending side and nothing checks them
against the receiving side. A pallet renumbered on the Asset Hub, or a `call_index` moved in a
pallet, changes the meaning of a constant nobody edited -- and the compiler cannot see it,
because each runtime builds perfectly on its own. The message still sends, still executes, and
runs whatever now sits at that address.

`check-pallet-indices.py` holds each runtime's map internally consistent and the twins equal to
each other. Neither of those is this: an index can be unique, and identical on both twins, and
still be the wrong number for the sender to be aiming at.

What is checked, for both twins:

  * every `Welati*PalletIndex` constant names the pallet the sender means to reach
  * every `*_CALL_INDEX` constant equals the `#[pezpallet::call_index]` of the call it names

Usage: check-cross-chain-call-addresses.py [--verbose]
Exit 1 on any address that no longer points where it says it does.
"""

import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
WELATI = REPO / "pezcumulus/teyrchains/pezpallets/welati/src/lib.rs"

# Each twin's People runtime, and the Asset Hub its calls are addressed to.
TWINS = [
    (
        "zagros",
        REPO / "pezcumulus/teyrchains/runtimes/people/people-zagros/src/people.rs",
        REPO / "pezcumulus/teyrchains/runtimes/assets/asset-hub-zagros/src/lib.rs",
        REPO / "pezkuwi/runtime/zagros/src/lib.rs",
    ),
    (
        "pezkuwichain",
        REPO / "pezcumulus/teyrchains/runtimes/people/people-pezkuwichain/src/people.rs",
        REPO / "pezcumulus/teyrchains/runtimes/assets/asset-hub-pezkuwichain/src/lib.rs",
        REPO / "pezkuwi/runtime/pezkuwichain/src/lib.rs",
    ),
]

# constant in the People runtime -> (which chain declares it, the pallet name it must land on)
PALLET_ADDRESSES = {
    "WelatiTreasuryPalletIndex": ("asset_hub", "PezTreasury"),
    "WelatiParametersPalletIndex": ("asset_hub", "Parameters"),
    "WelatiAirdropPotPalletIndex": ("asset_hub", "AirdropPot"),
    "WelatiPresalePotPalletIndex": ("asset_hub", "PresalePot"),
    "RelayWhitelistPalletIndex": ("relay", "Whitelist"),
}

# constant in the welati pallet -> (file declaring the call, the call it names)
CALL_ADDRESSES = {
    "ACTIVATE_DISTRIBUTION_CALL_INDEX": (
        "pezcumulus/teyrchains/pezpallets/pez-treasury/src/lib.rs",
        "activate_distribution",
    ),
    "SPEND_FROM_GOVERNMENT_POT_CALL_INDEX": (
        "pezcumulus/teyrchains/pezpallets/pez-treasury/src/lib.rs",
        "spend_from_government_pot",
    ),
    "TREASURY_SPEND_CALL_INDEX": (
        "bizinikiwi/pezframe/treasury/src/lib.rs",
        "spend",
    ),
    "SET_PARAMETER_CALL_INDEX": (
        "bizinikiwi/pezframe/parameters/src/lib.rs",
        "set_parameter",
    ),
    "WHITELIST_CALL_INDEX": (
        "bizinikiwi/pezframe/whitelist/src/lib.rs",
        "whitelist_call",
    ),
}

# `Name: pallet::<Instance2> = 68,` -- the instance syntax is part of the name, and leaving it
# out of the pattern is how two of these were first read as pointing at nothing.
ENTRY = re.compile(r"^\s*(\w+)\s*:\s*[\w:<>]+\s*=\s*(\d+)\s*,", re.M)
# `#[pezpallet::call_index(3)]` ... `pub fn pay_from_incentive_pot(`, with the weight attribute
# and any number of doc comments in between.
CALL = re.compile(
    r"#\[pezpallet::call_index\((\d+)\)\](?:\s*//[^\n]*\n|\s*#\[[^\]]*\]\s*|\s)*?pub fn (\w+)",
    re.S,
)


def indices(path: Path) -> dict:
    """Pallet name -> index, from a runtime's construct_runtime."""
    if not path.exists():
        return {}
    return {m.group(1): int(m.group(2)) for m in ENTRY.finditer(path.read_text())}


def call_indices(path: Path) -> dict:
    """Call name -> call_index, from a pallet."""
    if not path.exists():
        return {}
    return {m.group(2): int(m.group(1)) for m in CALL.finditer(path.read_text())}


def constants(text: str) -> dict:
    """`const Name: u8 = N;` -- the sender's idea of where something lives.

    `pub` is optional and that matters: the call indices inside the welati pallet are private
    to it, so a pattern that required `pub` found none of them and reported five addresses as
    deleted rather than checking them.
    """
    return {
        m.group(1): int(m.group(2))
        for m in re.finditer(r"(?:pub )?const (\w+): u8 = (\d+)", text)
    }


def main() -> int:
    verbose = "--verbose" in sys.argv
    problems = []
    checked = 0

    if not WELATI.exists():
        print(f"::error::{WELATI} not found", file=sys.stderr)
        return 1
    welati_consts = constants(WELATI.read_text())

    # --- the call half: does the number name the call the sender thinks it does? -----------
    for const, (rel, call) in CALL_ADDRESSES.items():
        if const not in welati_consts:
            problems.append(f"{const} is gone from the welati pallet; this check is stale")
            continue
        target = REPO / rel
        found = call_indices(target)
        if not found:
            problems.append(f"{const}: no calls parsed out of {rel}")
            continue
        if call not in found:
            problems.append(f"{const} names `{call}`, which {rel} no longer declares")
            continue
        checked += 1
        if welati_consts[const] != found[call]:
            problems.append(
                f"{const} is {welati_consts[const]} but `{call}` sits at call_index "
                f"{found[call]} in {rel} -- the message would reach a different call"
            )
        elif verbose:
            print(f"  ok  {const} = {welati_consts[const]} -> {call}")

    # --- the pallet half, once per twin ----------------------------------------------------
    for twin, people, asset_hub, relay in TWINS:
        if not people.exists():
            continue
        people_consts = constants(people.read_text())
        maps = {"asset_hub": indices(asset_hub), "relay": indices(relay)}
        for const, (chain, pallet) in PALLET_ADDRESSES.items():
            if const not in people_consts:
                problems.append(f"[{twin}] {const} is gone; this check is stale")
                continue
            table = maps[chain]
            if not table:
                problems.append(f"[{twin}] could not read the {chain} runtime's pallet map")
                continue
            want = people_consts[const]
            at = [name for name, i in table.items() if i == want]
            checked += 1
            if not at:
                problems.append(
                    f"[{twin}] {const} = {want}, and the {chain} runtime has no pallet at "
                    f"{want} -- the message would be rejected or land on nothing"
                )
            elif at[0] != pallet:
                problems.append(
                    f"[{twin}] {const} = {want}, meaning `{pallet}`, but the {chain} runtime "
                    f"has `{at[0]}` at {want} -- the call would run in the wrong pallet"
                )
            elif verbose:
                print(f"  ok  [{twin}] {const} = {want} -> {chain}.{pallet}")

    if problems:
        for p in problems:
            print(f"::error::{p}", file=sys.stderr)
        return 1
    print(f"cross-chain call addresses: {checked} checked, all point where they say")
    return 0


if __name__ == "__main__":
    sys.exit(main())
