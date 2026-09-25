# Geam Providers

This repository implements native functions for existing Gleam packages through Geam's public provider API. Each provider is a separate Cargo workspace member, and applications explicitly select the providers they use.

| Gleam package | Rust provider | Verified version |
| --- | --- | --- |
| [`filepath`](https://hex.pm/packages/filepath) | [`geam-filepath`](filepath/README.md) | 1.1.2 |
| [`gleam_regexp`](https://hex.pm/packages/gleam_regexp) | [`geam-regexp`](gleam-regexp/README.md) | 1.1.1 |
| [`gleam_otp`](https://hex.pm/packages/gleam_otp) | [`geam-otp`](gleam-otp/README.md) | 1.3.0 |
| [`houdini`](https://hex.pm/packages/houdini) | [`geam-houdini`](houdini/README.md) | 1.2.0 |
| [`gzlib`](https://hex.pm/packages/gzlib) | [`geam-gzlib`](gzlib/README.md) | 2.0.0 |
| [`gleam_crypto`](https://hex.pm/packages/gleam_crypto) | [`geam-crypto`](gleam-crypto/README.md) | 1.6.0 |
| [`simplifile`](https://hex.pm/packages/simplifile) | [`geam-simplifile`](simplifile/README.md) | 2.7.0 |
| [`platform`](https://hex.pm/packages/platform) | [`geam-platform`](platform/README.md) | 1.0.0 |
| [`logging`](https://hex.pm/packages/logging) | [`geam-logging`](logging/README.md) | 1.5.0 |
| [`term_size`](https://hex.pm/packages/term_size) | [`geam-term-size`](term-size/README.md) | 1.0.1 |
| [`birl`](https://hex.pm/packages/birl) | [`geam-birl`](birl/README.md) | 2.0.0 |
| [`gleam_httpc`](https://hex.pm/packages/gleam_httpc) | [`geam-httpc`](gleam-httpc/README.md) | 5.0.0 |
| [`global_value`](https://hex.pm/packages/global_value) | [`geam-global-value`](global-value/README.md) | 1.0.0 |
| [`argv`](https://hex.pm/packages/argv) | [`geam-argv`](argv/README.md) | 1.1.0 |

The Geam dependencies are pinned to `main` commit `b23d82a23d31c77eca749d18f67e73f0d9d952c7` rather than a released version or local path. The Gleam packages come from Hex without changes to their upstream source. Publishing provider crates and testing consumption of published packages are separate steps after the required Geam features are released.

See [provider compatibility](docs/design/provider-compatibility.md) for the design boundary, the [testing guide](docs/development/testing.md) for verification steps and coverage requirements, and the [review policy](docs/development/review-policy.md) for code review criteria.
