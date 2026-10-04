//! Statically declared native signatures and exact, call-scoped conversions.
use crate::schema::{ErlangTcpOption, Four, ListenSocket, One, Socket, SocketReason, Three, Two};
use crate::socket::types;
use crate::sockets::{Active, Connection, Resource, Transport};
use crate::{A, B, Call, Component, GlistenProfile, active, settings, sockets};
use futures_util::TryFutureExt;
use futures_util::future::{Either, ready};
use geam::__macro_support::{
    ProviderConstruction, ProviderConstructionIndex0, ProviderConstructionList,
    ProviderConstructionRequirements, ProviderConstructions, ProviderOutputValue, ProviderValue,
};
use geam::gleam_erlang::{Atom, Pid, service};
use geam::gleam_stdlib::{
    DictOf,
    provider_support::{Dynamic, DynamicSchema, GleamError, GleamOk, GleamResult},
};
use geam::host::native::{NativeCall, NativeRules};
use geam::host::{
    HostCallCompletion, HostCallContinuation, HostCallError, HostCreatedFunction, HostExternal,
    HostList, HostListType, HostProviderModule, HostRegistrationError, HostTupleType, HostType,
    HostTypeIndex0, HostTypeIndexNext, HostValue,
};
use geam::provider::advanced::NativeValue;
use geam::provider::{BigInt, BitArrayValue, HostFailure, StringValue};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

type I0 = HostTypeIndex0;
type I1 = HostTypeIndexNext<I0>;
type I2 = HostTypeIndexNext<I1>;
type OptionList = HostListType<ErlangTcpOption>;
type UnitResult = GleamResult<(), SocketReason>;
type ListenResult = GleamResult<ListenSocket, SocketReason>;
type SocketResult = GleamResult<Socket, SocketReason>;
type BytesResult = GleamResult<BitArrayValue, SocketReason>;
type HandoffResult = GleamResult<(), Atom>;
type AddressResult = GleamResult<HostTupleType<Two<Dynamic, BigInt>>, SocketReason>;
type OptionItem = HostTupleType<Two<Atom, Dynamic>>;
type OptionItems = HostListType<OptionItem>;
type OptionsResult = GleamResult<OptionItems, SocketReason>;
type AcceptTargets = Three<SocketResult, Socket, HostCreatedFunction<active::Reader>>;
type Timer = Pin<Box<dyn Future<Output = ()> + Send>>;
type HandshakeResult = GleamResult<Socket, ()>;
type ProtocolResult = GleamResult<StringValue, StringValue>;
type Ipv4 = HostTupleType<Four<BigInt, BigInt, BigInt, BigInt>>;
type InfoCall<'call, Profile, Key, Item> =
    NativeCall<'call, Profile, Component, DictOf<Key, Item>, Three<DictOf<Key, Item>, Key, Item>>;

type ReasonRequirements = <types::SocketReason as ProviderValue>::OutputRequirements;
type ReasonTargets =
    <ReasonRequirements as ProviderConstructionRequirements>::Types<geam::host::HostTypeListEnd>;
type CloseRequirements = ProviderConstructionList<
    ReasonRequirements,
    ProviderConstructionList<ProviderConstruction<Socket>, ProviderConstruction<ListenSocket>>,
>;
type CloseTargets =
    <CloseRequirements as ProviderConstructionRequirements>::Types<geam::host::HostTypeListEnd>;
type ListenRequirements =
    ProviderConstructionList<ReasonRequirements, ProviderConstruction<ListenSocket>>;
type ListenTargets =
    <ListenRequirements as ProviderConstructionRequirements>::Types<geam::host::HostTypeListEnd>;
type Address = HostTupleType<Two<Dynamic, BigInt>>;
type AddressRequirements = ProviderConstructionList<
    ReasonRequirements,
    ProviderConstructionList<ProviderConstruction<Dynamic>, ProviderConstruction<Address>>,
>;
type AddressTargets =
    <AddressRequirements as ProviderConstructionRequirements>::Types<geam::host::HostTypeListEnd>;
type OptionsRequirements = ProviderConstructionList<
    ReasonRequirements,
    ProviderConstructionList<
        ProviderConstruction<Dynamic>,
        ProviderConstructionList<
            ProviderConstruction<OptionItem>,
            ProviderConstruction<OptionItems>,
        >,
    >,
>;
type OptionsTargets =
    <OptionsRequirements as ProviderConstructionRequirements>::Types<geam::host::HostTypeListEnd>;
type I3 = HostTypeIndexNext<I2>;

fn typed_error<'call, Profile: GlistenProfile, Success: HostType>(
    mut call: Call<'call, Profile, GleamResult<Success, SocketReason>>,
    proof: ProviderConstructions<'call, ReasonRequirements>,
    reason: types::SocketReason,
) -> Result<HostCallCompletion<'call, GleamResult<Success, SocketReason>>, HostCallError> {
    reason
        .into_host(&mut call, &proof)
        .map(|reason| call.return_custom::<GleamError<Success, SocketReason>>((reason, ())))
}

enum LookupFailure {
    Closed,
    WrongTransport,
}
impl LookupFailure {
    fn native_reason(self) -> &'static str {
        match self {
            Self::Closed => "closed",
            Self::WrongTransport => "badarg",
        }
    }
    fn source_reason(self) -> types::SocketReason {
        match self {
            Self::Closed => types::SocketReason::Closed,
            Self::WrongTransport => types::SocketReason::Badarg,
        }
    }
}

pub(crate) fn ok(value: NativeValue) -> NativeValue {
    NativeValue::tuple([NativeValue::symbol("ok"), value])
}
pub(crate) fn fail(reason: &'static str) -> NativeValue {
    NativeValue::tuple([NativeValue::symbol("error"), NativeValue::symbol(reason)])
}
fn nil() -> NativeValue {
    NativeValue::symbol("nil")
}

pub(crate) fn tcp<Profile: GlistenProfile>(
    module: HostProviderModule<Profile>,
) -> Result<HostProviderModule<Profile>, HostRegistrationError> {
    shared::<Profile, false>(module)
        .and_then(|module| module.with_scoped_function_and_constructions::<Component, (BigInt, OptionList), ListenResult, ListenTargets, _>("do_listen_tcp", listen::<Profile, false>))
        .and_then(|module| module.with_native_function::<Component, (A,), UnitResult, CloseTargets, _>("close", NativeRules::default(), close::<Profile>))
        .and_then(|module| module.with_native_function::<Component, (Socket,), DictOf<A, B>, Three<DictOf<A, B>, A, B>, _>("socket_info", service::native_rules(NativeRules::default()), info::<Profile, A, B>))
        .and_then(|module| module.with_scoped_diverging_function::<Component, (Socket,), A, _>("negotiated_protocol", no_tcp_protocol::<Profile>))
}

pub(crate) fn ssl<Profile: GlistenProfile>(
    module: HostProviderModule<Profile>,
) -> Result<HostProviderModule<Profile>, HostRegistrationError> {
    shared::<Profile, true>(module)
        .and_then(|module| module.with_scoped_function_and_constructions::<Component, (BigInt, OptionList), ListenResult, ListenTargets, _>("do_listen", listen::<Profile, true>))
        .and_then(|module| module.with_scoped_function::<Component, (Socket,), UnitResult, _>("close", ssl_close::<Profile>))
        .and_then(|module| module.with_resumable_native_function::<Component, (Socket,), GleamResult<Socket, ()>, One<GleamResult<Socket, ()>>, _>("handshake", NativeRules::default(), handshake::<Profile>))
        .and_then(|module| module.with_scoped_function::<Component, (Socket,), ProtocolResult, _>("negotiated_protocol", protocol::<Profile>))
}

fn shared<Profile: GlistenProfile, const TLS: bool>(
    module: HostProviderModule<Profile>,
) -> Result<HostProviderModule<Profile>, HostRegistrationError> {
    module
        .with_resumable_native_function::<Component, (ListenSocket, BigInt), SocketResult, AcceptTargets, _>("accept_timeout", NativeRules::default(), accept_timeout::<Profile, TLS>)
        .and_then(|module| module.with_resumable_native_function::<Component, (ListenSocket,), SocketResult, AcceptTargets, _>("accept", NativeRules::default(), accept::<Profile, TLS>))
        .and_then(|module| module.with_resumable_native_function::<Component, (Socket, BigInt, BigInt), BytesResult, One<BytesResult>, _>("receive_timeout", NativeRules::default(), receive_timeout::<Profile, TLS>))
        .and_then(|module| module.with_resumable_native_function::<Component, (Socket, BigInt), BytesResult, One<BytesResult>, _>("receive", NativeRules::default(), receive::<Profile, TLS>))
        .and_then(|module| module.with_resumable_native_function::<Component, (Socket, Pid), HandoffResult, One<HandoffResult>, _>("controlling_process", service::native_rules(NativeRules::default()), controlling::<Profile, TLS>))
        .and_then(|module| module.with_scoped_function_and_constructions::<Component, (Socket, OptionList), UnitResult, ReasonTargets, _>("do_set_opts", set_opts::<Profile, TLS>))
        .and_then(|module| module.with_resumable_native_function::<Component, (Socket, Atom), UnitResult, One<UnitResult>, _>("do_shutdown", service::native_rules(NativeRules::default()), shutdown::<Profile, TLS>))
        .and_then(|module| module.with_scoped_function_and_constructions::<Component, (Socket,), AddressResult, AddressTargets, _>("peername", peername::<Profile, TLS>))
        .and_then(|module| module.with_scoped_function_and_constructions::<Component, (ListenSocket,), AddressResult, AddressTargets, _>("sockname", sockname::<Profile, TLS>))
        .and_then(|module| module.with_scoped_function_and_constructions::<Component, (Socket, HostListType<Atom>), OptionsResult, OptionsTargets, _>("get_socket_opts", get_opts::<Profile, TLS>))
}

fn transport_kind<const TLS: bool>() -> Transport {
    const { if TLS { Transport::Tls } else { Transport::Tcp } }
}

fn listen<'call, Profile: GlistenProfile, const TLS: bool>(
    call: Call<'call, Profile, ListenResult>,
    constructions: geam::host::HostConstructions<'call, ListenTargets>,
    port: BigInt,
    options: HostList<'call, ErlangTcpOption>,
) -> Result<HostCallCompletion<'call, ListenResult>, HostCallError> {
    call.with_execution_unit(move |mut call, owner| {
        let listener = u16::try_from(port)
            .map_err(|_| types::SocketReason::Badarg)
            .and_then(|port| {
                let mut builder = settings::ListenBuilder::new(port);
                let mut index = 0;
                while let Some(option) = call.list_item::<ErlangTcpOption>(options, index) {
                    builder
                        .push(&call.external_payload(option).typed)
                        .map_err(|_| types::SocketReason::Badarg)?;
                    index += 1;
                }
                builder.finish(TLS).map_err(|_| types::SocketReason::Badarg)
            })
            .and_then(|(options, settings)| {
                call.state()
                    .network
                    .listen(options)
                    .map(|io| (io, settings))
                    .map_err(sockets::reason)
            });
        match listener {
            Ok((io, settings)) => {
                let key = call.service::<Component>().insert(
                    &owner,
                    Resource::Listener {
                        owner: owner.clone(),
                        io,
                        settings,
                        transport: transport_kind::<TLS>(),
                    },
                );
                let value =
                    call.construct_external(constructions.at::<I1>(), types::ListenSocket { key });
                Ok(call.return_custom::<GleamOk<ListenSocket, SocketReason>>((value, ())))
            }
            Err(reason) => typed_error(
                call,
                ProviderConstructions::<ListenRequirements>::new(&constructions)
                    .select::<ProviderConstructionIndex0>(),
                reason,
            ),
        }
    })
}

