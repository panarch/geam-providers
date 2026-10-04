# Testing

This document defines the test roles and verification baseline for Geam
Providers. The current production packages are `geam-filepath`,
`geam-regexp`, `geam-otp`, `geam-houdini`, `geam-gzlib`, `geam-crypto`,
`geam-simplifile`, `geam-platform`, `geam-logging`, `geam-term-size`,
`geam-birl`, `geam-httpc`, `geam-global-value`, `geam-argv`, `geam-splitter`,
`geam-envoy`, `geam-operating-system`, `geam-exception`, and `geam-glisten`.
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

### Cross-Provider Package Integrations

An integration under [`integrations/`](../../integrations/) executes one
unchanged Gleam package that composes existing providers. It is not a provider
workspace member or a substitute for the providers' owner and source-backed
contract tests. The [`directories` 1.2.0 case](../../integrations/directories/README.md)
selects `geam-envoy`, `geam-platform`, `geam-simplifile`, and `geam-filepath`.
Its common Gleam scenario runs on original Erlang and Geam standalone, while
the Rust embedding consumer also checks isolated initialization and state.
The package's 11 public functions are checked against exact OS-specific
expectations, including expected errors and representative fallback paths.
The [`clip` 1.2.2 case](../../integrations/clip/README.md) runs the complete
package contract and a file-search CLI with `geam-argv`, `geam-simplifile`,
`geam-filepath`, and `geam-regexp`. Its embedding host owns argument snapshots,
IO and temporary files, and checks structured outcomes through typed Gleam
projections, repeated execution, state isolation and recovery after errors.
An unpublished, application-owned native fixture maps the CLI's exit request
to Geam's public `Call::exit` API. The embedding host checks `Exited(1/2)`,
output before exit, prevented continuation, and fresh scopes after exit.

## Dependency Preparation

The Rust workspace, fixture consumers, and report service example use Geam
`main` commit `e5e1f5f772c6f48369050bdf3ee35c7a324277e2`. Houdini also
uses the public `geam-core` byte-slice helper from that commit. The Gleam
projects resolve the unmodified `filepath` 1.1.2, `gleam_regexp` 1.1.1,
`gleam_otp` 1.3.0, `houdini` 1.2.0, `gzlib` 2.0.0, `gleam_crypto` 1.6.0,
`simplifile` 2.7.0, `platform` 1.0.0, `logging` 1.5.0, `term_size` 1.0.1,
`birl` 2.0.0, `gleam_httpc` 5.0.0, `global_value` 1.0.0, `argv` 1.1.0,
`splitter` 1.3.0, `envoy` 1.2.0, `operating_system` 1.0.1, `exception` 2.1.1,
and `glisten` 9.0.1 packages from Hex.
The crypto, simplifile, logging, term_size, birl, global_value, splitter,
envoy, and exception fixtures and the `operating_system` standalone fixture pin
`gleam_stdlib` 1.0.3 for compatibility with this Geam commit; the simplifile
fixtures also select `geam-filepath`. Resolving crypto with stdlib 1.0.5 failed
at the `gleam/bit_array.pad_to_bytes` linkage check. From the
repository root, download fixture dependencies before running source-backed
Rust tests:

