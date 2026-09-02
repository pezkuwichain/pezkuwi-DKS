#!/usr/bin/env python3
"""Refuse a push that no workflow will pick up.

The workflows here fire on `push` to `main` and on `pull_request`. A branch with no open
pull request therefore gets no CI at all -- pushes land, report nothing, and look fine.

Nothing announces this. `git push` succeeds, GitHub shows no failure because there is no run
to fail, and the branch looks as healthy as one that passed -- so an unbuilt push is
indistinguishable from a green one until somebody goes looking for the run.

The gap opens whenever a pull request merges and work continues on the same branch.

So this asks the question before the push: is this branch one that CI will look at? Yes if it
is a `push` target in some workflow, or if it has an open pull request. No otherwise, and then
the push is refused with the two ways to fix it.

Needs the network and the `gh` CLI to see pull requests. Without either it says so and exits
clean rather than blocking work offline -- a check that cannot run is not a branch that is
covered, and pretending otherwise is the failure this file exists to prevent.

Usage: check-branch-has-ci.py [--verbose]
Exit 1 only when the branch is provably uncovered.
"""

import json
import subprocess
import sys
from pathlib import Path

import yaml

REPO = Path(__file__).resolve().parents[2]


def run(*args, timeout=20, **kw):
    """Every call is bounded. A gate that hangs is worse than one that is wrong: it stops the
    commit it was meant to check, and the reason is invisible."""
    try:
        return subprocess.run(args, cwd=REPO, capture_output=True, text=True,
                              timeout=timeout, **kw)
    except (subprocess.TimeoutExpired, FileNotFoundError):
        return None


def current_branch():
    out = run("git", "rev-parse", "--abbrev-ref", "HEAD")
    if out is None or out.returncode != 0:
        return None
    b = out.stdout.strip()
    return None if b == "HEAD" else b


def push_targets():
    """Branch names some workflow watches for `push`.

    Parsed rather than matched: a regex spanning the lines between `push:` and `branches:`
    needs a nested quantifier, and on these files that backtracks long enough to be
    indistinguishable from a stopped machine when it runs inside a hook.
    """
    names = set()
    for wf in sorted((REPO / ".github" / "workflows").glob("*.yml")):
        try:
            doc = yaml.safe_load(wf.read_text(errors="replace")) or {}
        except yaml.YAMLError:
            continue
        # `on` is the YAML 1.1 boolean True once parsed, which is why it is looked up twice.
        trig = doc.get("on") if "on" in doc else doc.get(True)
        if not isinstance(trig, dict):
            continue
        push = trig.get("push")
        if not isinstance(push, dict):
            continue
        for b in push.get("branches") or []:
            names.add(str(b))
    return names


def has_open_pr(branch):
    """None when the answer cannot be obtained -- not the same as False."""
    out = run("gh", "pr", "list", "--head", branch, "--state", "open", "--json", "number")
    if out is None or out.returncode != 0:
        return None
    try:
        return len(json.loads(out.stdout or "[]")) > 0
    except json.JSONDecodeError:
        return None


def main():
    verbose = "--verbose" in sys.argv
    branch = current_branch()
    if branch is None:
        print("detached HEAD -- nothing to check")
        return 0

    targets = push_targets()
    if not targets:
        print("  no workflow declares a `push:` branch list -- the parser stopped seeing them,")
        print("  which is not the same as there being none")
        return 1

    if branch in targets:
        print(f"'{branch}' is a push target in the workflows; CI runs on every push")
        return 0

    pr = has_open_pr(branch)
    if pr is None:
        print(f"cannot reach GitHub to check for an open pull request on '{branch}';")
        print("if there is none, this push will not be built by anything")
        return 0
    if pr:
        print(f"'{branch}' has an open pull request; CI runs on it")
        return 0

    print(f"  '{branch}' is not a push target and has no open pull request.")
    print("  Nothing will build this push. Open a pull request, or push to a branch CI watches.")
    print()
    print("Bu iki kez oldu ve ikisinde de sessizdi: PR merge edildi, dal açık kaldı, sonraki")
    print("commit'ler hiç derlenmeden gitti. Kırmızı yok, çünkü koşu yok.")
    if verbose:
        print(f"  push hedefleri: {sorted(targets)}")
    return 1


if __name__ == "__main__":
    sys.exit(main())
