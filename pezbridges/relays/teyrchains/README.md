# Teyrchains Finality Relay

The teyrchains finality relay works with two chains - source relay chain and target chain (which may be standalone
chain, relay chain or a teyrchain). The source chain must have the
[`paras` pezpallet](https://github.com/paritytech/polkadot/tree/master/runtime/parachains/src/paras) deployed at its
runtime. The target chain must have the [bridge teyrchains pezpallet](../../modules/teyrchains/) deployed at its runtime.

The relay is configured to submit heads of one or several teyrchains. It pokes source chain periodically and compares
teyrchain heads that are known to the source relay chain to heads at the target chain. If there are new heads,
the relay submits them to the target chain.

More: [Teyrchains Finality Relay Sequence Diagram](../../docs/teyrchains-pez-finality-relay.html).

## How to Use the Teyrchains Finality Relay

There are only two traits that need to be implemented. The [`SourceChain`](./src/teyrchains_loop.rs) implementation
is supposed to connect to the source chain node. It must be able to read teyrchain heads from the `Heads` map of
the [`paras` pezpallet](https://github.com/paritytech/polkadot/tree/master/runtime/parachains/src/paras).
It also must create storage proofs of `Heads` map entries, when required.

The [`TargetChain`](./src/teyrchains_loop.rs) implementation connects to the target chain node. It must be able
to return the best known head of given teyrchain. When required, it must be able to craft and submit teyrchains
finality delivery transaction to the target node.

The main entrypoint for the crate is the [`run` function](./src/teyrchains_loop.rs), which takes source and target
clients and [`TeyrchainSyncParams`](./src/teyrchains_loop.rs) parameters. The most important parameter is the
`teyrchains` - it is the set of teyrchains, which relay tracks and updates. The other important parameter that
may affect the relay operational costs is the `strategy`. If it is set to `Any`, then the finality delivery
transaction is submitted if at least one of tracked teyrchain heads is updated. The other option is `All`. Then
the relay waits until all tracked teyrchain heads are updated and submits them all in a single finality delivery
transaction.

## Teyrchain Finality Relay Metrics

Every teyrchain in PezkuwiChain is identified by the 32-bit number. All metrics, exposed by the teyrchains finality
relay have the `teyrchain` label, which is set to the teyrchain id. And the metrics are prefixed with the prefix,
that depends on the name of the source relay and target chains. The list below shows metrics names for
pezkuwichain (source relay chain) to BridgeHubzagros (target chain) teyrchains finality relay. For other chains, simply
change chain names. So the metrics are:

- `pezkuwichain_to_BridgeHubzagros_Teyrchains_best_teyrchain_block_number_at_source` - returns best known teyrchain block
  number, registered in the `paras` pezpallet at the source relay chain (pezkuwichain in our example);

- `pezkuwichain_to_BridgeHubzagros_Teyrchains_best_teyrchain_block_number_at_target` - returns best known teyrchain block
  number, registered in the bridge teyrchains pezpallet at the target chain (BridgeHubzagros in our example).

If relay operates properly, you should see that
the `pezkuwichain_to_BridgeHubzagros_Teyrchains_best_teyrchain_block_number_at_target` tries to reach
the `pezkuwichain_to_BridgeHubzagros_Teyrchains_best_teyrchain_block_number_at_source`.
And the latter one always increases.
