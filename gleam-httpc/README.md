# geam-httpc

This crate implements the four native calls of the unchanged
[`gleam_httpc` 5.0.0](https://hex.pm/packages/gleam_httpc/5.0.0) package for
Geam. The original Gleam code still owns `send`, `dispatch`, `send_bits`, and
`dispatch_bits`; this Rust provider supplies their HTTP transport, user-agent,
and native error conversion. It uses Geam's `gleam_erlang` service for the
package's `Charlist` values. The supported Gleam range is exactly 5.0.0, and
the Geam dependency is pinned to `main` commit
`b23d82a23d31c77eca749d18f67e73f0d9d952c7`.

Add `gleam_httpc` to the Gleam project, then select this crate for that package
with Geam's `provider add --path` command. Embedding applications depend on
`geam-httpc` directly and initialize the generated provider run-state input.
The provider has no required configuration and normally uses the platform trust
store. To use a private CA, pass a `root_certificate_pem` string containing PEM
root certificates in the provider's TOML configuration (standalone) or its
`HostProviderConfiguration` (embedding). When set, this bundle replaces the
platform trust store; include every root the application needs.
An embedding host can supply a different network capability with
`State::set_transport` before execution. The public `transport::Transport`
contract takes owned requests and returns a cancellable future; the provider
does not retain Geam value borrows across the network await.
For example, from the standalone fixture directory:

```sh
python3 ../server.py geam run --provider-config gleam_httpc=../config/provider.toml
```

The default verifies HTTPS certificates, does not follow redirects, and uses
the original package's 30-second timeout. `verify_tls(False)` disables server
certificate verification for that request. The client uses HTTP/1.1, does not
use an ambient proxy or transparently decompress response bodies, and follows
at most ten redirects when enabled. The Rust HTTP client reports one connection
failure for a request; the provider places that same failure in both the
`ip4` and `ip6` fields of the upstream `FailedToConnect` type. It does not
claim Erlang `httpc`'s separate IPv4/IPv6 dial diagnostics or exact socket
error text. The TLS alert for an untrusted certificate may be `unknown_ca` or
`bad_certificate`, depending on the platform verifier. A closed local port may
also time out instead of reporting `econnrefused` on Windows. Invalid request
syntax and body shapes fail at the host boundary.

The [source contract](fixtures/CONTRACTS.md) pins the original Hex contents.
The [standalone fixture](fixtures/gleam) and [embedding fixture](fixtures/embedding)
exercise the original source against local HTTP and HTTPS servers; the fixture
CA and server key are test-only. Its HTTP, redirect, binary, and TLS scenarios
run as separate entry modules with the standard Geam CLI. See the
[testing guide](../docs/development/testing.md) for the validation commands.
The checked-in localhost TLS leaf certificate expires on 24 September 2027.
Before then, regenerate the test CA and leaf certificate together, replace
`fixtures/root-ca.pem`, `fixtures/localhost-cert.pem`, and
`fixtures/localhost-key.pem`, and update the PEM in
`fixtures/config/provider.toml` so standalone HTTPS checks remain valid.
This crate has not been published.