```sh
(cd filepath/fixtures/gleam && gleam deps download)
(cd filepath/fixtures/embedding/gleam && gleam deps download)
(cd gleam-regexp/fixtures/gleam && gleam deps download)
(cd gleam-regexp/fixtures/embedding/gleam && gleam deps download)
(cd gleam-otp/fixtures/gleam && gleam deps download)
(cd gleam-otp/fixtures/embedding/gleam && gleam deps download)
(cd gleam-crypto/fixtures/gleam && gleam deps download)
(cd gleam-crypto/fixtures/embedding/gleam && gleam deps download)
(cd gleam-otp/examples/report_service && gleam deps download)
(cd houdini/fixtures/gleam && gleam deps download)
(cd houdini/fixtures/embedding/gleam && gleam deps download)
(cd gzlib/fixtures/gleam && gleam deps download)
(cd gzlib/fixtures/embedding/gleam && gleam deps download)
(cd simplifile/fixtures/gleam && gleam deps download)
(cd simplifile/fixtures/embedding/gleam && gleam deps download)
(cd platform/fixtures/gleam && gleam deps download)
(cd platform/fixtures/embedding/gleam && gleam deps download)
(cd logging/fixtures/gleam && gleam deps download)
(cd logging/fixtures/embedding/gleam && gleam deps download)
(cd term-size/fixtures/gleam && gleam deps download)
(cd term-size/fixtures/embedding/gleam && gleam deps download)
(cd birl/fixtures/gleam && gleam deps download)
(cd birl/fixtures/embedding/gleam && gleam deps download)
(cd global-value/fixtures/gleam && gleam deps download)
(cd global-value/fixtures/embedding/gleam && gleam deps download)
(cd gleam-httpc/fixtures/gleam && gleam deps download)
(cd gleam-httpc/fixtures/embedding/gleam && gleam deps download)
(cd argv/fixtures/gleam && gleam deps download)
(cd argv/fixtures/embedding/gleam && gleam deps download)
(cd splitter/fixtures/gleam && gleam deps download)
(cd splitter/fixtures/embedding/gleam && gleam deps download)
(cd envoy/fixtures/gleam && gleam deps download)
(cd envoy/fixtures/embedding/gleam && gleam deps download)
(cd operating-system/fixtures/gleam && gleam deps download)
(cd operating-system/fixtures/embedding/gleam && gleam deps download)
(cd exception/fixtures/gleam && gleam deps download)
(cd exception/fixtures/embedding/gleam && gleam deps download)
(cd filepath/fixtures/gleam && gleam format --check && gleam check)
(cd filepath/fixtures/embedding/gleam && gleam format --check && gleam check)
(cd gleam-regexp/fixtures/gleam && gleam format --check && gleam check)
(cd gleam-regexp/fixtures/embedding/gleam && gleam format --check && gleam check)
(cd gleam-otp/fixtures/gleam && gleam format --check && gleam check)
(cd gleam-otp/fixtures/embedding/gleam && gleam format --check && gleam check)
(cd gleam-crypto/fixtures/gleam && gleam format --check && gleam check)
(cd gleam-crypto/fixtures/embedding/gleam && gleam format --check && gleam check)
(cd gleam-otp/examples/report_service && gleam format --check && gleam check)
(cd gleam-otp/fixtures/gleam/build/packages/gleam_otp && shasum -a 256 -c ../../../../upstream.sha256)
(cd houdini/fixtures/gleam && gleam format --check && gleam check)
(cd houdini/fixtures/embedding/gleam && gleam format --check && gleam check)
(cd gleam-crypto/fixtures/gleam/build/packages/gleam_crypto && shasum -a 256 -c ../../../../upstream.sha256)
(cd gzlib/fixtures/gleam/build/packages/gzlib && shasum -a 256 -c ../../../../upstream.sha256)
(cd gzlib/fixtures/gleam && gleam format --check && gleam check)
(cd gzlib/fixtures/embedding/gleam && gleam format --check && gleam check)
(cd simplifile/fixtures/gleam && gleam format --check && gleam check)
(cd simplifile/fixtures/embedding/gleam && gleam format --check && gleam check)
(cd platform/fixtures/gleam && gleam format --check && gleam check)
(cd platform/fixtures/embedding/gleam && gleam format --check && gleam check)
(cd platform/fixtures/gleam/build/packages/platform && shasum -a 256 -c ../../../../upstream.sha256)
(cd logging/fixtures/gleam && gleam format --check && gleam check)
(cd logging/fixtures/embedding/gleam && gleam format --check && gleam check)
(cd logging/fixtures/gleam/build/packages/logging && shasum -a 256 -c ../../../../upstream.sha256)
(cd term-size/fixtures/gleam/build/packages/term_size && shasum -a 256 -c ../../../../upstream.sha256)
(cd term-size/fixtures/gleam && gleam format --check && gleam check)
(cd term-size/fixtures/embedding/gleam && gleam format --check && gleam check)
(cd birl/fixtures/gleam/build/packages/birl && shasum -a 256 -c ../../../../upstream.sha256)
(cd birl/fixtures/gleam && gleam format --check && gleam check)
(cd birl/fixtures/embedding/gleam && gleam format --check && gleam check)
(cd global-value/fixtures/gleam/build/packages/global_value && shasum -a 256 -c ../../../../upstream.sha256)
(cd global-value/fixtures/gleam && gleam format --check && gleam check)
(cd global-value/fixtures/embedding/gleam && gleam format --check && gleam check)
(cd gleam-httpc/fixtures/gleam && gleam format --check && gleam check)
(cd gleam-httpc/fixtures/embedding/gleam && gleam format --check && gleam check)
(cd argv/fixtures/gleam && gleam format --check && gleam check)
(cd argv/fixtures/embedding/gleam && gleam format --check && gleam check)
(cd argv/fixtures/gleam/build/packages/argv && shasum -a 256 -c ../../../../upstream.sha256)
(cd splitter/fixtures/gleam && gleam format --check && gleam check)
(cd splitter/fixtures/embedding/gleam && gleam format --check && gleam check)
(cd splitter/fixtures/gleam/build/packages/splitter && shasum -a 256 -c ../../../../upstream.sha256)
(cd gleam-httpc/fixtures/gleam/build/packages/gleam_httpc && shasum -a 256 -c ../../../../upstream.sha256)
(cd envoy/fixtures/gleam/build/packages/envoy && shasum -a 256 -c ../../../../upstream.sha256)
(cd envoy/fixtures/gleam && gleam format --check && gleam check)
(cd envoy/fixtures/embedding/gleam && gleam format --check && gleam check)
(cd operating-system/fixtures/gleam/build/packages/operating_system && shasum -a 256 -c ../../../../upstream.sha256)
(cd operating-system/fixtures/gleam && gleam format --check && gleam check)
(cd operating-system/fixtures/embedding/gleam && gleam format --check && gleam check)
(cd exception/fixtures/gleam/build/packages/exception && shasum -a 256 -c ../../../../upstream.sha256)
(cd exception/fixtures/gleam && gleam format --check && gleam check)
(cd exception/fixtures/embedding/gleam && gleam format --check && gleam check)
```

When updating the common Geam revision, rebuild the pinned CLI and synchronize
the OTP and httpc prepared embedding data before running the Rust workspace
checks. These generated programs target that commit's prepared representation:

```sh
(cd gleam-otp/fixtures/embedding && "$GEAM_BIN" embedding sync)
(cd gleam-httpc/fixtures/embedding && "$GEAM_BIN" embedding sync)
```

