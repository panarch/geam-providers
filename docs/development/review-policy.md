# Review Policy

Geam Providers is built to keep each Gleam-to-Rust host boundary explicit and
reviewable. Prefer visible ownership and static contracts over clever
compression. Keep each change small enough to review its behavior, tests,
failure boundary, and cost model together.

For the repository's test layout and commands, see [testing.md](testing.md).
For practical test construction and coverage diagnosis, see
[test-development.md](test-development.md).

## Provider Boundary Rules

The Gleam package owns its public Gleam API. A companion Rust crate supplies
the implementation required when that API runs through Geam. The provider must
not create a second source API or reinterpret the package's semantics.

Use Geam's static provider registration and typed host boundary. Do not add a
parallel ABI, runtime inventory, reflection, `Any`, downcasts, string dispatch,
or a second Rust-to-Gleam type table for provider convenience.

Derive the schema, registration, callback adapter, state projection, and exact
construction capabilities from the Rust declarations visible at the provider
site. Unsupported Rust declarations must fail during macro expansion or Rust
type checking. A mismatch with the separately compiled Gleam declaration must
remain an exact hosted-linkage failure rather than becoming runtime validation
or fallback behavior.

Configuration, mutable state, and caller-supplied capabilities are separate
owners:

- Configuration is explicit input to provider initialization.
- Mutable state belongs to the provider component that declares and projects
  it.
- IO, clocks, entropy, network clients, and similar effects remain explicit
  host-owned capabilities.
- Initialization failure belongs before source execution.

Do not hide configuration in environment lookup, global state, syntactic
defaults, or runtime fallback. A default must have one visible, deterministic
meaning.

External storage, ordinary Gleam custom values, and provider run state are
different domains. Preserve the exact specialized Gleam shape of stored
external values. Do not use Rust payload identity, allocation addresses, or
inspection output as source equality or hashing semantics.

Callable invocation is a call-scoped capability. End mutable state and payload
borrows before nested execution re-enters Gleam. Call-scoped handles may remain
live across re-entry, but they must not escape the host invocation. Preserve
the actual source or provider identity when a nested call fails.

Provider adapters must preserve the ownership and cost model of Geam's typed
host path. Retain or pass through existing handles when the public mapping
promises a view or identity. Decode lazy values only when requested, and
construct newly returned source containers exactly once. Do not introduce
eager whole-value materialization, whole-container preconversion, payload or
item clones, or reconstruction of pass-through values.

## Ownership Rules

Determine ownership from construction, mutation, lifetime, dependencies, and
actual callers before assigning a semantic role.

- Phase-local builders, parsers, and accumulators belong to that phase.
- Data that survives a phase belongs to the downstream domain that stores and
  reads it.
- A crate boundary does not justify wider visibility. Expose only the narrow
  production contract required by a real caller; test access is not a
  production caller.
- Aggregate roots must not flatten unrelated child domains for convenience.
- Within an owner module, declare the representative owner or protocol before
  its variants, payloads, IDs, adapters, and supporting types.
- Arrange production functions in reader order: representative entry points
  first, followed by the private implementation chain in first-use order.
- Shared helpers belong to the narrowest domain that owns their behavior.
- File size can justify a module split, but not an ownership change.

Background work and retained resources require one explicit lifecycle owner.
Creation, cancellation, shutdown, and cleanup must not be split across
unrelated modules or left to process termination.

## Panic Rules

Production provider logic must not use panic paths for control flow,
configuration validation, recoverable dependency failures, or internal state
handling. Do not use `panic!`, `unreachable!`, `unwrap`, or `expect` in
non-test logic.

Reject invalid configuration before the provider is made available. Return
structured provider errors for failures caused by external systems or
caller-owned input. If an operation has already crossed a validating boundary,
make the remaining internal path total by construction instead of adding an
internal error or fallback.

Test code may use panic paths only to assert fixture shape. Keep those paths
local, visible, and covered by explicit tests.

## Error Rules

Errors make ownership boundaries visible.

- Use one stable variant for one concrete failure boundary.
- Preserve dynamic values such as provider, operation, field, and dependency
  names as structured fields.
- Do not merge configuration, dependency, linkage, callback, and source
  failures into a catch-all merely because they share a provider.
- Each stable leaf error has one production construction owner. Callers
  propagate it without reconstructing the same decision.
- Stable variants must correspond to reachable production behavior. Test-only
  references do not justify an error variant.
- Use `Option` only when the caller owns the meaning of absence. If all callers
  translate `None` into the same failure, return a structured `Result`.
- Do not erase a boundary-carrying `Result` with `.ok()`.
- An internal helper may return `Result` only when it creates or propagates a
  real failure boundary.

## Import And Module Rules

Do not use wildcard imports or re-exports. The only exception is an intentional
parent facade re-exporting child modules as its public surface.

Child modules must not become shared utility surfaces for siblings. If sibling
modules need a helper, keep it in the parent facade or in the child that owns
the helper's domain.

Split modules by production ownership or protocol. Do not move tests into a
detached file merely to shorten the production file.

## Test Rules

Owner unit tests live beside the module that owns the behavior. They keep
representative inputs and exact expected results in the owning test module.

Provider contract tests compile the real Gleam declaration and exercise the
public Geam provider boundary. They prove exact registration, configuration,
state, value mapping, callback, external storage, error, and lifecycle behavior
that the provider exposes. Integration coverage does not replace owner tests.

Formatter, serializer, diagnostic, and protocol implementations require
owner-local exact tests. Generic or delegating implementations must cover every
distinct concrete family they support.

Use compile-fail tests when the reviewed contract is a public Rust type-system
restriction with no runtime representation.

Do not publish production APIs solely for tests. Cross-crate tests use the
existing public boundary; genuinely shared behavior exposes only the narrow
contract required by production callers.

Do not commit ignored tests. Every test used as compatibility, coverage, or
release evidence must run in a mandatory verification path.

## Helper Rules

Test helpers may reduce real repetition and stable fixture setup, but they must
not hide the behavior under review.

- Avoid single-use shallow wrappers.
- Name helpers after their fixture role, not their implementation detail.
- Keep helpers in the nearest module that uses them.
- Keep declarations, provider configuration, selected operation, expected
  value, and expected effects visible in the test that reviews them.
- Do not layer test-only constructors that hide defaults, sentinel values,
  synthetic identities, or unreachable states.
- Keep test-only panic guards visible and covered.

## Coverage Rules

Every production crate keeps full-scope line and region coverage at 100%.

Coverage gaps are work, but coverage does not determine the production design.
Do not add unreachable states, test-only production access, fallback paths,
internal errors, or unclear tests solely to move the report.

A covered test that hides the contract is not acceptable. Use
[test-development.md](test-development.md) to distinguish public behavior,
structural, and reachability gaps before choosing a fix.
