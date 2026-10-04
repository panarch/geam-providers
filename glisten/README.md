# geam-glisten

This crate implements all 38 Erlang native declarations used by the unchanged
[`glisten` 9.0.1](https://hex.pm/packages/glisten/9.0.1) Gleam package through
Geam's public typed provider API. The original Gleam builders, acceptor pool,
connection handlers, selectors, and supervision run as upstream source. The
supported package range is limited to 9.0.1 until other versions are tested.

Geam is pinned to `main` commit
`e5e1f5f772c6f48369050bdf3ee35c7a324277e2`. The
[standalone fixture](fixtures/gleam/) selects `geam-glisten`, `geam-otp`,
`geam-logging`, and `geam-argv` explicitly. The
[embedding fixture](fixtures/embedding/) composes the same providers with
Geam's stdlib and Erlang components. Its execution host enables time only;
Glisten owns the IO reactor. Fixture Gleam dependencies are locked to stdlib
1.0.3, Erlang 1.3.0, OTP 1.3.0, logging 1.5.0, and argv 1.1.0.

## Effects And Resource Ownership

`Component` registers the native functions and an execution service. `State`
selects an explicit `network::Network` capability. Default component
initialization accepts empty configuration and creates one `TokioNetwork`
reactor before source execution; unknown keys and reactor initialization errors
fail initialization. Embedding callers can use `State::with_network` to supply
another implementation of the public listener and connection contracts.

Listeners and connections belong to the execution domain. Socket values retain
logical identity, not live IO. Aliases keep their equality and hash after close;
owner exit, kill, application exit, execution cancellation, or domain shutdown
closes resources and wakes pending IO. Ownership transfer pauses active delivery, forwards only
that socket's queued records in order, then changes the owner. Unrelated mailbox
messages retain their order. A cancelled transfer restores delivery.

TCP and TLS support passive reads, `Once`, `Count`, and `Active` delivery,
partial stream reads, timeouts, BytesTree writes, write shutdown, and repeated
close. BytesTree input is retained through the stdlib's public consumer and is
materialized only for the write; input aliases are preserved. TLS uses rustls
with explicit certificate/key files, TLS 1.2/1.3, and server ALPN preference.
The explicitly selected rustls crypto backend is AWS-LC, matching the existing
workspace TLS dependency. The package exposes no client certificate
authentication option.

## Backend Behavior

The compatibility boundary is the
[repository policy](../docs/design/provider-compatibility.md). TCP byte streams,
TLS authentication by clients, ALPN, typed results, message order, and resource
lifetime are preserved. OS read chunk sizes and receive-buffer values remain
platform dependent. Applications must accumulate a stream rather than assume
one packet per read.

`socket_info` returns portable Atom/Dynamic entries `domain`, `type`, `protocol`,
`local_address`, and `peer_address`. It does not emulate Erlang VM counters or
internal socket metadata. The generic TCP declaration accepts compatible
specializations, including `Dict(Atom, Dynamic)`; incompatible keys or items
fail at the host conversion boundary. Address parsing accepts standard IPv4
and IPv6 literals. Erlang's legacy abbreviated IPv4 syntax is not a promised
Glisten input and is not accepted by this backend.

The provider keeps the declared Gleam error shapes where upstream FFI returns
another native shape. For example, a failed TLS handshake returns `Error(Nil)`;
ALPN absence returns `Error("Socket not negotiated")` as a Gleam String, while
the original FFI returns an Erlang charlist. Option errors return the declared
`SocketReason`, including `Badarg` for invalid caller values and `Einval` for
creation-only options applied to an existing connection. `Reuseaddr` remains
mutable and is applied to the OS socket. The original
TCP `negotiated_protocol` external refers to an undefined Erlang function; a
direct call remains a host failure. The unchanged transport wrapper returns
its documented TCP negotiation error before calling that external.

The original handler `rescue` catches Erlang `throw` only. Geam source execution
has no throw class: successful generic callbacks return `Ok`, while source
panics, host failures, execution cancellation, and intentional application exit
retain their original identity. They are not converted into a socket error or
a successful result. Application exit closes the domain and pending IO before
the embedding caller receives `ExecutionOutcome::Exited(status)`, while the
embedding process stays alive.

TLS listeners have no public close function in the original typed API. The
fixtures give them a process owner and verify cleanup on that owner's exit.

## Verification

[fixtures/CONTRACTS.md](fixtures/CONTRACTS.md) maps every declaration to its
owner, caller contract, and mandatory tests. The same client driver exercises
seven scenarios in original Erlang, live/prepared embedding, Geam `run`, and
built executables outside the fixture: TCP controls, concurrent acceptor pool,
user selector, supervised IPv6, TLS with server-preferred `h2`, no client ALPN,
and ALPN mismatch. Test certificates and their private key are public fixture
credentials, valid from 2020 to 2100, supplied as explicit arguments.

Owner tests cover typed options and storage, generic specialization, native
failures, active records, mailbox transfer, callback exit, partial IO,
backpressure, TLS failure, cancellation, and cleanup. The
[testing guide](../docs/development/testing.md#glisten-tcp-and-tls-servers)
contains the mandatory commands and file-by-file 100% line/region coverage
gate. CI declares Linux, macOS, and Windows; local execution is a separate
result from hosted CI on those runners.

The [fixture hook](fixtures/ci.sh) registers all four execution phases through
the provider's Cargo metadata. Use the common
[CI runner](../docs/development/ci.md#local-execution) for local execution;
the shared workflow retains coverage, quality, binding, test, and package gates.

The crate is intended for source-based use with the pinned Geam commit.
Publication and consumption through released crates are separate steps.
