# Pezkuwi

Implementation of a <https://pezkuwichain.io> node in Rust based on the Bizinikiwi framework.

The README provides information about installing the `pezkuwi` binary and developing on the codebase. For more specific
guides, like how to run a validator node, see the [Pezkuwi SDK docs website](https://docs.pezkuwichain.io/).

## Installation

### Using a pre-compiled binary

If you just wish to run a Pezkuwi node without compiling it yourself, you may either:

- run the [latest released binary](https://github.com/pezkuwichain/pezkuwi-sdk/releases/latest) (make sure to also
  download all the `worker` binaries and put them in the same directory as `pezkuwi`), or
- install Pezkuwi from one of our package repositories.

### Debian-based (Debian, Ubuntu)

Currently supports Debian 10 (Buster) and Ubuntu 20.04 (Focal), and derivatives. Run the following
commands as the `root` user.

```bash
# Import the security@pezkuwichain.io GPG key
gpg --recv-keys --keyserver hkps://keys.mailvelope.com 9D4B2B6EB8F97156D19669A9FF0812D491B96798
gpg --export 9D4B2B6EB8F97156D19669A9FF0812D491B96798 > /usr/share/keyrings/pezkuwi.gpg
# Add the Pezkuwi repository and update the package index
echo 'deb [signed-by=/usr/share/keyrings/pezkuwi.gpg] https://releases.pezkuwichain.io/deb release main' > /etc/apt/sources.list.d/pezkuwi.list
apt update
# Install the `pezkuwi-keyring` package - This will ensure the GPG key
# used by APT remains up-to-date
apt install pezkuwi-keyring
# Install pezkuwi
apt install pezkuwi

```

### RPM-based
Currently supports Rocky Linux 10 and Alma Linux 10, and derivatives.

```bash
# Install dnf-plugins-core (This might already be installed)
dnf install dnf-plugins-core
# Add the repository and enable it
dnf config-manager --add-repo https://releases.pezkuwichain.io/rpm/pezkuwi.repo
dnf config-manager --set-enabled pezkuwi
# Install pezkuwi (You may have to confirm the import of the GPG key, which
# should have the following fingerprint: 90BD75EBBB8E95CB3DA6078F94A4029AB4B35DAE)
dnf install pezkuwi

```

Installation from Debian or RPM repository will create a `systemd` service that can be used to run a
Pezkuwi node. This is disabled by default, and can be started by running `systemctl start pezkuwi`
on demand (use `systemctl enable pezkuwi` to make it auto-start after reboot). By default, it will
run as the `pezkuwi` user.  Command-line flags passed to the binary can be customized by editing
`/etc/default/pezkuwi`. This file will not be overwritten on updating Pezkuwi. You may also just
run the node directly from the command-line.

## Building

Since the Pezkuwi node is based on Bizinikiwi, first set up your build environment according to the
[Bizinikiwi installation instructions](https://docs.pezkuwichain.io/install/).

### Install via Cargo

Make sure you have the support software installed from the **Build from Source** section below this
section.

If you want to install Pezkuwi in your PATH, you can do so with:

```bash
cargo install --git https://github.com/pezkuwichain/pezkuwi-sdk --tag <version> pezkuwi --locked
```

### Build from Source

Build the client by cloning this repository and running the following commands from the root
directory of the repo:

```bash
git checkout <latest tagged release>
cargo build --release
```

**Note:** if you want to move the built `pezkuwi` binary somewhere (e.g. into $PATH) you will also
need to move `pezkuwi-execute-worker` and `pezkuwi-prepare-worker`. You can let cargo do all this
for you by running:

```sh
cargo install --path . --locked
```

#### Build from Source with Docker

You can also build from source using [Pezkuwi CI docker image](https://github.com/pezkuwichain/scripts/tree/master/dockerfiles/ci-linux):

```bash
git checkout <latest tagged release>
docker run --rm -it -w /shellhere/pezkuwi \
                    -v $(pwd):/shellhere/pezkuwi \
                    pezkuwichain/ci-linux:production cargo build --release
sudo chown -R $(id -u):$(id -g) target/
```

If you want to reproduce other steps of CI process you can use the following
[guide](https://github.com/pezkuwichain/scripts#gitlab-ci-for-building-docker-images).

## Networks

This repo supports runtimes for PezkuwiChain, Dicle, and Zagros.

### Connect to Pezkuwi Mainnet

Connect to the global Pezkuwi Mainnet network by running:

```bash
../target/release/pezkuwi --chain=pezkuwi
```

You can see your node on [Pezkuwi telemetry](https://telemetry.pezkuwichain.io/#list/0x91b171bb158e2d3848fa23a9f1c25182fb8e20313b2c1eb49219da7a70ce90c3)
(set a custom name with `--name "my custom name"`).

### Connect to the "Dicle" Canary Network

Connect to the global Dicle canary network by running:

```bash
../target/release/pezkuwi --chain=dicle
```

You can see your node on [Dicle telemetry](https://telemetry.polkadot.io/#list/0xb0a8d493285c2df73290dfb7e61f870f17b41801197a149ca93654499ea3dafe)
(set a custom name with `--name "my custom name"`).

### Connect to the Zagros Testnet

Connect to the global Zagros testnet by running:

```bash
../target/release/pezkuwi --chain=zagros
```

You can see your node on [Zagros telemetry](https://telemetry.pezkuwichain.io/#list/0xe143f23803ac50e8f6f8e62695d1ce9e4e1d68aa36c1cd2cfd15340213f3423e)
(set a custom name with `--name "my custom name"`).

### Obtaining HEZ, DCL, or TYR

If you want to do anything on PezkuwiChain, Dicle, or Zagros, then you'll need to get an account and
some HEZ, DCL, or TYR tokens, respectively. Follow the
[instructions](https://wiki.network.pezkuwichain.io/docs/learn-HEZ#obtaining-testnet-tokens) on the Wiki to obtain tokens for
your testnet of choice.

## Hacking on Pezkuwi

If you'd actually like to hack on Pezkuwi, you can grab the source code and build it. Ensure you
have Rust and the support software installed.

Then, grab the Pezkuwi source code:

```bash
git clone https://github.com/pezkuwichain/pezkuwi-sdk.git
cd pezkuwi-sdk
```

Then build the code. You will need to build in release mode (`--release`) to start a network. Only
use debug mode for development (faster compile times for development and testing).

```bash
cargo build
```

You can run the tests if you like:

```bash
cargo test --workspace --profile testnet
# Or run only the tests for specified crated
cargo test -p <crate-name> --profile testnet
```

You can start a development chain with:

```bash
cargo run --bin pezkuwi -- --dev
```

Detailed logs may be shown by running the node with the following environment variables set:

```bash
RUST_LOG=debug RUST_BACKTRACE=1 cargo run --bin pezkuwi -- --dev
```

### Development

You can run a simple single-node development "network" on your machine by running:

```bash
cargo run --bin pezkuwi --release -- --dev
```

You can muck around by heading to <https://js.pezkuwichain.io> and choosing "Local Node" from the
Settings menu.

### Local Two-node Testnet

If you want to see the multi-node consensus algorithm in action locally, then you can create a local
testnet. You'll need two terminals open. In one, run:

```bash
pezkuwi --dev --alice -d /tmp/alice
```

And in the other, run:

```bash
pezkuwi --dev --bob -d /tmp/bob --bootnodes '/ip4/127.0.0.1/tcp/30333/p2p/ALICE_BOOTNODE_ID_HERE'
```

Ensure you replace `ALICE_BOOTNODE_ID_HERE` with the node ID from the output of the first terminal.

### Monitoring

[Setup Prometheus and Grafana](https://docs.pezkuwichain.io/infrastructure/running-a-validator/operational-tasks/general-management/#monitor-your-node).

Once you set this up you can take a look at the [Pezkuwi Grafana dashboards](grafana/README.md)
that we currently maintain.

### Using Docker

[Using Docker](https://github.com/pezkuwichain/pezkuwi-sdk/blob/master/docs/contributor/docker.md)

### Shell Completion

[Shell Completion](https://github.com/pezkuwichain/pezkuwi-sdk/blob/master/pezkuwi/doc/shell-completion.md)

## Contributing

### Contributing Guidelines

[Contribution Guidelines](https://github.com/pezkuwichain/pezkuwi-sdk/blob/master/docs/contributor/CONTRIBUTING.md)

### Contributor Code of Conduct

[Code of Conduct](https://github.com/pezkuwichain/pezkuwi-sdk/blob/master/docs/contributor/CODE_OF_CONDUCT.md)

## License

Pezkuwi is [GPL 3.0 licensed](https://github.com/pezkuwichain/pezkuwi-sdk/blob/master/pezkuwi/LICENSE).
