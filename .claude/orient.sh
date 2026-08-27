#!/usr/bin/env bash
# Printed at session start AND after every compact -- compact is the one that matters, because
# that is where a session loses what it knew and starts guessing. Nothing here is remembered
# from the last turn; every line is measured now.
#
# This must be able to FAIL LOUDLY. Its predecessor was a one-liner ending in `|| true` with a
# malformed path, so it printed nothing for its whole life and no one could tell the difference
# between "nothing to report" and "never ran".
set -uo pipefail
cd "${CLAUDE_PROJECT_DIR:-/home/myhez/pezkuwi-DKS}" || { echo "ORIENT: no project dir"; exit 0; }

PLAN=/home/myhez/res/plans/PLAN.md

echo "=============================================================================="
echo " THE PLAN IS ONE FILE:  $PLAN"
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
