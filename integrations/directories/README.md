# `directories` integration

This fixture runs the unmodified [`directories` 1.2.0](https://hex.pm/packages/directories/1.2.0) Gleam package through Geam. `directories` has no native external of its own, so there is no `geam-directories` provider crate. Its Gleam code combines `envoy` for environment values, `platform` for OS selection, and `simplifile` for directory checks. `simplifile` also requires `filepath`. The two Geam consumers select the existing `geam-envoy`, `geam-platform`, `geam-simplifile`, and `geam-filepath` crates through their public interfaces.

The [standalone Gleam fixture](fixtures/gleam/) owns a shared source-backed scenario. The [embedding Gleam project](fixtures/embedding/gleam/) imports it as a local dependency, and the [Rust embedding consumer](fixtures/embedding/src/main.rs) supplies an isolated initial environment. The scenario creates directories under a temporary root, calls all 11 public `directories` functions with exact OS-specific expectations, checks environment priority and missing or empty directory candidates, then deletes the root. The same standalone Gleam module runs on original Erlang as an independent reference; it does not derive Geam's expected answers from Erlang output. The exported API is `preference_dir`, despite the plural spelling in the upstream README example.

Both fixtures lock the original Hex release and Geam `main` commit `cf5fb100c9220d9e30ff60a11d5bfbeb77f1ebef`. [Checksums](fixtures/upstream.sha256) cover the downloaded `directories` Gleam module and manifest. The source is not copied or patched in this repository.

Build the Geam CLI from that commit as described in the [testing guide](../../docs/development/testing.md), then set `GEAM_BIN` to the resulting executable. From the repository root:

```sh
(cd integrations/directories/fixtures/gleam && gleam deps download && gleam format --check && gleam check && gleam run)
(cd integrations/directories/fixtures/embedding/gleam && gleam deps download && gleam format --check && gleam check)
(cd integrations/directories/fixtures/gleam/build/packages/directories && if command -v sha256sum >/dev/null; then sha256sum --check ../../../../upstream.sha256; else shasum -a 256 -c ../../../../upstream.sha256; fi)
(cd integrations/directories/fixtures/embedding && cargo fmt --all --check && "$GEAM_BIN" embedding check && cargo test --locked && cargo run --locked)
(cd integrations/directories/fixtures/gleam && "$GEAM_BIN" prepare && "$GEAM_BIN" run && "$GEAM_BIN" build)
```

Run the built `geam_directories_fixture` executable from a directory outside the fixture as a separate check. Its test root is relative to the caller's current directory, and it removes that root after a successful run.

The scenario passed locally on macOS with Erlang, Geam standalone, and Rust embedding. The [CI workflow](../../.github/workflows/ci.yml) is configured to run this integration on Ubuntu, macOS, and Windows when its fixture, one of its four providers, or relevant shared build/CI inputs change. A configured runner is not a verified result until that hosted job passes. This fixture is integration evidence; each provider keeps its own contract tests and 100% production line and region coverage gate under the [review policy](../../docs/development/review-policy.md).
