# geam-splitter

This crate implements the six Erlang externals of the unmodified [`splitter` 1.3.0](https://hex.pm/packages/splitter/1.3.0) package through Geam's public typed provider API. Add `splitter` to the Gleam project and select this crate as its Geam provider. The verified package range is limited to 1.3.0 until other releases are tested.

The Geam dependency is pinned to `main` commit `b23d82a23d31c77eca749d18f67e73f0d9d952c7`. The [standalone fixture](fixtures/gleam/) and [embedding fixture](fixtures/embedding/) use the original Hex package. `splitter.new` and its empty-pattern filtering remain in the original Gleam source; this crate supplies only its native calls.

Matching selects the earliest byte position, then the longest delimiter at that position, as observed in the original Erlang implementation. Put longer delimiters before their prefixes as the upstream documentation recommends; JavaScript can otherwise select a different delimiter. A splitter with no nonempty delimiters preserves the original Erlang FFI results, including `split(empty, input) == #("", "", input)`, while `split_all(empty, input) == [input]`. Repeated use of a splitter reuses compiled substring finders. Distinct nonempty compiled splitters retain separate opaque identities; empty splitters compare equal.

The crate is ready for source-based use at the pinned Git commit. A crates.io release is a separate step while Geam remains a Git-pinned dependency.
