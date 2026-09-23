# Testing

This document defines the test roles and verification baseline for Geam
Providers. The current production package is `geam-regexp`.
It must remain the source of truth for the checks actually run.

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

## Dependency Preparation

The Rust workspace and both fixture consumers use Geam `main` commit
`76c4ab7c6a2c0c35975bdd97e5de7c6284f895e7`. The Gleam fixtures resolve the
unmodified `gleam_regexp` 1.1.1 package from Hex. From the repository root,
download the fixture dependencies before running the source-backed Rust test:

```sh
(cd gleam-regexp/fixtures/gleam && gleam deps download)
(cd gleam-regexp/fixtures/embedding/gleam && gleam deps download)
(cd gleam-regexp/fixtures/gleam && gleam format --check && gleam check)
(cd gleam-regexp/fixtures/embedding/gleam && gleam format --check && gleam check)
```

Use a `geam` CLI built from the same Git commit for the standalone and embedding
checks. For a local, repository-scoped installation:

```sh
cargo install --git https://github.com/panarch/geam.git \
  --rev 76c4ab7c6a2c0c35975bdd97e5de7c6284f895e7 \
  --locked --root "$PWD/target/geam-cli" --bin geam geam
GEAM_BIN="$PWD/target/geam-cli/bin/geam"
```

## Standard Verification

Once the workspace is initialized, the baseline local and CI checks are:

```sh
cargo fmt --all --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked
```

The independent embedding fixture has its own Cargo lockfile and is not a root
workspace member. Check and run it separately:

```sh
(cd gleam-regexp/fixtures/embedding && cargo fmt --all --check)
(cd gleam-regexp/fixtures/embedding && "$GEAM_BIN" embedding check)
(cd gleam-regexp/fixtures/embedding && cargo test --locked)
(cd gleam-regexp/fixtures/embedding && cargo run --locked)
(cd gleam-regexp/fixtures/embedding && cargo clippy --all-targets --locked -- -D warnings)
```

The standalone fixture has its own Cargo lockfile too. Run `prepare`, `run`, and
`build`, then execute the built program from outside the fixture directory:

```sh
FIXTURE="$PWD/gleam-regexp/fixtures/gleam"
(cd "$FIXTURE" && "$GEAM_BIN" prepare)
(cd "$FIXTURE" && "$GEAM_BIN" run)
(cd "$FIXTURE" && "$GEAM_BIN" build)
(cd /tmp && "$FIXTURE/build/geam/target/debug/geam_regexp_fixture")
```

Inspect the production package's published-file view:

```sh
cargo package --list --package geam-regexp --locked
```

Confirm that the list contains `LICENSE` along with the manifest, README, and
provider source. The repository root and the package each carry the same
Apache-2.0 license text.

Use `--allow-dirty` before the first commit or when verifying uncommitted edits.
The current Git-pinned Geam dependency is not publishable through Cargo's
registry-only package resolution. Archive creation, registry publication, and
consumption of a published crate are a separate gate after Geam publishes the
required API and features. Do not treat `--list` as proof of registry readiness.

## GitHub Actions

The [CI workflow](../../.github/workflows/ci.yml) runs on pushes and pull
requests to `main`, and can be started manually. It has four independent jobs:

- `repository` checks the root license and local links in tracked Markdown
  files, including each provider's README.
- `quality` checks the shared Geam revision, original Gleam package source,
  both languages' formatting, the source-backed Rust tests, Clippy, Rust docs,
  and the provider's package file list.
- `coverage` downloads the original Gleam package, starts with a clean profile,
  and requires independent line and full-scope region coverage of 100% for
  `geam-regexp`.
- `integration` installs the Geam CLI from the pinned Git commit, checks the
  generated embedding bindings, tests and runs the Rust embedding consumer,
  and verifies standalone `prepare`, `run`, `build`, and execution outside the
  fixture.

The workflow needs only read access to the repository. It does not publish a
crate or assume that a Git-pinned provider can already be uploaded to crates.io.
After the repository is pushed, the hosted job results must be checked
separately; local command success does not establish a GitHub Actions run.

## Coverage

Geam Providers uses `cargo-llvm-cov` and requires 100% line and full-scope
region coverage for every production crate.

Install the local tooling:

```sh
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov --locked
```

The single production package is measured independently, with owner and
source-backed integration tests together:

```sh
cargo llvm-cov clean --workspace
cargo llvm-cov --package geam-regexp --locked \
  --json --summary-only --output-path target/gleam-regexp-coverage.json \
  --fail-under-lines 100 \
  --fail-under-regions 100
```

Read the package file entry in the JSON report to confirm line and region counts
for `gleam-regexp/src/lib.rs`. As the repository gains production crates, add an
independent closure for each one. A consumer may execute another crate's code,
but its coverage must not compensate for missing owner coverage.

When a gap is unclear, inspect region and monomorph detail for the affected
package:

```sh
cargo llvm-cov report --package geam-regexp \
  --text \
  --show-instantiations \
  --show-missing-lines
```

Generate a package-scoped HTML report from the same profile when source context
is easier to inspect visually:

```sh
cargo llvm-cov report --package geam-regexp --html
```

The report is written under `target/llvm-cov/html/`. Follow the diagnostic
process in [test-development.md](test-development.md) rather than treating each
uncovered region as an instruction to add another test.

## Keeping This Document Current

Whenever the workspace, provider layout, CI jobs, toolchain, or coverage closure
changes, update this document in the same commit. A command documented here
must either run as written or be explicitly identified as a future setup step.
