# Testing

This document defines the test roles and verification baseline for Geam
Providers. The current production packages are `geam-filepath`,
`geam-regexp`, `geam-otp`, `geam-houdini`, `geam-gzlib`, and `geam-simplifile`.
This document must remain the source of truth for the checks actually run.

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

The Rust workspace, fixture consumers, and report service example use Geam
`main` commit `76c4ab7c6a2c0c35975bdd97e5de7c6284f895e7`. Houdini also
uses the public `geam-core` byte-slice helper from that commit. The Gleam
projects resolve the unmodified `filepath` 1.1.2, `gleam_regexp` 1.1.1,
`gleam_otp` 1.3.0, `houdini` 1.2.0, `gzlib` 2.0.0, and `simplifile` 2.7.0
packages from Hex. The `simplifile` fixtures also select `geam-filepath` and
pin `gleam_stdlib` 1.0.3 for compatibility with this Geam commit. From the
repository root, download fixture dependencies before running source-backed
Rust tests:

```sh
(cd filepath/fixtures/gleam && gleam deps download)
(cd filepath/fixtures/embedding/gleam && gleam deps download)
(cd gleam-regexp/fixtures/gleam && gleam deps download)
(cd gleam-regexp/fixtures/embedding/gleam && gleam deps download)
(cd gleam-otp/fixtures/gleam && gleam deps download)
(cd gleam-otp/fixtures/embedding/gleam && gleam deps download)
(cd gleam-otp/examples/report_service && gleam deps download)
(cd houdini/fixtures/gleam && gleam deps download)
(cd houdini/fixtures/embedding/gleam && gleam deps download)
(cd gzlib/fixtures/gleam && gleam deps download)
(cd gzlib/fixtures/embedding/gleam && gleam deps download)
(cd simplifile/fixtures/gleam && gleam deps download)
(cd simplifile/fixtures/embedding/gleam && gleam deps download)
(cd filepath/fixtures/gleam && gleam format --check && gleam check)
(cd filepath/fixtures/embedding/gleam && gleam format --check && gleam check)
(cd gleam-regexp/fixtures/gleam && gleam format --check && gleam check)
(cd gleam-regexp/fixtures/embedding/gleam && gleam format --check && gleam check)
(cd gleam-otp/fixtures/gleam && gleam format --check && gleam check)
(cd gleam-otp/fixtures/embedding/gleam && gleam format --check && gleam check)
(cd gleam-otp/examples/report_service && gleam format --check && gleam check)
(cd gleam-otp/fixtures/gleam/build/packages/gleam_otp && shasum -a 256 -c ../../../../upstream.sha256)
(cd houdini/fixtures/gleam && gleam format --check && gleam check)
(cd houdini/fixtures/embedding/gleam && gleam format --check && gleam check)
(cd gzlib/fixtures/gleam/build/packages/gzlib && shasum -a 256 -c ../../../../upstream.sha256)
(cd gzlib/fixtures/gleam && gleam format --check && gleam check)
(cd gzlib/fixtures/embedding/gleam && gleam format --check && gleam check)
(cd simplifile/fixtures/gleam && gleam format --check && gleam check)
(cd simplifile/fixtures/embedding/gleam && gleam format --check && gleam check)
```

The Houdini standalone Gleam fixture can also run on Erlang with `gleam run`
to check the original FFI as an independent behavioral reference.

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

The independent embedding fixtures have their own Cargo lockfiles and are not
root workspace members. Check and run each separately:

```sh
(cd filepath/fixtures/embedding && cargo fmt --all --check)
(cd filepath/fixtures/embedding && "$GEAM_BIN" embedding check)
(cd filepath/fixtures/embedding && cargo test --locked)
(cd filepath/fixtures/embedding && cargo run --locked)
(cd filepath/fixtures/embedding && cargo clippy --all-targets --locked -- -D warnings)
(cd gleam-regexp/fixtures/embedding && cargo fmt --all --check)
(cd gleam-regexp/fixtures/embedding && "$GEAM_BIN" embedding check)
(cd gleam-regexp/fixtures/embedding && cargo test --locked)
(cd gleam-regexp/fixtures/embedding && cargo run --locked)
(cd gleam-regexp/fixtures/embedding && cargo clippy --all-targets --locked -- -D warnings)
(cd gleam-otp/fixtures/embedding && cargo fmt --all --check)
(cd gleam-otp/fixtures/embedding && "$GEAM_BIN" embedding check)
(cd gleam-otp/fixtures/embedding && cargo test --locked)
(cd gleam-otp/fixtures/embedding && cargo run --locked)
(cd gleam-otp/fixtures/embedding && cargo clippy --all-targets --locked -- -D warnings)
(cd gleam-otp/fixtures/embedding && RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --locked)
(cd houdini/fixtures/embedding && cargo fmt --all --check)
(cd houdini/fixtures/embedding && "$GEAM_BIN" embedding check)
(cd houdini/fixtures/embedding && cargo test --locked)
(cd houdini/fixtures/embedding && cargo run --locked)
(cd houdini/fixtures/embedding && cargo clippy --all-targets --locked -- -D warnings)
(cd houdini/fixtures/embedding && RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --locked)
(cd gzlib/fixtures/embedding && cargo fmt --all --check)
(cd gzlib/fixtures/embedding && "$GEAM_BIN" embedding check)
(cd gzlib/fixtures/embedding && cargo test --locked)
(cd gzlib/fixtures/embedding && cargo run --locked)
(cd gzlib/fixtures/embedding && cargo clippy --all-targets --locked -- -D warnings)
(cd gzlib/fixtures/embedding && RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --locked)
(cd simplifile/fixtures/embedding && cargo fmt --all --check)
(cd simplifile/fixtures/embedding && "$GEAM_BIN" embedding check)
(cd simplifile/fixtures/embedding && cargo test --locked)
(cd simplifile/fixtures/embedding && cargo run --locked)
(cd simplifile/fixtures/embedding && cargo clippy --all-targets --locked -- -D warnings)
(cd simplifile/fixtures/embedding && RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --locked)
```

