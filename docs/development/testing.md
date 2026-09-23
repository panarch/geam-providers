# Testing

This document defines the test roles and verification baseline for Geam
Providers. Each provider adds its exact local and CI commands here when its
crate, original Gleam package, and fixtures are introduced.

For acceptance rules, see [review-policy.md](review-policy.md). For practical
test construction and difficult coverage work, see
[test-development.md](test-development.md).

## Test Layers

Each provider is verified at the narrowest owner that can state its contract.

### Owner Tests

Provider modules own unit tests for their registration, configuration, mutable
state, value mapping, callback adapters, external storage, dependency adapters,
errors, and resource lifecycle.

These tests use exact inputs and outputs. Effectful providers use injected
clocks, entropy, clients, channels, and failure controls rather than real time,
network access, or scheduler luck.

### Gleam Contract Tests

A provider's integration tests compile its real Gleam declarations and execute
them through Geam's public hosted boundary. Keep the complete source-visible
operation and expected result together. These tests verify the Rust provider
against the Gleam API; they do not replace owner-local tests.

When a provider supports callbacks, retained values, mutable state, or nested
execution, include repeated calls and exact state/effect ordering. A successful
return value alone is insufficient evidence for those contracts.

### Cross-Crate And Packaging Tests

Cross-crate tests verify only boundaries that cannot be proved inside one
provider crate: public re-exports, independently compiled linkage, packaged
metadata, and installation-shaped consumption.

Run package checks against the files that will actually be published. A local
workspace path is not evidence that the packaged crate resolves and links in a
consumer project.

Keep network-backed live tests separate from deterministic acceptance tests.
Live services may provide compatibility evidence, but credentials, availability,
rate limits, and remote data must not decide the mandatory owner suite.

## Coverage

Every production crate requires independent full-scope line and region coverage
of 100%. Workspace-wide totals cannot compensate for a crate's missed code.
Record the runnable package-specific collection and failure commands here when
a provider is added.

For a coverage gap, inspect the affected package's text report, including
missing lines, regions, and generic instantiations. Diagnose the exact owner
and path before changing production code or writing a retained test. Follow
[test-development.md](test-development.md) for that process.

## Keeping This Document Current

Whenever the workspace, provider layout, CI jobs, toolchain, or coverage closure
changes, update this document in the same commit. A command documented here
must either run as written or be explicitly identified as a future setup step.
