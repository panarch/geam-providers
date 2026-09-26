# geam-regexp

This crate implements the six Erlang externals of the unmodified [`gleam_regexp` 1.1.1](https://hex.pm/packages/gleam_regexp/1.1.1) package through Geam's public typed provider API. Add `gleam_regexp` to the Gleam project and select this crate as its Geam provider. The supported package range is limited to 1.1.1 until other upstream versions are tested.

The Geam dependency is pinned to `main` commit `cf5fb100c9220d9e30ff60a11d5bfbeb77f1ebef`. The [standalone fixture](fixtures/gleam/) shows explicit provider selection, and the [embedding fixture](fixtures/embedding/) shows a Rust consumer with the provider as a direct dependency. Both use the original package from Hex.

Patterns use the Rust `regex` engine. Unicode character classes are enabled; look-around and backreferences in patterns are unsupported. The two `Options` fields control case-insensitive and multi-line matching. `split` includes captured separators in source order and uses an empty string for an optional capture that did not participate, as the original Erlang and JavaScript implementations do. `replace` uses Rust replacement references such as `$1`, `${name}`, and `$$` for a literal dollar sign. Erlang `re` replacement references such as `\1` are not interpreted as capture references here. Compile error text and byte positions reflect the Rust parser and may differ from Erlang or JavaScript. In `scan`, empty and absent captures become `None`; trailing absent captures are omitted, following the source package's observed result shape.

The regular expression engine is deliberately platform-specific in the original package. Applications should test their own patterns on every platform they use. This crate does not add a second Gleam API or modify the upstream Gleam declarations.

The crate is ready for source-based use at the pinned Git commit. A crates.io release is a separate step: Cargo packaging cannot yet resolve all required features through the published Geam metadata.
