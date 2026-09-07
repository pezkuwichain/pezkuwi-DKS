#!/usr/bin/env python3
"""Fail when a test only passes because the retries nearly ran out.

`retries = 5` exists to absorb genuine intermittence -- a port collision, a slow runner --
and it earns its keep. But a test that needs five or six attempts is not intermittent, it
is broken, and nextest reports it as `FLAKY 6/6` and counts it among the passes. The run
goes green, the summary says so, and nobody looks again.

Absorbing the failure is right; hiding how close it came is not. This reads the JUnit
report nextest already writes and fails the job when any test consumed more than
`--max-attempts` tries, so the suite keeps its tolerance for flakiness without letting a
broken test hide inside it.
"""
import argparse
import pathlib
import sys
import xml.etree.ElementTree as ET


def attempts_by_test(path):
    """Map each test to how many attempts it took. One run plus one per recorded retry."""
    root = ET.parse(path).getroot()
    out = {}
    for case in root.iter("testcase"):
        # nextest records every failed-then-passed attempt as a `flakyFailure`, and every
        # failed-then-errored one as a `flakyError`.
        retries = len(case.findall("flakyFailure")) + len(case.findall("flakyError"))
        if not retries:
            continue
        name = f"{case.get('classname', '?')} {case.get('name', '?')}"
        out[name] = max(out.get(name, 0), retries + 1)
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("reports", nargs="+", help="JUnit XML files written by nextest")
    ap.add_argument(
        "--max-attempts",
        type=int,
        default=3,
        help="a test needing more than this many attempts fails the job",
    )
    args = ap.parse_args()

    found, missing = {}, []
    for pattern in args.reports:
        for path in sorted(pathlib.Path().glob(pattern)) or [pathlib.Path(pattern)]:
            if not path.exists():
                missing.append(str(path))
                continue
            for name, n in attempts_by_test(path).items():
                found[name] = max(found.get(name, 0), n)

    if missing and not found:
        # A report that was never written is not a pass. The job either did not run the
        # tests or did not write the file, and either way this check saw nothing.
        print(f"no JUnit report found at: {', '.join(missing)}", file=sys.stderr)
        return 1

    if not found:
        print("no test needed a retry")
        return 0

    over = {k: v for k, v in found.items() if v > args.max_attempts}
    for name, n in sorted(found.items(), key=lambda kv: -kv[1]):
        mark = "FAIL" if n > args.max_attempts else "ok  "
        print(f"{mark}  {n} attempts  {name}")

    if over:
        print(
            f"\n{len(over)} test(s) passed only after more than {args.max_attempts} "
            f"attempts. That is not intermittence; fix the test or the thing it depends on.",
            file=sys.stderr,
        )
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
