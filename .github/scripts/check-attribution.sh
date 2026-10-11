#!/usr/bin/env bash
# Parity attribution guard.
#
# GitHub reviewed this tree's license attribution under DMCA ticket #4389053: Parity
# Technologies' copyright notices are retained in the work derived from polkadot-sdk.
# The defect in the notice was a copyright line replaced by ours instead of ours being
# added beside it. pezkuwi-sdk runs the same script on every sync from here, so a change
# that would fail there fails here first.
#
# Usage: check-attribution.sh <base-rev> [<head-rev>]
#
# Fails when:
#   - a file that names Parity Technologies at <base-rev> still exists at <head-rev>
#     but no longer names it (the shape of the defect in the notice: a copyright
#     line replaced, not added to);
#   - NOTICE does not credit Parity Technologies and polkadot-sdk;
#   - the workspace authors, which the vendored subxt crates inherit, do not
#     include Parity Technologies.
set -euo pipefail

base=${1:?usage: check-attribution.sh <base-rev> [<head-rev>]}
head=${2:-HEAD}
fail=0

mapfile -t had < <(git grep -l -I 'Parity Technologies' "$base" -- . | sed "s|^$base:||")
mapfile -t kept < <(git ls-tree -r --name-only "$head" -- "${had[@]}")
lost=()
if ((${#kept[@]})); then
	mapfile -t lost < <(git grep -L -I 'Parity Technologies' "$head" -- "${kept[@]}" | sed "s|^$head:||")
fi
if ((${#lost[@]})); then
	echo "These files named Parity Technologies at $base and no longer do:"
	printf '  %s\n' "${lost[@]}"
	fail=1
fi
echo "${#had[@]} files name Parity Technologies at $base; ${#kept[@]} still exist; ${#lost[@]} lost it."

notice=$(git show "$head:NOTICE")
for want in 'Parity Technologies (UK) Ltd.' 'github.com/paritytech/polkadot-sdk'; do
	if ! grep -qF "$want" <<<"$notice"; then
		echo "NOTICE no longer contains: $want"
		fail=1
	fi
done

if ! git show "$head:Cargo.toml" | sed -n '/^\[workspace.package\]/,/^\[/p' | grep -q 'Parity Technologies'; then
	echo "[workspace.package] authors in Cargo.toml no longer include Parity Technologies"
	fail=1
fi

exit $fail
