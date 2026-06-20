# Subkey

Subkey is a commandline utility included with Bizinikiwi. It allows generating and restoring keys for Bizinikiwi based
chains such as PezkuwiChain, Dicle and a growing number of teyrchains and Bizinikiwi based projects.

`pez_subkey` provides a few sub-commands to generate keys, check keys, sign messages, verify messages, etc...

You can see the full list of commands with `pez_subkey --help`. Most commands have additional help available with for
instance `pez_subkey generate --help` for the `generate` command.

## Safety first

`pez_subkey` does not need an internet connection to work. Indeed, for the best security, you should be using `pez_subkey` on a
machine that is **not connected** to the internet.

`pez_subkey` deals with **seeds** and **private keys**. Make sure to use `pez_subkey` in a safe environment (ie. no one looking
over your shoulder) and on a safe computer (ie. no one able to check your command history).

If you save any output of `pez_subkey` into a file, make sure to apply proper permissions and/or delete the file as soon as
possible.

## Usage

The following guide explains *some* of the `pez_subkey` commands. For the full list and the most up to date documentation,
make sure to check the integrated help with `pez_subkey --help`.

### Install with Cargo

You will need to have the Bizinikiwi build dependencies to install Subkey. Use the following two commands to install the
dependencies and Subkey, respectively:

Command:

```bash
# Install only `pez_subkey`, at a specific version of the pez_subkey crate
cargo install --force pez_subkey --git https://github.com/pezkuwichain/pezkuwi-sdk --version <SET VERSION> --locked
# If you run into issues building, you likely are missing deps defined in https://docs.pezkuwichain.io/install/
```

### Run in a container

```bash
# Use `--pull=always` with the `latest` tag, or specify a version in a tag
docker run -it --pull=always docker.io/parity/pez_subkey:latest <command to pez_subkey>
```

### Generate a random account

Generating a new key is as simple as running:

```bash
pez_subkey generate
```

The output looks similar to:

```text
Secret phrase `hotel forest jar hover kite book view eight stuff angle legend defense` is account:
  Secret seed:      0xa05c75731970cc7868a2fb7cb577353cd5b31f62dccced92c441acd8fee0c92d
  Public key (hex): 0xfec70cfbf1977c6965b5af10a4534a6a35d548eb14580594d0bc543286892515
  Account ID:       0xfec70cfbf1977c6965b5af10a4534a6a35d548eb14580594d0bc543286892515
  SS58 Address:     5Hpm9fq3W3dQgwWpAwDS2ZHKAdnk86QRCu7iX4GnmDxycrte
```

---
☠️ DO NT RE-USE ANY OF THE SEEDS AND SECRETS FROM THIS PAGE ☠️.

