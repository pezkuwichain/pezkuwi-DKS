# Why each `osv-baseline.json` entry is there

The baseline only shrinks: an entry that stops applying turns the gate red until it is removed.
Every entry has to say here why it cannot be closed today and what closes it.

## block-buffer 0.9.0 / 0.10.4 — GHSA-qwgh-2vcv-g2f7 (medium, published 2026-08-19)

**What:** if the `compress` callback passed to `EagerBuffer::digest_blocks` (or `ReadBuffer`)
panics and the panic is caught, the buffer's cursor is left invalid and later use reads or writes
out of bounds. Fixed only in block-buffer 0.12.1, which belongs to the digest 0.11 generation.

**Why it cannot be closed (measured 2026-10-28, crates.io latest releases):** everything RustCrypto
publishes has moved to digest 0.11 (blake2 0.11, sha1 0.11, sha2 0.11, sha3 0.12, hmac 0.13,
pbkdf2 0.13, ripemd 0.2, curve25519-dalek 5.0), but three dependencies with no digest 0.11 release
keep block-buffer 0.10 / 0.9 in the graph whatever we move:
- `schnorrkel` 0.11.5 (latest) — sr25519 — `curve25519-dalek ^4.1.3`, `sha2 ^0.10.7`;
- `ark-ff`, `ark-serialize` 0.6.0, `ark-vrf` 0.6.1, `ark-transcript` 0.0.6 (latest) —
  bandersnatch — `digest ^0.10`;
- `ssz_rs` 0.9.0 (latest) — `sha2 ^0.9.8`.

Moving only the crates that have releases would leave the advisory exactly where it is, at the
cost of churn across the crypto stack before mainnet; forking sr25519 and bandersnatch to close it
would trade a medium advisory for an unreviewed signature scheme.

**Why it is not reachable here:** every `compress` these buffers are given is a hash function's
compression round (SHA-1/2/3, BLAKE2, RIPEMD, Keccak), which does not panic, and no caller wraps a
digest in `catch_unwind` and keeps using it.

**What closes it:** schnorrkel, the ark crates and ssz_rs releasing on digest 0.11 / sha2 0.11.
Then move the workspace's own hashing (`pezsp-crypto-hashing` and the RustCrypto crates above) in
the same change; the gate reports `GONE` when block-buffer 0.9/0.10 leave the graph.
