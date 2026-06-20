# Bridge Teyrchains Pezpallet

The bridge teyrchains pezpallet is a light client for one or several teyrchains of the bridged relay chain.
It serves as a source of finalized teyrchain headers and is used when you need to build a bridge with
a teyrchain.

The pezpallet requires [bridge GRANDPA pezpallet](../grandpa/) to be deployed at the same chain - it is used
to verify storage proofs, generated at the bridged relay chain.

## A Brief Introduction into Teyrchains Finality

You can find detailed information on teyrchains finality in the
[PezkuwiChain-SDK](https://github.com/pezkuwichain/pezkuwi-sdk) repository. This section gives a brief overview of how the
teyrchain finality works and how to build a light client for a teyrchain.

The main thing there is that the teyrchain generates blocks on its own, but it can't achieve finality without
help of its relay chain. Instead, the teyrchain collators create a block and hand it over to the relay chain
validators. Validators validate the block and register the new teyrchain head in the
[`Heads` map](https://github.com/pezkuwichain/pezkuwi-sdk/blob/bc5005217a8c2e7c95b9011c96d7e619879b1200/polkadot/runtime/parachains/src/paras/mod.rs#L683-L686)
of the [`paras`](https://github.com/paritytech/polkadot-sdk/tree/master/polkadot/runtime/parachains/src/paras) pezpallet,
deployed at the relay chain. Keep in mind that this pezpallet, deployed at a relay chain, is **NOT** a bridge pezpallet,
even though the names are similar.

And what the bridge teyrchains pezpallet does, is simply verifying storage proofs of teyrchain heads within that
`Heads` map. It does that using relay chain header, that has been previously imported by the
[bridge GRANDPA pezpallet](../grandpa/). Once the proof is verified, the pezpallet knows that the given teyrchain
header has been finalized by the relay chain. The teyrchain header fields may then be used to verify storage
proofs, coming from the teyrchain. This allows the pezpallet to be used e.g. as a source of finality for the messages
pezpallet.

## Pezpallet Operations

The main entrypoint of the pezpallet is the `submit_teyrchain_heads` call. It has three arguments:

- storage proof of teyrchain heads from the `Heads` map;

- teyrchain identifiers and hashes of their heads from the storage proof;

- the relay block, at which the storage proof has been generated.

The pezpallet may track multiple teyrchains. And the teyrchains may use different primitives - one may use 128-bit block
numbers, other - 32-bit. To avoid extra decode operations, the pezpallet is using relay chain block number to order
teyrchain headers. Any finalized descendant of finalized relay block `RB`, which has teyrchain block `PB` in
its `Heads` map, is guaranteed to have either `PB`, or its descendant. So teyrchain block number grows with relay
block number.

The pezpallet may reject teyrchain head if it already knows better (or the same) head. In addition, pezpallet rejects
heads of untracked teyrchains.

The pezpallet doesn't track anything behind teyrchain heads. So it requires no initialization - it is ready to accept
headers right after deployment.

## Non-Essential Functionality

There may be a special account in every runtime where the bridge teyrchains module is deployed. This
account, named 'module owner', is like a module-level sudo account - he's able to halt and
resume all module operations without requiring runtime upgrade. Calls that are related to this
account are:

- `fn set_owner()`: current module owner may call it to transfer "ownership" to another account;

- `fn set_operating_mode()`: the module owner (or sudo account) may call this function to stop all
  module operations. After this call, all finality proofs will be rejected until further `set_operating_mode` call'.
  This call may be used when something extraordinary happens with the bridge.

If pezpallet owner is not defined, the governance may be used to make those calls.

## Signed Extension to Reject Obsolete Headers

It'd be better for anyone (for chain and for submitters) to reject all transactions that are submitting
already known teyrchain heads to the pezpallet. This way, we leave block space to other useful transactions and
we don't charge concurrent submitters for their honest actions.

To deal with that, we have a [signed extension](./src/call_ext.rs) that may be added to the runtime.
It does exactly what is required - rejects all transactions with already known heads. The submitter
pays nothing for such transactions - they're simply removed from the transaction pool, when the block
is built.

The signed extension, however, is a bit limited - it only works with transactions that provide single
teyrchain head. So it won't work with multiple teyrchain heads transactions. This fits our needs
for [Dicle <> PezkuwiChain bridge](../../docs/pezkuwi-dicle-bridge-overview.md). If you need to deal
with other transaction formats, you may implement similar extension for your runtime.

You may also take a look at the [`generate_bridge_reject_obsolete_headers_and_messages`](../../bin/runtime-common/src/lib.rs)
macro that bundles several similar signed extensions in a single one.

## Teyrchains Finality Relay

We have an offchain actor, who is watching for new teyrchain heads and submits them to the bridged chain.
It is the teyrchains relay - you may look at the [crate level documentation and the code](../../relays/teyrchains/).