Each standalone fixture has its own Cargo lockfile too. Run `prepare`, `run`,
and `build`, then execute each built program from outside its fixture directory:

```sh
FIXTURE="$PWD/filepath/fixtures/gleam"
(cd "$FIXTURE" && "$GEAM_BIN" prepare)
(cd "$FIXTURE" && "$GEAM_BIN" run)
(cd "$FIXTURE" && "$GEAM_BIN" build)
(cd /tmp && "$FIXTURE/build/geam/target/debug/geam_filepath_fixture")

FIXTURE="$PWD/gleam-regexp/fixtures/gleam"
(cd "$FIXTURE" && "$GEAM_BIN" prepare)
(cd "$FIXTURE" && "$GEAM_BIN" run)
(cd "$FIXTURE" && "$GEAM_BIN" build)
(cd /tmp && "$FIXTURE/build/geam/target/debug/geam_regexp_fixture")

FIXTURE="$PWD/gleam-otp/fixtures/gleam"
(cd "$FIXTURE" && "$GEAM_BIN" prepare)
(cd "$FIXTURE" && "$GEAM_BIN" run)
(cd "$FIXTURE" && "$GEAM_BIN" build)
(cd /tmp && "$FIXTURE/build/geam/target/debug/otp_service_fixture")

EXAMPLE="$PWD/gleam-otp/examples/report_service"
(cd "$EXAMPLE" && "$GEAM_BIN" prepare)
(cd "$EXAMPLE" && output=$("$GEAM_BIN" run) && diff -u expected-output.txt <(printf '%s\n' "$output"))

FIXTURE="$PWD/houdini/fixtures/gleam"
(cd "$FIXTURE" && "$GEAM_BIN" prepare)
(cd "$FIXTURE" && "$GEAM_BIN" run)
(cd "$FIXTURE" && "$GEAM_BIN" build)
(cd /tmp && "$FIXTURE/build/geam/target/debug/geam_houdini_fixture")

FIXTURE="$PWD/gzlib/fixtures/gleam"
(cd "$FIXTURE" && "$GEAM_BIN" prepare)
(cd "$FIXTURE" && "$GEAM_BIN" run)
(cd "$FIXTURE" && "$GEAM_BIN" build)
(cd /tmp && "$FIXTURE/build/geam/target/debug/geam_gzlib_fixture")

FIXTURE="$PWD/simplifile/fixtures/gleam"
(cd "$FIXTURE" && "$GEAM_BIN" prepare)
(cd "$FIXTURE" && "$GEAM_BIN" run)
(cd "$FIXTURE" && "$GEAM_BIN" build)
(cd /tmp && "$FIXTURE/build/geam/target/debug/geam_simplifile_fixture")
```

Inspect all production packages' published-file views:

```sh
cargo package --list --package geam-filepath --locked
cargo package --list --package geam-regexp --locked
cargo package --list --package geam-otp --locked
cargo package --list --package geam-houdini --locked
cargo package --list --package geam-gzlib --locked
cargo package --list --package geam-simplifile --locked
```

Confirm that each list contains `LICENSE` along with the manifest, README, and
provider source, and excludes fixtures and repository-only tests. The repository
root and each package carry the same Apache-2.0 license text.

Use `--allow-dirty` before the first commit or when verifying uncommitted edits.
The current Git-pinned Geam dependency is not publishable through Cargo's
registry-only package resolution. Archive creation, registry publication, and
consumption of a published crate are a separate gate after Geam publishes the
required API and features. Do not treat `--list` as proof of registry readiness.

## GitHub Actions

