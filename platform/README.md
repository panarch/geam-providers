# geam-platform

This crate supplies the three Erlang externals used by the unmodified [`platform` 1.0.0](https://hex.pm/packages/platform/1.0.0) Gleam package through Geam's public typed provider API. Add `platform` to the Gleam project and select this crate as its Geam provider. The supported package range is limited to 1.0.0 until other upstream versions are tested.

The provider binds `runtime_`, `os_`, and `arch_` in the original `platform` source module. It returns the Erlang-target runtime string `"erlang"`, an OS name corresponding to Erlang's `os:type/0`, and an architecture name. The original Gleam package continues to own the public `Runtime`, `Os`, and `Arch` types and their string classification. JavaScript externals are outside this provider's scope.

For native targets, the OS and architecture come from Rust's compilation target. `macos` is reported as Erlang's `darwin`, Windows as `win32`, and Solaris or illumos as `sunos`; unsupported target OS names report `unknown`. On Windows the architecture follows the original Erlang FFI's VM word-size convention (`ia32` or `x64`), including Windows ARM. On Unix the Rust target architecture is returned; unlike BEAM's `system_architecture`, Rust's architecture constant has no target-triple suffix. The locally tested macOS ARM64 host yields the same public variants as BEAM. The CI fixture is configured to compare them on Ubuntu x86_64; exact strings for other target triples have not been independently verified against BEAM.

The Geam dependency is pinned to `main` commit `bd95b872c578df88f76ed5c4175fb1ea55ee8607`. The [standalone fixture](fixtures/gleam/) and [embedding fixture](fixtures/embedding/) use the unmodified Hex release. The standalone fixture has passed locally on macOS ARM64 and is configured for CI Ubuntu x86_64; Windows mapping is owner-tested but has no native Geam execution in this repository yet.

The crate is ready for source-based use at the pinned Git commit. A crates.io release is a separate step: Cargo packaging cannot yet resolve all required features through the published Geam metadata.