You can read more about security and risks in [SECURITY.md](./SECURITY.md) and in the [PezkuwiChain
Wiki](https://wiki.network.pezkuwichain.io/docs/learn-account-generation).

---

The output above shows a **secret phrase** (also called **mnemonic phrase**) and the **secret seed** (also called
**Private Key**). Those 2 secrets are the pieces of information you MUST keep safe and secret. All the other information
below can be derived from those secrets.

The output above also shows the **public key** and the **Account ID**. Those are the independent from the network where
you will use the key.

The **SS58 address** (or **Public Address**) of a new account is a representation of the public keys of an account for
a given network (for instance Dicle or PezkuwiChain).

You can read more about the [SS58 format in the Bizinikiwi Docs](https://docs.pezkuwichain.io/reference/address-formats/) and see the list of reserved prefixes in the [SS58 Registry](https://docs.pezkuwichain.io/ss58-registry).

For instance, considering the previous seed `0xa05c75731970cc7868a2fb7cb577353cd5b31f62dccced92c441acd8fee0c92d` the
SS58 addresses are:

- PezkuwiChain: `16m4J167Mptt8UXL8aGSAi7U2FnPpPxZHPrCgMG9KJzVoFqM`
- Dicle: `JLNozAv8QeLSbLFwe2UvWeKKE4yvmDbfGxTuiYkF2BUMx4M`

### Json output

`pez_subkey` can also generate the output as *json*. This is useful for automation.

command:

```bash
pez_subkey generate --output-type json
```

output:

```json
{
  "accountId": "0xfec70cfbf1977c6965b5af10a4534a6a35d548eb14580594d0bc543286892515",
  "publicKey": "0xfec70cfbf1977c6965b5af10a4534a6a35d548eb14580594d0bc543286892515",
  "secretPhrase": "hotel forest jar hover kite book view eight stuff angle legend defense",
  "secretSeed": "0xa05c75731970cc7868a2fb7cb577353cd5b31f62dccced92c441acd8fee0c92d",
  "ss58Address": "5Hpm9fq3W3dQgwWpAwDS2ZHKAdnk86QRCu7iX4GnmDxycrte"
}
```

So if you only want to get the `secretSeed` for instance, you can use:

command:

```bash
pez_subkey generate --output-type json | jq -r .secretSeed
```

output:

```text
0xa05c75731970cc7868a2fb7cb577353cd5b31f62dccced92c441acd8fee0c92d
```

### Additional user-defined password

`pez_subkey` supports an additional user-defined secret that will be appended to the seed. Let's see the following example:

```bash
pez_subkey generate --password extra_secret
```

output:

```text
Secret phrase `soup lyrics media market way crouch elevator put moon useful question wide` is account:
  Secret seed:      0xe7cfd179d6537a676cb94bac3b5c5c9cb1550e846ac4541040d077dfbac2e7fd
  Public key (hex): 0xf6a233c3e1de1a2ae0486100b460b3ce3d7231ddfe9dadabbd35ab968c70905d
  Account ID:       0xf6a233c3e1de1a2ae0486100b460b3ce3d7231ddfe9dadabbd35ab968c70905d
  SS58 Address:     5He5pZpc7AJ8evPuab37vJF6KkFDqq9uDq2WXh877Qw6iaVC
```

Using the `inspect` command (see more details below), we see that knowing only the **secret seed** is no longer
sufficient to recover the account:

```bash
pez_subkey inspect "soup lyrics media market way crouch elevator put moon useful question wide"
```

which recovers the account `5Fe4sqj2K4fRuzEGvToi4KATqZfiDU7TqynjXG6PZE2dxwyh` and not
`5He5pZpc7AJ8evPuab37vJF6KkFDqq9uDq2WXh877Qw6iaVC` as we expected. The additional user-defined **password**
(`extra_secret` in our example) is now required to fully recover the account. Let's inspect the previous mnemonic,
this time passing also the required `password` as shown below:

```bash
pez_subkey inspect --password extra_secret "soup lyrics media market way crouch elevator put moon useful question wide"
```

This time, we properly recovered `5He5pZpc7AJ8evPuab37vJF6KkFDqq9uDq2WXh877Qw6iaVC`.

### Inspecting a key

If you have *some data* about a key, `pez_subkey inspect` will help you discover more information about it.

If you have **secrets** that you would like to verify for instance, you can use:

```bash
pez_subkey inspect < mnemonic | seed >
```

If you have only **public data**, you can see a subset of the information:

```bash
pez_subkey inspect --public < pubkey | address >
```

**NOTE**: While you will be able to recover the secret seed from the mnemonic, the opposite is not possible.

**NOTE**: For obvious reasons, the **secrets** cannot be recovered from passing **public data** such as `pubkey` or
`address` as input.

command:

```bash
pez_subkey inspect 0xa05c75731970cc7868a2fb7cb577353cd5b31f62dccced92c441acd8fee0c92d
```

output:

```text
Secret Key URI `0xa05c75731970cc7868a2fb7cb577353cd5b31f62dccced92c441acd8fee0c92d` is account:
  Secret seed:      0xa05c75731970cc7868a2fb7cb577353cd5b31f62dccced92c441acd8fee0c92d
  Public key (hex): 0xfec70cfbf1977c6965b5af10a4534a6a35d548eb14580594d0bc543286892515
  Account ID:       0xfec70cfbf1977c6965b5af10a4534a6a35d548eb14580594d0bc543286892515
  SS58 Address:     5Hpm9fq3W3dQgwWpAwDS2ZHKAdnk86QRCu7iX4GnmDxycrte
```

### Signing

`pez_subkey` allows using a **secret key** to sign a random message. The signature can then be verified by anyone using your
**public key**:

```bash
echo -n <msg> | pez_subkey sign --suri <seed|mnemonic>
```

example:

```text
MESSAGE=hello
SURI=0xa05c75731970cc7868a2fb7cb577353cd5b31f62dccced92c441acd8fee0c92d
echo -n $MESSAGE | pez_subkey sign --suri $SURI
```

output:

```text
9201af3788ad4f986b800853c79da47155f2e08fde2070d866be4c27ab060466fea0623dc2b51f4392f4c61f25381a62848dd66c5d8217fae3858e469ebd668c
```

**NOTE**: Each run of the `sign` command will yield a different output. While each signature is different, they are all
valid.

### Verifying a signature

Given a message, a signature and an address, `pez_subkey` can verify whether the **message** has been digitally signed by
the holder (or one of the holders) of the **private key** for the given **address**:

```bash
echo -n <msg> | pez_subkey verify <sig> <address>
```

example:

```bash
MESSAGE=hello
URI=0xfec70cfbf1977c6965b5af10a4534a6a35d548eb14580594d0bc543286892515
SIGNATURE=9201af3788ad4f986b800853c79da47155f2e08fde2070d866be4c27ab060466fea0623dc2b51f4392f4c61f25381a62848dd66c5d8217fae3858e469ebd668c
echo -n $MESSAGE | pez_subkey verify $SIGNATURE $URI
```

output:

```text
Signature verifies correctly.
```

A failure looks like:

```text
Error: SignatureInvalid
```

### Using the vanity generator

You can use the included vanity generator to find a seed that provides an address which includes the desired pattern. Be
warned, depending on your hardware this may take a while.

command:

```bash
pez_subkey vanity --network pezkuwi --pattern bob
```

output:

```text
Generating key containing pattern 'bob'
best: 190 == top: 189
Secret Key URI `0x8c9a73097f235b84021a446bc2826a00c690ea0be3e0d81a84931cb4146d6691` is account:
  Secret seed:      0x8c9a73097f235b84021a446bc2826a00c690ea0be3e0d81a84931cb4146d6691
  Public key (hex): 0x1a8b32e95c1f571118ea0b84801264c3c70f823e320d099e5de31b9b1f18f843
  Account ID:       0x1a8b32e95c1f571118ea0b84801264c3c70f823e320d099e5de31b9b1f18f843
  SS58 Address:     1bobYxBPjZWRPbVo35aSwci1u5Zmq8P6J2jpa4kkudBZMqE
```

`Bob` now got a nice address starting with their name: 1**bob**YxBPjZWRPbVo35aSwci1u5Zmq8P6J2jpa4kkudBZMqE.

**Note**: While `Bob`, having a short name (3 chars), got a result rather quickly, it will take much longer for `Alice`
who has a much longer name, thus the chances to generate a random address that contains the chain `alice` will be much
smaller.

## License

License: GPL-3.0-or-later WITH Classpath-exception-2.0
