#!/usr/bin/env python3
"""No runtime may bind a `WeightInfo` whose fallback returns zero.

`type WeightInfo = ()` is legitimate: for most pallets the `()` impl carries upstream's
reference weights, which are wrong for this hardware but not dangerous. For some it returns
`Weight::zero()`, and that is a different thing entirely.

Both cases were live here on 2026-08-30 and neither announced itself:

  - `pezpallet_tnpos` -- all seven calls free on both People chains. `join` reads six storage
    items and writes two. The benchmarks existed; the pallet had simply never been added to
    `define_benchmarks!`, so no weights file was generated, so the runtime fell back to `()`,
    and `()` was zero. Each layer looked fine from the one above it.
  - `pezpallet_accumulate_and_forward` -- worse, because `send_native()` is not a call weight.
    `on_initialize` reads it as `meter.try_consume(...)` so the pallet can decline to teleport
    when the block is full. At zero the consume always succeeds: the guard could never refuse.
    A free call costs the caller nothing; that one spent a budget it had not checked.

So this pairs the two facts nothing else pairs: which pallets have a zero fallback, and which
runtimes bind it. Either alone is harmless to look at and useless to check.

Usage: check-zero-weights.py [--verbose]
Exit 1 if any runtime binds a zero fallback, or if the scan finds no pallets at all.
"""

import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]

PALLET_GLOBS = [
    "bizinikiwi/pezframe/*/src/weights.rs",
    "pezcumulus/teyrchains/pezpallets/*/src/weights.rs",
    "pezkuwi/pezpallets/*/src/weights.rs",
]
RUNTIME_GLOBS = ["pezkuwi/runtime/*/src", "pezcumulus/teyrchains/runtimes/*/*/src"]
# The files a runtime spreads its `impl ...::Config` blocks across.
CONFIG_FILES = ("lib.rs", "people.rs", "staking.rs", "xcm_config.rs", "governance.rs")

FALLBACK = re.compile(r"impl WeightInfo for \(\)\s*\{(.*?)\n\}", re.S)
CONFIG = re.compile(r"impl ([a-z_0-9]+)::Config(?:<[^>]*>)? for Runtime\s*\{(.*?)\n\}", re.S)
BINDS_UNIT = re.compile(r"\n\ttype WeightInfo = \(\);")


def zero_fallbacks():
    """Pallet crates whose `impl WeightInfo for ()` returns zero anywhere."""
    found = {}
    for pattern in PALLET_GLOBS:
        for w in REPO.glob(pattern):
            body = FALLBACK.search(w.read_text(errors="replace"))
            if body and "Weight::zero()" in body.group(1):
                found[w.parts[-3]] = w.relative_to(REPO)
    return found


def main():
    verbose = "--verbose" in sys.argv
    zero = zero_fallbacks()

    runtimes = [p for pattern in RUNTIME_GLOBS for p in REPO.glob(pattern)
                if (p / "lib.rs").is_file()]
    if not runtimes:
        print("  no runtimes found -- the parser stopped seeing them, which is not the same")
        print("  as there being none")
        return 1

    checked, bad = 0, []
    for r in runtimes:
        src = "".join((r / f).read_text(errors="replace")
                      for f in CONFIG_FILES if (r / f).is_file())
        for m in CONFIG.finditer(src):
            pallet, body = m.group(1), m.group(2)
            if not BINDS_UNIT.search(body):
                continue
            checked += 1
            crate = pallet.removeprefix("pezpallet_").removeprefix("pezframe_").replace("_", "-")
            if crate in zero:
                bad.append((r.parts[-2], pallet, zero[crate]))
            elif verbose:
                print(f"  ok       {r.parts[-2]}: {pallet} (fallback carries real weights)")

    for runtime, pallet, where in sorted(bad):
        print(f"  {runtime}: `{pallet}` binds `()` and its fallback returns zero")
        print(f"           {where}")
        print("           generate weights for it, or give the fallback a real figure")

    if bad:
        print()
        print("Sıfır ağırlık iki şekilde zarar verir ve ikisi de sessizdir: çağrı bedava olur,")
        print("ya da `try_consume` ile okunan bir bütçe koruması hiç reddedemez. İkisi de")
        print("2026-08-30'da canlıydı -- `pezpallet_tnpos` ve `pezpallet_accumulate_and_forward`.")
        return 1

    print(f"{checked} runtime bindings of `()`, none with a zero fallback "
          f"({len(zero)} pallets have one, no runtime uses them)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
