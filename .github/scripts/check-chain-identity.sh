#!/usr/bin/env bash
# Fail when the tree names a network this project does not operate, or points a node at
# infrastructure this project does not run.
#
# Neither is something the compiler can check. A chain id kept as an alias for one of our own
# chains is a valid string literal and builds fine; a bootnode entry in a chain spec is JSON the
# compiler never reads at all. Both have shipped here, and both were found by a person reading
# the tree rather than by any gate. This is that gate.
#
# TWO DISTINCT THINGS, and conflating them makes the gate useless:
#
#   Identity and connectivity - what this checks.
#     A foreign network name, or a host in a bootnode list / endpoint URL that belongs to someone
#     else. In a chain spec this is not cosmetic: a node started from that spec dials hosts we do
#     not run, and the operator has no idea.
#
#   Attribution - deliberately NOT checked.
#     Author and maintainer fields crediting the upstream project are kept on purpose. This fork
#     does not erase where it came from, and a gate that flags credit lines trains everyone to
#     ignore it.
#
# Usage: check-chain-identity.sh [path...]   (default: the shipped source trees)
# Exit 1 on any finding outside the allowlist.

set -uo pipefail
cd "$(git rev-parse --show-toplevel)" || exit 1

ROOTS=("$@")
# templates/ and docs/ ship too: a template is what someone starts a chain from, and the
# guides are read as instructions. Both were outside this gate until 2026-08-21, which is
# how three `#[frame_construct_runtime]` attributes survived the rebrand there -- the
# compiler caught those, but a chain name in the same files would have gone out unread.
[ ${#ROOTS[@]} -eq 0 ] && ROOTS=(pezcumulus pezkuwi pezbridges bizinikiwi templates docs)

# Networks we do not operate, and the legacy names of chains that were never ours.
FOREIGN_NET='rococo|westend|kusama|statemint|statemine|westmint|rockmine'
# A host only matters when something dials it: inside a multiaddr, or a ws/wss/https endpoint.
FOREIGN_HOST='(/dns[46]?/|wss?://|https://)[^"[:space:],]*(parity-testnet|parity\.io|polkadot\.io)'

# Every exclusion carries its reason. An unexplained one is how a gate goes quietly green.
#
#   staking-async/runtimes/{rc,teyrchain}
#       Reference runtimes kept verbatim so they can be diffed against their source. They are
#       excluded from the build gate for the same reason. Renaming them defeats their purpose.
#   pezkuwi/xcm/src/v5/junction.rs
#       Two comments recording which foreign network's genesis hash these constants used to hold.
#       That is the record of a fixed defect, not branding; removing it removes the warning.
allow() {
  case "$1" in
    *staking-async/runtimes/rc/*) return 0 ;;
    *staking-async/runtimes/teyrchain/*) return 0 ;;
    pezkuwi/xcm/src/v5/junction.rs) return 0 ;;
    # A link to the Parity article the guide is summarising. It cites a source; it names no
    # network we would connect to, and the heritage note says attribution stays.
    docs/sdk/src/guides/enable_elastic_scaling.rs) return 0 ;;
  esac
  return 1
}

report() {
  printf '  %s:%s\n      %s\n' "$1" "$2" \
    "$(printf '%s' "$3" | sed 's/^[[:space:]]*//' | cut -c1-110)"
}

found=0
while IFS=: read -r file line text; do
  [ -z "$file" ] && continue
  allow "$file" && continue
  report "$file" "$line" "$text"; found=1
done < <(grep -rniE "$FOREIGN_NET" \
          --include='*.rs' --include='*.toml' --include='*.json' "${ROOTS[@]}" 2>/dev/null)

while IFS=: read -r file line text; do
  [ -z "$file" ] && continue
  allow "$file" && continue
  report "$file" "$line" "$text"; found=1
done < <(grep -rniE "$FOREIGN_HOST" \
          --include='*.rs' --include='*.toml' --include='*.json' "${ROOTS[@]}" 2>/dev/null)

if [ "$found" -eq 0 ]; then
  echo "chain identity clean: no foreign network named, no foreign host dialled"
else
  echo
  echo "The lines above name a network we do not operate, or point a node at a host we do not run."
  echo "In a chain spec this is not cosmetic: a node started from it will dial someone else."
  echo "Replace them with ours, or add a deliberate exception to allow() WITH ITS REASON."
  echo "Author and maintainer credit lines are not findings - this gate does not look at them."
fi
exit $found
