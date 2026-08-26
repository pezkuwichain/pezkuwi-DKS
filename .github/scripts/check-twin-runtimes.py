#!/usr/bin/env python3
"""Hold the Zagros and Pezkuwichain runtimes to the same pallet index map.

Mainnet is to be relaunched as a mirror of a Zagros that has been validated live. A change
that lands on one twin and is forgotten on the other therefore does not stay on a testnet:
it reaches mainnet as an absence. And an index is not a name -- it is the address a call is
sent to, so the same pallet sitting at two numbers means one encoded call reaches different
code on the two chains.

The compiler cannot see any of this: each runtime builds perfectly on its own.

Differences that exist today are listed below with the reason each is tolerated. The point
of the list is not to bless them; it is to make a NEW difference fail while the known ones
stay visible and countable.
"""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

PAIRS = [
	("relay", "pezkuwi/runtime/{}/src/lib.rs"),
	("people", "pezcumulus/teyrchains/runtimes/people/people-{}/src/lib.rs"),
	("asset-hub", "pezcumulus/teyrchains/runtimes/assets/asset-hub-{}/src/lib.rs"),
	("bridge-hub", "pezcumulus/teyrchains/runtimes/bridge-hubs/bridge-hub-{}/src/lib.rs"),
	("coretime", "pezcumulus/teyrchains/runtimes/coretime/coretime-{}/src/lib.rs"),
]

# (pair, pallet) -> reason. A pallet listed here may differ or be absent on one side.
ACCEPTED = {
	("asset-hub", "Sudo"):
		"Zagros keeps sudo as a testnet; mainnet's Asset Hub must never have one.",
	("asset-hub", "Revive"):
		"Contracts run on the testnet first. Issue #10 is the gate before mainnet gets them.",
	("asset-hub", "AssetsPrecompilesPermit"):
		"Rides with Revive; same gate.",
	("asset-hub", "AhOps"):
		"Migration operations pallet, testnet only.",
	("bridge-hub", "EthereumSystemV2"):
		"Snowbridge v2 is installed on Zagros only. Recorded as an open item: the same three "
		"pallets have to reach bridge-hub-pezkuwichain, at these indices.",
	("bridge-hub", "EthereumInboundQueueV2"): "See EthereumSystemV2.",
	("bridge-hub", "EthereumOutboundQueueV2"): "See EthereumSystemV2.",
	# The two below are not extra pallets on one side -- they are the SAME pallet at two
	# different numbers, which is the dangerous kind. Inherited from upstream's two bridge
	# hubs, never a decision of ours. Left as they are because renumbering a published index
	# is its own decision; recorded so it is settled before any bridge hub opens (issue #4).
	("bridge-hub", "BridgeRelayers"):
		"INDEX DRIFT, inherited: 41 on Zagros, 47 on Pezkuwichain. Settle before launch.",
	("bridge-hub", "MessageQueue"):
		"INDEX DRIFT, inherited: 250 on Zagros, 175 on Pezkuwichain. Settle before launch.",
}

DECL = re.compile(r"^\s*([A-Za-z0-9_]+)\s*:\s*[a-z_:<>0-9]+\s*=\s*(\d+)\s*,", re.M)


def index_map(path: Path) -> dict[str, int]:
	"""Pallet name -> index, read from the macro invocation.

	Anchored on the line that opens the macro, not on the first place the name appears: every
	one of these files mentions `construct_runtime!` in a `recursion_limit` comment near the
	top, and starting there made this gate read a comment block and report zero pallets --
	passing because it was not looking, which is the failure it exists to catch.
	"""
	lines = path.read_text().splitlines()
	# Both spellings are in the tree: `construct_runtime! {` on the relays, and
	# `construct_runtime!(` on the teyrchains.
	start = next(
		(i for i, l in enumerate(lines) if l.strip().startswith("construct_runtime!")
			and l.rstrip().endswith(("{", "("))),
		None,
	)
	if start is None:
		raise SystemExit(f"no construct_runtime! invocation in {path}")
	end = next((i for i in range(start + 1, len(lines)) if lines[i] in ("}", ");")), None)
	if end is None:
		raise SystemExit(f"unterminated construct_runtime! in {path}")
	block = "\n".join(lines[start:end])
	found = {m.group(1): int(m.group(2)) for m in DECL.finditer(block)}
	if not found:
		raise SystemExit(f"parsed zero pallets from {path} -- the gate would pass blind")
	return found


def main() -> int:
	failures, accepted_seen = [], 0
	for pair, template in PAIRS:
		za, pe = ROOT / template.format("zagros"), ROOT / template.format("pezkuwichain")
		if not za.exists() or not pe.exists():
			print(f"skip {pair}: one side missing")
			continue
		a, b = index_map(za), index_map(pe)
		for name in sorted(set(a) | set(b)):
			if a.get(name) == b.get(name):
				continue
			if (pair, name) in ACCEPTED:
				accepted_seen += 1
				continue
			failures.append(
				f"{pair}: {name} is {a.get(name, 'absent')} on Zagros and "
				f"{b.get(name, 'absent')} on Pezkuwichain"
			)
		print(f"{pair}: {len(a)} vs {len(b)} pallets")

	if failures:
		print("\nThe twins have drifted:\n")
		for f in failures:
			print(f"  {f}")
		print(
			"\nMainnet will be a mirror of a validated Zagros, so a change made on one twin "
			"and not the other reaches mainnet as an absence. Apply it to both, or add it to "
			"ACCEPTED in this script with the reason it is deliberate."
		)
		return 1

	print(f"\ntwins agree ({accepted_seen} recorded differences)")
	return 0


if __name__ == "__main__":
	sys.exit(main())
