# Provider Compatibility

Geam Providers enables existing Gleam packages to run through Geam without
changing their upstream source. Each provider implements the native functions
required by a package through Geam's public provider API. The package owns its
Gleam-facing API and semantics; the provider does not define a replacement API.
For packages supported here, package-specific native behavior belongs in their
providers while Geam core supplies general host capabilities.

## Compatibility Boundary

The goal is compatibility as experienced by a Gleam caller, not complete
emulation of Erlang, an original native library, or another target's runtime.
Preserve the package's public types and callable behavior, defined failure
conditions and error shapes, observable effects and ordering, resource
lifecycle, and promised data formats or interoperability. Publicly promised
security properties also matter even when they are not expressed in a return
value.

A provider may use a different algorithm or native backend where the package
permits it. That freedom does not cover differences that change the promised
Gleam behavior or break an external format. A value produced by one native
implementation is not automatically a canonical value. Conversely, when the
package specifies exact bytes, fields, ordering, or error behavior, the
provider must preserve them.

Geam runs the package's Erlang-targeted Gleam source. Its Erlang native
implementation helps identify target-specific API behavior, but its internal
choices are not independently part of the contract. Behavior that the package
deliberately normalizes across targets remains part of the shared contract;
platform-specific behavior need not become identical across targets unless
the package promises that identity.

## Package Decisions

For each supported package version, identify the caller-visible guarantees and
permitted variation using its public documentation, Gleam source, relevant
format or protocol specifications, and original implementation. Upstream tests
provide evidence, but a literal result from one backend does not by itself
establish a cross-backend guarantee. Describe intentional differences and why
they remain within the package's contract in the provider's README.

For example, a compression API that promises a valid zlib stream and recovery
of the original data may emit different compressed bytes across backends. A
checksum or canonical encoding with a specified output must match exactly.