The [CI workflow](../../.github/workflows/ci.yml) runs on pushes and pull
requests to `main`, and can be started manually. Its `providers` job reads
workspace members marked with `[package.metadata.geam.provider]` through
`cargo metadata` and passes their crate names and directories to the other
jobs. CI fails if that provider list is empty. Each provider uses the standard
`<provider>/fixtures/embedding` and `<provider>/fixtures/gleam` layout; the
standalone executable defaults to the crate name with hyphens replaced by
underscores, followed by `_fixture`. A provider can override that name with
`[package.metadata.geam.ci]`'s `fixture-bin` field. `geam-otp` uses this for
`otp_service_fixture` and declares its runnable report service with
`example-dir`.

- `quality` checks the root license and Geam revision, downloads both Gleam
  fixture projects per provider, verifies any `fixtures/upstream.sha256`
  against the original Hex source, and checks workspace Rust formatting,
  tests, Clippy, and documentation once.
- `coverage` runs independently for each provider. Every matrix job downloads
  both fixture projects, starts with a clean profile, and requires 100% line
  and full-scope region coverage for that production crate and every file
  reported under its `src/` directory on each declared operating system.
- `integration` runs independently for each provider. Every matrix job checks
  fixture Geam revisions, Gleam source and embedding Rust formatting, the
  embedding consumer's Clippy and Rust documentation, and the package file
  list. It installs the pinned Geam CLI, checks generated bindings, tests and
  runs the embedding consumer, then verifies standalone `prepare`, `run`,
  `build`, and execution outside the fixture. If `example-dir` is set, it
  checks the example's Geam revision and Gleam source, then compares its
  output with `expected-output.txt`.

To register another provider in CI, add its crate to the Cargo workspace,
declare its Gleam package in `[package.metadata.geam.provider]`, and provide
both standard fixtures. The workflow does not need another package-specific
entry. If its fixture executable differs from the default or it has a runnable
example, declare those paths in `[package.metadata.geam.ci]`. Its optional
`runners` list declares operating systems; without it, the provider runs on
`ubuntu-24.04`. The discovery job builds provider-by-runner rows. Currently
`geam-simplifile` declares `ubuntu-24.04`, `macos-15`, and `windows-2025`;
the other providers keep the Ubuntu default. `quality` runs once on Ubuntu,
while `coverage` and `integration` run for every row. Local Markdown links are
not checked by this workflow.

The `filepath` fixtures check the active host's `split` branch and both
explicit split functions. A `simplifile` Windows row also exercises its
`filepath` dependency. Declaring a runner does not establish a passing CI
result; inspect the hosted jobs before claiming platform verification.

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

Each production package is measured independently, with owner and source-backed
integration tests together:

```sh
cargo llvm-cov clean --workspace
cargo llvm-cov --package geam-filepath --locked \
  --json --summary-only --output-path target/filepath-coverage.json \
  --fail-under-lines 100 \
  --fail-under-regions 100
cargo llvm-cov clean --workspace
cargo llvm-cov --package geam-regexp --locked \
  --json --summary-only --output-path target/gleam-regexp-coverage.json \
  --fail-under-lines 100 \
  --fail-under-regions 100
cargo llvm-cov clean --workspace
cargo llvm-cov --package geam-otp --locked \
  --json --summary-only --output-path target/gleam-otp-coverage.json \
  --fail-under-lines 100 \
  --fail-under-regions 100
cargo llvm-cov clean --workspace
cargo llvm-cov --package geam-houdini --locked \
  --json --summary-only --output-path target/houdini-coverage.json \
  --fail-under-lines 100 \
  --fail-under-regions 100
cargo llvm-cov clean --workspace
cargo llvm-cov --package geam-simplifile --locked \
  --json --summary-only --output-path target/simplifile-coverage.json \
  --fail-under-lines 100 \
  --fail-under-regions 100
cargo llvm-cov clean --workspace
cargo llvm-cov --package geam-gzlib --locked \
  --json --summary-only --output-path target/gzlib-coverage.json \
  --fail-under-lines 100 \
  --fail-under-regions 100
```

Read each package file entry in its JSON report to confirm line and region counts
for every reported file under that provider's `src/` directory, including
test-only support when present. As the repository gains production crates, add
an independent closure for each one. A consumer may execute another crate's
code, but its coverage must not compensate for missing owner coverage. Run the
same closure on every declared OS so conditional source paths are included.

For Houdini and gzlib, confirm the `houdini/src/lib.rs` and `gzlib/src/lib.rs`
file counts directly.

When a gap is unclear, inspect region and monomorph detail for the affected
package:

```sh
cargo llvm-cov report --package geam-regexp \
  --text \
  --show-instantiations \
  --show-missing-lines
```

Substitute `geam-filepath`, `geam-otp`, `geam-houdini`, `geam-gzlib`, or
`geam-simplifile` when
inspecting those crates.

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
