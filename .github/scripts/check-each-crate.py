#!/usr/bin/env python3

# A script that checks each workspace crate individually.
# It's relevant to check workspace crates individually because otherwise their compilation problems
# due to feature misconfigurations won't be caught, as exemplified by
# https://github.com/pezkuwichain/pezkuwi-sdk/issues/163 (upstream tracking)
#
# `check-each-crate.py target_group groups_total`
#
# - `target_group`: Integer starting from 1, the group this script should execute.
# - `groups_total`: Integer starting from 1, total number of groups.
# - `disable_forklift`: Boolean, whether to disable forklift or not.

import subprocess, sys

# Get all crates
output = subprocess.check_output(["cargo", "tree", "--locked", "--workspace", "--depth", "0", "--prefix", "none"])

# Convert the output into a proper list
crates = []
for line in output.splitlines():
	if line != b"":
		line = line.decode('utf8').split(" ")
		crate_name = line[0]
		# The crate path is always the last element in the line.
		crate_path = line[len(line) - 1].replace("(", "").replace(")", "")
		crates.append((crate_name, crate_path))

# Make the list unique and sorted
crates = list(set(crates))
crates.sort()

# Skip crates that have their own workspace and can't be checked standalone
# These vendor crates have workspace.dependencies that aren't in the main workspace
SKIP_CRATES = [
	"pezkuwi-subxt",
	"pezkuwi-subxt-codegen",
	"pezkuwi-subxt-core",
	"pezkuwi-subxt-lightclient",
	"pezkuwi-subxt-macro",
	"pezkuwi-subxt-metadata",
	"pezkuwi-subxt-rpcs",
	"pezkuwi-subxt-signer",
	"pezkuwi-subxt-utils-fetchmetadata",
	"pezkuwi-subxt-utils-stripmetadata",
	"pezkuwi-zombienet-cli",
	"pezkuwi-zombienet-configuration",
	"pezkuwi-zombienet-orchestrator",
	"pezkuwi-zombienet-pjs-helper",
	"pezkuwi-zombienet-prom-metrics-parser",
	"pezkuwi-zombienet-provider",
	"pezkuwi-zombienet-sdk",
	"pezkuwi-zombienet-support",
	"pezsp-ss58-registry",
]
crates = [(name, path) for name, path in crates if name not in SKIP_CRATES]
print(f"Crates after skipping vendor workspaces: {len(crates)}", file=sys.stderr)

target_group = int(sys.argv[1]) - 1
groups_total = int(sys.argv[2])
# Forklift is disabled by default since Pezkuwi doesn't have access to Parity's GCP infrastructure
disable_forklift = True

print(f"Target group: {target_group}, Total groups: {groups_total}, Disable forklift: {disable_forklift}", file=sys.stderr)

if len(crates) == 0:
	print("No crates detected!", file=sys.stderr)
	sys.exit(1)

print(f"Total crates: {len(crates)}", file=sys.stderr)

crates_per_group = len(crates) // groups_total

# If this is the last runner, we need to take care of crates
# after the group that we lost because of the integer division.
if target_group + 1 == groups_total:
	overflow_crates = len(crates) % groups_total
else:
	overflow_crates = 0

print(f"Crates per group: {crates_per_group}", file=sys.stderr)

# Check each crate
for i in range(0, crates_per_group + overflow_crates):
	crate = crates_per_group * target_group + i

	print(f"Checking {crates[crate][0]}", file=sys.stderr)

	cmd = ["cargo", "check", "--locked"]

	cmd.insert(0, 'forklift') if not disable_forklift else None

	res = subprocess.run(cmd, cwd = crates[crate][1])

	if res.returncode != 0:
		sys.exit(1)
