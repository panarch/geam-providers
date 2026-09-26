# geam-envoy

This crate supplies the four Erlang externals used by the unchanged
[`envoy` 1.2.0](https://hex.pm/packages/envoy/1.2.0) Gleam package: `get`,
`set`, `unset`, and `all`. The original package keeps its public API and source
bodies. Only version 1.2.0 is verified.

One provider `RunState` owns the environment visible to those four functions.
With no configuration, initialization takes one snapshot of the host process
environment. A `values` configuration table instead supplies a complete,
deterministic starting environment, including an explicitly empty one. For
example, a host can supply `values = { PORT = "8080" }`. A manual embedding host
can also create an isolated `RunState` with `RunState::from_values`. Calls that
reuse a state see prior `set` and `unset` operations; a new state has its own
values. `all` returns an immutable original `gleam/dict.Dict`, so later changes
do not alter an earlier result.

`set` and `unset` change this provider state, not the OS process environment.
They do not dynamically reconfigure other providers. Names containing `=` or
NUL and values containing NUL produce a host failure; empty names, empty
values, `=` within values, and Unicode strings are supported. A non-Unicode
host environment entry makes default initialization fail instead of silently
dropped data. On Windows, ASCII letter casing in names is treated
case-insensitively; on Unix names are case-sensitive. Dict iteration order is
unspecified.

Geam is pinned to `main` commit
`cf5fb100c9220d9e30ff60a11d5bfbeb77f1ebef`. The
[standalone fixture](fixtures/gleam/) and
[embedding fixture](fixtures/embedding/) resolve the original Hex package
without editing it. [Source checksums](fixtures/upstream.sha256) cover its
Gleam module, both native FFI files, and package manifest. The CI matrix
declares Ubuntu, macOS, and Windows; a runner is verified only after its hosted
job passes. Publishing the crate and consuming a registry copy are separate
steps after its Geam dependency is released.
