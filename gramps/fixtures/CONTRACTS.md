# Native contracts

The provider targets unchanged Hex `gramps` **6.0.1**, archive SHA-256
`d55636072dee173f6586a5679d3c02ec7a0de3f8646b78c351b72908ff223df7`.
[upstream.sha256](upstream.sha256) pins the three native-calling Gleam modules
and the Erlang FFI. Fixtures select stdlib 1.0.3, Erlang 1.3.0, HTTP 4.3.0 and
crypto 1.6.0. The original Gleam function bodies remain unchanged.

`CompressionContext` below is the opaque type in
`gramps/websocket/compression`; `Context` is its original record with
`context: CompressionContext` and `no_takeover: Bool`. `Default`, `Deflated`,
`Flush` and `ShaHash` are the original private nominal types. `Atom` and `Pid`
belong to gleam_erlang; `BytesTree` and `Dynamic` belong to gleam_stdlib.

| Module / native | Exact Gleam arguments → return | Rust owner / source caller |
| --- | --- | --- |
| websocket / crypto_exor | a: BitArray, b: BitArray → BitArray | websocket.rs / apply_mask, mask_data |
| websocket / crypto_hash | hash: ShaHash, data: String → String | websocket.rs / parse_websocket_key |
| websocket / base64_encode | data: String → String | websocket.rs / parse_websocket_key |
| websocket/compression / inflate_init | CompressionContext, Int → Atom | compression.rs / init |
| websocket/compression / deflate_init | CompressionContext, Default, Deflated, Int, Int, Default → Atom | compression.rs / init |
| websocket/compression / open | () → CompressionContext | compression.rs / init |
| websocket/compression / do_inflate | CompressionContext, BitArray → BytesTree | compression.rs, compression/stream.rs / inflate |
| websocket/compression / do_deflate | CompressionContext, BitArray, Flush → BytesTree | compression.rs, compression/stream.rs / deflate |
| websocket/compression / set_controlling_process | Context, Pid → Atom | compression.rs / public ownership transfer |
| websocket/compression / do_close | CompressionContext → Nil | compression.rs / close |
| websocket/compression / inflate_reset | CompressionContext → Nil | compression.rs / inflate with no_client takeover |
| websocket/compression / deflate_reset | CompressionContext → Nil | compression.rs / deflate with no_server takeover |
| http / decode_packet | PacketType, BitArray, List(options) → Result(#(DecodedPacket, BitArray), DecodePacketError) | http.rs, http/parser.rs / read_request, read_response, get_headers |

The websocket module path is `gramps/websocket`; the HTTP module path is
`gramps/http`. HTTP's private nominal declarations are statically defined in
[schema.rs](../src/http/schema.rs), including original field labels and
constructor order. Registration and source compilation verify these shapes;
there is no replacement source API or runtime type lookup.

The native arities are, in the table's order, **2, 2, 1, 2, 6, 0, 2, 3, 2,
1, 1, 1, 3**. `crypto_exor` explicitly labels arguments `a` and `b`;
`crypto_hash` labels `hash` and `data`; `base64_encode` labels `data`. The
other native arguments have no explicit source call labels. `decode_packet`
is universally specialized over its `options` variable; only the original
caller's empty options list is supported. A nonempty list fails at the host
boundary before parsing.

Nominal identities are source package/module/type triples:

- `gramps / gramps/websocket / ShaHash`: private `Sha`.
- `gramps / gramps/websocket/compression`: opaque `CompressionContext`;
  public `Context(context: CompressionContext, no_takeover: Bool)`; private
  `Default { Default }`, `Deflated { Deflated }`, `Flush { Sync }`.
- `gramps / gramps/http / PacketType`: `HttphBin`, then `HttpBin`.
- `gramps / gramps/http / DecodePacketError`: `More(length: Int)`, then
  `HttpError(reason: String)`.
- `gramps / gramps/http / UriPacket`: private `AbsPath(String)`, then
  `AbsoluteUri(scheme: Scheme, host: String, port: Int, path: String)`.
- `gramps / gramps/http / DecodedPacket`: private
  `HttpRequest(method: String, uri: UriPacket, version: #(Int, Int))`,
  `HttpResponse(version: #(Int, Int), status: Int, text: String)`,
  `HttpHeader(unknown: Int, field: Dynamic, raw_field: String, value: String)`,
  then `HttpEoh`.
- `gleam_http / gleam/http / Scheme`: `Http`, then `Https`.
- `gleam_erlang / gleam/erlang/atom / Atom` and
  `gleam_erlang / gleam/erlang/process / Pid` use the Erlang producer's
  schema, binding, and construction capabilities.
- `gleam_stdlib / gleam/bytes_tree / BytesTree` and
  `gleam_stdlib / gleam/dynamic / Dynamic` use the stdlib producer's public
  adapters. Dynamic construction uses the statically composed Erlang binding
  selected by Geam for that producer-owned type.

The SHA-1 result is a String containing **20 raw bytes**, then encoded with
padded standard base64. XOR accepts equal-length complete bytes and preserves
input aliases. Other alignment or length failures are host failures.

Compression uses stateful raw DEFLATE, window bits -15, default compression
level/strategy, memory level 8 and Sync flush. The original Gleam code removes
or restores the four-byte permessage-deflate suffix and selects takeover
resets. Rust returns the original producer's BytesTree. Compressed bytes are
backend-dependent; interoperability and recovered messages are the contract.

The execution domain owns resources. Handles keep creator/serial identity,
never live streams. Operations require the current owner. Transfer unwraps the
public Context and requires a live target in the same execution. Reset and
close check ownership; a closed alias remains equal/hashable but cannot access
a stream. Current-owner termination and domain closure release streams.
Returned byte trees retain immutable data independently of these resources.
An already initialized handle is rejected before allocating another backend.
A backend failure marks the stream failed until the corresponding reset;
invalid alignment and operation mode are rejected before changing stream state. Both close and
reset on a closed handle fail. Self transfer succeeds without changing identity.

HTTP decoding consumes one start line, header or end-of-headers packet. The
original source owns method parsing, lowercasing, header order and path/query
splitting. Valid custom methods remain `Other(method)`. Header values retain
raw bytes. The body and following packets are returned unchanged. The current
public BitArray API requires one copy for the newly returned remainder.

Incomplete packets return `More(0)` (unknown required total length). Absolute
HTTP/HTTPS URIs default to ports 80/443; explicit ports and bracketed IPv6 are
preserved. Unsupported asterisk/authority/other-scheme targets and malformed
packets return `HttpError(String)`. These typed values replace original FFI
outputs such as `undefined` or `absoluteURI` that do not match the Gleam
declarations. The provider does not reproduce Erlang's unused header ID table.

The shared [contracts.gleam](gleam/src/contracts.gleam) separates original
Erlang-compatible scenarios (`common`) from typed Geam normalization scenarios
(`normalized`). In-memory owner-test additions expose private native callers
without editing downloaded source or adding production APIs.

## Mandatory evidence

| Contract | Mandatory test / source |
| --- | --- |
| Exact provider configuration, three modules and 13 names; complete declaration linkage | `src/lib.rs::configuration_and_the_thirteen_original_native_declarations_are_exact`; `tests/original_gramps.rs::unchanged_package_runs_through_the_public_boundary_and_prepares` |
| Raw SHA-1, padded base64, XOR bytes/aliases/failures, computed native function calls | `src/websocket.rs` owner tests; `common()` RFC accept, short masked text and binary frames |
| Actual composed crypto entropy and entropy failure | `src/websocket.rs::original_client_key_uses_the_composed_crypto_entropy_and_preserves_its_failure`; embedding `checks.rs` client key |
| Raw stream chunk draining, four message families, dictionary/reset, independent vectors, failed stream/reset, final-block trailing input and capacity failure | `src/compression/stream.rs` owner tests; `common()` compression and asymmetric duplex scenarios |
| Initialization, modes, alignment, mode rejection without poisoning either stream, repeated close/reset-after-close, alias equality/hash and retained BytesTree | `src/compression.rs` initialization, `mode_rejection_preserves_both_contexts_without_a_reset`, and BytesTree owner tests |
| Self/live/dead/foreign Pid transfer; non-owner failure preserves the new owner; old/new owner exit and kill; failure/cancel/application exit cleanup | `src/compression.rs` lifecycle tests with real process mailboxes, monitors and an audit around the production service's hooks |
| Independent simultaneous domains and BytesTree after all executions/states are dropped | `src/compression.rs::independent_live_domains_reject_retained_handles_and_pids_and_trees_outlive_execution` |
| Request/status/header/EOH, exact consumption/raw fields, More/malformed/URI/error shape, generic options | `src/http/parser.rs` and `src/http.rs` owner tests; `common()` HTTP messages/response builder; `normalized()` URI and partial retry cases |
| Text/binary, 16/64-bit lengths, leftovers, fragmented frames, aggregation, ping/pong/close and source-owned invalid cases | `common()` and [upgrade_echo/session.gleam](../examples/upgrade_echo/src/upgrade_echo/session.gleam) |
| Rust to Erlang and Erlang to actual provider, with takeover and reset | [embedding checks/interop.rs](embedding/src/checks/interop.rs) launches [permessage_deflate.escript](permessage_deflate.escript), checks empty/large/repeated/binary messages, and reads retained outputs after fresh domains close; both live and prepared are mandatory |
| Repeated/fresh execution, recovery after failed calls, prepared library load and actual execution | [embedding checks.rs](embedding/src/checks.rs) and its mandatory Cargo test; generated data is checked by `geam embedding check` |
| Original upstream oracle | `fixtures/ci.sh erlang` runs `gleam run --module reference`, whose `common()` uses unchanged Erlang FFI |
| Installation-shaped linkage and runnable example | Common CI runs standalone prepare/run/build, the binary from another directory, and the example against `expected-output.txt` |

The original aggregator accepts a lone complete continuation as an ordinary
frame; interleaving a new frame while a fragmented frame is pending returns
`Error(Nil)`. These source-owned choices are tested without imposing an
additional WebSocket parser in the provider. The native-private helpers in
owner tests consume private values locally to avoid exporting private types.
The domain-isolation test retains a real creator identity and reconstructs
that identity through its declared producer in the independent store; it never
copies the live stream or manufactures execution IDs.
