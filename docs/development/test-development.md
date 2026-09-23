# Test Development

This guide collects practical ways to construct provider tests and diagnose
difficult coverage gaps. For suite layout and commands, see
[testing.md](testing.md). For acceptance rules, see
[review-policy.md](review-policy.md).

These are working heuristics rather than additional policy. Coverage is both a
diagnostic signal and an acceptance gate, but it does not decide the production
design or make an unclear test useful.

## Test Roles

Tests are easiest to understand when each has one primary role.

- An owner unit test proves one registration, adapter, configuration rule,
  state transition, failure mapping, resource lifecycle, or value contract
  beside the module that owns it.
- A source-backed provider example keeps the complete Gleam declaration,
  provider setup, and exact result together while entering through the public
  Geam host boundary.
- A fixture integration test documents a cross-crate, packaged, or
  source-visible workflow.
- A compile-fail test fixes a public Rust type-system restriction that has no
  runtime representation.

Integration coverage may execute an owner without documenting its exact
contract. Conversely, a narrow owner probe may answer a reachability question
without replacing a readable provider example.

When production ownership moves to another crate, move owner tests with it. Do
not preserve former private access by publishing a test API.

## Choose The Strategy

An uncovered path does not always mean that another test is missing. Classify
the gap before changing code:

- A public behavior gap is best expressed as a complete source-backed provider
  example or fixture.
- A structural gap appears when an owner represents impossible states, repeats
  one decision across adapters, or instantiates unrelated generic paths.
  Clarify the representation or ownership instead of adding a test.
- A reachability gap remains when the structure is sound but it is unclear
  which concrete branch, callback shape, or monomorph an existing scenario
  exercises. A small owner probe can answer that question directly.

These strategies are complementary rather than sequential. Address an obvious
structural problem before searching for more fixtures. Use a probe when it is
unclear whether the reported path represents a valid provider state.

A structural change should remain useful without its coverage effect. Good
signals include a narrower contract, one clear correctness owner, removal of an
impossible variant, or elimination of duplicated dispatch. A gap that only
moves into a new abstraction has not been structurally resolved.

## Diagnose And Probe Coverage

Use the detailed report described in [testing.md](testing.md) to identify:

- the uncovered file and production owner;
- the exact line and region;
- the generic instantiation, when present;
- the missing branch outcome;
- the test binary or package that instantiated it.

Line, region, and instantiation coverage answer different questions. A line may
run while one expression outcome remains uncovered. The same source line may
also run through a different generic monomorph from the one shown in the
report. New closures, wrappers, callback adapters, and test binaries can create
another generated copy instead of reaching the intended one.

Use this bottom-up loop when the exact path remains unclear:

1. Identify the function, branch, state transition, callback shape, or
   monomorph that owns the gap.
2. Exercise it with the smallest owner-local unit or temporary probe that can
   establish reachability.
3. Run focused coverage and confirm that the exact reported target moved.
4. If the state is valid, connect it outward to the nearest meaningful provider
   contract or source-backed behavior.
5. Remove the diagnostic probe once the retained test records the same path.

The initial probe may have little semantic value; it gives a binary answer
about reachability. If it requires malformed registration, impossible state,
invalid payload ownership, or data rejected by an earlier boundary, the
representation or owner boundary is the likely problem.

When line coverage is complete but a region remains, inspect assertion and
closure shapes. A `matches!`, `let ... else panic!`, short-circuit expression,
or generic selector can introduce an unexecuted region even when production
behavior is covered. Comparing the exact result directly may state the
contract more clearly.

Repeated attempts that do not move the exact target are a signal to revisit
the hypothesis, concrete instantiation, or representation instead of varying
more test syntax. Return to the previous baseline after a rejected experiment
so the next result remains interpretable.

## Deterministic Effects And Lifecycles

Providers often adapt systems that are nondeterministic from a test's point of
view. Make their production boundaries controllable:

- inject clocks and advance controlled time instead of sleeping;
- inject clients or transports instead of contacting a live service;
- inject entropy and environment-derived configuration;
- use controlled channels, responses, and failure hooks;
- assert cancellation, shutdown, cleanup, and state ordering explicitly;
- keep live compatibility checks outside the deterministic owner suite.

Do not add test-only branches to production code. The same dependency boundary
that makes a test deterministic should make production ownership and failure
handling clearer.

For retry, timeout, callback, or background-task behavior, verify the exact
event order rather than only the final return value. A test that waits for wall
clock time or relies on scheduler order is not stable evidence for a lifecycle
contract.

## Experiments And Scenario Locality

Gleam declarations, provider setup, and expected behavior are easiest to review
together. A focused scenario should include only the state, dependencies, and
operations it exercises. Broad setup can instantiate unrelated generic paths
and make coverage movement difficult to interpret.

Shared helpers are useful for stable registration, compilation, execution, and
assertion plumbing. Inline the setup when a helper hides the declaration,
configuration, selected operation, expected result, semantic variant, or
effect ordering under review.

When an end-to-end result is wrong, inspect the nearest intermediate owner:
registration, initialization, adapter conversion, dependency call, callback,
state projection, or result mapping. Narrowing the fault is usually more useful
than expanding the scenario.

For long investigations, keep a lightweight temporary work log recording the
exact target, current hypothesis, observed result, and next decision. This
prevents failed hypotheses from being retried after interruption.

Coverage work is probably aimed at the wrong layer when it requires test-only
production visibility, impossible provider states, fallback paths, new
internal errors, or generic helpers used only by tests. The acceptance
boundaries for those cases belong to [review-policy.md](review-policy.md).

A clean result covers the exact target, retains a test that documents a real
owner contract or public provider behavior, and removes every purely diagnostic
probe.
