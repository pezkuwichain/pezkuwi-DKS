#!/usr/bin/env python3
"""Turn a weights request into `cmd.py bench` runs.

Two callers, one path. A `workflow_dispatch` passes its four inputs as arguments; a push that
touched `.github/weights-request.yml` passes nothing and the file is read instead. The file
exists because `workflow_dispatch` only works for workflows already on the default branch, so
until a branch merges there is no way to ask for a measurement except by hand on the box --
and a hand-run leaves no record of what was measured, which is exactly how two bridge hubs
were skipped in a re-measurement and nobody found out for a day.

Nothing here is passed to a shell. `cmd.py` is invoked with an argument list, so a value in
the request file is data even if it contains a semicolon.

Usage: weights_request.py [--runtime R] [--pallet P] [--repeat N] [--steps N]
Exit 1 if any run fails, or if the request cannot be read.
"""

import argparse
import subprocess
import sys
from pathlib import Path

import yaml

REPO = Path(__file__).resolve().parents[2]
REQUEST = REPO / ".github" / "weights-request.yml"
CMD = REPO / ".github" / "scripts" / "cmd" / "cmd.py"


def digits(name, value):
    """A number stays a number. A shape check beats a promise in a description."""
    if value in (None, ""):
        return None
    text = str(value)
    if not text.isdigit():
        sys.exit(f"{name} must be digits, got {text!r}")
    return text


def from_file():
    """Read the request file into a list of runs.

    A request is a list so one push can ask for the same pallet on both twins -- measuring one
    and not the other is how they drift, and the twin check does not cover weights.
    """
    if not REQUEST.exists():
        sys.exit(f"no inputs given and {REQUEST.relative_to(REPO)} does not exist")

    doc = yaml.safe_load(REQUEST.read_text()) or {}
    runs = doc.get("runs")
    if not isinstance(runs, list) or not runs:
        sys.exit("the request file must hold a non-empty `runs:` list")

    out = []
    for i, run in enumerate(runs):
        if not isinstance(run, dict):
            sys.exit(f"runs[{i}] is not a mapping")
        unknown = set(run) - {"runtime", "pallet", "repeat", "steps", "why"}
        if unknown:
            sys.exit(f"runs[{i}] has keys this script does not know: {sorted(unknown)}")
        out.append(run)
    return out


def build(run):
    args = []
    for key in ("runtime", "pallet"):
        value = run.get(key)
        if value:
            args += [f"--{'pezpallet' if key == 'pallet' else key}", str(value)]
    for key in ("repeat", "steps"):
        value = digits(key, run.get(key))
        if value:
            args += [f"--{key}", value]
    return args


def main():
    p = argparse.ArgumentParser()
    for name in ("runtime", "pallet", "repeat", "steps"):
        p.add_argument(f"--{name}", default="")
    a = p.parse_args()

    dispatched = {k: v for k, v in vars(a).items() if v}
    runs = [dispatched] if dispatched else from_file()

    failed = []
    for run in runs:
        args = build(run)
        why = run.get("why")
        print(f"\n=== cmd.py bench {' '.join(args)}", flush=True)
        if why:
            print(f"    {why}", flush=True)
        # A list, never a string: the values came from a file somebody pushed.
        result = subprocess.run([sys.executable, str(CMD), "bench", *args])
        if result.returncode != 0:
            failed.append(args)
            print(f"::error::bench failed: {' '.join(args)}")

    if failed:
        # Every run is attempted before failing. A first failure that skipped the rest would
        # leave the tree half-measured, which is worse than not measuring: some files new,
        # some old, and no way to tell which from the diff.
        print(f"\n{len(failed)} of {len(runs)} runs failed")
        return 1
    print(f"\n{len(runs)} runs finished")
    return 0


if __name__ == "__main__":
    sys.exit(main())