fn timer<Profile: GlistenProfile, Return: HostType>(
    call: &Call<'_, Profile, Return>,
    timeout: Option<BigInt>,
) -> Result<Option<Timer>, &'static str> {
    timeout
        .map(|timeout| {
            let timeout = u64::try_from(timeout).map_err(|_| "badarg")?;
            let deadline = call
                .clock()
                .now()
                .checked_add(Duration::from_millis(timeout))
                .ok_or("badarg")?;
            Ok(call.clock().sleep_until(deadline))
        })
        .transpose()
}

async fn timed<T>(
    future: impl Future<Output = Result<T, &'static str>>,
    timer: Option<Timer>,
) -> Result<T, &'static str> {
    match timer {
        Some(timer) => {
            tokio::select! { biased; result = future => result, () = timer => Err("timeout") }
        }
        None => future.await,
    }
}

fn accept<'call, Profile: GlistenProfile, const TLS: bool>(
    native: NativeCall<'call, Profile, Component, SocketResult, AcceptTargets>,
    socket: HostExternal<'call, ListenSocket>,
) -> Result<HostCallContinuation<'call, SocketResult>, HostCallError> {
    accept_impl::<Profile, TLS>(native, socket, None)
}
fn accept_timeout<'call, Profile: GlistenProfile, const TLS: bool>(
    native: NativeCall<'call, Profile, Component, SocketResult, AcceptTargets>,
    socket: HostExternal<'call, ListenSocket>,
    timeout: BigInt,
) -> Result<HostCallContinuation<'call, SocketResult>, HostCallError> {
    accept_impl::<Profile, TLS>(native, socket, Some(timeout))
}

fn accept_impl<'call, Profile: GlistenProfile, const TLS: bool>(
    mut native: NativeCall<'call, Profile, Component, SocketResult, AcceptTargets>,
    socket: HostExternal<'call, ListenSocket>,
    timeout: Option<BigInt>,
) -> Result<HostCallContinuation<'call, SocketResult>, HostCallError> {
    let key = native.call().external_payload(socket).key.clone();
    native.call().require_execution_unit().map(|owner| {
        let timer = timer(native.call(), timeout);
        let listener = match native.call().service::<Component>().resources.get(&key) {
            Some(Resource::Listener {
                io,
                settings,
                transport,
                ..
            }) if *transport == transport_kind::<TLS>() => Ok((Arc::clone(io), settings.clone())),
            _ => Err("closed"),
        };
        let network = Arc::clone(&native.call().state().network);
        native.resume::<I0>(move |context| {
            Box::pin(async move {
                let result = match (listener, timer) {
                    (Ok((listener, settings)), Ok(timer)) => timed(
                        async { listener.accept().await.map_err(sockets::native_reason) },
                        timer,
                    )
                    .await
                    .map(|io| (io, settings)),
                    (Err(reason), _) | (_, Err(reason)) => Err(reason),
                };
                match result {
                    Err(reason) => Ok(fail(reason)),
                    Ok((io, settings)) => {
                        let mut accepted = PendingAccept::new(io);
                        context
                            .with_constructions(move |mut call, constructions| {
                                let socket = Arc::new(Connection::new(
                                    Arc::clone(&accepted.io),
                                    network,
                                    owner.clone(),
                                    settings,
                                    transport_kind::<TLS>(),
                                ));
                                let key = call
                                    .service::<Component>()
                                    .insert(&owner, Resource::Connection(socket));
                                accepted.commit();
                                let socket = call.construct_external(
                                    constructions.at::<I1>(),
                                    types::Socket { key },
                                );
                                let reader = call.construct_function::<active::Reader>(
                                    constructions.at::<I2>(),
                                    (socket, ()),
                                );
                                call.spawn(reader);
                                ok(call.native_value::<Socket>(socket))
                            })
                            .await
                    }
                }
            })
        })
    })
}

// Accepted IO belongs to this admission phase until the live execution
// service owns it. Geam rejects service requests from inactive execution units.
struct PendingAccept {
    io: Arc<dyn crate::network::Connection>,
    admitted: bool,
}
impl PendingAccept {
    fn new(io: Arc<dyn crate::network::Connection>) -> Self {
        Self {
            io,
            admitted: false,
        }
    }
    fn commit(&mut self) {
        self.admitted = true;
    }
}
impl Drop for PendingAccept {
    fn drop(&mut self) {
        if !self.admitted {
            self.io.close();
        }
    }
}

fn connection<'call, Profile: GlistenProfile, Return: HostType, const TLS: bool>(
    call: &mut Call<'call, Profile, Return>,
    socket: HostExternal<'call, Socket>,
) -> Result<Arc<Connection>, LookupFailure> {
    let key = call.external_payload(socket).key.clone();
    let socket = call
        .service::<Component>()
        .connection(&key)
        .ok_or(LookupFailure::Closed)?;
    if socket.transport != transport_kind::<TLS>() {
        return Err(LookupFailure::WrongTransport);
    }
    Ok(socket)
}

fn receive<'call, Profile: GlistenProfile, const TLS: bool>(
    native: NativeCall<'call, Profile, Component, BytesResult, One<BytesResult>>,
    socket: HostExternal<'call, Socket>,
    length: BigInt,
) -> Result<HostCallContinuation<'call, BytesResult>, HostCallError> {
    receive_impl::<Profile, TLS>(native, socket, length, None)
}
fn receive_timeout<'call, Profile: GlistenProfile, const TLS: bool>(
    native: NativeCall<'call, Profile, Component, BytesResult, One<BytesResult>>,
    socket: HostExternal<'call, Socket>,
    length: BigInt,
    timeout: BigInt,
) -> Result<HostCallContinuation<'call, BytesResult>, HostCallError> {
    receive_impl::<Profile, TLS>(native, socket, length, Some(timeout))
}

fn receive_impl<'call, Profile: GlistenProfile, const TLS: bool>(
    mut native: NativeCall<'call, Profile, Component, BytesResult, One<BytesResult>>,
    socket: HostExternal<'call, Socket>,
    length: BigInt,
    timeout: Option<BigInt>,
) -> Result<HostCallContinuation<'call, BytesResult>, HostCallError> {
    let socket =
        connection::<Profile, _, TLS>(native.call(), socket).map_err(LookupFailure::native_reason);
    let length = usize::try_from(length).map_err(|_| "badarg");
    let timer = timer(native.call(), timeout);
    Ok(native.resume::<I0>(move |context| {
        Box::pin(async move {
            let bytes = match (socket, length, timer) {
                (Ok(socket), Ok(length), Ok(timer)) => {
                    timed(crate::active::receive(&socket, length), timer).await
                }
                (Err(reason), _, _) | (_, Err(reason), _) | (_, _, Err(reason)) => Err(reason),
            };
            match bytes {
                Ok(bytes) => {
                    context
                        .with_call(move |call| {
                            ok(
                                call.native_value::<BitArrayValue>(BitArrayValue::from_bytes(
                                    bytes,
                                )),
                            )
                        })
                        .await
                }
                Err(reason) => Ok(fail(reason)),
            }
        })
    }))
}

fn close<'call, Profile: GlistenProfile>(
    mut native: NativeCall<'call, Profile, Component, UnitResult, CloseTargets>,
    value: HostValue<'call, A>,
) -> Result<HostCallCompletion<'call, UnitResult>, HostCallError> {
    let source = native.source::<A>(value);
    let key = if let Some(socket) = native.convert::<I1>(&source) {
        Some(native.call().external_payload(socket).key.clone())
    } else {
        native
            .convert::<I2>(&source)
            .map(|socket| native.call().external_payload(socket).key.clone())
    };
    let (mut call, constructions) = native.into_call();
    match key {
        Some(key) => {
            call.service::<Component>().remove(&key);
            Ok(call.return_custom::<GleamOk<(), SocketReason>>(((), ())))
        }
        None => typed_error(
            call,
            ProviderConstructions::<CloseRequirements>::new(&constructions)
                .select::<ProviderConstructionIndex0>(),
            types::SocketReason::Badarg,
        ),
    }
}

fn ssl_close<'call, Profile: GlistenProfile>(
    mut call: Call<'call, Profile, UnitResult>,
    socket: HostExternal<'call, Socket>,
) -> Result<HostCallCompletion<'call, UnitResult>, HostCallError> {
    let key = call.external_payload(socket).key.clone();
    call.service::<Component>().remove(&key);
    Ok(call.return_custom::<GleamOk<(), SocketReason>>(((), ())))
}

fn set_opts<'call, Profile: GlistenProfile, const TLS: bool>(
    mut call: Call<'call, Profile, UnitResult>,
    constructions: geam::host::HostConstructions<'call, ReasonTargets>,
    socket: HostExternal<'call, Socket>,
    options: HostList<'call, ErlangTcpOption>,
) -> Result<HostCallCompletion<'call, UnitResult>, HostCallError> {
    let identity = call.native_value::<Socket>(socket);
    let result = connection::<Profile, _, TLS>(&mut call, socket)
        .map_err(LookupFailure::source_reason)
        .and_then(|socket| {
            let mut control = socket.control.lock();
            let mut next = control.settings.clone();
            let mut passive = false;
            let mut index = 0;
            while let Some(option) = call.list_item::<ErlangTcpOption>(options, index) {
                passive |=
                    settings::update_connection(&mut next, &call.external_payload(option).typed)?;
                index += 1;
            }
            socket.io.configure(&next.io).map_err(sockets::reason)?;
            control.settings = next;
            if passive {
                service::Processes::new(&mut call).send(
                    &control.owner,
                    NativeValue::tuple([
                        NativeValue::symbol(socket.transport.passive_tag()),
                        identity,
                    ]),
                );
            }
            drop(control);
            socket.changed.notify_waiters();
            Ok(())
        });
    match result {
        Ok(()) => Ok(call.return_custom::<GleamOk<(), SocketReason>>(((), ()))),
        Err(reason) => typed_error(call, ProviderConstructions::new(&constructions), reason),
    }
}

fn shutdown<'call, Profile: GlistenProfile, const TLS: bool>(
    mut native: NativeCall<'call, Profile, Component, UnitResult, One<UnitResult>>,
    socket: HostExternal<'call, Socket>,
    how: HostExternal<'call, Atom>,
) -> Result<HostCallContinuation<'call, UnitResult>, HostCallError> {
    let how = native.source::<Atom>(how).as_symbol();
    let socket =
        connection::<Profile, _, TLS>(native.call(), socket).map_err(LookupFailure::native_reason);
    Ok(native.resume::<I0>(move |_| {
        Box::pin(async move {
            match socket {
                Ok(socket) if how.as_deref() == Some("write") => {
                    Ok(match socket.io.shutdown_write().await {
                        Ok(()) => ok(nil()),
                        Err(error) => fail(sockets::native_reason(error)),
                    })
                }
                Ok(_) => Ok(fail("badarg")),
                Err(reason) => Ok(fail(reason)),
            }
        })
    }))
}

fn handshake<'call, Profile: GlistenProfile>(
    mut native: NativeCall<'call, Profile, Component, HandshakeResult, One<HandshakeResult>>,
    socket: HostExternal<'call, Socket>,
) -> Result<HostCallContinuation<'call, HandshakeResult>, HostCallError> {
    let identity = native.source::<Socket>(socket);
    let socket = connection::<Profile, _, true>(native.call(), socket);
    Ok(native.resume::<I0>(move |_| {
        Box::pin(async move {
            match socket {
                Ok(socket) => match socket.io.handshake().await {
                    Ok(()) => {
                        socket.control.lock().ready = true;
                        socket.changed.notify_waiters();
                        Ok(ok(identity))
                    }
                    Err(_) => {
                        socket.close();
                        Ok(fail("nil"))
                    }
                },
                Err(_) => Ok(fail("nil")),
            }
        })
    }))
}

