# PezkuwiChain claim statements (canonical, hash-pinned)

These are the **canonical** legal attestation documents that a token claimant agrees
to in the `claims` pallet (`pezkuwi-runtime-common`). The SHA-256 hash of each file is
embedded in `StatementKind::to_text()`, which is part of the payload the claimant signs.

**The bytes in this directory, the bytes served at `statement.pex.network`, and the hash
in the runtime MUST stay identical.** If they ever diverge, claimants can no longer
verify what they signed. These files are version-controlled here precisely so the exact
bytes are always recoverable and re-deployable.

| Document | URL | SHA-256 (in `to_text()`) |
|---|---|---|
| `regular.html` | https://statement.pex.network/regular.html | `95bf22e1fd1bbcd6a06ccac523a391a94e48c517c7db183b734ba72f955c21e8` |
| `saft.html` | https://statement.pex.network/saft.html | `3921445529820f15f000cef9f143b74194516c96873fff4a829bac0162b35c59` |

Verify:

```sh
sha256sum docs/statement/regular.html docs/statement/saft.html
curl -s https://statement.pex.network/regular.html | sha256sum
```

## Changing a statement

Editing a statement changes its hash, which changes `to_text()`, which changes the
on-chain runtime — so it is a **runtime upgrade** and invalidates signatures made against
the previous text. To change one: edit the file, re-deploy to `statement.pex.network`,
recompute the SHA-256, update `StatementKind::to_text()`, bump `spec_version`, and ship a
runtime upgrade. Treat the legal wording as reviewed-and-final before pinning.

Hosting: served as a static nginx vhost on the bootnode host (TLS via Let's Encrypt).
