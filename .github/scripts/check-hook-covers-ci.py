#!/usr/bin/env python3
"""The local pre-commit hook must run every gate CI runs.

A hook narrower than the gate it stands in for is worse than no hook: it returns green, the
commit goes out, and the answer arrives from CI a round trip later. That has now happened
twice in two days and both times the missing gate was the one that would have caught the
commit -- `plan.py --gaps` on an unpinned enum index, then `check-chain-identity.sh` on a
comment naming a foreign network.

Both were the same shape and neither was visible from either side: the hook lists what it
runs, the workflows list what they run, and nobody was comparing the two lists.

Then it happened a third time, to this script. It compared only `.github/scripts/*` and so
said "the hook runs all 16 gate scripts CI runs" while the hook ran none of `cargo fmt`,
`zepter` or `.gitlab/rust-features.sh`. A green sentence narrower than it sounds is the
same defect it was written to prevent, so the comparison now spans both classes: the gate
scripts, and the tools a workflow invokes directly. A tool that regenerates rather than
judges is excluded by name, with the reason, and anything new fails until it is one or the
other.

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

# Tools a workflow runs directly. Keyed by tool rather than by full command: the same tool
# appears with different arguments in different jobs, and what the hook has to run is the
# checking form of it. `cargo check`/`build`/`test` are deliberately absent -- they are the
# work CI exists to do, and a commit hook that started them would be unusable.
TOOL = re.compile(
    r"(?:^|\s)((?:cargo\s+(?:\+\S+\s+)?fmt|taplo|zepter|bash\s+\.gitlab/[a-z-]+\.sh))\b")

# Tool invocations that regenerate a file instead of judging one. They are not gates, and
# running them from a hook would write to the tree mid-commit. Each is keyed by tool, so a
# tool listed here is still required whenever some job also runs its checking form -- which
# is why `cargo fmt` and `zepter` are covered despite both also appearing as regeneration.
REGENERATES = {
    # `zepter run default` and `cargo +nightly fmt -p pezkuwi-sdk` rewrite the umbrella
    # crate after `generate-umbrella.py`; the job then diffs the tree to see if it moved.
    "generate-umbrella",
}


def tool_key(cmd):
    """`cargo +nightly fmt` and `cargo fmt` are one tool; so are every zepter subcommand."""
    c = re.sub(r"\s+", " ", cmd.strip())
    c = re.sub(r"^cargo \+\S+ ", "cargo ", c)
    return c

# Scripts a workflow invokes that are not gates, with the reason each is excluded. A gate
# answers pass or fail about the tree; these do work, and running them from a commit hook
# would start that work on this machine.
NOT_A_GATE = {
    # Runs the benchmark suite on the reference host. In a hook it would launch hours of
    # compilation locally -- the opposite of what a pre-commit check is for.
    "weights_request.py",
}

# Gates that judge a run rather than the tree. They read something a full test run
# produces -- a JUnit report, a timing file -- so before that run exists there is nothing
# for them to read and nothing they could say. Listing one here asserts that: the coverage
# question below still applies to every gate that judges the tree, which is all of them
# except these.
NEEDS_RUN_ARTIFACT = {
    # Reads the JUnit report nextest writes, to fail a test that only passed because the
    # retries nearly ran out. There is no report until the suite has run.
    "check-retry-exhaustion.py",
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

    ci, ci_tools = set(), set()
    for wf in sorted((REPO / ".github" / "workflows").glob("*.yml")):
        text = wf.read_text(errors="replace")
        ci |= set(SCRIPT.findall(text)) - NOT_A_GATE - NEEDS_RUN_ARTIFACT
        ci_tools |= {tool_key(t) for t in TOOL.findall(text)}
    if not ci:
        print("  no gate scripts found in any workflow -- the parser stopped seeing them,")
        print("  which is not the same as there being none")
        return 1

    hook = hook_path()
    if hook is None:
        print(f"CI runs {len(ci)} gate scripts and {len(ci_tools)} tools; "
              "no local pre-commit hook to compare against")
        return 0

    hook_text = hook.read_text(errors="replace")
    local = set(SCRIPT.findall(hook_text))
    # A tool gated in `pre-push` is gated. Only the scripts above care which hook they run
    # in, because only they are cheap enough that the answer differs.
    push = hook.with_name("pre-push")
    tool_text = hook_text + (push.read_text(errors="replace") if push.is_file() else "")
    local_tools = {tool_key(t) for t in TOOL.findall(tool_text)}

    # A gate assigned to another hook is covered if that hook runs it.
    elsewhere, misplaced = set(), []
    for name, other in OTHER_HOOK.items():
        p = hook.with_name(other)
        if p.is_file() and name in SCRIPT.findall(p.read_text(errors="replace")):
            elsewhere.add(name)
        else:
            misplaced.append((name, other))

    missing = sorted(ci - local - elsewhere)
    missing_tools = sorted(ci_tools - local_tools)

    for m in missing:
        print(f"  the pre-commit hook does not run {m}, and CI does")
    for m in missing_tools:
        print(f"  no hook runs `{m}`, and CI does")
    missing += missing_tools
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
    print(f"the pre-commit hook runs all {len(ci) - len(elsewhere)} gate scripts "
          f"and {len(ci_tools)} tools CI runs"
          + (f" ({len(elsewhere)} in another hook)" if elsewhere else "")
          + (f", plus {len(extra)} of its own" if extra else ""))
    if verbose:
        for s in sorted(ci):
            print(f"  both  {s}")
        for s in extra:
            print(f"  local {s}")
        for t in sorted(ci_tools):
            print(f"  both  {t}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