fn protocol<'call, Profile: GlistenProfile>(
    mut call: Call<'call, Profile, ProtocolResult>,
    socket: HostExternal<'call, Socket>,
) -> Result<HostCallCompletion<'call, ProtocolResult>, HostCallError> {
    // The upstream FFI deliberately maps every negotiation error to this
    // source-visible String. Keep that normalization explicit.
    let protocol = match connection::<Profile, _, true>(&mut call, socket) {
        Ok(socket) => socket.io.negotiated_protocol(),
        Err(_) => None,
    };
    match protocol {
        Some(protocol) => {
            let protocol: StringValue = String::from_utf8(protocol)
                .map_err(|_| HostFailure::new("negotiated ALPN is not a source String"))?
                .into();
            Ok(call.return_custom::<GleamOk<StringValue, StringValue>>((protocol, ())))
        }
        None => Ok(call.return_custom::<GleamError<StringValue, StringValue>>((
            "Socket not negotiated".into(),
            (),
        ))),
    }
}

fn no_tcp_protocol<'call, Profile: GlistenProfile>(
    _: Call<'call, Profile, A>,
    _: HostExternal<'call, Socket>,
) -> Result<std::convert::Infallible, HostCallError> {
    Err(HostFailure::new("undefined Erlang target tcp:negotiated_protocol/1").into())
}

fn native_address<Profile: GlistenProfile, Return: HostType>(
    call: &Call<'_, Profile, Return>,
    address: std::net::SocketAddr,
) -> NativeValue {
    let values = call.native_values();
    let ip = NativeValue::tuple(
        crate::network::ip_address(address.ip())
            .into_iter()
            .map(|part| values.integer(part.into())),
    );
    NativeValue::tuple([ip, values.integer(address.port().into())])
}

fn peername<'call, Profile: GlistenProfile, const TLS: bool>(
    mut call: Call<'call, Profile, AddressResult>,
    constructions: geam::host::HostConstructions<'call, AddressTargets>,
    socket: HostExternal<'call, Socket>,
) -> Result<HostCallCompletion<'call, AddressResult>, HostCallError> {
    let address = connection::<Profile, _, TLS>(&mut call, socket)
        .map_err(LookupFailure::source_reason)
        .and_then(|socket| socket.io.peer_address().map_err(sockets::reason));
    address_result(call, constructions, address)
}

fn sockname<'call, Profile: GlistenProfile, const TLS: bool>(
    mut call: Call<'call, Profile, AddressResult>,
    constructions: geam::host::HostConstructions<'call, AddressTargets>,
    socket: HostExternal<'call, ListenSocket>,
) -> Result<HostCallCompletion<'call, AddressResult>, HostCallError> {
    let key = call.external_payload(socket).key.clone();
    let address = match call.service::<Component>().resources.get(&key) {
        Some(Resource::Listener { io, transport, .. }) if *transport == transport_kind::<TLS>() => {
            io.address().map_err(sockets::reason)
        }
        _ => Err(types::SocketReason::Closed),
    };
    address_result(call, constructions, address)
}

fn address_result<'call, Profile: GlistenProfile>(
    mut call: Call<'call, Profile, AddressResult>,
    constructions: geam::host::HostConstructions<'call, AddressTargets>,
    address: Result<std::net::SocketAddr, types::SocketReason>,
) -> Result<HostCallCompletion<'call, AddressResult>, HostCallError> {
    match address {
        Ok(address) => {
            let ip = NativeValue::tuple(
                crate::network::ip_address(address.ip())
                    .into_iter()
                    .map(|part| call.native_values().integer(part.into())),
            );
            let ip = call.construct_external_with_binding::<geam::gleam_erlang::Component<Profile>, DynamicSchema, geam::host::HostTypeListEnd>(constructions.at::<I1>(), geam::gleam_stdlib::Dynamic::from_native(ip));
            let value =
                call.construct_tuple(constructions.at::<I2>(), (ip, (address.port().into(), ())));
            Ok(call.return_custom::<GleamOk<Address, SocketReason>>((value, ())))
        }
        Err(reason) => typed_error(
            call,
            ProviderConstructions::<AddressRequirements>::new(&constructions)
                .select::<ProviderConstructionIndex0>(),
            reason,
        ),
    }
}

fn get_opts<'call, Profile: GlistenProfile, const TLS: bool>(
    mut call: Call<'call, Profile, OptionsResult>,
    constructions: geam::host::HostConstructions<'call, OptionsTargets>,
    socket: HostExternal<'call, Socket>,
    options: HostList<'call, Atom>,
) -> Result<HostCallCompletion<'call, OptionsResult>, HostCallError> {
    let result = (|| {
        let socket = connection::<Profile, _, TLS>(&mut call, socket)
            .map_err(LookupFailure::source_reason)?;
        let settings = socket.control.lock().settings.clone();
        let mut items = Vec::new();
        let mut index = 0;
        while let Some(atom) = call.list_item::<Atom>(options, index) {
            let key = call.native_value::<Atom>(atom);
            let value = match key.as_symbol().as_deref() {
                Some("recbuf") => call
                    .native_values()
                    .integer(socket.io.receive_buffer().map_err(sockets::reason)?.into()),
                Some("buffer") => call.native_values().integer(settings.io.buffer.into()),
                Some("nodelay") => {
                    NativeValue::symbol(if settings.io.nodelay { "true" } else { "false" })
                }
                Some("reuseaddr") => NativeValue::symbol(if settings.io.reuse_address {
                    "true"
                } else {
                    "false"
                }),
                Some("mode") => NativeValue::symbol("binary"),
                Some("active") => match settings.active {
                    Active::Passive => NativeValue::symbol("false"),
                    Active::Once => NativeValue::symbol("once"),
                    Active::All => NativeValue::symbol("true"),
                    Active::Count(count) => call.native_values().integer(count.into()),
                },
                Some("linger") => NativeValue::tuple([
                    NativeValue::symbol(if settings.io.linger.is_some() {
                        "true"
                    } else {
                        "false"
                    }),
                    call.native_values().integer(
                        settings
                            .io
                            .linger
                            .map_or(0, |duration| duration.as_secs())
                            .into(),
                    ),
                ]),
                Some("send_timeout") => call
                    .native_values()
                    .integer(settings.send_timeout.as_millis().into()),
                Some("send_timeout_close") => NativeValue::symbol(if settings.send_timeout_close {
                    "true"
                } else {
                    "false"
                }),
                _ => return Err(types::SocketReason::Badarg),
            };
            let value = call.construct_external_with_binding::<geam::gleam_erlang::Component<Profile>, DynamicSchema, geam::host::HostTypeListEnd>(constructions.at::<I1>(), geam::gleam_stdlib::Dynamic::from_native(value));
            let item = call.construct_tuple(constructions.at::<I2>(), (atom, (value, ())));
            items.push(item);
            index += 1;
        }
        Ok(items)
    })();
    match result {
        Err(reason) => typed_error(
            call,
            ProviderConstructions::<OptionsRequirements>::new(&constructions)
                .select::<ProviderConstructionIndex0>(),
            reason,
        ),
        Ok(items) => {
            let list = call.construct_list(constructions.at::<I3>(), items);
            Ok(call.return_custom::<GleamOk<OptionItems, SocketReason>>((list, ())))
        }
    }
}

pub(crate) fn transport<Profile: GlistenProfile>()
-> Result<HostProviderModule<Profile>, HostRegistrationError> {
    HostProviderModule::new("glisten", "glisten/transport")
        .and_then(|module| module.with_native_function::<Component, (A,), HostTupleType<Four<BigInt, BigInt, BigInt, BigInt>>, One<HostTupleType<Four<BigInt, BigInt, BigInt, BigInt>>>, _>("convert_address", NativeRules::default(), mapped_address::<Profile>))
        .and_then(|module| module.with_scoped_function_and_constructions::<Component, (Socket,), DictOf<Atom, Dynamic>, Three<DictOf<Atom, Dynamic>, Atom, Dynamic>, _>("socket_info", typed_info::<Profile>))
}

fn mapped_address<'call, Profile: GlistenProfile>(
    native: NativeCall<'call, Profile, Component, Ipv4, One<Ipv4>>,
    address: HostValue<'call, A>,
) -> Result<HostCallCompletion<'call, Ipv4>, HostCallError> {
    let address = settings::address(&native.source::<A>(address)).map_err(HostFailure::new)?;
    let address = match address {
        std::net::IpAddr::V4(address) => address,
        std::net::IpAddr::V6(address) => {
            // inet:ipv4_mapped_ipv6_address reads the last two segments.
            // The unchanged Gleam decoder already selected its mapped branch.
            let segments = address.segments();
            let [a, b] = segments[6].to_be_bytes();
            let [c, d] = segments[7].to_be_bytes();
            std::net::Ipv4Addr::new(a, b, c, d)
        }
    };
    let [a, b, c, d] = address.octets().map(BigInt::from);
    let (mut call, constructions) = native.into_call();
    let value = call.construct_tuple(constructions.at::<I0>(), (a, (b, (c, (d, ())))));
    Ok(call.return_value(value))
}

fn info<'call, Profile: GlistenProfile, Key: HostType, Item: HostType>(
    mut native: InfoCall<'call, Profile, Key, Item>,
    socket: HostExternal<'call, Socket>,
) -> Result<HostCallCompletion<'call, DictOf<Key, Item>>, HostCallError> {
    let key = native.call().external_payload(socket).key.clone();
    let socket = native.call().service::<Component>().connection(&key);
    let metadata = socket_metadata(socket)?;
    let mut entries = Vec::new();
    for (key, value) in metadata_values(native.call(), metadata) {
        let key = native
            .convert::<I1>(&NativeValue::symbol(key))
            .ok_or_else(|| {
                HostFailure::new("socket info key specialization does not accept native symbols")
            })?;
        let value = native.convert::<I2>(&value).ok_or_else(|| {
            HostFailure::new("socket info item specialization does not accept native data")
        })?;
        entries.push((key, value));
    }
    let (mut call, constructions) = native.into_call();
    let dict = geam::gleam_stdlib::service::dict_from_entries(
        &mut call,
        constructions.at::<I0>(),
        entries,
    );
    Ok(call.return_value(dict))
}

struct SocketMetadata {
    local: std::net::SocketAddr,
    peer: std::net::SocketAddr,
    transport: Transport,
}
fn socket_metadata(socket: Option<Arc<Connection>>) -> Result<SocketMetadata, HostCallError> {
    let socket =
        socket.ok_or_else(|| HostFailure::new("socket info requires an open connection"))?;
    let local = socket
        .io
        .local_address()
        .map_err(|error| HostFailure::new(error.to_string()))?;
    let peer = socket
        .io
        .peer_address()
        .map_err(|error| HostFailure::new(error.to_string()))?;
    Ok(SocketMetadata {
        local,
        peer,
        transport: socket.transport,
    })
}
fn metadata_values<Profile: GlistenProfile, Return: HostType>(
    call: &Call<'_, Profile, Return>,
    metadata: SocketMetadata,
) -> [(&'static str, NativeValue); 5] {
    [
        (
            "domain",
            NativeValue::symbol(if metadata.local.is_ipv4() {
                "inet"
            } else {
                "inet6"
            }),
        ),
        ("type", NativeValue::symbol("stream")),
        ("protocol", NativeValue::symbol(metadata.transport.tag())),
        ("local_address", native_address(call, metadata.local)),
        ("peer_address", native_address(call, metadata.peer)),
    ]
}
fn typed_info<'call, Profile: GlistenProfile>(
    mut call: Call<'call, Profile, DictOf<Atom, Dynamic>>,
    constructions: geam::host::HostConstructions<
        'call,
        Three<DictOf<Atom, Dynamic>, Atom, Dynamic>,
    >,
    socket: HostExternal<'call, Socket>,
) -> Result<HostCallCompletion<'call, DictOf<Atom, Dynamic>>, HostCallError> {
    let key = call.external_payload(socket).key.clone();
    let metadata = socket_metadata(call.service::<Component>().connection(&key))?;
    let entries = metadata_values(&call, metadata).map(|(key, value)| {
        let key = call.construct_external_with_binding::<geam::gleam_erlang::Component<Profile>, geam::gleam_erlang::AtomSchema, geam::host::HostTypeListEnd>(constructions.at::<I1>(), NativeValue::symbol(key));
        let value = call.construct_external_with_binding::<geam::gleam_erlang::Component<Profile>, DynamicSchema, geam::host::HostTypeListEnd>(constructions.at::<I2>(), geam::gleam_stdlib::Dynamic::from_native(value));
        (key, value)
    });
    let dict = geam::gleam_stdlib::service::dict_from_entries(
        &mut call,
        constructions.at::<I0>(),
        entries,
    );
    Ok(call.return_value(dict))
}

