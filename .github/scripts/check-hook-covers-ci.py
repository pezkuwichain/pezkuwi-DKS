#!/usr/bin/env python3
"""The local pre-commit hook must run every gate CI runs.

A hook narrower than the gate it stands in for is worse than no hook: it returns green, the
commit goes out, and the answer arrives from CI a round trip later. That has now happened
twice in two days and both times the missing gate was the one that would have caught the
commit -- `plan.py --gaps` on an unpinned enum index, then `check-chain-identity.sh` on a
comment naming a foreign network.

Both were the same shape and neither was visible from either side: the hook lists what it
runs, the workflows list what they run, and nobody was comparing the two lists.

The hook is local and outside the repository -- it is not shared, and a checkout does not
install it. So this check is advisory when the hook is absent (a fresh clone, or CI itself)
and enforcing when it is present. What it will not do is let a hook that exists quietly
cover less than CI.

Usage: check-hook-covers-ci.py [--verbose]
Exit 1 only if the hook exists and misses a gate CI runs.
"""

import re
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
SCRIPT = re.compile(r"\.github/scripts/([a-z0-9_-]+\.(?:py|sh))")

# Scripts a workflow invokes that are not gates, with the reason each is excluded. A gate
# answers pass or fail about the tree; these do work, and running them from a commit hook
# would start that work on this machine.
NOT_A_GATE = {
    # Runs the benchmark suite on the reference host. In a hook it would launch hours of
    # compilation locally -- the opposite of what a pre-commit check is for.
    "weights_request.py",
}

# Gates that belong to a different hook. Each names the one it runs in, and that hook is
# checked for it instead -- so the coverage question is still answered, just against the
# right file. A gate listed here and wired nowhere still fails below.
OTHER_HOOK = {
    # Asks whether anything will build this push. Meaningless before a commit and correct
    # before a push, which is where it runs.
    "check-branch-has-ci.py": "pre-push",
}


def hook_path():
    """The hook lives in the common git dir, which is not `.git` inside a worktree."""
    out = subprocess.run(["git", "rev-parse", "--git-common-dir"],
                         cwd=REPO, capture_output=True, text=True)
    if out.returncode != 0:
        return None
    d = Path(out.stdout.strip())
    if not d.is_absolute():
        d = REPO / d
    p = d / "hooks" / "pre-commit"
    return p if p.is_file() else None


def main():
    verbose = "--verbose" in sys.argv

    ci = set()
    for wf in sorted((REPO / ".github" / "workflows").glob("*.yml")):
        ci |= set(SCRIPT.findall(wf.read_text(errors="replace"))) - NOT_A_GATE
    if not ci:
        print("  no gate scripts found in any workflow -- the parser stopped seeing them,")
        print("  which is not the same as there being none")
        return 1

    hook = hook_path()
    if hook is None:
        print(f"CI runs {len(ci)} gate scripts; no local pre-commit hook to compare against")
        return 0

    local = set(SCRIPT.findall(hook.read_text(errors="replace")))

    # A gate assigned to another hook is covered if that hook runs it.
    elsewhere, misplaced = set(), []
    for name, other in OTHER_HOOK.items():
        p = hook.with_name(other)
        if p.is_file() and name in SCRIPT.findall(p.read_text(errors="replace")):
            elsewhere.add(name)
        else:
            misplaced.append((name, other))

    missing = sorted(ci - local - elsewhere)

    for m in missing:
        print(f"  the pre-commit hook does not run {m}, and CI does")
    for name, other in misplaced:
        print(f"  {name} is assigned to the {other} hook and that hook does not run it")
    missing += [n for n, _ in misplaced]

    if missing:
        print()
        print("Yerel kapı, yerine geçtiği kapıdan dar. İki günde iki kez oldu ve iki seferinde")
        print("de eksik olan kapı, commit'i yakalayacak olan kapıydı.")
        print(f"Hook: {hook}")
        return 1

    extra = sorted(local - ci)
    print(f"the pre-commit hook runs all {len(ci) - len(elsewhere)} gate scripts CI runs"
          + (f" ({len(elsewhere)} in another hook)" if elsewhere else "")
          + (f", plus {len(extra)} of its own" if extra else ""))
    if verbose:
        for s in sorted(ci):
            print(f"  both  {s}")
        for s in extra:
            print(f"  local {s}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