Commit the generated changes together with the pin; use `embedding check` to
verify that they match the fixed CLI, and never edit prepared data by hand.

The platform, birl, splitter, envoy, `operating_system`, and exception
standalone Gleam fixtures can also run on Erlang with `gleam run` to check the
original FFI as an independent behavioral reference. The envoy Erlang fixture
uses an ASCII value; its embedding fixture separately checks Unicode values
through Geam.
The platform fixture asserts the macOS ARM64 or Linux x86_64 public result;
the birl fixture checks portable date calculations and live clock calls:

```sh
(cd platform/fixtures/gleam && gleam run)
(cd birl/fixtures/gleam && gleam run)
(cd splitter/fixtures/gleam && gleam run)
(cd envoy/fixtures/gleam && gleam run)
(cd operating-system/fixtures/gleam && gleam run)
(cd exception/fixtures/gleam && gleam run)
```

Use a `geam` CLI built from the same Git commit for the standalone and embedding
checks. Build it as a workspace so Geam and `geam-core` both come from that
commit (a Git-package `cargo install` can resolve a published `geam-core`):

```sh
git init -q target/geam-source
git -C target/geam-source fetch --depth=1 https://github.com/panarch/geam.git e5e1f5f772c6f48369050bdf3ee35c7a324277e2
git -C target/geam-source checkout --detach -q FETCH_HEAD
CARGO_TARGET_DIR="$PWD/target/geam-cli-build" \
  cargo build --manifest-path "$PWD/target/geam-source/Cargo.toml" \
  --locked --release --bin geam
export GEAM_BIN="$PWD/target/geam-cli-build/release/geam"
export GEAM_REV=e5e1f5f772c6f48369050bdf3ee35c7a324277e2
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
(cd gleam-crypto/fixtures/embedding && cargo fmt --all --check)
(cd gleam-crypto/fixtures/embedding && "$GEAM_BIN" embedding check)
(cd gleam-crypto/fixtures/embedding && cargo test --locked)
(cd gleam-crypto/fixtures/embedding && cargo run --locked)
(cd gleam-crypto/fixtures/embedding && cargo clippy --all-targets --locked -- -D warnings)
(cd gleam-crypto/fixtures/embedding && RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --locked)
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
(cd platform/fixtures/embedding && cargo fmt --all --check)
(cd platform/fixtures/embedding && "$GEAM_BIN" embedding check)
(cd platform/fixtures/embedding && cargo test --locked)
(cd platform/fixtures/embedding && cargo run --locked)
(cd platform/fixtures/embedding && cargo clippy --all-targets --locked -- -D warnings)
(cd platform/fixtures/embedding && RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --locked)
(cd logging/fixtures/embedding && cargo fmt --all --check)
(cd logging/fixtures/embedding && "$GEAM_BIN" embedding check)
(cd logging/fixtures/embedding && cargo test --locked)
(cd logging/fixtures/embedding && cargo run --locked)
(cd logging/fixtures/embedding && cargo clippy --all-targets --locked -- -D warnings)
(cd logging/fixtures/embedding && RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --locked)
(cd term-size/fixtures/embedding && cargo fmt --all --check)
(cd term-size/fixtures/embedding && "$GEAM_BIN" embedding check)
(cd term-size/fixtures/embedding && cargo test --locked)
(cd term-size/fixtures/embedding && cargo run --locked)
(cd term-size/fixtures/embedding && cargo clippy --all-targets --locked -- -D warnings)
(cd term-size/fixtures/embedding && RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --locked)
(cd birl/fixtures/embedding && cargo fmt --all --check)
(cd birl/fixtures/embedding && "$GEAM_BIN" embedding check)
(cd birl/fixtures/embedding && cargo test --locked)
(cd birl/fixtures/embedding && cargo run --locked)
(cd birl/fixtures/embedding && cargo clippy --all-targets --locked -- -D warnings)
(cd birl/fixtures/embedding && RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --locked)
(cd global-value/fixtures/embedding && cargo fmt --all --check)
(cd global-value/fixtures/embedding && "$GEAM_BIN" embedding check)
(cd global-value/fixtures/embedding && cargo test --locked)
(cd global-value/fixtures/embedding && cargo run --locked)
(cd global-value/fixtures/embedding && cargo clippy --all-targets --locked -- -D warnings)
(cd global-value/fixtures/embedding && RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --locked)
(cd gleam-httpc/fixtures/embedding && cargo fmt --all --check)
(cd gleam-httpc/fixtures/embedding && "$GEAM_BIN" embedding check)
(cd gleam-httpc/fixtures/embedding && cargo test --locked)
(cd gleam-httpc/fixtures/embedding && cargo run --locked)
(cd gleam-httpc/fixtures/embedding && cargo clippy --all-targets --locked -- -D warnings)
(cd gleam-httpc/fixtures/embedding && RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --locked)
(cd argv/fixtures/embedding && cargo fmt --all --check)
(cd argv/fixtures/embedding && "$GEAM_BIN" embedding check)
(cd argv/fixtures/embedding && cargo test --locked)
(cd argv/fixtures/embedding && cargo run --locked)
(cd argv/fixtures/embedding && cargo clippy --all-targets --locked -- -D warnings)
(cd argv/fixtures/embedding && RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --locked)
(cd splitter/fixtures/embedding && cargo fmt --all --check)
(cd splitter/fixtures/embedding && "$GEAM_BIN" embedding check)
(cd splitter/fixtures/embedding && cargo test --locked)
(cd splitter/fixtures/embedding && cargo run --locked)
(cd splitter/fixtures/embedding && cargo clippy --all-targets --locked -- -D warnings)
(cd splitter/fixtures/embedding && RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --locked)
(cd envoy/fixtures/embedding && cargo fmt --all --check)
(cd envoy/fixtures/embedding && "$GEAM_BIN" embedding check)
(cd envoy/fixtures/embedding && cargo test --locked)
(cd envoy/fixtures/embedding && cargo run --locked)
(cd envoy/fixtures/embedding && cargo clippy --all-targets --locked -- -D warnings)
(cd envoy/fixtures/embedding && RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --locked)
(cd operating-system/fixtures/embedding && cargo fmt --all --check)
(cd operating-system/fixtures/embedding && "$GEAM_BIN" embedding check)
(cd operating-system/fixtures/embedding && cargo test --locked)
(cd operating-system/fixtures/embedding && cargo run --locked)
(cd operating-system/fixtures/embedding && cargo clippy --all-targets --locked -- -D warnings)
(cd operating-system/fixtures/embedding && RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --locked)
(cd exception/fixtures/embedding && cargo fmt --all --check)
(cd exception/fixtures/embedding && "$GEAM_BIN" embedding check)
(cd exception/fixtures/embedding && cargo test --locked)
(cd exception/fixtures/embedding && cargo run --locked)
(cd exception/fixtures/embedding && cargo clippy --all-targets --locked -- -D warnings)
(cd exception/fixtures/embedding && RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --locked)
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

FIXTURE="$PWD/gleam-crypto/fixtures/gleam"
(cd "$FIXTURE" && "$GEAM_BIN" prepare)
(cd "$FIXTURE" && "$GEAM_BIN" run)
(cd "$FIXTURE" && "$GEAM_BIN" build)
(cd /tmp && "$FIXTURE/build/geam/target/debug/geam_crypto_fixture")

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

FIXTURE="$PWD/platform/fixtures/gleam"
(cd "$FIXTURE" && "$GEAM_BIN" prepare)
(cd "$FIXTURE" && "$GEAM_BIN" run)
(cd "$FIXTURE" && "$GEAM_BIN" build)
(cd /tmp && "$FIXTURE/build/geam/target/debug/geam_platform_fixture")

FIXTURE="$PWD/logging/fixtures/gleam"
(cd "$FIXTURE" && "$GEAM_BIN" prepare)
(cd "$FIXTURE" && "$GEAM_BIN" run)
(cd "$FIXTURE" && "$GEAM_BIN" build)
(cd /tmp && "$FIXTURE/build/geam/target/debug/geam_logging_fixture")

FIXTURE="$PWD/term-size/fixtures/gleam"
(cd "$FIXTURE" && "$GEAM_BIN" prepare)
(cd "$FIXTURE" && "$GEAM_BIN" run)
(cd "$FIXTURE" && "$GEAM_BIN" build)
(cd /tmp && "$FIXTURE/build/geam/target/debug/geam_term_size_fixture")

FIXTURE="$PWD/birl/fixtures/gleam"
(cd "$FIXTURE" && "$GEAM_BIN" prepare)
(cd "$FIXTURE" && "$GEAM_BIN" run)
(cd "$FIXTURE" && "$GEAM_BIN" build)
(cd /tmp && "$FIXTURE/build/geam/target/debug/geam_birl_fixture")

FIXTURE="$PWD/global-value/fixtures/gleam"
(cd "$FIXTURE" && "$GEAM_BIN" prepare)
(cd "$FIXTURE" && "$GEAM_BIN" run)
(cd "$FIXTURE" && "$GEAM_BIN" build)
(cd /tmp && "$FIXTURE/build/geam/target/debug/geam_global_value_fixture")

FIXTURE="$PWD/gleam-httpc/fixtures/gleam"
(cd "$FIXTURE" && "$GEAM_BIN" prepare)
(cd "$FIXTURE" && for module in geam_httpc_fixture httpc_redirect httpc_binary httpc_tls; do
  python3 ../server.py "$GEAM_BIN" run --module "$module" \
    --provider-config gleam_httpc=../config/provider.toml
done)
(cd "$FIXTURE" && "$GEAM_BIN" build)
(cd /tmp && GEAM_CONFIG="$FIXTURE/../config/runtime.toml" \
  python3 "$FIXTURE/../server.py" \
  "$FIXTURE/build/geam/target/debug/geam_httpc_fixture")

FIXTURE="$PWD/argv/fixtures/gleam"
(cd "$FIXTURE" && "$GEAM_BIN" prepare)
(cd "$FIXTURE" && "$GEAM_BIN" run)
(cd "$FIXTURE" && "$GEAM_BIN" run --)
(cd "$FIXTURE" && "$GEAM_BIN" run -- --flag '' key=value 한글)
(cd "$FIXTURE" && gleam run && gleam run -- --flag '' key=value 한글)
(cd "$FIXTURE" && "$GEAM_BIN" build)
(cd /tmp && "$FIXTURE/build/geam/target/debug/geam_argv_fixture")
(cd /tmp && "$FIXTURE/build/geam/target/debug/geam_argv_fixture" --flag '' key=value 한글)

FIXTURE="$PWD/splitter/fixtures/gleam"
(cd "$FIXTURE" && "$GEAM_BIN" prepare)
(cd "$FIXTURE" && "$GEAM_BIN" run)
(cd "$FIXTURE" && "$GEAM_BIN" build)
(cd /tmp && "$FIXTURE/build/geam/target/debug/geam_splitter_fixture")

FIXTURE="$PWD/envoy/fixtures/gleam"
(cd "$FIXTURE" && "$GEAM_BIN" prepare)
(cd "$FIXTURE" && "$GEAM_BIN" run)
(cd "$FIXTURE" && "$GEAM_BIN" build)
(cd /tmp && "$FIXTURE/build/geam/target/debug/geam_envoy_fixture")

FIXTURE="$PWD/operating-system/fixtures/gleam"
(cd "$FIXTURE" && "$GEAM_BIN" prepare)
(cd "$FIXTURE" && "$GEAM_BIN" run)
(cd "$FIXTURE" && "$GEAM_BIN" build)
(cd /tmp && "$FIXTURE/build/geam/target/debug/geam_operating_system_fixture")

FIXTURE="$PWD/exception/fixtures/gleam"
(cd "$FIXTURE" && "$GEAM_BIN" prepare)
(cd "$FIXTURE" && "$GEAM_BIN" run)
(cd "$FIXTURE" && "$GEAM_BIN" build)
(cd /tmp && "$FIXTURE/build/geam/target/debug/geam_exception_fixture")
```

