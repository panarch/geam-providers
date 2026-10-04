# Original glisten 9.0.1 contracts

The unchanged Hex source matches the v9.0.1 tag. The inventory links every original declaration to the implementation, caller
contract, and tests below. Native targets are recorded as upstream declares them;
Geam links the original module/name/type scheme, not a second FFI API.

| Original module | Declaration | Erlang target | Original signature | Owner and mandatory evidence |
| --- | --- | --- | --- | --- |
| `glisten/internal/handler` | `rescue` | `glisten_ffi:rescue` | `@external(erlang, "glisten_ffi", "rescue") fn rescue(func: fn() -> anything) -> Result(anything, Dynamic)` | [rescue](../src/handler.rs); [rescue](#rescue) |
| `glisten/internal/handler` | `socket_data` | `glisten_ffi:socket_data` | `@external(erlang, "glisten_ffi", "socket_data") fn socket_data(record: Dynamic) -> BitArray` | [project](../src/handler.rs); [records](#records) |
| `glisten/internal/handler` | `socket_error` | `glisten_ffi:socket_data` | `@external(erlang, "glisten_ffi", "socket_data") fn socket_error(record: Dynamic) -> SocketReason ` | [project](../src/handler.rs); [records](#records) |
| `glisten/socket/options` | `to_erl_options` | `glisten_ffi:to_erl_tcp_options` | `@external(erlang, "glisten_ffi", "to_erl_tcp_options") pub fn to_erl_options(options: List(TcpOption)) -> List(ErlangTcpOption)` | [to_erl_options](../src/options.rs); [options](#options) |
| `glisten/socket/options` | `merge_type_list` | `glisten_ffi:merge_type_list` | `@external(erlang, "glisten_ffi", "merge_type_list") pub fn merge_type_list(original: List(a), override: List(a)) -> List(a)` | [merge](../src/options.rs); [options](#options) |
| `glisten/ssl` | `controlling_process` | `glisten_ssl_ffi:controlling_process` | `@external(erlang, "glisten_ssl_ffi", "controlling_process") pub fn controlling_process(socket: Socket, pid: Pid) -> Result(Nil, Atom)` | [controlling](../src/native.rs); [handoff](#handoff) |
| `glisten/ssl` | `do_listen` | `ssl:listen` | `@external(erlang, "ssl", "listen") fn do_listen( port: Int, options: List(options.ErlangTcpOption), ) -> Result(ListenSocket, SocketReason)` | [listen](../src/native.rs); [listen](#listen) |
| `glisten/ssl` | `accept_timeout` | `ssl:transport_accept` | `@external(erlang, "ssl", "transport_accept") pub fn accept_timeout( socket: ListenSocket, timeout: Int, ) -> Result(Socket, SocketReason)` | [accept_impl](../src/native.rs); [accept](#accept) |
| `glisten/ssl` | `accept` | `ssl:transport_accept` | `@external(erlang, "ssl", "transport_accept") pub fn accept(socket: ListenSocket) -> Result(Socket, SocketReason)` | [accept_impl](../src/native.rs); [accept](#accept) |
| `glisten/ssl` | `receive_timeout` | `ssl:recv` | `@external(erlang, "ssl", "recv") pub fn receive_timeout( socket: Socket, length: Int, timeout: Int, ) -> Result(BitArray, SocketReason)` | [receive_impl](../src/native.rs); [receive](#receive) |
| `glisten/ssl` | `receive` | `ssl:recv` | `@external(erlang, "ssl", "recv") pub fn receive(socket: Socket, length: Int) -> Result(BitArray, SocketReason)` | [receive_impl](../src/native.rs); [receive](#receive) |
| `glisten/ssl` | `send` | `glisten_ssl_ffi:send` | `@external(erlang, "glisten_ssl_ffi", "send") pub fn send(socket: Socket, packet: BytesTree) -> Result(Nil, SocketReason)` | [send](../src/send.rs); [send](#send) |
| `glisten/ssl` | `close` | `glisten_ssl_ffi:close` | `@external(erlang, "glisten_ssl_ffi", "close") pub fn close(socket: Socket) -> Result(Nil, SocketReason)` | [close / ssl_close](../src/native.rs); [close](#close) |
| `glisten/ssl` | `do_shutdown` | `glisten_ssl_ffi:shutdown` | `@external(erlang, "glisten_ssl_ffi", "shutdown") pub fn do_shutdown(socket: Socket, write: Atom) -> Result(Nil, SocketReason)` | [shutdown](../src/native.rs); [shutdown](#shutdown) |
| `glisten/ssl` | `do_set_opts` | `glisten_ssl_ffi:set_opts` | `@external(erlang, "glisten_ssl_ffi", "set_opts") fn do_set_opts( socket: Socket, opts: List(options.ErlangTcpOption), ) -> Result(Nil, SocketReason)` | [set_opts](../src/native.rs); [options](#options) |
| `glisten/ssl` | `handshake` | `ssl:handshake` | `@external(erlang, "ssl", "handshake") pub fn handshake(socket: Socket) -> Result(Socket, Nil)` | [handshake](../src/native.rs); [handshake](#handshake) |
| `glisten/ssl` | `negotiated_protocol` | `glisten_ssl_ffi:negotiated_protocol` | `@external(erlang, "glisten_ssl_ffi", "negotiated_protocol") pub fn negotiated_protocol(socket: Socket) -> Result(String, String)` | [protocol / no_tcp_protocol](../src/native.rs); [protocol](#protocol) |
| `glisten/ssl` | `peername` | `ssl:peername` | `@external(erlang, "ssl", "peername") pub fn peername(socket: Socket) -> Result(#(Dynamic, Int), SocketReason)` | [peername](../src/native.rs); [address-and-options](#address-and-options) |
| `glisten/ssl` | `sockname` | `ssl:sockname` | `@external(erlang, "ssl", "sockname") pub fn sockname(socket: ListenSocket) -> Result(#(Dynamic, Int), SocketReason)` | [sockname](../src/native.rs); [address-and-options](#address-and-options) |
| `glisten/ssl` | `get_socket_opts` | `ssl:getopts` | `@external(erlang, "ssl", "getopts") pub fn get_socket_opts( socket: Socket, opts: List(Atom), ) -> Result(List(#(Atom, Dynamic)), SocketReason) ` | [get_opts](../src/native.rs); [address-and-options](#address-and-options) |
| `glisten/tcp` | `controlling_process` | `glisten_tcp_ffi:controlling_process` | `@external(erlang, "glisten_tcp_ffi", "controlling_process") pub fn controlling_process(socket: Socket, pid: Pid) -> Result(Nil, Atom)` | [controlling](../src/native.rs); [handoff](#handoff) |
| `glisten/tcp` | `do_listen_tcp` | `gen_tcp:listen` | `@external(erlang, "gen_tcp", "listen") fn do_listen_tcp( port: Int, options: List(options.ErlangTcpOption), ) -> Result(ListenSocket, SocketReason)` | [listen](../src/native.rs); [listen](#listen) |
| `glisten/tcp` | `accept_timeout` | `gen_tcp:accept` | `@external(erlang, "gen_tcp", "accept") pub fn accept_timeout( socket: ListenSocket, timeout: Int, ) -> Result(Socket, SocketReason)` | [accept_impl](../src/native.rs); [accept](#accept) |
| `glisten/tcp` | `accept` | `gen_tcp:accept` | `@external(erlang, "gen_tcp", "accept") pub fn accept(socket: ListenSocket) -> Result(Socket, SocketReason)` | [accept_impl](../src/native.rs); [accept](#accept) |
| `glisten/tcp` | `receive_timeout` | `gen_tcp:recv` | `@external(erlang, "gen_tcp", "recv") pub fn receive_timeout( socket: Socket, length: Int, timeout: Int, ) -> Result(BitArray, SocketReason)` | [receive_impl](../src/native.rs); [receive](#receive) |
| `glisten/tcp` | `receive` | `gen_tcp:recv` | `@external(erlang, "gen_tcp", "recv") pub fn receive(socket: Socket, length: Int) -> Result(BitArray, SocketReason)` | [receive_impl](../src/native.rs); [receive](#receive) |
| `glisten/tcp` | `send` | `glisten_tcp_ffi:send` | `@external(erlang, "glisten_tcp_ffi", "send") pub fn send(socket: Socket, packet: BytesTree) -> Result(Nil, SocketReason)` | [send](../src/send.rs); [send](#send) |
| `glisten/tcp` | `socket_info` | `socket:info` | `@external(erlang, "socket", "info") pub fn socket_info(socket: Socket) -> Dict(a, b)` | [info / typed_info](../src/native.rs); [information](#information) |
| `glisten/tcp` | `close` | `glisten_tcp_ffi:close` | `@external(erlang, "glisten_tcp_ffi", "close") pub fn close(socket: a) -> Result(Nil, SocketReason)` | [close / ssl_close](../src/native.rs); [close](#close) |
| `glisten/tcp` | `do_shutdown` | `glisten_tcp_ffi:shutdown` | `@external(erlang, "glisten_tcp_ffi", "shutdown") pub fn do_shutdown(socket: Socket, write: Atom) -> Result(Nil, SocketReason)` | [shutdown](../src/native.rs); [shutdown](#shutdown) |
| `glisten/tcp` | `do_set_opts` | `glisten_tcp_ffi:set_opts` | `@external(erlang, "glisten_tcp_ffi", "set_opts") fn do_set_opts( socket: Socket, opts: List(options.ErlangTcpOption), ) -> Result(Nil, SocketReason)` | [set_opts](../src/native.rs); [options](#options) |
| `glisten/tcp` | `negotiated_protocol` | `tcp:negotiated_protocol` | `@external(erlang, "tcp", "negotiated_protocol") pub fn negotiated_protocol(socket: Socket) -> a` | [protocol / no_tcp_protocol](../src/native.rs); [protocol](#protocol) |
| `glisten/tcp` | `peername` | `inet:peername` | `@external(erlang, "inet", "peername") pub fn peername(socket: Socket) -> Result(#(Dynamic, Int), SocketReason)` | [peername](../src/native.rs); [address-and-options](#address-and-options) |
| `glisten/tcp` | `get_socket_opts` | `inet:getopts` | `@external(erlang, "inet", "getopts") pub fn get_socket_opts( socket: Socket, opts: List(Atom), ) -> Result(List(#(Atom, Dynamic)), SocketReason)` | [get_opts](../src/native.rs); [address-and-options](#address-and-options) |
| `glisten/tcp` | `sockname` | `inet:sockname` | `@external(erlang, "inet", "sockname") pub fn sockname(socket: ListenSocket) -> Result(#(Dynamic, Int), SocketReason) ` | [sockname](../src/native.rs); [address-and-options](#address-and-options) |
| `glisten/transport` | `convert_address` | `inet:ipv4_mapped_ipv6_address` | `@external(erlang, "inet", "ipv4_mapped_ipv6_address") fn convert_address(address: address) -> #(Int, Int, Int, Int)` | [mapped_address](../src/native.rs); [mapped-address](#mapped-address) |
| `glisten/transport` | `socket_info` | `socket:info` | `@external(erlang, "socket", "info") pub fn socket_info(socket: Socket) -> Dict(Atom, Dynamic)` | [info / typed_info](../src/native.rs); [information](#information) |
| `glisten` | `parse_address` | `glisten_ffi:parse_address` | `@external(erlang, "glisten_ffi", "parse_address") fn parse_address(value: Charlist) -> Result(ip_address, Nil)` | [parse](../src/options.rs); [parse](#parse) |

## Mandatory Evidence

The owner tests named below live beside their production owner in
[`native.rs`](../src/native.rs), [`options.rs`](../src/options.rs),
[`active.rs`](../src/active.rs), [`settings.rs`](../src/settings.rs), [`sockets.rs`](../src/sockets.rs),
[`handler.rs`](../src/handler.rs), [`network.rs`](../src/network.rs), and
[`lib.rs`](../src/lib.rs). `cargo test --workspace --all-targets --locked` and
provider coverage run them without ignored cases.

[`original_glisten.rs`](../tests/original_glisten.rs) checks all nine pinned
upstream files, links the entire original package and all native signatures,
and prepares its generic bind entry. Nominal constructors, function schemes,
and exact construction capabilities are checked by the typed host boundary.

The public source entry [`geam_glisten_fixture`](gleam/src/geam_glisten_fixture.gleam)
runs seven scenarios through [`run.py`](run.py). They are mandatory in the
original Erlang oracle, embedding compatibility test (live and fresh prepared
bindings), Geam `run`, and the built executable outside the fixture. Commands
and CI ownership are in the [testing guide](../../docs/development/testing.md#glisten-tcp-and-tls-servers).
The oracle does not assert VM-specific metadata or the original FFI's invalid
representation of a declared String/Nil error.

## Listen

Validated port, address, backlog, credentials, and creation options produce a
new `ListenSocket` owned by the calling execution unit. Invalid caller values
return `Badarg` before IO; network errors map to the declared `SocketReason`.
Reactor configuration errors fail component initialization before source runs.

Owner tests: `original_tcp_and_tls_options_select_explicit_listener_capabilities`,
`source_representable_invalid_options_fail_before_host_io`,
`listener_and_handshake_io_failures_keep_the_original_result_shapes`, and
`initializer_preserves_the_network_dependency_failure_identity`.
Actual IO tests include IPv6, unavailable interfaces, conflicting binds,
missing/malformed PEM, and empty/mismatched certificates.
The pool and supervised IPv6 scenarios use the original builder, listener and
factory names, multiple acceptors, and port-0 listener information.

## Accept

Timed and untimed accept return a newly admitted `Socket` with the listener's
settings. TCP starts ready; TLS requires handshake. The calling unit owns the
connection and its active reader. Admission owns completed IO until the
execution service accepts it; cancellation closes unadmitted IO. Negative or
unrepresentable deadlines return `Badarg`; an expired pending accept returns
`Timeout`, and a closed listener returns `Closed`. Ready IO wins a zero timeout.

Owner tests: `accept_timeout_uses_the_execution_clock_and_rejects_negative_time`,
`accept_and_receive_reject_deadlines_outside_the_selected_clock_range`,
`original_tcp_controls_preserve_typed_results_values_and_effect_order`,
`cancellation_before_accepted_io_admission_closes_the_unregistered_connection`,
and `cancelling_the_execution_releases_pending_io_and_live_domain_resources`.
Original server scenarios execute acceptor-to-handler handoff and readiness.

## Receive

Passive length-zero receive returns available bytes; a positive length waits
for exactly that many bytes. Partial data survives timeout for the next call.
Invalid length/deadline returns `Badarg`; active mode returns `Einval`, an
unhandshaken TLS connection returns `Badarg`, and EOF/closed aliases return
`Closed`. Socket errors and execution cancellation keep their own boundary.

Owner tests: `passive_receive_timeout_preserves_partial_data_for_the_next_call`,
`changing_active_mode_or_closing_interrupts_an_existing_passive_receive`,
`tcp_and_tls_io_failures_do_not_commit_options_or_hide_failure_reasons`,
`killing_the_owner_cancels_passive_io_and_closes_retained_aliases`, and
`cancelling_the_execution_releases_pending_io_and_live_domain_resources`.
Actual network tests interrupt pending read, backpressured write, and shutdown.

## Send

The stdlib's retained BytesTree consumer materializes the write bytes once;
aliases and padded bit-array semantics remain unchanged. Success is `Ok(Nil)`.
Wrong transport returns `Badarg`; closed aliases return `Closed`; IO failures
map to `SocketReason`. Send timeout follows `SendTimeoutClose`, closing and
waking pending operations when enabled. TCP writes preserve byte order, not
application packet boundaries.

Owner tests: `bytes_tree_shapes_and_aliases_preserve_exact_tcp_and_tls_writes`,
`send_timeout_respects_close_policy_and_retains_padded_bytes_tree_across_await`,
`transport_mismatch_is_rejected_without_io_and_closed_tls_aliases_stay_closed`,
and `tcp_and_tls_io_failures_do_not_commit_options_or_hide_failure_reasons`.
Every successful client scenario compares complete response bytes.

## Close

Generic TCP close accepts a retained Socket or ListenSocket; unrelated source
specializations return `Badarg`. TLS close uses its declared Socket type.
Repeated close succeeds. Handles keep logical equality/hash and do not keep IO
alive. Process exit/kill, application exit, domain close, and cancellation close
owned resources.
TLS listener cleanup uses process ownership because the original public typed
API does not expose a listener close function.

Owner tests: `socket_aliases_keep_source_equality_hashing_and_inspection_after_close`,
`transport_mismatch_is_rejected_without_io_and_closed_tls_aliases_stay_closed`,
`closing_the_execution_service_releases_all_live_resources_and_invalidates_aliases`,
and the cancellation/owner-kill tests above. The ALPN source scenario monitors
normal owner exit; original server scenarios terminate their supervisors.

## Shutdown

The unchanged public wrapper selects Atom `write`. Success shuts down writing
and returns `Ok(Nil)`; invalid modes return `Badarg`, closed connections return
`Closed`, and dependency errors retain the typed IO classification.

Owner tests: `original_tcp_controls_preserve_typed_results_values_and_effect_order`,
`tls_native_calls_preserve_handshake_alias_protocol_and_mapped_addresses`, and
`tcp_and_tls_io_failures_do_not_commit_options_or_hide_failure_reasons`.
The native reactor test checks actual pending shutdown cancellation.

## Handoff

Only the current owner may transfer a live Socket to a live Pid. Failures return
Atom reasons (`not_owner`, `badarg`, `closed`, or `einval`). Transfer pauses
active delivery, selectively forwards the socket's queued data/error/passive/
closed records in FIFO order, then commits the new owner. Other socket records
and user messages retain their order. Late IO is buffered; cancellation restores
delivery. Retained callbacks do not bypass closed-resource checks.

Owner tests: `controlling_process_moves_only_its_socket_records_in_mailbox_order`,
`tcp_and_tls_handoff_requires_current_owner_and_live_target`,
`transfer_phases_reject_closed_or_busy_resources_dead_targets_and_restore_on_cancel`,
`completed_reads_preserve_data_and_endings_during_passive_transfer_or_close`,
`a_retained_reader_invoked_after_close_finishes_without_reading`, and
`becoming_passive_cancels_an_active_reader_waiting_for_the_read_owner`.
The original pool, user-selector and TLS scenarios execute normal handoff.

## Options

All 13 TcpOption constructors retain their exact source schema. ALPN List input
stays retained and is decoded on demand. Conversion maps active/IP/TLS options
to native views; equality, hashing, inspection, and views agree. Generic merge
uses the original type key and override order; unsupported native keys fail at
the host boundary. Creation-only options are validated at listen and return `Einval` when supplied
to connection setopts. Reuseaddr is mutable and applied to the real OS socket.
Mutable connection settings commit after full input validation and IO configure.

Defaults follow original `merge_with_defaults`: binary, passive, nodelay,
reuseaddr, 30-second send timeout, and close on send timeout. The provider's
application buffer is 65536 bytes. `Once`, `Count`, `Active`, and `Passive`
produce exact TCP/TLS data/error/closed/passive records. Count updates are
relative and bounded; non-positive count switches to passive.

Owner tests: `original_options_keep_type_keys_override_order_and_captured_values`,
`option_storage_callbacks_agree_with_the_original_native_view_without_cloning_payloads`,
`generic_merge_reports_unsupported_keys_as_host_failures`,
`tcp_and_tls_getopts_preserve_defaults_updates_and_atomic_validation`,
`tcp_and_tls_once_count_and_active_deliver_exact_records_and_errors`, and
`rearming_after_passive_eof_delivers_closed_for_tcp_and_tls`.
Source server scenarios use Once, Count(2), and the original handler's rearm.

## Address And Options

Peer/listener address returns `Ok(#(Dynamic, Int))` with an IPv4 four-integer
or IPv6 eight-integer tuple and actual port. Defined socket failures map to
`SocketReason`. Getopts preserves request order and Atom handles. Queries are
`recbuf`, `buffer`, `nodelay`, `reuseaddr`, `mode`, `active`, `linger`, `send_timeout`, and
`send_timeout_close`; unknown options return `Badarg`. Kernel `recbuf` is OS
owned, while application `buffer` follows the configured value.

Owner tests: `original_tcp_controls_preserve_typed_results_values_and_effect_order`,
`tls_native_calls_preserve_handshake_alias_protocol_and_mapped_addresses`,
`tcp_and_tls_getopts_preserve_defaults_updates_and_atomic_validation`, and
`tcp_and_tls_io_failures_do_not_commit_options_or_hide_failure_reasons`.
Actual TCP/TLS and IPv6 scenarios use public address and information wrappers.

## Information

Concrete transport and generic TCP info produce a Dict with portable fields
`domain`, `type`, `protocol`, `local_address`, and `peer_address`. Generic
specializations must accept Atom keys and the returned native data; incompatible
key/item types, closed sockets, and metadata IO errors remain host failures.
Erlang VM counters and internal socket state are not emulated.

Owner tests: `original_tcp_controls_preserve_typed_results_values_and_effect_order`,
`tls_native_calls_preserve_handshake_alias_protocol_and_mapped_addresses`, and
`native_fatal_boundaries_preserve_the_operation_and_actual_specialization`.

## Handshake

TLS handshake returns the same logical Socket on success. Failed handshake
returns the declared `Error(Nil)`, closes IO, and wakes readers. Cancellation
and close cannot reinstall completed TLS IO into a closed connection.

Owner tests: `tls_native_calls_preserve_handshake_alias_protocol_and_mapped_addresses`,
`listener_and_handshake_io_failures_keep_the_original_result_shapes`,
`cancelling_or_failing_tls_handshake_closes_the_connection`,
`explicit_close_interrupts_an_already_pending_tls_handshake`, and
`tls_handshake_and_duplex_io_use_only_the_selected_reactor`.
Actual TLS fixtures verify the certificate and hostname, h2, absent ALPN, and
fatal ALPN mismatch at both endpoints.

## Protocol

TLS returns the negotiated protocol as String or the normalized
`Error("Socket not negotiated")`. Invalid UTF-8 from a custom host capability
fails at the String boundary. A direct call to the original undefined TCP
external remains a host failure; the transport's TCP wrapper returns
`Error("Can't negotiate protocol on tcp")` without calling it.

Owner tests: `tls_native_calls_preserve_handshake_alias_protocol_and_mapped_addresses`,
`native_fatal_boundaries_preserve_the_operation_and_actual_specialization`, and
`original_tcp_controls_preserve_typed_results_values_and_effect_order`.
The original ALPN FFI's charlist error is a representation bug, documented in
[README.md](../README.md#backend-behavior). The h2 client deliberately offers
http/1.1 first to prove server preference rather than client preference.

## Parse

The original private Charlist signature produces its caller-specialized IPv4
or IPv6 result. Standard literals succeed; invalid text returns `Error(Nil)`.
A specialization that cannot represent the address remains a host failure.
Legacy abbreviated Erlang IPv4 parsing is outside documented builder inputs.

Owner test: `generic_parse_preserves_original_charlist_signature_and_return_specialization`.
Original bind source covers localhost, IPv4, and IPv6; the pool and supervised
IPv6 scenarios bind and communicate through the unchanged public builder.

## Mapped Address

The generic native tuple input accepts source IPv4 and IPv6 address families,
validates their integer ranges, and returns the original four-integer mapping.
Bad shape or specialization fails at the host boundary.

Owner test: `generic_address_conversion_accepts_both_address_families_and_rejects_bad_data`.
The TLS mapped-address owner test compares the original wrapper's result.

## Records

The unchanged handler consumes Dynamic socket records. Projection validates
tag, arity, and the requested BitArray or SocketReason payload at the host
boundary. TCP and TLS data/error families preserve source values.

Owner tests: `socket_record_projection_validates_tags_arity_and_requested_payload`
and `socket_error_projection_uses_the_declared_reason_family_and_rejects_other_payloads`.
The original handler tests and public server scenarios execute record decoding.

## Rescue

Generic callback success returns the same typed source value inside `Ok`.
Upstream rescue catches Erlang throw only; Geam source has no throw class.
Source panic, provider failure, and cancellation keep their identity, causing
normal original handler/supervision cleanup. Intentional application exit also
propagates through rescue: the domain closes pending readers, accepts, and live
sockets before returning `ExecutionOutcome::Exited(status)`. The embedding
process stays alive and can start a fresh domain in the same module.

Owner tests: `unchanged_tcp_and_tls_handlers_preserve_callback_state_and_close_after_eof`
`unchanged_handler_normal_stop_abnormal_stop_panic_and_io_error_release_the_owner`,
and `application_exit_in_rescue_closes_tcp_and_tls_servers_with_pending_io`.
The user selector scenario exercises retained generic callbacks in both live
and prepared executions.
