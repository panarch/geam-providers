# Geam Providers

This repository implements native functions for existing Gleam packages through Geam's public provider API. Each provider is a separate Cargo workspace member, and applications explicitly select the providers they use.

| Gleam package | Rust provider | Verified version |
| --- | --- | --- |
| [`filepath`](https://hex.pm/packages/filepath) | [`geam-filepath`](filepath/README.md) | 1.1.2 |
| [`gleam_regexp`](https://hex.pm/packages/gleam_regexp) | [`geam-regexp`](gleam-regexp/README.md) | 1.1.1 |
| [`gleam_otp`](https://hex.pm/packages/gleam_otp) | [`geam-otp`](gleam-otp/README.md) | 1.3.0 |
| [`houdini`](https://hex.pm/packages/houdini) | [`geam-houdini`](houdini/README.md) | 1.2.0 |

The Geam dependencies are pinned to `main` commit `76c4ab7c6a2c0c35975bdd97e5de7c6284f895e7` rather than a released version or local path. The Gleam packages come from Hex without changes to their upstream source. Publishing provider crates and testing consumption of published packages are separate steps after the required Geam features are released.

See [provider compatibility](docs/design/provider-compatibility.md) for the design boundary, the [testing guide](docs/development/testing.md) for verification steps and coverage requirements, and the [review policy](docs/development/review-policy.md) for code review criteria.
