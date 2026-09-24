# geam-global-value

Rust provider for the unmodified Gleam [`global_value` 1.0.0](https://hex.pm/packages/global_value/1.0.0) package when it runs through Geam. Add the original Gleam package to the Gleam project and this provider crate to the Geam runner or embedding host. The provider implements the package's six Erlang native externals; the public `create_with_unique_name` function remains the original Gleam source.

Within one Geam execution domain, processes share a value by name and concurrent initialisation is serialized for that name. An initializer that fails or is cancelled before storing a value leaves the name available for retry. Once stored, the value remains available until that domain closes. Distinct Geam execution domains have separate caches. The provider does not share Erlang `persistent_term` storage or coordinate distributed BEAM nodes.

The [standalone fixture](fixtures/gleam/) and [embedding fixture](fixtures/embedding/) use the original Hex package without editing its Gleam source. The source-backed tests also check `Result`, function and custom values, cancellation and failure, and concurrent Gleam processes. The fixtures pin `gleam_stdlib` 1.0.3 for compatibility with the Geam commit below. See the [testing guide](../docs/development/testing.md) for the verification commands.

This crate depends on Geam's public typed provider API at commit `76c4ab7c6a2c0c35975bdd97e5de7c6284f895e7`. The pinned Git dependency currently means `cargo package --list` checks the package contents but does not establish crates.io publication readiness.
