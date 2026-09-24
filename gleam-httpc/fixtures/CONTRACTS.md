# gleam_httpc 5.0.0 native contract

The fixtures resolve the unchanged Hex release `gleam_httpc` 5.0.0 (outer
checksum `C545172618D07811494E97AAA4A0FB34DA6F6D0061FDC8041C2F8E3BE2B2E48F`).
The resolved dependencies in the initial preflight were `gleam_http` 4.3.0,
`gleam_erlang` 1.3.0, and `gleam_stdlib` 1.0.3. Verify the complete extracted
package against [`upstream.sha256`](upstream.sha256) after `gleam deps download`.

The only native calls in `src/gleam/httpc.gleam` are:

| Gleam function | Original Erlang external | Arguments | Return |
| --- | --- | --- | --- |
| `default_user_agent` | `gleam_httpc_ffi:default_user_agent/0` | none | `#(Charlist, Charlist)` |
| `normalise_error` | `gleam_httpc_ffi:normalise_error/1` | `Dynamic` | `HttpError` |
| `erl_request` | `httpc:request/4` | `Method`, `#(Charlist, List(#(Charlist, Charlist)), Charlist, BitArray)`, `List(ErlHttpOption)`, `List(ErlOption)` | `Result(#(#(Charlist, Int, Charlist), List(#(Charlist, Charlist)), BitArray), Dynamic)` |
| `erl_request_no_body` | `httpc:request/4` | `Method`, `#(Charlist, List(#(Charlist, Charlist)))`, `List(ErlHttpOption)`, `List(ErlOption)` | same Result |

The source body selects the no-body external for `Options`, `Head`, and `Get`;
all other methods use the body-bearing external. It inserts a default
`user-agent` when the exact lower-case header name is absent, defaults the
body's content type to `application/octet-stream`, passes binary response
bodies through `dispatch_bits`, and returns `InvalidUtf8Response` from
`dispatch` when response bytes are not UTF-8. The default configuration
verifies TLS, disables redirects, and uses a 30,000 ms timeout.

`ErlHttpOption` has `Ssl([Verify(VerifyNone)])`, `Autoredirect(Bool)`, and
`Timeout(Int)`; `ErlOption` has `BodyFormat(Binary)` and
`SocketOpts([Ipfamily(Inet6fb4)])`. The public `HttpError` constructors are
`InvalidUtf8Response`, `FailedToConnect(ip4: ConnectError, ip6: ConnectError)`,
and `ResponseTimeout`; `ConnectError` has `Posix(code: String)` and
`TlsAlert(code: String, detail: String)`. The original Erlang FFI recognizes
only `timeout` and `{failed_connect, [...]}` errors and raises on unexpected
shapes. Its user-agent value is `gleam_httpc/<application version>`, falling
back to `0.0.0` only when the Erlang application version is unavailable.

The exact status reason phrase, socket error spelling, and transport internals
are not exposed by the Gleam `Response` body. The provider must preserve
observable methods, headers, status, bytes, documented options and security
defaults, and the defined error variants/fields. Tests must distinguish that
contract from Erlang `httpc` implementation details.
