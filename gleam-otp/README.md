# geam-otp

This crate provides the Rust native implementation for the unmodified [`gleam_otp` 1.3.0](https://hex.pm/packages/gleam_otp/1.3.0) Gleam package when it runs through Geam. It uses Geam's existing `gleam_erlang` process service, so OTP actors and supervisors share process identities, mailboxes, links, names, timers, and shutdown with other providers.

Add `gleam_otp` and `gleam_erlang` to the Gleam project, then select `geam-otp` as the `gleam_otp` provider. Rust embedding applications depend on this crate directly. The supported Gleam package range is limited to 1.3.0 until other releases are tested. The Geam dependency is pinned to commit `b23d82a23d31c77eca749d18f67e73f0d9d952c7`.

The [standalone fixture](https://github.com/panarch/geam-providers/tree/main/gleam-otp/fixtures/gleam) and [embedding fixture](https://github.com/panarch/geam-providers/tree/main/gleam-otp/fixtures/embedding) run the original Hex package and verify the native boundary. The [report service example](https://github.com/panarch/geam-providers/tree/main/gleam-otp/examples/report_service) shows an application-shaped actor and supervision tree.

The provider reproduces behavior observable through the Gleam package. It does not connect to an Erlang runtime or implement distributed Erlang protocols. The report service uses only in-memory input and does not persist or automatically retry in-flight work across failures.

This crate is intended for source-based use at the pinned Geam commit. A crates.io release is a separate step while the required Geam API and feature metadata are unavailable in its published version.
