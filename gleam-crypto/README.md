# geam-crypto

This crate implements the five Erlang externals of the unmodified [`gleam_crypto` 1.6.0](https://hex.pm/packages/gleam_crypto/1.6.0) package through Geam's public typed provider API. Add `gleam_crypto` to the Gleam project and select `geam-crypto` as its Geam provider. The supported package range is limited to 1.6.0 until other versions are verified.

The provider supplies SHA-224, SHA-256, SHA-384, SHA-512, SHA-1, and MD5 hashing, incremental hash state, HMAC, and OS-backed random bytes. It preserves the original Gleam implementations of `hash`, `sign_message`, `verify_signed_message`, and `secure_compare`. The [standalone fixture](fixtures/gleam/) and [Rust embedding fixture](fixtures/embedding/) run the unchanged Hex package. Their source-backed checks cover all six digests and HMACs, hash-state branching, signed-message verification and tampering, and random-byte behavior.

The standalone runner uses operating-system entropy. An embedding host can provide a custom `Entropy` implementation through `RunState::with_entropy`, including deterministic or failing sources for controlled tests. Invalid byte counts, unaligned bit arrays passed to native crypto operations, allocation failures, and entropy failures become host errors. MD5 and SHA-1 remain available for upstream compatibility, but applications should choose stronger algorithms for new security uses.

## Current limitations

- **`secure_compare` is not verified as constant-time on Geam.** `gleam_crypto` 1.6.0 implements it in Gleam and scans bytes of equal-length inputs, but Geam's execution has no verified input-independent timing guarantee for that function. Functional checks of `verify_signed_message` do not establish timing safety. Review timing-sensitive uses separately; this crate does not claim complete support for downstream packages such as Wisp.
- **Only `gleam_stdlib` 1.0.3 has been verified.** Both fixtures pin that version, which is allowed by `gleam_crypto` 1.6.0. With the pinned Geam commit, resolving stdlib 1.0.5 produced a `gleam/bit_array.pad_to_bytes` linkage failure.

The Geam dependency is pinned to `main` commit `b23d82a23d31c77eca749d18f67e73f0d9d952c7`. See the [testing guide](../docs/development/testing.md) for the current verification commands. Publishing the crate and testing registry-based consumption are separate steps while Geam remains Git-pinned.
