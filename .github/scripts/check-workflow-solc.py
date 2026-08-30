#!/usr/bin/env python3
"""Every job that builds with `runtime-benchmarks` must install solc first.

`pezpallet-revive-fixtures` compiles Solidity in its build script, and it is pulled in by any
runtime built with the benchmarking feature. Without solc the job dies before it checks
anything -- not with a compile error somebody would read as a finding, but with
`Failed to execute solc`, which reads like infrastructure noise and gets ignored.

Three jobs have now been written without the step and each one was found by running it: the
`workspace` job, `check-runtime-benchmarks` on its first working run, and `weights` on its
first run in the ten days it had existed. The decision each time was the same -- install it,
never `SKIP_PALLET_REVIVE_FIXTURES=1`, because skipping leaves a job reporting success over
code it never built. Three identical fixes is where the rule stops being a habit and becomes
a check.

It reads the workflows rather than a list of job names: a job added tomorrow is covered
without anybody remembering this file exists.

Usage: check-workflow-solc.py [--verbose]
Exit 1 if a job builds with the feature and does not install solc.
"""

import re
import sys
from pathlib import Path

import yaml

REPO = Path(__file__).resolve().parents[2]
WORKFLOWS = REPO / ".github" / "workflows"

# What "builds with the benchmarking feature" looks like in a `run:` block. `cmd.py bench`
# counts: it builds the runtimes itself, which is exactly how the weights job was caught.
NEEDS = re.compile(r"--features[= ]\S*runtime-benchmarks|cmd\.py bench|weights_request\.py")

# What installing it looks like. Both the composite action and a direct install are accepted;
# the point is that solc is there, not which line put it there.
INSTALLS = re.compile(r"install-solidity|solc-select|apt-get install[^\n]*\bsolc\b")


def steps_of(job):
    return job.get("steps") or []


def main():
    verbose = "--verbose" in sys.argv
    failed = False
    checked = 0

    for path in sorted(WORKFLOWS.glob("*.yml")):
        doc = yaml.safe_load(path.read_text()) or {}
        for name, job in (doc.get("jobs") or {}).items():
            steps = steps_of(job)
            # Where in the step list does the first build-with-the-feature appear, and where
            # does the install? Order matters: installing afterwards is the same as not.
            build_at = installs_at = None
            for i, step in enumerate(steps):
                blob = f"{step.get('run', '')} {step.get('uses', '')}"
                if build_at is None and NEEDS.search(blob):
                    build_at = i
                if installs_at is None and INSTALLS.search(blob):
                    installs_at = i

            if build_at is None:
                continue
            checked += 1

            if installs_at is None:
                print(f"  {path.name} :: {name}")
                print("           builds with runtime-benchmarks and never installs solc")
                failed = True
            elif installs_at > build_at:
                print(f"  {path.name} :: {name}")
                print(f"           installs solc at step {installs_at}, after the build at "
                      f"{build_at}")
                failed = True
            elif verbose:
                print(f"  ok       {path.name} :: {name}")

    if checked == 0:
        print("  no job matched the build pattern -- the parser stopped seeing them, which is")
        print("  not the same as there being none")
        return 1
    if failed:
        print()
        print("Bir iş `runtime-benchmarks` ile derliyor ama solc kurmuyor. Atlamak")
        print("(`SKIP_PALLET_REVIVE_FIXTURES=1`) çözüm değil: derlemediği kodun üstüne")
        print("yeşil bildiren bir iş bırakır.")
        return 1
    print(f"{checked} benchmark-building jobs, all install solc first")
    return 0


if __name__ == "__main__":
    sys.exit(main())
