# Pezcumulus Overview

This document provides high-level documentation for Pezcumulus.

## Runtime

Each Bizinikiwi blockchain provides a runtime. The runtime is the state transition function of the
blockchain. Pezcumulus provides interfaces and extensions to convert a Bizinikiwi FRAME runtime into a
Teyrchain runtime. Pezkuwi expects each runtime exposes an interface for validating a
Teyrchain's state transition and also provides interfaces for the Teyrchain to send and receive
messages of other Teyrchains.

To convert a Bizinikiwi runtime into a Teyrchain runtime, the following code needs to be added to the
runtime:
```rust
cumulus_pallet_teyrchain_system::register_validate_block!(Block, Executive);
```

This macro call expects the `Block` and `Executive` type. It generates the `validate_block` function
that is expected by Pezkuwi to validate the state transition.

When compiling a runtime that uses Pezcumulus, a WASM binary is generated that contains the *full* code
of the Teyrchain runtime plus the `validate_block` functionality. This binary is required to
register a Teyrchain on the relay chain.

When the Teyrchain validator calls the `validate_block` function, it passes the PoVBlock (See [Block
building](#block-building) for more information) and the parent header of the Teyrchain that is
stored on the relay chain. From the PoVBlock witness data, Pezcumulus reconstructs the partial trie.
This partial trie is used as storage while executing the block. Pezcumulus also redirects all storage
related host functions to use the witness data storage. After the setup is done, Pezcumulus calls
`execute_block` with the transactions and the header stored in the PoVBlock. On success, the new
Teyrchain header is returned as part of the `validate_block` result.

## Node

Teyrchains support light-clients, full nodes, and authority nodes. Authority nodes are called
Collators in the Pezkuwi ecosystem. Pezcumulus provides the consensus implementation for a
Teyrchain and the block production logic.

The Teyrchain consensus will follow the relay chain to get notified about which Teyrchain blocks are
included in the relay-chain and which are finalized. Each block that is built by a Collator is sent
to a validator that is assigned to the particular Teyrchain. Pezcumulus provides the block production
logic that notifies each Collator of the Teyrchain to build a Teyrchain block. The
notification is triggered on a relay-chain block import by the Collator. This means that every
Collator of the Teyrchain can send a block to the Teyrchain validators. For more sophisticated
authoring logic, the Teyrchain will be able to use Aura, BABE, etc. (Not supported at the moment)

A Teyrchain Collator will join the Teyrchain network and the relay-chain network. The Teyrchain
network will be used to gossip Teyrchain blocks and to gossip transactions. Collators will only
gossip blocks to the Teyrchain network that have a high chance of being included in the relay
chain. To prove that a block is probably going to be included, the Collator will send along side
the notification the so-called candidate message. This candidate message is issued by a Teyrchain
validator after approving a block. This proof of possible inclusion prevents spamming other collators
of the network with useless blocks.
The Collator joins the relay-chain network for two reasons. First, the Collator uses it to send the
Teyrchain blocks to the Teyrchain validators. Secondly, the Collator participates as a full-node
of the relay chain to be informed of new relay-chain blocks. This information will be used for the
consensus and the block production logic.

## Block Building

Pezkuwi requires that a Teyrchain block is transmitted in a fixed format. These blocks sent by a
Teyrchain to the Teyrchain validators are called proof-of-validity blocks (PoVBlock). Such a
PoVBlock contains the header and the transactions of the Teyrchain as opaque blobs (`Vec<u8>`). They
are opaque, because Pezkuwi can not and should not support all kinds of possible Teyrchain block
formats. Besides the header and the transactions, it also contains the witness data and the outgoing
messages.

A Teyrchain validator needs to validate a given PoVBlock, but without requiring the full state of
the Teyrchain. To still make it possible to validate the Teyrchain block, the PoVBlock contains the
witness data. The witness data is a proof that is collected while building the block. The proof will
contain all trie nodes that are read during the block production. Pezcumulus uses the witness data to
reconstruct a partial trie and uses this as storage when executing the block.

The outgoing messages are also collected at block production. These are messages from the Teyrchain
the block is built for to other Teyrchains or to the relay chain itself.

## Runtime Upgrade

Every Bizinikiwi blockchain supports runtime upgrades. Runtime upgrades enable a blockchain to update
its state transition function without requiring any client update. Such a runtime upgrade is applied
by a special transaction in a Bizinikiwi runtime. Pezkuwi and Pezcumulus provide support for these
runtime upgrades, but updating a Teyrchain runtime is not as easy as updating a standalone
blockchain runtime. In a standalone blockchain, the special transaction needs to be included in a
block and the runtime is updated.

A Teyrchain will follow the same paradigm, but the relay chain needs to be informed before
the update. Pezcumulus will provide functionality to notify the relay chain about the runtime update. The
update will not be enacted directly; instead it takes `X` relay blocks (a value that is configured
by the relay chain) before the relay chain allows the update to be applied. The first Teyrchain
block that will be included after `X` relay chain blocks needs to apply the upgrade.
If the update is applied before the waiting period is finished, the relay chain will reject the
Teyrchain block for inclusion. The Pezcumulus runtime pezpallet will provide the functionality to
register the runtime upgrade and will also make sure that the update is applied at the correct block.

After updating the Teyrchain runtime, a Teyrchain needs to wait a certain amount of time `Y`
(configured by the relay chain) before another update can be applied.

The WASM blob update not only contains the Teyrchain runtime, but also the `validate_block`
function provided by Pezcumulus. So, updating a Teyrchain runtime on the relay chain involves a
complete update of the validation WASM blob.
