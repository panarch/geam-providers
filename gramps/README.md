# geam-gramps

Rust native provider for the unchanged [`gramps`](https://hex.pm/packages/gramps)
6.0.1 Gleam package on Geam. It implements HTTP packet decoding, WebSocket
SHA-1/base64/XOR functions, and process-owned permessage-deflate streams.

Use Geam `main` commit `08b5651661f83d59423ff20e271c62fb42a2f2ae`.
Select both this provider and `geam-crypto`, then prepare the original Gleam
application. From the included [fixture](fixtures/gleam/):

```sh
gleam deps download
geam prepare
geam run
geam build
```

The fixture already selects local provider crates. For another application,
add the unchanged Hex `gramps` 6.0.1 package and select provider crates from
this checkout with `geam provider add --path PATH --package geam-gramps` and
`--package geam-crypto`. Both need Geam's stdlib and Erlang components. The
[embedding consumer](fixtures/embedding/) composes the same components with
an explicit execution host and initialized provider configuration.

Streams belong to their current process in one execution domain. Transfer
requires a live Pid in that domain; owner termination, cancellation, application
exit and domain closure release streams. Handles retain identity, and returned
byte trees retain immutable data after reset, close or execution shutdown.
Empty provider configuration is accepted; unknown keys fail initialization.

On Geam, `set_controlling_process` transfers the handle in the declared
`Context` to the target `Pid`. The original Erlang native call passes the
Context record directly to zlib and fails with `badarg`.

The backend uses raw DEFLATE through `miniz_oxide`. Recovered messages and
takeover/reset behavior are portable; compressed bytes depend on the backend.
HTTP decoding preserves raw header values and unconsumed bytes. Incomplete
input returns `More(0)`, and absolute HTTP/HTTPS URIs use default ports 80/443.
Unsupported request-target forms return `HttpError`; the provider supplies
the declared Gleam types rather than Erlang VM-specific values. The original
Gleam code owns frame parsing and aggregation, including its own limitations.

See [the Upgrade echo example](examples/upgrade_echo/README.md) and
[native contracts](fixtures/CONTRACTS.md).
