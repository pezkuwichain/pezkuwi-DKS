#!/usr/bin/env bash
# Printed at session start AND after every compact -- compact is the one that matters, because
# that is where a session loses what it knew and starts guessing. Nothing here is remembered
# from the last turn; every line is measured now.
#
# This must be able to FAIL LOUDLY. Its predecessor was a one-liner ending in `|| true` with a
# malformed path, so it printed nothing for its whole life and no one could tell the difference
# between "nothing to report" and "never ran".
set -uo pipefail
# No default. This file lives in two worktrees on two different branches, and a hardcoded
# fallback means running it without the variable set measures the *other* tree and reports the
# answer as if it were this one -- which happened, and read as a regression that was not there.
if [ -z "${CLAUDE_PROJECT_DIR:-}" ]; then
	echo "ORIENT: CLAUDE_PROJECT_DIR is unset. Refusing to guess which worktree you mean --"
	echo "        run it as: CLAUDE_PROJECT_DIR=\$PWD bash .claude/orient.sh"
	exit 0
fi
cd "$CLAUDE_PROJECT_DIR" || { echo "ORIENT: $CLAUDE_PROJECT_DIR is not a directory"; exit 0; }

PLAN=/home/myhez/res/plans/PLAN.md

echo "=============================================================================="
echo " TWO HATS, AND YOU WEAR BOTH. Default is the CEO who builds. Put on the Serok"
echo " hat -- the one who ACCEPTS the work -- before saying anything is ready, and the"
echo " moment an anomaly appears. Serok's test: would an independent audit firm pass"
echo " this? If you never put that hat on, the user has to audit you, and that is the"
echo " failure that cost 2026-08-29."
echo ""
echo " AND: never report a belief as a measurement. On that day it happened four times"
echo " -- a grep cut with 'head -5', a git pathspec outside the repo with the error"
echo " swallowed by 2>/dev/null, a grep that missed a backtick, paths searched against"
echo " the wrong repo. Each one produced a confident wrong answer. Before you write"
echo " 'I measured', check the command actually measured what you claim."
echo "=============================================================================="
echo ""
echo "=============================================================================="
echo " THE PLAN IS ONE FILE:  $PLAN"
echo ""
echo " AND A PLAN ENTRY IS A CLAIM THAT EXPIRES. Before working an item, measure the"
echo " REASON it gives, not just the item. On 2026-08-30 six were measured: F0-4's"
echo " \"47,000 tokens of drift\" was 111 and the rest was a tool blind spot; F0-6's"
echo " \"upstream migrated\" was one runtime of six; F0-7 cited a merged PR about"
echo " something else; ten of the (#N) references pointed at unrelated closed PRs;"
echo " T-11's timing rested on mainnet getting another upgrade, which the 2026-08-13"
echo " genesis-reset decision had already ruled out -- and that decision was sitting"
echo " in memory while the stale line was being repeated. Measuring the reason is"
echo " cheap. Acting on a stale one costs a day."
echo " Read it before starting work. Do not open a second plan file. Do not re-derive"
echo " what it already records, and do not re-ask a question section 8 says is decided."
echo "=============================================================================="

if [ ! -f "$PLAN" ]; then
	echo "ORIENT FAILED: the plan is missing. Do not proceed on memory -- say so."
	exit 0
fi

echo
echo "-- design landed? (gate: nonzero exit means a regression) ---------------------"
python3 .github/scripts/plan.py --work 2>&1 | tail -3 || echo "ORIENT: --work did not run"

echo
echo "-- open where work can start now ----------------------------------------------"
echo "   (FAZ 2 onwards is open by definition -- those items wait on a phase, not on me)"
awk -F'|' '/\*\*AÇIK/ {
		id=$2; subj=$3; gsub(/^ +| +$|\*/, "", id); gsub(/^ +| +$|\*/, "", subj)
		printf "  %-6s %s\n", id, substr(subj, 1, 68)
	}' "$PLAN"
echo "   (why each one is open, and who closes it: sections 3-7 and 12)"

echo
echo "-- waiting on Serok, not on me ------------------------------------------------"
sed -n '/^## 8\./,/^## 9\./p' "$PLAN" | grep -oE '^\| \*\*[^|]+\*\* \| [^|]{0,90}' |
	sed -E 's/^\| \*\*([^*]+)\*\* \| /  \1: /'
echo
echo "-- heavy builds do not run on this box. CI, or VPS-CI-DKS. ---------------------"
