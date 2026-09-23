# geam-filepath

This crate supplies the Erlang external `is_windows/0` for the unmodified [`filepath` 1.1.2](https://hex.pm/packages/filepath/1.1.2) Gleam package through Geam's public typed provider API. Add `filepath` to the Gleam project and select this crate as its Geam provider. The supported package range is limited to 1.1.2 until other upstream versions are tested.

The Geam dependency is pinned to `main` commit `76c4ab7c6a2c0c35975bdd97e5de7c6284f895e7`. The [standalone fixture](fixtures/gleam/) shows explicit provider selection, and the [embedding fixture](fixtures/embedding/) shows a Rust consumer with the provider as a direct dependency. Both use the original package from Hex.

The provider reports whether the native target is Windows, matching the original Erlang FFI's Windows versus non-Windows distinction. The original Gleam package retains ownership of `split`, `split_windows`, `split_unix`, and all other path operations. Some upstream operations still have Windows support TODOs; this provider does not change their behavior. The fixtures check the active host's `split` branch and both explicit split functions; they do not constitute a native Windows Geam run. The JavaScript external is outside this Erlang-targeted provider's scope.

The crate is ready for source-based use at the pinned Git commit. A crates.io release is a separate step: Cargo packaging cannot yet resolve all required features through the published Geam metadata.
