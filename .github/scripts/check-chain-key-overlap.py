#!/usr/bin/env python3
"""Zagros and Pezkuwichain must not share a genesis key.

The twins share a pallet index map on purpose and `check-twin-runtimes.py` holds them to it.
Genesis is the opposite: an address written into both chains is one key controlling funds on
both, and a testnet is the half of that pair which exists to be broken into. It is also the
half that has to be restartable from nothing -- which it is not, if its validators are the
production validators and restarting means touching their keystores.

Measured on 2026-08-31, before this check existed: 178 addresses appeared in both. The whole
Zagros validator set, its founder, its treasury, four collators, and -- added the same day, in
a commit meant to *fix* the single-key presale -- the live exchange's cold multisig as the
holder of Zagros's PEZ. That last one is why this is a check and not a cleanup: the defect was
reintroduced by someone who had spent the morning removing it.

Two things are deliberately allowed:
  - `mainnet_simulation`, whose entire purpose is to run mainnet's genesis locally, real keys
    and all. A preset that imitates mainnet must be allowed to use mainnet's addresses.
  - Placeholders that are not keys -- the founding-citizen NFT hash is `0x00..01` on both.

Usage: check-chain-key-overlap.py [--verbose]
Exit 1 on any other shared address, or if the scan finds no presets.
"""

import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]

PEZKUWICHAIN = [
    "pezkuwi/runtime/pezkuwichain/src/genesis_config_presets.rs",
    "pezcumulus/teyrchains/runtimes/*/*-pezkuwichain/src/genesis_config_presets.rs",
]
ZAGROS = [
    "pezkuwi/runtime/zagros/src/genesis_config_presets.rs",
    "pezcumulus/teyrchains/runtimes/*/*-zagros/src/genesis_config_presets.rs",
]

HEX = re.compile(r'hex!\("([0-9a-f]{64,66})"\)')
FN = re.compile(r"\nfn (\w+)\(")

# Presets that may hold mainnet's addresses, by the function that builds them.
WAIVED_FNS = {"pezkuwichain_mainnet_simulation_genesis"}

# Values that are not keys. An all-but-one-bit-zero word is a marker, not an account.
def is_placeholder(h: str) -> bool:
    return h.strip("0") in ("", "1", "2")


def collect(patterns):
    """Every hex literal in these files, with the function it sits in."""
    found = {}
    for pattern in patterns:
        for f in sorted(REPO.glob(pattern)):
            text = f.read_text(errors="replace")
            bounds = [(m.start(), m.group(1)) for m in FN.finditer(text)]

            def owner(pos):
                name = "<top level>"
                for start, fn in bounds:
                    if start < pos:
                        name = fn
                    else:
                        break
                return name

            for m in HEX.finditer(text):
                found.setdefault(m.group(1), []).append(
                    (str(f.relative_to(REPO)), owner(m.start()))
                )
    return found


def main():
    verbose = "--verbose" in sys.argv
    mainnet = collect(PEZKUWICHAIN)
    zagros = collect(ZAGROS)

    if not mainnet or not zagros:
        print("  no genesis presets found -- the parser stopped seeing them, which is not")
        print("  the same as there being none")
        return 1

    bad = []
    waived = 0
    for h, sites in sorted(zagros.items()):
        if h not in mainnet or is_placeholder(h):
            continue
        live = [(f, fn) for f, fn in sites if fn not in WAIVED_FNS]
        if not live:
            waived += len(sites)
            continue
        bad.append((h, live))

    for h, sites in bad:
        print(f"  0x{h[:16]}... is in both chains' genesis")
        for f, fn in sites:
            print(f"           {f}  ({fn})")

    if bad:
        print()
        print("Aynı adres iki zincirde: testnette sızan anahtar mainnet fonunun anahtarıdır,")
        print("ve validatörleri mainnet'in olan bir testnet sıfırdan yeniden başlatılamaz.")
        print("Zagros'un kendi cüzdan seti var; oradan bir adres kullan.")
        return 1

    print(f"{len(zagros)} Zagros genesis literals, none shared with Pezkuwichain "
          f"({waived} waived in mainnet_simulation)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
