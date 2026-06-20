# Glossary

Here you can find definitions of a bunch of jargon, usually specific to the Pezkuwi project.

- **Approval Checker:** A validator who randomly self-selects so to perform validity checks on a parablock which is
  pending approval.
- **BABE:** (Blind Assignment for Blockchain Extension). The algorithm validators use to safely extend the Relay Chain.
  See [the Pezkuwi wiki][0] for more information.
- **Backable Candidate:** A Teyrchain Candidate which is backed by a majority of validators assigned to a given
  teyrchain.
- **Backed Candidate:** A Backable Candidate noted in a relay-chain block
- **Backing:** A set of statements proving that a Teyrchain Candidate is backable.
- **Collator:** A node who generates Proofs-of-Validity (PoV) for blocks of a specific teyrchain.
- **DMP:** (Downward Message Passing). Message passing from the relay-chain to a teyrchain. Also there is a runtime
  teyrchains module with the same name.
- **DMQ:** (Downward Message Queue). A message queue for messages from the relay-chain down to a teyrchain. A teyrchain
has exactly one downward message queue.
- **Extrinsic:** An element of a relay-chain block which triggers a specific entry-point of a runtime module with given
  arguments.
- **GRANDPA:** (Ghost-based Recursive ANcestor Deriving Prefix Agreement). The algorithm validators use to guarantee
  finality of the Relay Chain.
- **HRMP:** (Horizontally Relay-routed Message Passing). A mechanism for message passing between teyrchains (hence
  horizontal) that leverages the relay-chain storage. Predates XCMP. Also there is a runtime teyrchains module with the
  same name.
- **Inclusion Pipeline:** The set of steps taken to carry a Teyrchain Candidate from authoring, to backing, to
  availability and full inclusion in an active fork of its teyrchain.
- **Module:** A component of the Runtime logic, encapsulating storage, routines, and entry-points.
- **Module Entry Point:** A recipient of new information presented to the Runtime. This may trigger routines.
- **Module Routine:** A piece of code executed within a module by block initialization, closing, or upon an entry point
  being triggered. This may execute computation, and read or write storage.
- **MQC:** (Message Queue Chain). A cryptographic data structure that resembles an append-only linked list which doesn't
  store original values but only their hashes. The whole structure is described by a single hash, referred as a "head".
  When a value is appended, it's contents hashed with the previous head creating a hash that becomes a new head.
- **Node:** A participant in the Pezkuwi network, who follows the protocols of communication and connection to other
  nodes. Nodes form a peer-to-peer network topology without a central authority.
- **Teyrchain Candidate, or Candidate:** A proposed block for inclusion into a teyrchain.
- **Parablock:** A block in a teyrchain.
- **Teyrchain:** A constituent chain secured by the Relay Chain's validators.
- **Teyrchain Validators:** A subset of validators assigned during a period of time to back candidates for a specific
  teyrchain
- **On-demand teyrchain:** A teyrchain which is scheduled on a pay-as-you-go basis.
- **Lease holding teyrchain:** A teyrchain possessing an active slot lease. The lease holder is assigned a single
  availability core for the duration of the lease, granting consistent blockspace scheduling at the rate 1 parablock per
  relay block.
- **PDK (Teyrchain Development Kit):** A toolset that allows one to develop a teyrchain. Pezcumulus is a PDK.
- **Preimage:** In our context, if `H(X) = Y` where `H` is a hash function and `Y` is the hash, then `X` is the hash
  preimage.
- **Proof-of-Validity (PoV):** A stateless-client proof that a teyrchain candidate is valid, with respect to some
  validation function.
- **PVF:** Teyrchain Validation Function. The validation code that is run by validators on teyrchains.
- **PVF Prechecking:** This is the process of checking a PVF when it appears
  on-chain, either when the teyrchain is onboarded or when it signalled an
  upgrade of its validation code. We attempt preparation of the PVF and make
  sure it that succeeds within a given timeout, plus some additional checks.
- **PVF Preparation:** This is the process of preparing the WASM blob and includes both prevalidation and compilation.
- **PVF Prevalidation:** Some basic checks for correctness of the PVF blob. The
  first step of PVF preparation, before compilation.
- **Relay Parent:** A block in the relay chain, referred to in a context where work is being done in the context of the
  state at this block.
- **Runtime:** The relay-chain state machine.
- **Runtime Module:** See Module.
- **Runtime API:** A means for the node-side behavior to access structured information based on the state of a fork of
  the blockchain.
- **Subsystem:** A long-running task which is responsible for carrying out a particular category of work.
- **UMP:** (Upward Message Passing) A vertical message passing mechanism from a teyrchain to the relay chain.
- **Validator:** Specially-selected node in the network who is responsible for validating teyrchain blocks and issuing
  attestations about their validity.
- **Validation Function:** A piece of Wasm code that describes the state-transition function of a teyrchain.
- **VMP:** (Vertical Message Passing) A family of mechanisms that are responsible for message exchange between the relay
  chain and teyrchains.
- **XCMP:** (Cross-Chain Message Passing) A type of horizontal message passing (i.e. between teyrchains) that allows
  secure message passing directly between teyrchains and has minimal resource requirements from the relay chain, thus
  highly scalable.

## See Also

Also of use is the [Bizinikiwi Glossary](https://bizinikiwi.dev/docs/en/knowledgebase/getting-started/glossary).

[0]: https://wiki.network.pezkuwichain.io/docs/learn-consensus