fn controlling<'call, Profile: GlistenProfile, const TLS: bool>(
    mut native: NativeCall<'call, Profile, Component, HandoffResult, One<HandoffResult>>,
    socket: HostExternal<'call, Socket>,
    target: HostExternal<'call, Pid>,
) -> Result<HostCallContinuation<'call, HandoffResult>, HostCallError> {
    let identity = native.source::<Socket>(socket);
    native.call().require_execution_unit().map(|current| {
        let target = service::Processes::new(native.call()).pid(target);
        let socket = connection::<Profile, _, TLS>(native.call(), socket)
            .map_err(LookupFailure::native_reason);
        let valid = service::Processes::new(native.call()).is_alive(&target);
        native.resume::<I0>(move |context| {
            Box::pin(async move {
                let socket = match socket {
                    Ok(socket) => socket,
                    Err(reason) => return Ok(fail(reason)),
                };
                let mut transfer =
                    match sockets::Transfer::begin(Arc::clone(&socket), current.id(), valid) {
                        Ok(transfer) => transfer,
                        Err(reason) => return Ok(fail(reason)),
                    };
                let mut completion = Ok(());
                let mut draining = true;
                while completion.is_ok() && draining {
                    let identity = identity.clone();
                    let receive = context
                        .with_call(move |mut call| {
                            let now = call.clock().now();
                            service::Processes::new(&mut call).receive_with(
                                move |values, record| {
                                    let tag = record.index(0)?.as_symbol()?;
                                    if !transport_kind::<TLS>().matches(&tag) {
                                        return None;
                                    }
                                    values
                                        .equal(&identity, &record.index(1)?)
                                        .then(|| record.clone())
                                },
                                Some(now),
                            )
                        })
                        .and_then(|result| {
                            ready(result.map_err(geam::host::HostExecutionError::from))
                        })
                        .and_then(|receive| receive.wait(&context))
                        .and_then(|record| match record {
                            Some(record) => {
                                let receiver = target.clone();
                                Either::Left(context.with_call(move |mut call| {
                                    service::Processes::new(&mut call).send(&receiver, record)
                                }))
                            }
                            None => {
                                draining = false;
                                Either::Right(ready(Ok(())))
                            }
                        });
                    completion = receive.await;
                }
                ready(completion)
                    .and_then(|()| {
                        context.with_call(move |mut call| {
                            let alive = service::Processes::new(&mut call).is_alive(&target);
                            transfer.commit(target, alive)
                        })
                    })
                    .await
                    .map(|changed| changed.map(|()| ok(nil())).unwrap_or_else(fail))
            })
        })
    })
}

#[cfg(test)]
mod tests {
    use crate::test_support::{
        execution_fixture::TestHost,
        network::{Event, ScriptedConnection, ScriptedNetwork},
        source_project,
    };
    use geam::Value;
    use std::net::SocketAddr;
    use std::sync::Arc;

    #[test]
    fn accept_and_receive_reject_deadlines_outside_the_selected_clock_range() {
        use geam::execution::ExecutionHost;
        let address = "127.0.0.1:4321".parse().unwrap();
        let network = Arc::new(ScriptedNetwork::new(
            address,
            vec![Arc::new(ScriptedConnection::new(
                address,
                "127.0.0.2:5678".parse().unwrap(),
                vec![],
            ))],
        ));
        let (mut execution, mut state) = source_project(
            r#"
import glisten/tcp
import glisten/socket
pub fn main() {
  let assert Ok(listener) = tcp.listen(0, [])
  assert tcp.accept_timeout(listener, 18446744073709551615) == Error(socket.Badarg)
  let assert Ok(connection) = tcp.accept(listener)
  assert tcp.receive_timeout(connection, 0, 18446744073709551615) == Error(socket.Badarg)
  let _ = tcp.close(connection)
  let _ = tcp.close(listener)
  Nil
}
"#,
            network,
        );
        let host = TestHost::default();
        let initial = host.now();
        let (mut low, mut high) = (0_u64, u64::MAX);
        while low < high {
            let midpoint = low + (high - low) / 2 + 1;
            if initial
                .checked_add(std::time::Duration::from_secs(midpoint))
                .is_some()
            {
                low = midpoint;
            } else {
                high = midpoint - 1;
            }
        }
        host.advance(std::time::Duration::from_secs(low));
        assert!(
            host.now()
                .checked_add(std::time::Duration::from_millis(u64::MAX))
                .is_none()
        );
        assert_eq!(
            host.block_on(execution.run_main(&host, &mut state, &mut Vec::new()))
                .unwrap(),
            Value::Nil
        );
    }

    #[test]
    fn generic_address_conversion_accepts_both_address_families_and_rejects_bad_data() {
        use crate::{
            Component,
            schema::One,
            test_support::{Profile, source_project_with},
        };
        use geam::host::{HostProviderModule, native::NativeRules};
        for (input, result) in [
            ("#(127, 0, 0, 1)", Some("#(127, 0, 0, 1)")),
            ("#(0, 0, 0, 0, 0, 65535, 32512, 1)", Some("#(127, 0, 0, 1)")),
            ("#(1, 2, 3)", None),
            ("#(256, 0, 0, 1)", None),
            ("[127, 0, 0, 1]", None),
            ("#(\"wrong\", 0, 0, 1)", None),
        ] {
            let provider = HostProviderModule::new("fixture", "fixture")
                .unwrap()
                .with_native_function::<Component, (crate::A,), super::Ipv4, One<super::Ipv4>, _>(
                    "convert",
                    NativeRules::default(),
                    super::mapped_address::<Profile>,
                )
                .unwrap();
            let source = format!(
                r#"
@external(erlang, "inet", "ipv4_mapped_ipv6_address")
fn convert(address: a) -> #(Int, Int, Int, Int)
pub fn main() {{ {body} Nil }}
"#,
                body = result.map_or_else(
                    || format!("let _ = convert({input})"),
                    |result| format!("assert convert({input}) == {result}")
                )
            );
            let network = Arc::new(ScriptedNetwork::new(
                "127.0.0.1:4321".parse().unwrap(),
                vec![],
            ));
            let (mut execution, mut state) = source_project_with(&source, network, [provider]);
            let host = TestHost::default();
            let actual = host.block_on(execution.run_main(&host, &mut state, &mut Vec::new()));
            if result.is_some() {
                assert_eq!(actual.unwrap(), Value::Nil);
            } else {
                assert!(
                    actual.unwrap_err().to_string().contains("badarg"),
                    "{input}"
                );
            }
        }
    }

    #[test]
    fn send_timeout_respects_close_policy_and_retains_padded_bytes_tree_across_await() {
        for (transport, close_on_timeout) in
            [("tcp", true), ("tcp", false), ("ssl", true), ("ssl", false)]
        {
            let local: SocketAddr = "127.0.0.1:4321".parse().unwrap();
            let io = Arc::new(ScriptedConnection::new(
                local,
                "127.0.0.2:5678".parse().unwrap(),
                vec![],
            ));
            io.write_pending
                .store(true, std::sync::atomic::Ordering::Release);
            let network = Arc::new(ScriptedNetwork::new(local, vec![io]));
            let source = format!(
                r#"
import gleam/bytes_tree
import glisten/{transport}
import glisten/tcp as tcp_close
import glisten/socket
import glisten/socket/options
pub fn main() {{
  let assert Ok(listener) = {transport}.listen(0, [options.CertKeyConfig(options.CertKeyFiles("cert", "key")),
    options.SendTimeout(7), options.SendTimeoutClose({close})])
  let assert Ok(connection) = {transport}.accept(listener)
  let assert Ok(alias) = {transport}.handshake(connection)
  let packet = bytes_tree.concat([bytes_tree.from_string("한"), bytes_tree.from_bit_array(<<255, 1:size(1)>>)])
  assert {transport}.send(alias, packet) == Error(socket.Timeout)
  assert bytes_tree.to_bit_array(packet) == <<"한":utf8, 255, 128>>
  assert {transport}.receive_timeout(alias, 0, 0) == Error(socket.{reason})
  assert {transport}.close(alias) == Ok(Nil)
  assert {transport}.send(connection, packet) == Error(socket.Closed)
  assert tcp_close.close(listener) == Ok(Nil)
  Nil
}}
"#,
                close = if close_on_timeout { "True" } else { "False" },
                reason = if close_on_timeout {
                    "Closed"
                } else {
                    "Timeout"
                }
            );
            let (mut execution, mut state) = source_project(&source, network.clone());
            let host = TestHost::default();
            let mut echo = Vec::new();
            let mut running = std::pin::pin!(execution.run_main(&host, &mut state, &mut echo));
            assert!(host.poll(running.as_mut()).is_pending());
            let mut expected = "한".as_bytes().to_vec();
            expected.extend([255, 128]);
            assert!(network.events.lock().contains(&Event::Write(expected)));
            network.timer.notify_one();
            assert_eq!(
                host.poll(running.as_mut()).map(Result::unwrap),
                std::task::Poll::Ready(Value::Nil),
                "{transport}, close={close_on_timeout}"
            );
            assert_eq!(
                network
                    .events
                    .lock()
                    .iter()
                    .filter(|event| **event == Event::CloseConnection)
                    .count(),
                1
            );
        }
    }

    #[test]
    fn passive_receive_timeout_preserves_partial_data_for_the_next_call() {
        for transport in ["tcp", "ssl"] {
            let local: SocketAddr = "127.0.0.1:4321".parse().unwrap();
            let io = Arc::new(ScriptedConnection::new(
                local,
                "127.0.0.2:5678".parse().unwrap(),
                vec![Ok(b"ab".to_vec())],
            ));
            let network = Arc::new(ScriptedNetwork::new(local, vec![io.clone()]));
            let source = format!(
                r#"
import glisten/{transport}
import glisten/tcp as tcp_close
import glisten/socket
import glisten/socket/options
pub fn main() {{
  let assert Ok(listener) = {transport}.listen(0, [options.CertKeyConfig(options.CertKeyFiles("cert", "key"))])
  let assert Ok(connection) = {transport}.accept(listener)
  let assert Ok(_) = {transport}.handshake(connection)
  assert {transport}.receive(connection, -1) == Error(socket.Badarg)
  assert {transport}.receive_timeout(connection, 1, -1) == Error(socket.Badarg)
  assert {transport}.receive_timeout(connection, 4, 5) == Error(socket.Timeout)
  assert {transport}.receive(connection, 4) == Ok(<<"abcd":utf8>>)
  assert {transport}.close(connection) == Ok(Nil)
  assert tcp_close.close(listener) == Ok(Nil)
  Nil
}}
"#
            );
            let (mut execution, mut state) = source_project(&source, network.clone());
            let host = TestHost::default();
            let mut echo = Vec::new();
            let mut running = std::pin::pin!(execution.run_main(&host, &mut state, &mut echo));
            assert!(host.poll(running.as_mut()).is_pending());
            host.advance(std::time::Duration::from_millis(5));
            assert!(host.poll(running.as_mut()).is_pending());
            io.incoming.lock().push_back(Ok(b"cd".to_vec()));
            io.changed.notify_waiters();
            assert_eq!(
                host.poll(running.as_mut()).map(Result::unwrap),
                std::task::Poll::Ready(Value::Nil),
                "{transport}"
            );
            assert_eq!(
                network
                    .events
                    .lock()
                    .iter()
                    .filter(|event| **event == Event::CloseConnection)
                    .count(),
                1
            );
        }
    }

