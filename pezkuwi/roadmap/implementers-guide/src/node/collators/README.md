# Collators

Collators are special nodes which bridge a teyrchain to the relay chain. They are simultaneously full nodes of the
teyrchain, and at least light clients of the relay chain. Their overall contribution to the system is the generation of
Proofs of Validity for teyrchain candidates.

The **Collation Generation** subsystem triggers collators to produce collations and then forwards them to **Collator
Protocol** to circulate to validators.