The `gleam-httpc` local server script binds only loopback, starts HTTP and
HTTPS before each command, and shuts both down afterwards. Its checked-in CA
and server key are for tests only. The fixture uses separate entry modules for
HTTP, redirect, binary, and TLS scenarios so each generated runner stays
within the pinned Geam compiler's recursion limit.

Inspect all production packages' published-file views:

```sh
cargo package --list --package geam-filepath --locked
cargo package --list --package geam-regexp --locked
cargo package --list --package geam-otp --locked
cargo package --list --package geam-houdini --locked
cargo package --list --package geam-crypto --locked
cargo package --list --package geam-gzlib --locked
cargo package --list --package geam-simplifile --locked
cargo package --list --package geam-platform --locked
cargo package --list --package geam-logging --locked
cargo package --list --package geam-term-size --locked
cargo package --list --package geam-birl --locked
cargo package --list --package geam-global-value --locked
cargo package --list --package geam-httpc --locked
cargo package --list --package geam-argv --locked
cargo package --list --package geam-splitter --locked
cargo package --list --package geam-envoy --locked
cargo package --list --package geam-operating-system --locked
cargo package --list --package geam-exception --locked
cargo package --list --package geam-glisten --locked
```

Confirm that each list contains `LICENSE` along with the manifest, README, and
provider source, and excludes fixtures and repository-only tests. The repository
root and each package carry the same Apache-2.0 license text.

