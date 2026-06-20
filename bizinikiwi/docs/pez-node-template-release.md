# Bizinikiwi Node Template Release Process

## This release process has to be run in a github checkout Bizinikiwi directory with your work committed into
`https://github.com/pezkuwichain/pezkuwi-sdk/`, because the build script will check the existence of your current git commit
ID in the remote repository.

Assume you are in root directory of Bizinikiwi. Run:

```bash
cd scripts/ci/ ./pez-node-template-release.sh <output tar.gz file>
```

## Expand the output tar gzipped file and replace files in current Bizinikiwi Node Template by running the following
command.

```bash
# This is where the tar.gz file uncompressed cd bizinikiwi-node-template # rsync with force copying. Note the
slash at the destination directory is important rsync -avh * <destination node-template directory>/ # For dry-running
add `-n` argument # rsync -avhn * <destination node-template directory>/
```

The above command only copies existing files from the source to the destination, but does not delete files/directories
that are removed from the source. So you need to manually check and remove them in the destination.

## There is a `Cargo.toml` file in the root directory. Inside, dependencies are listed form and linked to a certain git
commit in Bizinikiwi remote repository, such as:

```toml
toml pezsp-core = { version = "7.0.0", git = "https://github.com/pezkuwichain/pezkuwi-sdk.git", rev =
"de80d0107336a9c7a2efdc0199015e4d67fcbdb5", default-features = false }
```

e will update each of them to link to the Rust	[crate registry](https://crates.io/). After confirming the versioned
package is published in the crate, the above will become:

```toml
[workspace.dependencies] pezsp-core = { version = "7.0.0", default-features = false }
```

P.S: This step can be automated if we update `pez-node-template-release` package in `scripts/ci/pez-node-template-release`.

## Once the `Cargo.toml` is updated, compile and confirm that the Node Template builds. Then commit the changes to a new
branch in [Bizinikiwi Node Template](https://github.com/bizinikiwi-developer-hub/bizinikiwi-node-template), and make a PR.

> Note that there is a chance the code in Bizinikiwi Node Template works with the linked Bizinikiwi git commit but not
with published packages due to the latest (as yet) unpublished features. In this case, rollback that section of the
Node Template to its previous version to ensure the Node Template builds.

## Once the PR is merged, tag the merged commit in master branch with the version number `vX.Y.Z+A` (e.g. `v3.0.0+1`)
The `X`(major), `Y`(minor), and `Z`(patch) version number should follow Bizinikiwi release version. The last digit is any
significant fixes made in the Bizinikiwi Node Template apart from Bizinikiwi. When the Bizinikiwi version is updated, this
digit is reset to 0.

## Troubleshooting

- Running the script `./pez-node-template-release.sh <output tar.gz file>`, after all tests passed successfully, seeing the
	following error message:

```
thread 'main' panicked at 'Creates output file: Os { code: 2, kind: NotFound, message: "No such file or directory"
}', src/main.rs:250:10 note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

This is likely due to that your output path is not a valid `tar.gz` filename or you don't have write permission to the
destination. Try with a simple output path such as `~/node-tpl.tar.gz`.
