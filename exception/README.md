# geam-exception

This crate implements the three Erlang externals in the unmodified [`exception` 2.1.1](https://hex.pm/packages/exception/2.1.1) package through Geam's public typed provider API. Add `exception` to a Gleam project and select `geam-exception` as its Geam provider. Other upstream versions have not been verified.

The Geam dependency is pinned to `main` commit `5adbf4e654c6e4ca518e60babe19c3dc88532d4d`. The [standalone fixture](fixtures/gleam/) and [embedding fixture](fixtures/embedding/) resolve the original Hex package.

`rescue` returns `Ok` for a successful callback and `Error(Errored(reason))` for a catchable Geam callback failure. The `Dynamic` reason is an inspectable Geam diagnostic symbol (`dynamic.classify(reason) == "Atom"`); its text and Erlang's original error term are not stable cross-target formats. Geam has no Erlang `throw` or `exit` failure class, so this provider does not produce `Thrown` or `Exited`. The original constructors remain part of the Gleam type.

`defer` runs cleanup after successful and failing callbacks. `on_crash` runs cleanup only after a catchable failure. Both propagate the original callback failure when cleanup succeeds; cleanup failure takes precedence, including after a cancelled body callback. A callback returning `Cancelled` is not converted into `Errored`; `defer` attempts cleanup first, while `on_crash` skips cleanup for cancellation. Dropping a suspended execution can prevent cleanup, so cleanup during execution cancellation is not guaranteed.

Both fixtures exercise a generic consumer that matches `exception.rescue(body)` and reconstructs `Result(value, String)`, including a panic-only callback with no successful return type. The consumer also checks retained scalar, container, custom, codepoint, callable, and opaque `Dynamic` values through all three functions, and verifies that cleanup return values are discarded. CI builds the standalone executable and runs it from outside the fixture directory.

The provider uses public APIs from the pinned Geam revision. Publishing the crate to crates.io is separate while Geam remains a Git-pinned dependency.
