# geam-gzlib

This crate supplies the five Erlang-targeted native functions of the unmodified [gzlib 2.0.0](https://hex.pm/packages/gzlib/2.0.0) Gleam package through Geam's public typed provider API. Add `gzlib` to the Gleam project and select `geam-gzlib` as its provider. Only package version 2.0.0 has been verified.

The provider implements zlib-wrapped compression and decompression with `miniz_oxide` and CRC32 with `crc32fast`. It accepts byte-aligned `BitArray` values, compression levels 0 through 9, and unsigned CRC seeds through `2^32 - 1`. Invalid inputs return the original package's `Error(Nil)` shape. `compress` uses level 6; the original Erlang backend chooses its own default level.

The compressed byte sequence and compression ratio can differ from the Erlang or JavaScript backend. The compatibility boundary is a valid zlib stream, recovery of the original bytes, interoperability with the original backend, and exact CRC values. A complete zlib stream followed by trailing bytes is accepted, as by the Erlang FFI. Decompression returns the entire result as a `BitArray` and can allocate as much memory as that result requires. This crate does not implement gzip or raw DEFLATE APIs that the Gleam package does not expose.

Geam is pinned to `main` commit `b23d82a23d31c77eca749d18f67e73f0d9d952c7`. The [standalone fixture](fixtures/gleam/) explicitly selects this provider, and the [embedding fixture](fixtures/embedding/) directly depends on this crate. Both use the original package from Hex. The [contract inventory](fixtures/CONTRACTS.md) records the release's five externals and their observable boundaries.

The crate is ready for source-based use with that Geam commit. Publishing to crates.io and testing a consumer of the published package are separate steps.
