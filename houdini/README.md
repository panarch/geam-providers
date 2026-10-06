# geam-houdini

This crate provides the Erlang native functions used by the unmodified [houdini 1.2.0](https://hex.pm/packages/houdini/1.2.0) Gleam package when it runs through Geam. Add houdini to the Gleam project and select geam-houdini as its provider. The supported package range is limited to 1.2.0; Houdini 1.2.1 has a different external declaration and has not been verified here.

The original Gleam package owns the public escape API and its HTML escaping algorithm. This provider registers its private generic coerce/1 external and byte-range slice/3 external. String/BitArray conversions preserve their original bytes, including non-UTF-8 bytes, and byte-aligned slices share their input storage. It does not provide a JavaScript implementation or claim support for all of Lustre.

The Geam and Geam Core dependencies use the same pinned main commit, 6a6cf4624447b53362bf394513578d708682f0e5. The [standalone fixture](fixtures/gleam/) selects this provider explicitly; the [embedding fixture](fixtures/embedding/) consumes this crate directly. Both use the original package from Hex.

The crate is ready for source-based use with the pinned Geam commit. A crates.io release is a separate step while Geam is a Git dependency.