Use `--allow-dirty` before the first commit or when verifying uncommitted edits.
The current Git-pinned Geam dependency is not publishable through Cargo's
registry-only package resolution. Archive creation, registry publication, and
consumption of a published crate are a separate gate after Geam publishes the
required API and features. Do not treat `--list` as proof of registry readiness.

### Glisten TCP And TLS Servers

`glisten/fixtures/gleam` pins the unchanged Hex `glisten` 9.0.1 package.
Its mandatory entry runs low-level TCP, two concurrent connections through the
original acceptor pool without a user message type annotation, a user selector,
a supervised IPv6 server, TLS with server-preferred ALPN `h2`, no client ALPN,
and ALPN mismatch. [The client driver](../../glisten/fixtures/run.py) waits for
an explicit port-0 readiness record, sends the exact client input, and requires
each matching completion record. It owns and cleans up the child process; no
fixed port or startup sleep is used.

The [owner hook](../../glisten/fixtures/ci.sh) handles `erlang`, `embedding`,
`standalone`, and `executable`. It uses the common runner's native Windows path
conversion and outside-fixture working directory. Preparation, builds, source
and Rust quality, binding checks, tests, package checks, and the 100% line and
region gate remain mandatory common workflow steps. CI declares Linux, macOS,
and Windows through [Cargo metadata](../../glisten/Cargo.toml), with no glisten
branch in the shared workflow.

