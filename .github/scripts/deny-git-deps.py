"""
Script to deny Git dependencies in the Cargo workspace. Can be passed one optional argument for the
root folder. If not provided, it will use the cwd.

## Usage
	python3 .github/scripts/deny-git-deps.py pezkuwi-sdk
"""

import os
import sys
import toml

from cargo_workspace import Workspace, DependencyLocation

# Some crates are allowed to have git dependencies until we fix them.
ALLOWED_GIT_DEPS = {
	'subwasmlib': ['pezkuwi-zombienet-sdk-tests'],
}

root = sys.argv[1] if len(sys.argv) > 1 else os.getcwd()
workspace = Workspace.from_path(root)
errors = []

def check_dep(dep, used_by):
	if dep.location != DependencyLocation.GIT:
		return

	if used_by in ALLOWED_GIT_DEPS.get(dep.name, []):
		print(f'🤨 Ignoring git dependency {dep.name} in {used_by}')
	else:
		errors.append(f'🚫 Found git dependency {dep.name} in {used_by}')

# Check the workspace dependencies directly from Cargo.toml to avoid
# cargo-workspace library bug with path+version combinations
cargo_toml_path = os.path.join(root, 'Cargo.toml')
with open(cargo_toml_path, 'r') as f:
	cargo_toml = toml.load(f)

workspace_deps = cargo_toml.get('workspace', {}).get('dependencies', {})
for dep_name, dep_value in workspace_deps.items():
	# Check if it's a git dependency
	if isinstance(dep_value, dict) and 'git' in dep_value:
		if 'workspace' not in ALLOWED_GIT_DEPS.get(dep_name, []):
			errors.append(f'🚫 Found git dependency {dep_name} in workspace')

	# Check if local dependency uses path
	if isinstance(dep_value, dict) and 'path' in dep_value:
		# This is a local dep with path, which is correct
		pass
	elif workspace.crates.find_by_name(dep_name):
		# Local crate exists but no path specified
		errors.append(f'🚫 Workspace must use path to link local dependency {dep_name}')

# And the dependencies of each crate:
for crate in workspace.crates:
	try:
		for dep in crate.dependencies:
			check_dep(dep, crate.name)
	except ValueError as e:
		# cargo-workspace library has a bug with path+version combinations
		# Parse TOML directly for this crate
		crate_toml_path = os.path.join(crate.path, 'Cargo.toml')
		if os.path.exists(crate_toml_path):
			with open(crate_toml_path, 'r') as f:
				crate_toml = toml.load(f)
			for section in ['dependencies', 'dev-dependencies', 'build-dependencies']:
				deps = crate_toml.get(section, {})
				for dep_name, dep_value in deps.items():
					if isinstance(dep_value, dict) and 'git' in dep_value:
						if crate.name not in ALLOWED_GIT_DEPS.get(dep_name, []):
							errors.append(f'🚫 Found git dependency {dep_name} in {crate.name}')

if errors:
	print('❌ Found errors:')
	for error in errors:
		print(error)
	sys.exit(1)