    #[test]
    fn bytes_tree_shapes_and_aliases_preserve_exact_tcp_and_tls_writes() {
        for transport in ["tcp", "ssl"] {
            let local: SocketAddr = "127.0.0.1:4321".parse().unwrap();
            let io = Arc::new(ScriptedConnection::new(
                local,
                "127.0.0.2:5678".parse().unwrap(),
                vec![],
            ));
            io.write_pending
                .store(true, std::sync::atomic::Ordering::Release);
            let network = Arc::new(ScriptedNetwork::new(local, vec![io.clone()]));
            let source = format!(
                r#"
import gleam/bytes_tree
import gleam/list
import gleam/string_tree
import glisten/{transport}
import glisten/tcp as tcp_close
import glisten/socket/options
pub fn main() {{
  let assert Ok(listener) = {transport}.listen(0, [options.CertKeyConfig(options.CertKeyFiles("cert", "key"))])
  let assert Ok(connection) = {transport}.accept(listener)
  let assert Ok(_) = {transport}.handshake(connection)
  let wide = list.repeat(bytes_tree.from_string("x"), 1000) |> bytes_tree.concat
  let deep = list.repeat(Nil, 1000) |> list.fold(bytes_tree.from_string("end"), fn(tree, _) {{ bytes_tree.append_tree(bytes_tree.new(), tree) }})
  let text = bytes_tree.from_string_tree(string_tree.from_strings(["α", "한"]))
  let packet = bytes_tree.concat([bytes_tree.new(), bytes_tree.from_bit_array(<<0,255>>), text, wide, deep])
  assert {transport}.send(connection, packet) == Ok(Nil)
  assert bytes_tree.byte_size(packet) == 1010
  assert {transport}.send(connection, bytes_tree.new()) == Ok(Nil)
  assert {transport}.close(connection) == Ok(Nil)
  assert tcp_close.close(listener) == Ok(Nil)
  Nil
}}
"#
            );
            let (mut execution, mut state) = source_project(&source, network.clone());
            let host = TestHost::default();
            let mut echo = Vec::new();
            let mut running = std::pin::pin!(execution.run_main(&host, &mut state, &mut echo));
            assert!(host.poll(running.as_mut()).is_pending());
            io.write_pending
                .store(false, std::sync::atomic::Ordering::Release);
            io.changed.notify_waiters();
            assert_eq!(
                host.poll(running.as_mut()).map(Result::unwrap),
                std::task::Poll::Ready(Value::Nil),
                "{transport}"
            );
            let mut expected = vec![0, 255];
            expected.extend("α한".as_bytes());
            expected.extend(vec![b'x'; 1000]);
            expected.extend(b"end");
            let writes = network
                .events
                .lock()
                .iter()
                .filter_map(|event| match event {
                    Event::Write(bytes) => Some(bytes.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(writes, [expected, vec![]]);
        }
    }

    #[test]
    fn listener_and_handshake_io_failures_keep_the_original_result_shapes() {
        let local: SocketAddr = "127.0.0.1:4321".parse().unwrap();
        let mut network = ScriptedNetwork::new(local, vec![]);
        network.listen_error = Some(std::io::ErrorKind::AddrInUse);
        let network = Arc::new(network);
        let (mut execution, mut state) = source_project(
            r#"
import glisten/tcp
import glisten/socket
pub fn main() { assert tcp.listen(0, []) == Error(socket.Eaddrinuse) Nil }
"#,
            network.clone(),
        );
        let host = TestHost::default();
        assert_eq!(
            host.poll(std::pin::pin!(execution.run_main(
                &host,
                &mut state,
                &mut Vec::new()
            )))
            .map(Result::unwrap),
            std::task::Poll::Ready(Value::Nil)
        );
        assert_eq!(
            *network.events.lock(),
            [Event::Listen("0.0.0.0:0".parse().unwrap())]
        );
        let mut io = ScriptedConnection::new(local, "127.0.0.2:5678".parse().unwrap(), vec![]);
        io.handshake_error = Some(std::io::ErrorKind::InvalidData);
        let network = Arc::new(ScriptedNetwork::new(local, vec![Arc::new(io)]));
        let (mut execution, mut state) = source_project(
            r#"
import glisten/ssl
import glisten/tcp
import glisten/socket
import glisten/socket/options
pub fn main() {
  let assert Ok(listener) = ssl.listen(0, [options.CertKeyConfig(options.CertKeyFiles("cert", "key"))])
  let assert Ok(connection) = ssl.accept(listener)
  assert ssl.handshake(connection) == Error(Nil)
  assert ssl.receive(connection, 0) == Error(socket.Closed)
  assert tcp.close(listener) == Ok(Nil)
  Nil
}
"#,
            network.clone(),
        );
        assert_eq!(
            host.poll(std::pin::pin!(execution.run_main(
                &host,
                &mut state,
                &mut Vec::new()
            )))
            .map(Result::unwrap),
            std::task::Poll::Ready(Value::Nil)
        );
        assert_eq!(
            network
                .events
                .lock()
                .iter()
                .filter(|event| **event == Event::CloseConnection)
                .count(),
            1
        );
    }

    #[test]
    fn cancellation_before_accepted_io_admission_closes_the_unregistered_connection() {
        let local = "127.0.0.1:4321".parse().unwrap();
        let io = Arc::new(ScriptedConnection::new(
            local,
            "127.0.0.2:5678".parse().unwrap(),
            vec![],
        ));
        let network = ScriptedNetwork::new(local, vec![io.clone()]);
        let admitted = super::PendingAccept::new(io);
        drop(admitted);
        assert_eq!(*network.events.lock(), [Event::CloseConnection]);
    }

    #[test]
    fn changing_active_mode_or_closing_interrupts_an_existing_passive_receive() {
        for transport in ["tcp", "ssl"] {
            for closing in [false, true] {
                let local = "127.0.0.1:4321".parse().unwrap();
                let io = Arc::new(ScriptedConnection::new(
                    local,
                    "127.0.0.2:5678".parse().unwrap(),
                    vec![],
                ));
                let network = Arc::new(ScriptedNetwork::new(local, vec![io]));
                let operation = if closing {
                    format!("{transport}.close(connection)")
                } else {
                    format!(
                        "{transport}.set_opts(connection, [options.ActiveMode(options.Active)])"
                    )
                };
                let source = format!(
                    r#"
import gleam/erlang/process
import glisten/{transport}
import glisten/tcp as close_listener
import glisten/socket
import glisten/socket/options
pub fn main() {{
  let assert Ok(listener) = {transport}.listen(0, [options.CertKeyConfig(options.CertKeyFiles("cert", "key"))])
  let assert Ok(connection) = {transport}.accept(listener)
  let assert Ok(_) = {transport}.handshake(connection)
  let result = process.new_subject()
  let _ = process.spawn_unlinked(fn() {{ process.send(result, {transport}.receive(connection, 0)) }})
  process.sleep(5)
  assert {operation} == Ok(Nil)
  assert process.receive(result, 100) == Ok(Error(socket.{reason}))
  let _ = {transport}.close(connection)
  let _ = close_listener.close(listener)
  Nil
}}
"#,
                    reason = if closing { "Closed" } else { "Einval" }
                );
                let (mut execution, mut state) = source_project(&source, network.clone());
                let host = TestHost::default();
                let mut echo = Vec::new();
                let mut running = Box::pin(execution.run_main(&host, &mut state, &mut echo));
                assert!(host.poll(running.as_mut()).is_pending());
                assert!(network.events.lock().contains(&Event::Read(65536)));
                host.advance(std::time::Duration::from_millis(5));
                assert_eq!(
                    host.poll(running.as_mut()).map(Result::unwrap),
                    std::task::Poll::Ready(Value::Nil)
                );
            }
        }
    }

    #[test]
    fn transport_mismatch_is_rejected_without_io_and_closed_tls_aliases_stay_closed() {
        for (owner, other) in [("tcp", "ssl"), ("ssl", "tcp")] {
            let local = "127.0.0.1:4321".parse().unwrap();
            let io = Arc::new(ScriptedConnection::new(
                local,
                "127.0.0.2:5678".parse().unwrap(),
                vec![],
            ));
            let network = Arc::new(ScriptedNetwork::new(local, vec![io]));
            let source = format!(
                r#"
import gleam/bytes_tree
import gleam/erlang/atom
import gleam/erlang/process
import glisten/tcp
import glisten/ssl
import glisten/socket
import glisten/socket/options
pub fn main() {{
  let assert Ok(listener) = {owner}.listen(0, [options.CertKeyConfig(options.CertKeyFiles("cert", "key"))])
  let assert Ok(connection) = {owner}.accept(listener)
  assert {other}.accept(listener) == Error(socket.Closed)
  assert {other}.sockname(listener) == Error(socket.Closed)
  assert {other}.receive(connection, 0) == Error(socket.Badarg)
  assert {other}.send(connection, bytes_tree.new()) == Error(socket.Badarg)
  assert {other}.shutdown(connection) == Error(socket.Badarg)
  assert {other}.set_opts(connection, [options.Nodelay(False)]) == Error(socket.Badarg)
  assert {other}.peername(connection) == Error(socket.Badarg)
  assert {other}.get_socket_opts(connection, []) == Error(socket.Badarg)
  assert {other}.controlling_process(connection, process.self()) == Error(atom.create("badarg"))
  {extra}
  let _ = {owner}.close(connection)
  assert {owner}.receive(connection, 0) == Error(socket.Closed)
  assert {owner}.peername(connection) == Error(socket.Closed)
  let _ = tcp.close(listener)
  assert {owner}.accept_timeout(listener, -1) == Error(socket.Closed)
  Nil
}}
"#,
                extra = if owner == "tcp" {
                    "assert ssl.handshake(connection) == Error(Nil) assert ssl.negotiated_protocol(connection) == Error(\"Socket not negotiated\")"
                } else {
                    "assert ssl.accept_timeout(listener, -1) == Error(socket.Badarg)"
                }
            );
            let (mut execution, mut state) = source_project(&source, network.clone());
            let host = TestHost::default();
            assert_eq!(
                host.block_on(execution.run_main(&host, &mut state, &mut Vec::new()))
                    .unwrap(),
                Value::Nil
            );
            assert_eq!(
                network
                    .events
                    .lock()
                    .iter()
                    .filter(|event| **event == Event::Accept)
                    .count(),
                1
            );
        }
    }

    #[test]
    fn cancelling_the_execution_releases_pending_io_and_live_domain_resources() {
        use crate::network::{Connection as _, Listener as _};
        use std::sync::atomic::Ordering;
        for (transport, operation, expected) in [
            ("tcp", "let _ = tcp.accept(listener)", Event::Accept),
            (
                "tcp",
                "let _ = tcp.receive(connection, 0)",
                Event::Read(65536),
            ),
            (
                "tcp",
                "let _ = tcp.send(connection, bytes_tree.from_string(\"held\"))",
                Event::Write(b"held".to_vec()),
            ),
            ("ssl", "let _ = ssl.handshake(connection)", Event::Handshake),
        ] {
            let local = "127.0.0.1:4321".parse().unwrap();
            let io = Arc::new(ScriptedConnection::new(
                local,
                "127.0.0.2:5678".parse().unwrap(),
                vec![],
            ));
            io.write_pending.store(true, Ordering::Release);
            io.handshake_pending.store(true, Ordering::Release);
            let accepting = expected == Event::Accept;
            let network = Arc::new(ScriptedNetwork::new(
                local,
                if accepting { vec![] } else { vec![io.clone()] },
            ));
            let source = format!(
                r#"
import gleam/bytes_tree
import glisten/{transport}
import glisten/socket/options
pub fn main() {{
  let assert Ok(listener) = {transport}.listen(0, [options.CertKeyConfig(options.CertKeyFiles("cert", "key"))])
  {accept}
  {operation}
  Nil
}}
"#,
                accept = if accepting {
                    String::new()
                } else {
                    format!("let assert Ok(connection) = {transport}.accept(listener)")
                }
            );
            let (mut execution, mut state) = source_project(&source, network.clone());
            let host = TestHost::default();
            let mut echo = Vec::new();
            let mut running = Box::pin(execution.run_main(&host, &mut state, &mut echo));
            assert!(host.poll(running.as_mut()).is_pending());
            let events = format!("{:?}", *network.events.lock());
            assert!(
                network.events.lock().contains(&expected),
                "{transport}: {operation}: {events}"
            );
            drop(running);
            host.step();
            let events = network.events.lock();
            assert_eq!(
                events
                    .iter()
                    .filter(|event| **event == Event::CloseListener)
                    .count(),
                1
            );
            assert_eq!(
                events
                    .iter()
                    .filter(|event| **event == Event::CloseConnection)
                    .count(),
                usize::from(!accepting)
            );
            drop(events);
            assert_eq!(
                network.listener.address().unwrap_err().kind(),
                std::io::ErrorKind::NotConnected
            );
            assert_eq!(
                host.block_on(network.listener.accept())
                    .err()
                    .unwrap()
                    .kind(),
                std::io::ErrorKind::NotConnected
            );
            if !accepting {
                assert_eq!(
                    host.block_on(io.read(1)).unwrap_err().kind(),
                    std::io::ErrorKind::NotConnected
                );
                assert_eq!(
                    host.block_on(io.write(b"late")).unwrap_err().kind(),
                    std::io::ErrorKind::NotConnected
                );
                assert_eq!(
                    host.block_on(io.handshake()).unwrap_err().kind(),
                    std::io::ErrorKind::NotConnected
                );
            }
        }
    }

    #[test]
    fn tls_native_calls_preserve_handshake_alias_protocol_and_mapped_addresses() {
        let local = "[::ffff:127.0.0.1]:4321".parse().unwrap();
        let mut io = ScriptedConnection::new(
            local,
            "[2001:db8::1]:5678".parse().unwrap(),
            vec![Ok(b"tls".to_vec())],
        );
        io.protocol = Some(b"h2".to_vec());
        let network = Arc::new(ScriptedNetwork::new(local, vec![Arc::new(io)]));
        let (mut execution, mut state) = source_project(
            r#"
import gleam/bytes_tree
import gleam/dict
import gleam/dynamic/decode
import gleam/erlang/atom
import glisten/socket
import glisten/socket/options
import glisten/ssl
import glisten/tcp
import glisten/transport
pub fn main() {
  assert ssl.listen(0, []) == Error(socket.Badarg)
  assert ssl.listen(0, [options.Buffer(-1)]) == Error(socket.Badarg)
  let assert Ok(listener) = ssl.listen(0, [
    options.Ipv6,
    options.CertKeyConfig(options.CertKeyFiles("certificate.pem", "key.pem")),
    options.AlpnPreferredProtocols(["h2"]),
  ])
  let assert Ok(#(options.IpV4(127, 0, 0, 1), 4321)) = transport.sockname(transport.Ssl, listener)
  let assert Ok(connection) = ssl.accept_timeout(listener, 10)
  assert ssl.receive(connection, 0) == Error(socket.Badarg)
  assert ssl.negotiated_protocol(connection) == Error("Socket not negotiated")
  let assert Ok(alias) = ssl.handshake(connection)
  assert alias == connection
  assert ssl.negotiated_protocol(alias) == Ok("h2")
  let info = transport.socket_info(alias)
  let assert Ok(domain) = dict.get(info, atom.create("domain"))
  assert decode.run(domain, atom.decoder()) == Ok(atom.create("inet6"))
  let assert Ok(#(options.IpV6(8193, 3512, 0, 0, 0, 0, 0, 1), 5678)) =
    transport.peername(transport.Ssl, alias)
  assert ssl.receive_timeout(alias, 3, 10) == Ok(<<"tls":utf8>>)
  assert ssl.set_opts(alias, [options.Buffer(8192)]) == Ok(Nil)
  assert transport.set_buffer_size(transport.Ssl, alias) == Ok(Nil)
  assert tcp.receive(alias, 0) == Error(socket.Badarg)
  assert tcp.send(alias, bytes_tree.from_string("wrong")) == Error(socket.Badarg)
  assert ssl.send(alias, bytes_tree.from_string("secure")) == Ok(Nil)
  assert ssl.shutdown(alias) == Ok(Nil)
  assert ssl.close(alias) == Ok(Nil)
  assert ssl.handshake(connection) == Error(Nil)
  assert ssl.receive(connection, 0) == Error(socket.Closed)
  assert ssl.negotiated_protocol(connection) == Error("Socket not negotiated")
  // SSL exposes connection close only; generic TCP close accepts listener identities.
  assert tcp.close(listener) == Ok(Nil)
  Nil
}
"#,
            network.clone(),
        );
        let host = TestHost::default();
        assert_eq!(
            host.poll(std::pin::pin!(execution.run_main(
                &host,
                &mut state,
                &mut Vec::new()
            )))
            .map(Result::unwrap),
            std::task::Poll::Ready(Value::Nil)
        );
        assert_eq!(
            network
                .events
                .lock()
                .iter()
                .filter(|event| **event == Event::Handshake)
                .count(),
            1
        );
        assert_eq!(
            network
                .events
                .lock()
                .iter()
                .filter(|event| **event == Event::CloseConnection)
                .count(),
            1
        );
    }

    #[test]
    fn accept_timeout_uses_the_execution_clock_and_rejects_negative_time() {
        for transport in ["tcp", "ssl"] {
            let network = Arc::new(ScriptedNetwork::new(
                "127.0.0.1:4321".parse().unwrap(),
                vec![],
            ));
            let (mut execution, mut state) = source_project(
                &format!(
                    r#"
import glisten/socket
import glisten/socket/options
import glisten/{transport}
import glisten/tcp as tcp_close
pub fn main() {{
  let assert Ok(listener) = {transport}.listen(0, [options.CertKeyConfig(options.CertKeyFiles("cert", "key"))])
  assert {transport}.accept_timeout(listener, -1) == Error(socket.Badarg)
  assert {transport}.accept_timeout(listener, 5) == Error(socket.Timeout)
  assert tcp_close.close(listener) == Ok(Nil)
  assert {transport}.accept(listener) == Error(socket.Closed)
  Nil
}}
"#
                ),
                network.clone(),
            );
            let host = TestHost::default();
            let mut echo = Vec::new();
            let mut running = Box::pin(execution.run_main(&host, &mut state, &mut echo));
            assert!(host.poll(running.as_mut()).is_pending());
            assert_eq!(
                *network.events.lock(),
                [Event::Listen("0.0.0.0:0".parse().unwrap()), Event::Accept]
            );
            host.advance(std::time::Duration::from_millis(4));
            assert!(host.poll(running.as_mut()).is_pending());
            host.advance(std::time::Duration::from_millis(1));
            assert_eq!(
                host.poll(running.as_mut()).map(Result::unwrap),
                std::task::Poll::Ready(Value::Nil)
            );
            assert_eq!(
                *network.events.lock(),
                [
                    Event::Listen("0.0.0.0:0".parse().unwrap()),
                    Event::Accept,
                    Event::CloseListener
                ]
            );
        }
    }

    #[test]
    fn killing_the_owner_cancels_passive_io_and_closes_retained_aliases() {
        let local = "127.0.0.1:4321".parse().unwrap();
        let io = Arc::new(ScriptedConnection::new(
            local,
            "127.0.0.2:5678".parse().unwrap(),
            vec![],
        ));
        let network = Arc::new(ScriptedNetwork::new(local, vec![io]));
        let (mut execution, mut state) = source_project(
            r#"
import gleam/erlang/process
import glisten/socket
import glisten/tcp
pub fn main() {
  let assert Ok(listener) = tcp.listen(0, [])
  let ready = process.new_subject()
  let owner = process.spawn_unlinked(fn() {
    let assert Ok(connection) = tcp.accept(listener)
    process.send(ready, connection)
    let _ = tcp.receive(connection, 4)
  })
  let assert Ok(alias) = process.receive(ready, 1000)
  process.kill(owner)
  assert tcp.receive(alias, 4) == Error(socket.Closed)
  assert tcp.close(alias) == Ok(Nil)
  assert tcp.close(listener) == Ok(Nil)
  Nil
}
"#,
            network.clone(),
        );
        let host = TestHost::default();
        assert_eq!(
            host.poll(std::pin::pin!(execution.run_main(
                &host,
                &mut state,
                &mut Vec::new()
            )))
            .map(Result::unwrap),
            std::task::Poll::Ready(Value::Nil)
        );
        assert_eq!(
            network
                .events
                .lock()
                .iter()
                .filter(|event| **event == Event::CloseConnection)
                .count(),
            1
        );
        assert_eq!(
            network
                .events
                .lock()
                .iter()
                .filter(|event| **event == Event::CloseListener)
                .count(),
            1
        );
    }

    fn raw_send<'call>(
        mut call: crate::Call<'call, crate::test_support::Profile, ()>,
        pid: geam::host::HostExternal<'call, geam::gleam_erlang::Pid>,
        value: geam::host::HostValue<'call, crate::A>,
    ) -> Result<geam::host::HostCallCompletion<'call, ()>, geam::host::HostCallError> {
        let value = call.native_value::<crate::A>(value);
        let target = geam::gleam_erlang::service::Processes::new(&mut call).pid(pid);
        geam::gleam_erlang::service::Processes::new(&mut call).send(&target, value);
        Ok(call.return_value(()))
    }

    #[test]
    fn controlling_process_moves_only_its_socket_records_in_mailbox_order() {
        for transport in ["tcp", "ssl"] {
            let local = "127.0.0.1:4321".parse().unwrap();
            let first = Arc::new(ScriptedConnection::new(
                local,
                "127.0.0.2:5678".parse().unwrap(),
                vec![Ok(b"first".to_vec()), Ok(b"second".to_vec())],
            ));
            let second = Arc::new(ScriptedConnection::new(
                local,
                "127.0.0.3:5679".parse().unwrap(),
                vec![Ok(b"other".to_vec())],
            ));
            let network = Arc::new(ScriptedNetwork::new(local, vec![first, second]));
            let original = r#"
import gleam/dynamic
import gleam/dynamic/decode
import gleam/erlang/atom
import gleam/erlang/process
import glisten/socket/options
import glisten/tcp
@external(erlang, "fixture", "raw_send") fn raw_send(pid: process.Pid, message: a) -> Nil
pub type Record { Data(BitArray) Passive Failure(atom.Atom) }
fn failure(record) {
  let decoder = { use reason <- decode.field(2, atom.decoder()) decode.success(Failure(reason)) }
  let assert Ok(decoded) = decode.run(record, decoder)
  decoded
}
fn data(record: dynamic.Dynamic) -> Record {
  let decoder = {
    use bytes <- decode.field(2, decode.bit_array)
    decode.success(Data(bytes))
  }
  let assert Ok(decoded) = decode.run(record, decoder)
  decoded
}
fn records() {
  process.new_selector()
  |> process.select_record(atom.create("tcp"), 2, data)
  |> process.select_record(atom.create("tcp_passive"), 1, fn(_) { Passive })
  |> process.select_record(atom.create("tcp_error"), 2, failure)
}
pub fn main() {
  let assert Ok(listener) = tcp.listen(0, [])
  let assert Ok(first) = tcp.accept(listener)
  let assert Ok(second) = tcp.accept(listener)
  let user = process.new_subject()
  let barrier = process.new_subject()
  let done = process.new_subject()
  let target_ready = process.new_subject()
  process.send(user, 42)
  assert tcp.set_opts(first, [options.ActiveMode(options.Count(2))]) == Ok(Nil)
  assert tcp.set_opts(second, [options.ActiveMode(options.Count(1))]) == Ok(Nil)
  let _ = process.spawn_unlinked(fn() { process.send(barrier, Nil) })
  // Waiting for this later worker gives both readers a deterministic turn.
  let assert Ok(Nil) = process.receive(barrier, 1000)
  let noise: process.Subject(Nil) = process.unsafely_create_subject(process.self(), atom.to_dynamic(atom.create("noise")))
  raw_send(process.self(), #(atom.create("noise"), Nil))
  raw_send(process.self(), dynamic.array([]))
  raw_send(process.self(), dynamic.array([atom.to_dynamic(atom.create("tcp"))]))
  raw_send(process.self(), #(atom.create("tcp_error"), first, atom.create("econnreset")))
  let target = process.spawn_unlinked(fn() {
    let release = process.new_subject()
    process.send(target_ready, release)
    let selector = records()
    assert process.selector_receive(selector, 1000) == Ok(Data(<<"first":utf8>>))
    assert process.selector_receive(selector, 1000) == Ok(Data(<<"second":utf8>>))
    assert process.selector_receive(selector, 1000) == Ok(Passive)
    assert process.selector_receive(selector, 1000) == Ok(Failure(atom.create("econnreset")))
    process.send(done, Nil)
    let assert Ok(Nil) = process.receive(release, 1000)
  })
  let assert Ok(release) = process.receive(target_ready, 1000)
  assert tcp.controlling_process(first, target) == Ok(Nil)
  assert process.receive(user, 1000) == Ok(42)
  assert process.receive(noise, 1000) == Ok(Nil)
  let selector = records()
  assert process.selector_receive(selector, 1000) == Ok(Data(<<"other":utf8>>))
  assert process.selector_receive(selector, 1000) == Ok(Passive)
  let remaining = process.new_selector() |> process.select_other(fn(record) { record })
  assert process.selector_receive(remaining, 1000) == Ok(dynamic.array([]))
  assert process.selector_receive(remaining, 1000) == Ok(dynamic.array([atom.to_dynamic(atom.create("tcp"))]))
  assert process.receive(done, 1000) == Ok(Nil)
  process.send(release, Nil)
  assert tcp.close(second) == Ok(Nil)
  assert tcp.close(listener) == Ok(Nil)
  Nil
}
"#;
            let source = if transport == "ssl" {
                original.replace("import glisten/tcp", "import glisten/ssl as tcp\nimport glisten/tcp as tcp_close")
                .replace("tcp.listen(0, [])", "tcp.listen(0, [options.CertKeyConfig(options.CertKeyFiles(\"cert\", \"key\"))])")
                .replace("let assert Ok(second) = tcp.accept(listener)", "let assert Ok(second) = tcp.accept(listener)\n  let assert Ok(_) = tcp.handshake(first)\n  let assert Ok(_) = tcp.handshake(second)")
                .replace("\"tcp\"", "\"ssl\"")
                .replace("\"tcp_passive\"", "\"ssl_passive\"")
                .replace("\"tcp_error\"", "\"ssl_error\"")
                .replace("tcp.close(listener)", "tcp_close.close(listener)")
            } else {
                original.to_string()
            };
            let provider = geam::host::HostProviderModule::new("fixture", "fixture").unwrap()
                .with_scoped_function::<crate::Component, (geam::gleam_erlang::Pid, crate::A), (), _>("raw_send", raw_send).unwrap();
            let (mut execution, mut state) =
                crate::test_support::source_project_with(&source, network.clone(), [provider]);
            let host = TestHost::default();
            assert_eq!(
                host.poll(std::pin::pin!(execution.run_main(
                    &host,
                    &mut state,
                    &mut Vec::new()
                )))
                .map(Result::unwrap),
                std::task::Poll::Ready(Value::Nil)
            );
            assert_eq!(
                network
                    .events
                    .lock()
                    .iter()
                    .filter(|event| **event == Event::CloseConnection)
                    .count(),
                2
            );
        }
    }

    #[test]
    fn socket_aliases_keep_source_equality_hashing_and_inspection_after_close() {
        let local = "127.0.0.1:4321".parse().unwrap();
        let first = Arc::new(ScriptedConnection::new(
            local,
            "127.0.0.2:5678".parse().unwrap(),
            vec![],
        ));
        let second = Arc::new(ScriptedConnection::new(
            local,
            "127.0.0.3:5679".parse().unwrap(),
            vec![],
        ));
        let network = Arc::new(ScriptedNetwork::new(local, vec![first, second]));
        let (mut execution, mut state) = source_project(
            r#"
import gleam/dict
import gleam/string
import glisten/tcp
pub fn main() {
  let assert Ok(listener) = tcp.listen(0, [])
  let assert Ok(other_listener) = tcp.listen(0, [])
  let assert Ok(first) = tcp.accept(listener)
  let assert Ok(second) = tcp.accept(listener)
  let assert Ok(alias) = tcp.handshake(first)
  assert first == alias
  assert first != second
  assert listener != other_listener
  let connections = dict.from_list([#(first, 1), #(second, 2)])
  assert dict.get(connections, alias) == Ok(1)
  assert dict.get(dict.from_list([#(listener, 3)]), listener) == Ok(3)
  assert string.contains(string.inspect(first), "glisten socket")
  assert string.contains(string.inspect(listener), "glisten socket")
  assert tcp.close(first) == Ok(Nil)
  assert dict.get(connections, alias) == Ok(1)
  assert first == alias
  assert tcp.close(listener) == Ok(Nil)
  assert tcp.close(other_listener) == Ok(Nil)
  Nil
}
"#,
            network.clone(),
        );
        let host = TestHost::default();
        assert_eq!(
            host.poll(std::pin::pin!(execution.run_main(
                &host,
                &mut state,
                &mut Vec::new()
            )))
            .map(Result::unwrap),
            std::task::Poll::Ready(Value::Nil)
        );
        assert_eq!(
            network
                .events
                .lock()
                .iter()
                .filter(|event| **event == Event::CloseConnection)
                .count(),
            2
        );
    }

    #[test]
    fn native_fatal_boundaries_preserve_the_operation_and_actual_specialization() {
        for (operation, expected, local_error, peer_error, protocol) in [
            (
                "let _: Int = tcp.negotiated_protocol(connection) Nil",
                "undefined Erlang target tcp:negotiated_protocol/1",
                None,
                None,
                None,
            ),
            (
                "let _: dict.Dict(Int, Dynamic) = tcp.socket_info(connection) Nil",
                "socket info key specialization does not accept native symbols",
                None,
                None,
                None,
            ),
            (
                "let _: dict.Dict(Atom, Int) = tcp.socket_info(connection) Nil",
                "socket info item specialization does not accept native data",
                None,
                None,
                None,
            ),
            (
                "let _ = tcp.close(connection) let _ = transport.socket_info(connection) Nil",
                "socket info requires an open connection",
                None,
                None,
                None,
            ),
            (
                "let _ = tcp.close(connection) let _: dict.Dict(Atom, Dynamic) = tcp.socket_info(connection) Nil",
                "socket info requires an open connection",
                None,
                None,
                None,
            ),
            (
                "let _ = transport.socket_info(connection) Nil",
                "permission denied",
                Some(std::io::ErrorKind::PermissionDenied),
                None,
                None,
            ),
            (
                "let _ = transport.socket_info(connection) Nil",
                "address not available",
                None,
                Some(std::io::ErrorKind::AddrNotAvailable),
                None,
            ),
            (
                "let _ = ssl.handshake(connection) let _ = ssl.negotiated_protocol(connection) Nil",
                "negotiated ALPN is not a source String",
                None,
                None,
                Some(vec![255]),
            ),
        ] {
            let local = "[2001:db8::1]:4321".parse().unwrap();
            let mut io =
                ScriptedConnection::new(local, "[2001:db8::2]:5678".parse().unwrap(), vec![]);
            io.local_error = local_error;
            io.peer_error = peer_error;
            let tls = protocol.is_some();
            io.protocol = protocol;
            let network = Arc::new(ScriptedNetwork::new(local, vec![Arc::new(io)]));
            let source = format!(
                r#"
import gleam/dict
import gleam/dynamic.{{type Dynamic}}
import gleam/erlang/atom.{{type Atom}}
import glisten/tcp
import glisten/ssl
import glisten/transport
import glisten/socket/options
pub fn main() {{
  let assert Ok(listener) = {owner}.listen(0, [options.CertKeyConfig(options.CertKeyFiles("cert", "key"))])
  let assert Ok(connection) = {owner}.accept(listener)
  {operation}
}}
"#,
                owner = if tls { "ssl" } else { "tcp" }
            );
            let (mut execution, mut state) = source_project(&source, network.clone());
            let host = TestHost::default();
            let error = host
                .block_on(execution.run_main(&host, &mut state, &mut Vec::new()))
                .unwrap_err();
            assert!(error.to_string().contains(expected), "{operation}: {error}");
            assert_eq!(
                network
                    .events
                    .lock()
                    .iter()
                    .filter(|event| **event == Event::CloseConnection)
                    .count(),
                1
            );
            assert_eq!(
                network
                    .events
                    .lock()
                    .iter()
                    .filter(|event| **event == Event::CloseListener)
                    .count(),
                1
            );
        }
    }

    #[test]
    fn tcp_and_tls_handoff_requires_current_owner_and_live_target() {
        for transport in ["tcp", "ssl"] {
            let local = "127.0.0.1:4321".parse().unwrap();
            let io = Arc::new(ScriptedConnection::new(
                local,
                "127.0.0.2:5678".parse().unwrap(),
                vec![],
            ));
            let network = Arc::new(ScriptedNetwork::new(local, vec![io]));
            let source = format!(
                r#"
import gleam/erlang/atom
import gleam/erlang/process
import glisten/{transport}
import glisten/tcp as tcp_close
import glisten/socket/options
pub fn main() {{
  let assert Ok(listener) = {transport}.listen(0, [options.CertKeyConfig(options.CertKeyFiles("cert", "key"))])
  let assert Ok(connection) = {transport}.accept(listener)
  let assert Ok(_) = {transport}.handshake(connection)
  let result = process.new_subject()
  let _ = process.spawn_unlinked(fn() {{
    process.send(result, {transport}.controlling_process(connection, process.self()))
  }})
  assert process.receive(result, 1000) == Ok(Error(atom.create("not_owner")))
  let release = process.new_subject()
  let dying = process.spawn_unlinked(fn() {{ let _ = process.receive(release, 1000) Nil }})
  let monitor = process.monitor(dying)
  process.send(release, Nil)
  let assert Ok(_) = process.new_selector() |> process.select_monitors(fn(value) {{ value }}) |> process.selector_receive(1000)
  process.demonitor_process(monitor)
  assert {transport}.controlling_process(connection, dying) == Error(atom.create("badarg"))
  assert {transport}.controlling_process(connection, process.self()) == Ok(Nil)
  assert {transport}.close(connection) == Ok(Nil)
  assert {transport}.controlling_process(connection, process.self()) == Error(atom.create("closed"))
  assert tcp_close.close(listener) == Ok(Nil)
  Nil
}}
"#
            );
            let (mut execution, mut state) = source_project(&source, network.clone());
            let host = TestHost::default();
            assert_eq!(
                host.poll(std::pin::pin!(execution.run_main(
                    &host,
                    &mut state,
                    &mut Vec::new()
                )))
                .map(Result::unwrap),
                std::task::Poll::Ready(Value::Nil),
                "{transport}"
            );
            assert_eq!(
                network
                    .events
                    .lock()
                    .iter()
                    .filter(|event| **event == Event::CloseConnection)
                    .count(),
                1
            );
        }
    }

    #[test]
    fn tcp_and_tls_getopts_preserve_defaults_updates_and_atomic_validation() {
        for transport in ["tcp", "ssl"] {
            let local = "127.0.0.1:4321".parse().unwrap();
            let io = Arc::new(ScriptedConnection::new(
                local,
                "127.0.0.2:5678".parse().unwrap(),
                vec![],
            ));
            let network = Arc::new(ScriptedNetwork::new(local, vec![io]));
            let source = format!(
                r#"
import gleam/dynamic/decode
import gleam/erlang/atom
import glisten/{transport}
import glisten/tcp as tcp_close
import glisten/socket
import glisten/socket/options
fn get(connection, key) {{
  let assert Ok([#(_, value)]) = {transport}.get_socket_opts(connection, [atom.create(key)])
  value
}}
fn linger(value) {{
  let decoder = {{
    use enabled <- decode.field(0, decode.bool)
    use seconds <- decode.field(1, decode.int)
    decode.success(#(enabled, seconds))
  }}
  decode.run(value, decoder)
}}
pub fn main() {{
  let assert Ok(listener) = {transport}.listen(0, [options.CertKeyConfig(options.CertKeyFiles("cert", "key"))])
  let assert Ok(connection) = {transport}.accept(listener)
  let assert Ok(_) = {transport}.handshake(connection)
  assert {transport}.get_socket_opts(connection, []) == Ok([])
  assert decode.run(get(connection, "recbuf"), decode.int) == Ok(8192)
  assert decode.run(get(connection, "buffer"), decode.int) == Ok(65536)
  assert decode.run(get(connection, "nodelay"), decode.bool) == Ok(True)
  assert decode.run(get(connection, "reuseaddr"), decode.bool) == Ok(True)
  assert decode.run(get(connection, "mode"), atom.decoder()) == Ok(atom.create("binary"))
  assert decode.run(get(connection, "active"), decode.bool) == Ok(False)
  assert linger(get(connection, "linger")) == Ok(#(False, 0))
  assert decode.run(get(connection, "send_timeout"), decode.int) == Ok(30000)
  assert decode.run(get(connection, "send_timeout_close"), decode.bool) == Ok(True)
  assert {transport}.get_socket_opts(connection, [atom.create("unknown")]) == Error(socket.Badarg)
  assert {transport}.set_opts(connection, [options.Linger(#(True, 2)), options.Nodelay(False), options.Reuseaddr(False), options.SendTimeout(7), options.SendTimeoutClose(False)]) == Ok(Nil)
  assert linger(get(connection, "linger")) == Ok(#(True, 2))
  assert decode.run(get(connection, "nodelay"), decode.bool) == Ok(False)
  assert decode.run(get(connection, "reuseaddr"), decode.bool) == Ok(False)
  assert {transport}.set_opts(connection, [options.Buffer(123), options.Backlog(1)]) == Error(socket.Einval)
  assert {transport}.set_opts(connection, [options.Ipv6]) == Error(socket.Einval)
  assert {transport}.set_opts(connection, [options.Ip(options.Loopback)]) == Error(socket.Einval)
  assert {transport}.set_opts(connection, [options.CertKeyConfig(options.CertKeyFiles("cert", "key"))]) == Error(socket.Einval)
  assert {transport}.set_opts(connection, [options.AlpnPreferredProtocols(["h2"])]) == Error(socket.Einval)
  assert {transport}.set_opts(connection, [options.Mode(options.Binary)]) == Ok(Nil)
  assert decode.run(get(connection, "buffer"), decode.int) == Ok(65536)
  assert decode.run(get(connection, "send_timeout"), decode.int) == Ok(7)
  assert decode.run(get(connection, "send_timeout_close"), decode.bool) == Ok(False)
  assert {transport}.set_opts(connection, [options.ActiveMode(options.Once)]) == Ok(Nil)
  assert decode.run(get(connection, "active"), atom.decoder()) == Ok(atom.create("once"))
  assert {transport}.receive(connection, 0) == Error(socket.Einval)
  assert {transport}.set_opts(connection, [options.ActiveMode(options.Active)]) == Ok(Nil)
  assert decode.run(get(connection, "active"), decode.bool) == Ok(True)
  assert {transport}.set_opts(connection, [options.ActiveMode(options.Count(32767))]) == Ok(Nil)
  assert decode.run(get(connection, "active"), decode.int) == Ok(32767)
  assert {transport}.set_opts(connection, [options.Buffer(1), options.ActiveMode(options.Count(1))]) == Error(socket.Badarg)
  assert decode.run(get(connection, "buffer"), decode.int) == Ok(65536)
  assert decode.run(get(connection, "active"), decode.int) == Ok(32767)
  assert {transport}.set_opts(connection, [options.ActiveMode(options.Passive), options.Linger(#(False, 0))]) == Ok(Nil)
  assert linger(get(connection, "linger")) == Ok(#(False, 0))
  assert {transport}.do_shutdown(connection, atom.create("read")) == Error(socket.Badarg)
  assert {transport}.close(connection) == Ok(Nil)
  assert {transport}.get_socket_opts(connection, []) == Error(socket.Closed)
  assert {transport}.shutdown(connection) == Error(socket.Closed)
  assert {transport}.set_opts(connection, []) == Error(socket.Closed)
  assert tcp_close.close(listener) == Ok(Nil)
  assert {transport}.sockname(listener) == Error(socket.Closed)
  assert tcp_close.close(42) == Error(socket.Badarg)
  Nil
}}
"#
            );
            let (mut execution, mut state) = source_project(&source, network.clone());
            let host = TestHost::default();
            assert_eq!(
                host.poll(std::pin::pin!(execution.run_main(
                    &host,
                    &mut state,
                    &mut Vec::new()
                )))
                .map(Result::unwrap),
                std::task::Poll::Ready(Value::Nil),
                "{transport}"
            );
            assert_eq!(
                network
                    .events
                    .lock()
                    .iter()
                    .filter(|event| **event == Event::CloseConnection)
                    .count(),
                1
            );
        }
    }

    #[test]
    fn tcp_and_tls_io_failures_do_not_commit_options_or_hide_failure_reasons() {
        for transport in ["tcp", "ssl"] {
            let local = "127.0.0.1:4321".parse().unwrap();
            let mut io = ScriptedConnection::new(
                local,
                "127.0.0.2:5678".parse().unwrap(),
                vec![Err(std::io::ErrorKind::ConnectionReset.into())],
            );
            io.configure_error = Some(std::io::ErrorKind::PermissionDenied);
            io.peer_error = Some(std::io::ErrorKind::AddrNotAvailable);
            io.buffer_error = Some(std::io::ErrorKind::NotConnected);
            io.shutdown_error = Some(std::io::ErrorKind::BrokenPipe);
            io.write_error = Some(std::io::ErrorKind::ConnectionReset);
            let network = Arc::new(ScriptedNetwork::new(local, vec![Arc::new(io)]));
            let source = format!(
                r#"
import gleam/bytes_tree
import gleam/dynamic/decode
import gleam/erlang/atom
import glisten/{transport}
import glisten/tcp as tcp_close
import glisten/socket
import glisten/socket/options
pub fn main() {{
  let assert Ok(listener) = {transport}.listen(0, [options.CertKeyConfig(options.CertKeyFiles("cert", "key"))])
  let assert Ok(connection) = {transport}.accept(listener)
  let assert Ok(_) = {transport}.handshake(connection)
  assert {transport}.set_opts(connection, [options.Buffer(1)]) == Error(socket.Eacces)
  let assert Ok([#(_, buffer)]) = {transport}.get_socket_opts(connection, [atom.create("buffer")])
  assert decode.run(buffer, decode.int) == Ok(65536)
  assert {transport}.get_socket_opts(connection, [atom.create("recbuf")]) == Error(socket.Closed)
  assert {transport}.peername(connection) == Error(socket.Eaddrnotavail)
  assert {transport}.shutdown(connection) == Error(socket.Closed)
  assert {transport}.send(connection, bytes_tree.from_string("failed")) == Error(socket.Econnreset)
  assert {transport}.receive(connection, 0) == Error(socket.Econnreset)
  assert {transport}.close(connection) == Ok(Nil)
  assert tcp_close.close(listener) == Ok(Nil)
  Nil
}}
"#
            );
            let (mut execution, mut state) = source_project(&source, network.clone());
            let host = TestHost::default();
            assert_eq!(
                host.poll(std::pin::pin!(execution.run_main(
                    &host,
                    &mut state,
                    &mut Vec::new()
                )))
                .map(Result::unwrap),
                std::task::Poll::Ready(Value::Nil),
                "{transport}"
            );
            assert_eq!(
                network
                    .events
                    .lock()
                    .iter()
                    .filter(|event| **event == Event::CloseConnection)
                    .count(),
                1
            );
        }
    }

    #[test]
    fn original_tcp_controls_preserve_typed_results_values_and_effect_order() {
        let local: SocketAddr = "127.0.0.1:4321".parse().unwrap();
        let peer: SocketAddr = "127.0.0.2:5678".parse().unwrap();
        let io = Arc::new(ScriptedConnection::new(
            local,
            peer,
            vec![Ok(b"abcdefgh".to_vec()), Ok(Vec::new())],
        ));
        let network = Arc::new(ScriptedNetwork::new(local, vec![io]));
        let (mut execution, mut state) = source_project(
            r#"
import gleam/bytes_tree
import gleam/dict
import gleam/dynamic.{type Dynamic}
import gleam/dynamic/decode
import gleam/erlang/atom.{type Atom}
import glisten/socket
import glisten/socket/options
import glisten/tcp
import glisten/transport
pub fn main() {
  let assert Ok(listener) = tcp.listen(0, [])
  let assert Ok(#(options.IpV4(127, 0, 0, 1), 4321)) =
    transport.sockname(transport.Tcp, listener)
  // A ready accept wins even when its timeout is zero.
  let assert Ok(connection) = tcp.accept_timeout(listener, 0)
  let assert Ok(#(options.IpV4(127, 0, 0, 2), 5678)) =
    transport.peername(transport.Tcp, connection)
  assert tcp.handshake(connection) == Ok(connection)
  assert transport.negotiated_protocol(transport.Tcp, connection) ==
    Error("Can't negotiate protocol on tcp")
  let info: dict.Dict(Atom, Dynamic) = tcp.socket_info(connection)
  let assert Ok(domain) = dict.get(info, atom.create("domain"))
  assert decode.run(domain, atom.decoder()) == Ok(atom.create("inet"))
  assert dict.size(transport.socket_info(connection)) == 5
  assert tcp.receive(connection, 3) == Ok(<<"abc":utf8>>)
  assert tcp.receive_timeout(connection, 2, 10) == Ok(<<"de":utf8>>)
  assert tcp.receive(connection, 0) == Ok(<<"fgh":utf8>>)
  assert tcp.receive(connection, 0) == Error(socket.Closed)
  assert tcp.set_opts(connection, [options.Buffer(4096), options.Nodelay(False)]) == Ok(Nil)
  assert transport.set_buffer_size(transport.Tcp, connection) == Ok(Nil)
  assert tcp.send(connection, bytes_tree.from_string("out")) == Ok(Nil)
  assert tcp.shutdown(connection) == Ok(Nil)
  assert tcp.close(connection) == Ok(Nil)
  assert tcp.close(connection) == Ok(Nil)
  assert tcp.receive(connection, 0) == Error(socket.Closed)
  assert tcp.close(listener) == Ok(Nil)
  Nil
}
"#,
            network.clone(),
        );
        let host = TestHost::default();
        assert_eq!(
            host.poll(std::pin::pin!(execution.run_main(
                &host,
                &mut state,
                &mut Vec::new()
            )))
            .map(Result::unwrap),
            std::task::Poll::Ready(Value::Nil)
        );
        assert_eq!(
            *network.events.lock(),
            [
                Event::Listen("0.0.0.0:0".parse().unwrap()),
                Event::Accept,
                Event::Read(3),
                Event::Read(2),
                Event::Read(65536),
                Event::Read(65536),
                Event::Configure(crate::network::ConnectionOptions {
                    reuse_address: true,
                    nodelay: false,
                    linger: None,
                    buffer: 4096,
                }),
                Event::Configure(crate::network::ConnectionOptions {
                    reuse_address: true,
                    nodelay: false,
                    linger: None,
                    buffer: 8192,
                }),
                Event::Write(b"out".to_vec()),
                Event::ShutdownWrite,
                Event::CloseConnection,
                Event::CloseListener,
            ]
        );
    }
}