From the repository root, export the pinned CLI as `GEAM_BIN` as described in
[dependency preparation](#dependency-preparation), then run:

```sh
(cd glisten/fixtures/gleam && gleam deps download && gleam format --check && gleam check)
bash .github/scripts/run_ci.sh provider glisten erlang
(cd glisten/fixtures/gleam && "$GEAM_BIN" prepare)
bash .github/scripts/run_ci.sh provider glisten standalone
(cd glisten/fixtures/gleam && "$GEAM_BIN" build)
bash .github/scripts/run_ci.sh provider glisten executable
(cd glisten/fixtures/embedding/gleam && gleam deps download && gleam format --check && gleam check)
(cd glisten/fixtures/embedding && "$GEAM_BIN" embedding check)
(cd glisten/fixtures/embedding && cargo fmt --all --check)
(cd glisten/fixtures/embedding && cargo clippy --all-targets --locked -- -D warnings)
(cd glisten/fixtures/embedding && RUSTDOCFLAGS='-D warnings' cargo doc --no-deps --locked)
(cd glisten/fixtures/embedding && cargo test --locked)
bash .github/scripts/run_ci.sh provider glisten embedding
```

The hook supplies the absolute certificate/key arguments for Erlang and
standalone execution. Those public fixture credentials are test data.
Do not run the server entry without the client driver: it waits for client IO.
The embedding integration test starts its consumer through the same driver and
requires all seven scenarios in live and freshly generated prepared execution.
The execution host enables time only; Glisten owns its explicitly selected IO
reactor. Generated bindings and the prepared program are checked in together.

## Package Integration Verification

The `directories` integration has independent Gleam and Rust lockfiles under
[`integrations/directories/fixtures/`](../../integrations/directories/fixtures/).
After building the pinned CLI above, run the original Erlang reference and both
Geam consumers from the repository root:

```sh
(cd integrations/directories/fixtures/gleam && gleam deps download && gleam format --check && gleam check)
(cd integrations/directories/fixtures/embedding/gleam && gleam deps download && gleam format --check && gleam check)
(cd integrations/directories/fixtures/gleam/build/packages/directories && if command -v sha256sum >/dev/null; then sha256sum --check ../../../../upstream.sha256; else shasum -a 256 -c ../../../../upstream.sha256; fi)
(cd integrations/directories/fixtures/gleam && gleam run && "$GEAM_BIN" prepare && "$GEAM_BIN" run && "$GEAM_BIN" build)
(cd integrations/directories/fixtures/embedding && cargo fmt --all --check && "$GEAM_BIN" embedding check && cargo test --locked && cargo run --locked)
(cd integrations/directories/fixtures/embedding && cargo clippy --all-targets --locked -- -D warnings)
(cd integrations/directories/fixtures/embedding && RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --locked)
```

Also execute `integrations/directories/fixtures/gleam/build/geam/target/debug/geam_directories_fixture`
from outside that fixture. Use `.exe` on Windows. The standalone and Erlang
executions create and delete a directory tree beneath the caller's working
directory; the embedding consumer uses a Rust temporary directory. The
original Erlang run is an independent behavioral reference, not a Geam
execution mode.

The `clip` case has separate package-contract, CLI, and embedding consumers:

```sh
for fixture in gleam cli embedding/gleam; do
  (cd "integrations/clip/fixtures/$fixture" && gleam deps download && gleam format --check && gleam check)
done
clip_checksum="$PWD/integrations/clip/fixtures/upstream.sha256"
for fixture in gleam cli embedding/gleam; do
  (cd "integrations/clip/fixtures/$fixture/build/packages/clip" && if command -v sha256sum >/dev/null; then sha256sum --check "$clip_checksum"; else shasum -a 256 -c "$clip_checksum"; fi)
done
(cd integrations/clip/fixtures/gleam && gleam run && "$GEAM_BIN" prepare && "$GEAM_BIN" run && "$GEAM_BIN" build)
python3 integrations/clip/fixtures/cli/check_cli.py --contracts --erlang-project integrations/clip/fixtures/gleam
python3 integrations/clip/fixtures/cli/check_cli.py --contracts --executable integrations/clip/fixtures/gleam/build/geam/target/debug/geam_clip_fixture
(cd integrations/clip/fixtures/cli && gleam run -- --help)
python3 integrations/clip/fixtures/cli/check_cli.py --erlang-project integrations/clip/fixtures/cli
(cd integrations/clip/fixtures/cli && "$GEAM_BIN" prepare && "$GEAM_BIN" run -m argv_contracts -- '' 'space value' -- -dash 한글 && "$GEAM_BIN" run -- search -p Rust ../../README.md && "$GEAM_BIN" build)
python3 integrations/clip/fixtures/cli/check_cli.py --executable integrations/clip/fixtures/cli/build/geam/target/debug/geam_clip_search
python3 integrations/clip/fixtures/cli/check_cli.py --geam-project integrations/clip/fixtures/cli --geam-bin "$GEAM_BIN"
(cd integrations/clip/fixtures/native && cargo fmt --all --check && cargo clippy --all-targets --locked -- -D warnings)
(cd integrations/clip/fixtures/native && RUSTDOCFLAGS='-D warnings' cargo doc --no-deps --locked)
(cd integrations/clip/fixtures/embedding && cargo fmt --all --check && "$GEAM_BIN" embedding check && cargo test --locked && cargo run --locked)
(cd integrations/clip/fixtures/embedding && cargo clippy --all-targets --locked -- -D warnings)
(cd integrations/clip/fixtures/embedding && RUSTDOCFLAGS='-D warnings' cargo doc --no-deps --locked)
```

Execute `integrations/clip/fixtures/gleam/build/geam/target/debug/geam_clip_fixture`
from outside the project as well. Use `.exe` on Windows; when invoking a native
Windows Python from Bash, pass native paths (`cygpath -w`) for absolute project
and executable arguments. The CLI harness creates and removes its own corpus
outside the project and checks exact exit status and both streams. It runs 32
portable cases and a native backslash-path case on Windows. Successful runs,
no matches, and help exit with 0; file failures exit with 1; argument and
pattern failures exit with 2. Ten additional `geam run` cases use absolute
corpus paths because Geam launches from the project directory. They require
the runner phase and successful Cargo build, then compare the complete app
stderr, stdout and exact status; runner diagnostics after the build fail the
check. Regexp behavior follows the
`geam-regexp` backend contract. The four providers retain their independent
100% line and full-scope region coverage gates; this integration has no new
production crate and does not replace those owner suites.

## GitHub Actions

The [CI workflow](../../.github/workflows/ci.yml) runs for main pushes, pull
requests and manual dispatch. Discovery reads provider Cargo metadata and each
integration's `ci.json`; target-owned Bash scripts contain custom execution
commands. Discovery also requires actionlint, ShellCheck and selector/runner
contract tests. See [CI registration and execution](ci.md) for declaration fields,
phase contracts, local commands, selection and cache behavior.

- `quality` runs once on Ubuntu for the entire workspace: license and Geam
  revision, original package downloads/checksums, Rust formatting, tests,
  Clippy and documentation.
- `validate` runs each selected provider on its declared runners. It first
  requires 100% production line and full-scope region coverage for every
  reported `src/` file, then checks pins, source, embedding formatting/Clippy/
  docs, package contents, generated bindings and embedding tests. The execution
  runner invokes the default command or declared owner hook for Erlang,
  embedding, standalone and outside-fixture execution. Preparation, build and
  runnable-example checks remain common mandatory steps.
- `integrations` groups selected cases by declared OS. It builds the pinned
  CLI once per OS and calls each case's source, Erlang, embedding, standalone
  and outside-fixture phases. Cases retain original package checksums and their
  source-backed scenarios.

Provider and fixture dependency changes select affected providers and cases.
Pure new target additions can stay scoped; shared execution changes, existing
lock record changes, unknown inputs and unavailable analysis use conservative
fallbacks. Ordinary CI still runs workspace quality even when the repeated
provider matrix is scoped. A runner declaration is not native-platform
verification until its hosted job passes.

Manual runs use all providers and omit integrations unless `run_integrations`
is enabled. Focused coverage requires both inputs for a declared pair:

```sh
gh workflow run ci.yml --ref your-branch \
  -f coverage_provider=geam-simplifile \
  -f coverage_runner=windows-2025
```

Focused runs skip workspace quality and all integrations while retaining the
same 100% line and region gate. The mandatory selector/runner tests are:

```sh
python3 -m unittest discover -s .github/scripts -p 'test_*.py'
```

Local Markdown links are not checked by the workflow.

The `filepath` fixtures check the active host's `split` branch and both
explicit split functions. A `simplifile` Windows row also exercises its
`filepath` dependency. Declaring a runner does not establish a passing CI
result; inspect the hosted jobs before claiming platform verification.

The `platform` provider uses the default Ubuntu runner. Its full validation
job also runs the original Erlang fixture with `gleam run`; local validation
checks macOS ARM64. Windows OS and word-size mapping have owner tests, but no
native Windows Geam execution is claimed.

The `term_size` provider uses the Ubuntu runner by default. Its source-backed
contract tests spawn children with all standard streams piped, and with
controlled PTYs on stdout, stderr, or stdin. They check the selection order,
rows-before-columns result, `Error(Nil)` without a TTY, and a size change within
one process. The ordinary standalone and embedding runs only smoke-test linkage
because an unconstrained runner may or may not have a TTY. The controlled tests
run in the mandatory workspace test and provider coverage steps.

The `birl` provider runs on Ubuntu, macOS, and Windows. Its owner and
source-backed tests inject a fixed monotonic clock, wall time, UTC offset, and
time-zone name. Each runner also executes the original Erlang fixture and the
default operating-system source. The Erlang FFI and Jiff may discover
different names from the same host; compare fixed weekday/offset behavior and
document any host-specific visible difference instead of comparing live clock
values. `birl` also depends on the captured-group `split` contract of
`geam-regexp`, so changes to that provider must revalidate `birl`.

The `argv` provider runs on Ubuntu, macOS, and Windows. The standalone fixture
checks that `geam run --` passes only application arguments and that a built
executable receives the same values when started outside its project. The
embedding fixture supplies an explicit host snapshot without changing process
arguments. Non-Unicode conversion is covered by OS-specific owner tests.

The `global_value` provider uses the Ubuntu runner by default. Its source-backed
contract tests compile the original Gleam body and check generic retained
values, wrong-type name reuse, concurrent initialisation, and retry after a
process fails or is cancelled during initialisation. The concurrency test uses
a controlled clock. The standalone fixture checks installation-shaped linkage;
the embedding fixture checks repeated calls and fresh values in a second
execution domain. Each domain owns its cache and releases it on close.

The `envoy` provider runs on Ubuntu, macOS, and Windows. Its original Erlang
fixture checks `get/set/unset/all` with its own variable name as an independent
reference. Embedding tests use complete host-supplied initial values; standalone
captures the runner's process environment once when a run starts. Provider
`set/unset` changes only that run's state.

The `splitter` provider uses the Ubuntu runner by default. The original Erlang
fixture fixes the observed first-match and longest-delimiter behavior, including
overlaps, Unicode, no match, and the empty-splitter results. The standard Geam
standalone and embedding consumers verify the same original Gleam API through
the provider. The Erlang reference run is a local independent check; the
mandatory CI path runs the source-backed Rust tests and both Geam consumers.

The `operating_system` provider runs on Ubuntu, macOS, and Windows. Its
source-backed Rust contract test compares the unmodified Gleam package's
`gleam run` result with the Geam-hosted result on the same runner. The
standalone fixture prints the original public `name()` result, and the
embedding consumer checks the native target's expected source string.

The `exception` provider declares Ubuntu for CI. Its Erlang oracle and Geam fixtures
exercise the original Hex 2.1.1 declarations, failure recovery, cleanup order,
and cleanup failure precedence. The embedding contract also catches a native
stdlib failure and inspects its `Errored` diagnostic. Both consumers match
`rescue` and rebuild a generic `Result`, including a panic-only callback and
retained scalar, container, custom, codepoint, callable, and opaque `Dynamic`
values through all three functions. They also check discarded cleanup return
values. The Rust contract suite injects cancellation through a resumable
callback and checks cleanup failure precedence after cancellation. See
[exception/README.md](../../exception/README.md) for the exact boundary.

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
cargo llvm-cov --package geam-crypto --locked \
  --json --summary-only --output-path target/gleam-crypto-coverage.json \
  --fail-under-lines 100 \
  --fail-under-regions 100
cargo llvm-cov clean --workspace
cargo llvm-cov --package geam-gzlib --locked \
  --json --summary-only --output-path target/gzlib-coverage.json \
  --fail-under-lines 100 \
  --fail-under-regions 100
cargo llvm-cov clean --workspace
cargo llvm-cov --package geam-platform --locked \
  --json --summary-only --output-path target/platform-coverage.json \
  --fail-under-lines 100 \
  --fail-under-regions 100
cargo llvm-cov clean --workspace
cargo llvm-cov --package geam-logging --locked \
  --json --summary-only --output-path target/logging-coverage.json \
  --fail-under-lines 100 \
  --fail-under-regions 100
cargo llvm-cov clean --workspace
cargo llvm-cov --package geam-term-size --locked \
  --json --summary-only --output-path target/term-size-coverage.json \
  --fail-under-lines 100 \
  --fail-under-regions 100
cargo llvm-cov clean --workspace
cargo llvm-cov --package geam-birl --locked \
  --json --summary-only --output-path target/birl-coverage.json \
  --fail-under-lines 100 \
  --fail-under-regions 100
cargo llvm-cov clean --workspace
cargo llvm-cov --package geam-global-value --locked \
  --json --summary-only --output-path target/global-value-coverage.json \
  --fail-under-lines 100 \
  --fail-under-regions 100
cargo llvm-cov clean --workspace
cargo llvm-cov --package geam-httpc --locked \
  --json --summary-only --output-path target/gleam-httpc-coverage.json \
  --fail-under-lines 100 \
  --fail-under-regions 100
cargo llvm-cov clean --workspace
cargo llvm-cov --package geam-argv --locked \
  --json --summary-only --output-path target/argv-coverage.json \
  --fail-under-lines 100 \
  --fail-under-regions 100
cargo llvm-cov clean --workspace
cargo llvm-cov --package geam-splitter --locked \
  --json --summary-only --output-path target/splitter-coverage.json \
  --fail-under-lines 100 \
  --fail-under-regions 100
cargo llvm-cov clean --workspace
cargo llvm-cov --package geam-envoy --locked \
  --json --summary-only --output-path target/envoy-coverage.json \
  --fail-under-lines 100 \
  --fail-under-regions 100
cargo llvm-cov clean --workspace
cargo llvm-cov --package geam-operating-system --locked \
  --json --summary-only --output-path target/operating-system-coverage.json \
  --fail-under-lines 100 \
  --fail-under-regions 100
cargo llvm-cov clean --workspace
cargo llvm-cov --package geam-exception --locked \
  --json --summary-only --output-path target/exception-coverage.json \
  --fail-under-lines 100 \
  --fail-under-regions 100
cargo llvm-cov clean --workspace
cargo llvm-cov --package geam-glisten --locked \
  --json --summary-only --output-path target/glisten-coverage.json \
  --fail-under-lines 100 \
  --fail-under-regions 100
```

Read each package file entry in its JSON report to confirm line and region counts
for every reported file under that provider's `src/` directory, including
test-only support when present. As the repository gains production crates, add
an independent closure for each one. A consumer may execute another crate's
code, but its coverage must not compensate for missing owner coverage. Run the
same closure on every declared OS so conditional source paths are included.

For Houdini, gzlib, platform, term_size, birl, global_value, argv, splitter,
envoy, operating_system, and exception, confirm the `houdini/src/lib.rs`,
`gzlib/src/lib.rs`, `platform/src/lib.rs`, `term-size/src/lib.rs`,
`birl/src/lib.rs`,
`global-value/src/lib.rs`, `argv/src/lib.rs`, `splitter/src/lib.rs`,
`envoy/src/lib.rs`, `envoy/src/state.rs`, `operating-system/src/lib.rs`, and
`exception/src/lib.rs` file counts directly.

When a gap is unclear, inspect region and monomorph detail for the affected
package:

```sh
cargo llvm-cov report --package geam-regexp \
  --text \
  --show-instantiations \
  --show-missing-lines
```

Substitute `geam-filepath`, `geam-otp`, `geam-houdini`, `geam-gzlib`,
`geam-crypto`, `geam-simplifile`, `geam-platform`, `geam-logging`,
`geam-term-size`, `geam-birl`, `geam-httpc`, `geam-global-value`, `geam-argv`,
`geam-splitter`, or `geam-envoy` when inspecting those crates.

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
