# geam-term-size

This crate implements the single native function required by the unchanged
[`term_size` 1.0.1](https://hex.pm/packages/term_size/1.0.1) Gleam package through
Geam's public typed provider API. Add `term_size` to the Gleam project and
select `geam-term-size` as its provider. Only package version 1.0.1 is verified.
The original Gleam bodies of `rows()` and `columns()` run unchanged.

The provider reads the Geam process's terminal using Rust `terminal_size`
0.4.4. It checks stdout, then stderr, then stdin and returns the first
available terminal size as `Ok(#(rows, columns))`. When none has a size, it
returns `Error(Nil)`. Every call queries again; it does not cache a size, read
`COLUMNS` or `LINES`, or assume an 80-by-24 terminal. An embedding application's
custom output sink or virtual terminal is not consulted unless it is also a
process standard-stream TTY. This process-terminal policy can differ from the
original Erlang and JavaScript backends' stream selection while preserving the
documented Gleam return shape and failure condition.

The initial CI target is Ubuntu. Other operating systems are not claimed as
verified until their provider and fixture checks run there.

Geam is pinned to `main` commit
`76c4ab7c6a2c0c35975bdd97e5de7c6284f895e7`. The
[standalone fixture](fixtures/gleam/) selects this provider, and the
[embedding fixture](fixtures/embedding/) directly depends on the crate. Both
use the original package from Hex. The [source contract](fixtures/CONTRACTS.md)
records the external and observable boundaries. Publishing this crate and
consuming a published copy are separate steps after the required Geam APIs
are released.
