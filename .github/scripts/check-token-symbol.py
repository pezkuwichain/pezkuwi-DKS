#!/usr/bin/env python3
"""One token, one symbol, everywhere it is written down.

HEZ is the native token of the relay, the Asset Hub and People alike. TYR is its subunit --
one HEZ is 10^12 TYR, the way a bitcoin is 10^8 satoshi. So `tokenSymbol` is always HEZ and
`tokenDecimals` is always 12; naming the subunit as the symbol while keeping twelve decimals
tells a wallet the unit subdivides twelve further places and renders one HEZ as "1 TYR".

Three different answers were live at once when this was written (2026-09-12):

  * the release workflow hardcoded `tokenSymbol=TYR` for both teyrchains,
  * the checked-in Zagros reference specs said `ZGR`,
  * the Pezkuwichain ones said `HEZ`, having been standardised by `1dc22d99` -- and the
    Zagros twin was simply left behind, which is the "both twins or neither" rule breaking
    in the quietest possible way.

A symbol is not cosmetic here. It is what a wallet shows a citizen holding their own money.

Usage: check-token-symbol.py [--verbose]
"""

import json
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
WANT_SYMBOL = "HEZ"
WANT_DECIMALS = 12


def main():
    verbose = "--verbose" in sys.argv
    bad, checked = [], 0

    for spec in sorted((REPO / "pezcumulus" / "teyrchains" / "chain-specs").glob("*.json")):
        try:
            doc = json.loads(spec.read_text())
        except Exception as e:
            bad.append(f"  {spec.name}: unreadable ({e})")
            continue
        # Not every file here is a chain spec; one is a bare list of genesis values. A file
        # that is not a spec makes no claim about the symbol and is not this gate's business.
        if not isinstance(doc, dict):
            continue
        props = doc.get("properties") or {}
        sym, dec = props.get("tokenSymbol"), props.get("tokenDecimals")
        if sym is None:
            continue          # a spec that states no symbol claims nothing
        checked += 1
        if sym != WANT_SYMBOL or dec != WANT_DECIMALS:
            bad.append(f"  {spec.name}: tokenSymbol={sym!r} tokenDecimals={dec} "
                       f"(want {WANT_SYMBOL!r}/{WANT_DECIMALS})")
        elif verbose:
            print(f"  ok    {spec.name}")

    # The workflow builds the published specs, and its properties are a command-line string --
    # so the file the chain actually launches from is only as right as this line.
    for wf in sorted((REPO / ".github" / "workflows").glob("*.yml")):
        # Only the lines that build a spec. The first version of this counted its own
        # explanatory comment as a violation, which is a gate reporting on its own prose.
        body = "\n".join(l for l in wf.read_text().splitlines() if not l.lstrip().startswith("#"))
        for m in re.finditer(r"tokenSymbol=([A-Za-z]+)", body):
            checked += 1
            if m.group(1) != WANT_SYMBOL:
                bad.append(f"  {wf.name}: tokenSymbol={m.group(1)} in a chain-spec-builder call")
            elif verbose:
                print(f"  ok    {wf.name}: tokenSymbol={m.group(1)}")

    if bad:
        print("\n".join(bad))
        print()
        print("HEZ tek isimdir; TYR onun alt birimidir (1 HEZ = 10^12 TYR). Sembolü TYR yazıp")
        print("ondalığı 12 bırakmak, bir HEZ'i cüzdanda '1 TYR' diye gösterir.")
        return 1

    print(f"one symbol everywhere: {WANT_SYMBOL}/{WANT_DECIMALS} across {checked} declarations")
    return 0


if __name__ == "__main__":
    sys.exit(main())
