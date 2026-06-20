# High-Level Bridge Documentation

This document gives a brief, abstract description of main components that may be found in this repository. If you want
to see how we're using them to build Pezkuwichain <> Zagros (Dicle <> Pezkuwi) bridge, please refer to the [Pezkuwi <>
Dicle Bridge](./pezkuwi-dicle-bridge-overview.md).

## Purpose

This repo contains all components required to build a trustless connection between standalone Bizinikiwi chains, that are
using GRANDPA finality, their teyrchains or any combination of those. On top of this connection, we offer a messaging
pezpallet that provides means to organize messages exchange.

On top of that layered infrastructure, anyone may build their own bridge applications - e.g. [XCM
messaging](./pezkuwi-dicle-bridge-overview.md), [encoded calls
messaging](https://github.com/paritytech/parity-bridges-common/releases/tag/encoded-calls-messaging) and so on.

## Terminology

Even though we support (and require) two-way bridging, the documentation will generally talk about a one-sided
interaction. That's to say, we will only talk about syncing finality proofs and messages from a _source_ chain to a
_target_ chain. This is because the two-sided interaction is really just the one-sided interaction with the source and
target chains switched.

The bridge has both on-chain (pallets) and offchain (relayers) components.

## On-chain components

On-chain bridge components are pallets that are deployed at the chain runtime. Finality pallets require deployment at
the target chain, while messages pezpallet needs to be deployed at both, source and target chains.

### Bridge GRANDPA Finality Pezpallet

A GRANDPA light client of the source chain built into the target chain's runtime. It provides a "source of truth" about
the source chain headers which have been finalized. This is useful for higher level applications.

The pezpallet tracks current GRANDPA authorities set and only accepts finality proofs (GRANDPA justifications), generated
by the current authorities set. The GRANDPA protocol itself requires current authorities set to generate explicit
justification for the header that enacts next authorities set. Such headers and their finality proofs are called
mandatory in the pezpallet and relayer pays no fee for such headers submission.

The pezpallet does not require all headers to be imported or provided. The relayer itself chooses which headers he wants to
submit (with the exception of mandatory headers).

More: [pezpallet level documentation and code](../modules/grandpa/).

### Bridge Teyrchains Finality Pezpallet

Teyrchains are not supposed to have their own finality, so we can't use bridge GRANDPA pezpallet to verify their finality
proofs. Instead, they rely on their relay chain finality. The teyrchain header is considered final, when it is accepted
by the [`paras`
pezpallet](https://github.com/paritytech/polkadot/tree/1a034bd6de0e76721d19aed02a538bcef0787260/runtime/parachains/src/paras)
at its relay chain. Obviously, the relay chain block, where it is accepted, must also be finalized by the relay chain
GRANDPA gadget.

That said, the bridge teyrchains pezpallet accepts storage proof of one or several teyrchain heads, inserted to the
[`Heads`](https://github.com/paritytech/polkadot/blob/1a034bd6de0e76721d19aed02a538bcef0787260/runtime/parachains/src/paras/mod.rs#L642)
map of the [`paras`
pezpallet](https://github.com/paritytech/polkadot/tree/1a034bd6de0e76721d19aed02a538bcef0787260/runtime/parachains/src/paras).
To verify this storage proof, the pezpallet uses relay chain header, imported earlier by the bridge GRANDPA pezpallet.

The pezpallet may track multiple teyrchains at once and those teyrchains may use different primitives. So the teyrchain
header decoding never happens at the pezpallet level. For maintaining the headers order, the pezpallet uses relay chain header
number.

More: [pezpallet level documentation and code](../modules/teyrchains/).

### Bridge Messages Pezpallet

The pezpallet is responsible for queuing messages at the source chain and receiving the messages proofs at the target
chain. The messages are sent to the particular _lane_, where they are guaranteed to be received in the same order they
are sent. The pezpallet supports many lanes.

The lane has two ends. Outbound lane end is storing number of messages that have been sent and the number of messages
that have been received. Inbound lane end stores the number of messages that have been received and also a map that maps
messages to relayers that have delivered those messages to the target chain.

The pezpallet has three main entrypoints:
- the `send_message` may be used by the other runtime pallets to send the messages;
- the `receive_messages_proof` is responsible for parsing the messages proof and handing messages over to the dispatch
code;
- the `receive_messages_delivery_proof` is responsible for parsing the messages delivery proof and rewarding relayers
that have delivered the message.

Many things are abstracted by the pezpallet:
- the message itself may mean anything, the pezpallet doesn't care about its content;
- the message dispatch happens during delivery, but it is decoupled from the pezpallet code;
- the messages proof and messages delivery proof are verified outside of the pezpallet;
- the relayers incentivization scheme is defined outside of the pezpallet.

Outside of the messaging pezpallet, we have a set of adapters, where messages and delivery proofs are regular storage
proofs. The proofs are generated at the bridged chain and require bridged chain finality. So messages pezpallet, in this
case, depends on one of the finality pallets. The messages are XCM messages and we are using XCM executor to dispatch
them on receival. You may find more info in [Pezkuwi <> Dicle Bridge](./pezkuwi-dicle-bridge-overview.md) document.

More: [pezpallet level documentation and code](../modules/messages/).

### Bridge Relayers Pezpallet

The pezpallet is quite simple. It just registers relayer rewards and has an entrypoint to collect them. When the rewards
are registered and the reward amount is configured outside of the pezpallet.

More: [pezpallet level documentation and code](../modules/relayers/).

## Offchain Components

Offchain bridge components are separate processes, called relayers. Relayers are connected both to the source chain and
target chain nodes. Relayers are reading state of the source chain, compare it to the state of the target chain and, if
state at target chain needs to be updated, submits target chain transaction.

### GRANDPA Finality Relay

The task of relay is to submit source chain GRANDPA justifications and their corresponding headers to the Bridge GRANDPA
Finality Pezpallet, deployed at the target chain. For that, the relay subscribes to the source chain GRANDPA justifications
stream and submits every new justification it sees to the target chain GRANDPA light client. In addition, relay is
searching for mandatory headers and submits their justifications - without that the pezpallet will be unable to move
forward.

More: [GRANDPA Finality Relay Sequence Diagram](./grandpa-pez-finality-relay.html), [pezpallet level documentation and
code](../relays/finality/).

### Teyrchains Finality Relay

The relay connects to the source _relay_ chain and the target chain nodes. It doesn't need to connect to the tracked
teyrchain nodes. The relay looks at the
[`Heads`](https://github.com/paritytech/polkadot/blob/1a034bd6de0e76721d19aed02a538bcef0787260/runtime/parachains/src/paras/mod.rs#L642)
map of the [`paras`
pezpallet](https://github.com/paritytech/polkadot/tree/1a034bd6de0e76721d19aed02a538bcef0787260/runtime/parachains/src/paras)
in source chain, and compares the value with the best teyrchain head, stored in the bridge teyrchains pezpallet at the
target chain. If new teyrchain head appears at the relay chain block `B`, the relay process **waits** until header `B`
or one of its ancestors appears at the target chain. Once it is available, the storage proof of the map entry is
generated and is submitted to the target chain.

As its on-chain component (which requires bridge GRANDPA pezpallet to be deployed nearby), the teyrchains finality relay
requires GRANDPA finality relay to be running in parallel. Without it, the header `B` or any of its children's finality
at source won't be relayed at target, and target chain won't be able to verify generated storage proof.

More: [Teyrchains Finality Relay Sequence Diagram](./teyrchains-pez-finality-relay.html), [code](../relays/teyrchains/).

### Messages Relay

Messages relay is actually two relays that are running in a single process: messages delivery relay and delivery
confirmation relay. Even though they are more complex and have many caveats, the overall algorithm is the same as in
other relays.

Message delivery relay connects to the source chain and looks at the outbound lane end, waiting until new messages are
queued there. Once they appear at the source block `B`, the relay start waiting for the block `B` or its descendant
appear at the target chain. Then the messages storage proof is generated and submitted to the bridge messages pezpallet at
the target chain. In addition, the transaction may include the storage proof of the outbound lane state - that proves
that relayer rewards have been paid and this data (map of relay accounts to the delivered messages) may be pruned from
the inbound lane state at the target chain.

Delivery confirmation relay connects to the target chain and starts watching the inbound lane end. When new messages are
delivered to the target chain, the corresponding _source chain account_ is inserted to the map in the inbound lane data.
Relay detects that, say, at the target chain block `B` and waits until that block or its descendant appears at the
source chain. Once that happens, the relay crafts a storage proof of that data and sends it to the messages pezpallet,
deployed at the source chain.

As you can see, the messages relay also requires finality relay to be operating in parallel. Since messages relay
submits transactions to both source and target chains, it requires both _source-to-target_ and _target-to-source_
finality relays. They can be GRANDPA finality relays or GRANDPA+teyrchains finality relays, depending on the type of
connected chain.

More: [Messages Relay Sequence Diagram](./pez-messages-relay.html), [pezpallet level documentation and
code](../relays/messages/).

### Complex Relay

Every relay transaction has its cost. The only transaction, that is "free" to relayer is when the mandatory GRANDPA
header is submitted. The relay that feeds the bridge with every relay chain and/or teyrchain head it sees, will have to
pay a (quite large) cost. And if no messages are sent through the bridge, that is just waste of money.

We have a special relay mode, called _complex relay_, where relay mostly sleeps and only submits transactions that are
required for the messages/confirmations delivery. This mode starts two message relays (in both directions). All required
finality relays are also started in a special _on-demand_ mode. In this mode they do not submit any headers without
special request. As always, the only exception is when GRANDPA finality relay sees the mandatory header - it is
submitted without such request.

The message relays are watching their lanes and when, at some block `B`, they see new messages/confirmations to be
delivered, they are asking on-demand relays to relay this block `B`. On-demand relays does that and then message relay
may perform its job. If on-demand relay is a teyrchain finality relay, it also runs its own on-demand GRANDPA relay,
which is used to relay required relay chain headers.

More: [Complex Relay Sequence Diagram](./complex-relay.html),
[code](../relays/bin-bizinikiwi/src/cli/relay_headers_and_messages/).
