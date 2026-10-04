//! Active IO is one domain-owned native invocation; source values retain no IO.
use crate::{
    Call, Component, GlistenProfile,
    schema::{One, Socket},
    sockets::{Active, Connection},
};
use geam::gleam_erlang::service::Processes;
use geam::host::{
    HostCallContinuation, HostCallError, HostCallableSchema, HostCaptures, HostConstructions,
    HostExecutionContext, HostExecutionError, HostOwnedCompletion, HostReturns, HostTypeListEnd,
};
use geam::provider::BitArrayValue;
use geam::provider::advanced::NativeValue;
use std::sync::Arc;

pub(crate) struct Reader;
impl HostCallableSchema for Reader {
    const PACKAGE: &'static str = "glisten";
    const MODULE: &'static str = "glisten/socket";
    const NAME: &'static str = "active_read";
    type Arguments = HostTypeListEnd;
    type Return = ();
    type Captures = One<Socket>;
    type Constructions = HostTypeListEnd;
    type Completion = HostReturns;
}

pub(crate) fn reader<'call, Profile: GlistenProfile>(
    mut call: Call<'call, Profile, ()>,
    captures: HostCaptures<'call, One<Socket>>,
    constructions: HostConstructions<'call, HostTypeListEnd>,
) -> Result<HostCallContinuation<'call, ()>, HostCallError> {
    let (socket, ()) = call.captures(captures);
    let key = call.external_payload(socket).key.clone();
    let identity = call.native_value::<Socket>(socket);
    let socket = call.service::<Component>().connection(&key);
    Ok(call.resume(constructions, move |context| {
        Box::pin(async move {
            let completion = match socket {
                Some(socket) => run(&context, socket, identity).await,
                None => Ok(()),
            };
            completion.map(|()| HostOwnedCompletion::new(|call, _| Ok(call.return_value(()))))
        })
    }))
}

async fn run<Profile: GlistenProfile>(
    context: &HostExecutionContext<'_, Profile, Component, HostTypeListEnd>,
    socket: Arc<Connection>,
    identity: NativeValue,
) -> Result<(), HostExecutionError> {
    let mut completion = Ok(());
    while completion.is_ok() {
        let changed = socket.changed.notified();
        tokio::pin!(changed);
        changed.as_mut().enable();
        let (enabled, limit, buffered) = {
            let mut control = socket.control.lock();
            if control.closed {
                break;
            }
            let enabled = control.ready
                && !control.transferring
                && control.settings.active != Active::Passive;
            let buffered = if enabled && !control.bytes.is_empty() {
                Some(Ok(std::mem::take(&mut control.bytes)))
            } else if enabled {
                control.ended.take().map(Err)
            } else {
                None
            };
            (enabled, control.settings.io.buffer, buffered)
        };
        if !enabled {
            changed.await;
            continue;
        }
        let event = match buffered {
            Some(event) => event,
            None => {
                let _read = tokio::select! { biased; () = &mut changed => continue, read = socket.read.lock() => read };
                tokio::select! {
                    biased;
                    () = &mut changed => continue,
                    result = socket.io.read(limit) => result.map_err(crate::sockets::native_reason),
                }
            }
        };
        let owned = Arc::clone(&socket);
        let source = identity.clone();
        completion = context
            .with_call(move |mut call| deliver(&mut call, &owned, source, event))
            .await;
    }
    completion
}

fn deliver<Profile: GlistenProfile>(
    call: &mut Call<'_, Profile, ()>,
    owned: &Connection,
    source: NativeValue,
    event: Result<Vec<u8>, &'static str>,
) {
    let mut control = owned.control.lock();
    if control.closed {
        return;
    }
    if control.transferring || control.settings.active == Active::Passive {
        match event {
            Ok(bytes) if !bytes.is_empty() => control.bytes.extend(bytes),
            Ok(_) => control.ended = Some("closed"),
            Err(reason) => control.ended = Some(reason),
        }
        return;
    }
    let event = match event {
        Ok(bytes) if bytes.is_empty() => Err("closed"),
        event => event,
    };
    match event {
        Ok(bytes) => {
            let data = call.native_value::<BitArrayValue>(BitArrayValue::from_bytes(bytes));
            Processes::new(call).send(
                &control.owner,
                NativeValue::tuple([
                    NativeValue::symbol(owned.transport.tag()),
                    source.clone(),
                    data,
                ]),
            );
            control.settings.active = match control.settings.active {
                Active::Once => Active::Passive,
                Active::Count(1) => {
                    Processes::new(call).send(
                        &control.owner,
                        NativeValue::tuple([
                            NativeValue::symbol(owned.transport.passive_tag()),
                            source,
                        ]),
                    );
                    Active::Passive
                }
                Active::Count(count) => Active::Count(count - 1),
                active => active,
            };
        }
        Err(reason) => {
            if reason != "closed" {
                Processes::new(call).send(
                    &control.owner,
                    NativeValue::tuple([
                        NativeValue::symbol(owned.transport.error_tag()),
                        source.clone(),
                        NativeValue::symbol(reason),
                    ]),
                );
            }
            Processes::new(call).send(
                &control.owner,
                NativeValue::tuple([NativeValue::symbol(owned.transport.closed_tag()), source]),
            );
            control.closed = true;
            drop(control);
            owned.io.close();
            owned.changed.notify_waiters();
        }
    }
}

pub(crate) async fn receive(socket: &Connection, length: usize) -> Result<Vec<u8>, &'static str> {
    {
        let control = socket.control.lock();
        if control.settings.active != Active::Passive {
            return Err("einval");
        }
    }
    let _read = socket.read.lock().await;
    loop {
        let changed = socket.changed.notified();
        tokio::pin!(changed);
        changed.as_mut().enable();
        let limit = {
            let mut control = socket.control.lock();
            if control.closed {
                return Err("closed");
            }
            if !control.ready {
                return Err("badarg");
            }
            if control.settings.active != Active::Passive {
                return Err("einval");
            }
            if (length == 0 && !control.bytes.is_empty())
                || (length > 0 && control.bytes.len() >= length)
            {
                let take = if length == 0 {
                    control.bytes.len()
                } else {
                    length
                };
                let rest = control.bytes.split_off(take);
                return Ok(std::mem::replace(&mut control.bytes, rest));
            }
            if let Some(reason) = control.ended {
                return Err(reason);
            }
            if length == 0 {
                control.settings.io.buffer
            } else {
                control.settings.io.buffer.min(length - control.bytes.len())
            }
        };
        let result = tokio::select! {
            biased;
            () = &mut changed => continue,
            result = socket.io.read(limit) => result.map_err(crate::sockets::native_reason),
        };
        let mut control = socket.control.lock();
        match result {
            Ok(bytes) if bytes.is_empty() => control.ended = Some("closed"),
            Ok(bytes) => control.bytes.extend(bytes),
            Err(reason) => control.ended = Some(reason),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test_support::{
        execution_fixture::TestHost,
        network::{Event, ScriptedConnection, ScriptedNetwork},
        source_project,
    };
    use std::sync::Arc;

    fn make_reader<'call>(
        mut call: crate::Call<
            'call,
            crate::test_support::Profile,
            geam::host::HostCreatedFunction<super::Reader>,
        >,
        constructions: geam::host::HostConstructions<
            'call,
            crate::schema::One<geam::host::HostCreatedFunction<super::Reader>>,
        >,
        socket: geam::host::HostExternal<'call, crate::schema::Socket>,
    ) -> Result<
        geam::host::HostCallCompletion<'call, geam::host::HostCreatedFunction<super::Reader>>,
        geam::host::HostCallError,
    > {
        let reader = call.construct_function::<super::Reader>(
            constructions.at::<geam::host::HostTypeIndex0>(),
            (socket, ()),
        );
        Ok(call.return_value(reader))
    }

    #[test]
    fn a_retained_reader_invoked_after_close_finishes_without_reading() {
        use crate::{Component, schema::One, schema::Socket, test_support::source_project_with};
        use geam::host::{HostCreatedFunction, HostProviderModule};
        let provider = HostProviderModule::new("fixture", "fixture")
            .unwrap()
            .with_scoped_function_and_constructions::<
                Component,
                (Socket,),
                HostCreatedFunction<super::Reader>,
                One<HostCreatedFunction<super::Reader>>,
                _,
            >("make_reader", make_reader)
            .unwrap();
        let source = r#"
import glisten/tcp
import glisten/socket
@external(erlang, "fixture", "make_reader")
fn make_reader(socket: socket.Socket) -> fn() -> Nil
pub fn main() {
  let assert Ok(listener) = tcp.listen(0, [])
  let assert Ok(connection) = tcp.accept(listener)
  let reader = make_reader(connection)
  assert tcp.close(connection) == Ok(Nil)
  reader()
  assert tcp.receive(connection, 0) == Error(socket.Closed)
  assert tcp.close(listener) == Ok(Nil)
  Nil
}
"#;
        let address = "127.0.0.1:4321".parse().unwrap();
        let io = Arc::new(ScriptedConnection::new(
            address,
            "127.0.0.2:1234".parse().unwrap(),
            vec![],
        ));
        let network = Arc::new(ScriptedNetwork::new(address, vec![io]));
        let (mut execution, mut state) = source_project_with(source, network.clone(), [provider]);
        let host = TestHost::default();
        assert_eq!(
            host.block_on(execution.run_main(&host, &mut state, &mut Vec::new()))
                .map(|outcome| outcome
                    .try_into_value()
                    .expect("fixture must return normally"))
                .unwrap(),
            geam::Value::Nil
        );
        let events = network.events.lock();
        assert!(!events.iter().any(|event| matches!(event, Event::Read(_))));
        assert_eq!(
            events
                .iter()
                .filter(|event| **event == Event::CloseConnection)
                .count(),
            1
        );
    }

    fn hold_read<'call>(
        mut native: geam::host::native::NativeCall<
            'call,
            crate::test_support::Profile,
            crate::Component,
            (),
            crate::schema::One<()>,
        >,
        socket: geam::host::HostExternal<'call, crate::schema::Socket>,
    ) -> Result<geam::host::HostCallContinuation<'call, ()>, geam::host::HostCallError> {
        let key = native.call().external_payload(socket).key.clone();
        let owned = native
            .call()
            .service::<crate::Component>()
            .connection(&key)
            .unwrap();
        let deadline = native.call().clock().now() + std::time::Duration::from_millis(10);
        let pause = native.call().clock().sleep_until(deadline);
        Ok(native.resume::<geam::host::HostTypeIndex0>(move |_| {
            Box::pin(async move {
                let _read = owned.read.lock().await;
                pause.await;
                Ok(geam::provider::advanced::NativeValue::symbol("nil"))
            })
        }))
    }

    #[test]
    fn becoming_passive_cancels_an_active_reader_waiting_for_the_read_owner() {
        use crate::{Component, schema::One, schema::Socket, test_support::source_project_with};
        use geam::host::{HostProviderModule, native::NativeRules};
        let provider = HostProviderModule::new("fixture", "fixture")
            .unwrap()
            .with_resumable_native_function::<Component, (Socket,), (), One<()>, _>(
                "hold_read",
                NativeRules::default(),
                hold_read,
            )
            .unwrap();
        let source = r#"
import gleam/erlang/process
import glisten/tcp
import glisten/socket
import glisten/socket/options
@external(erlang, "fixture", "hold_read") fn hold_read(socket: socket.Socket) -> Nil
pub fn main() {
  let assert Ok(listener) = tcp.listen(0, [])
  let assert Ok(connection) = tcp.accept(listener)
  let done = process.new_subject()
  let _ = process.spawn(fn() {
    hold_read(connection)
    process.send(done, Nil)
  })
  process.sleep(1)
  assert tcp.set_opts(connection, [options.ActiveMode(options.Once)]) == Ok(Nil)
  process.sleep(1)
  assert tcp.set_opts(connection, [options.ActiveMode(options.Passive)]) == Ok(Nil)
  assert process.receive(done, 100) == Ok(Nil)
  assert tcp.close(connection) == Ok(Nil)
  assert tcp.close(listener) == Ok(Nil)
  Nil
}
"#;
        let address = "127.0.0.1:4321".parse().unwrap();
        let io = Arc::new(ScriptedConnection::new(
            address,
            "127.0.0.2:1234".parse().unwrap(),
            vec![],
        ));
        let network = Arc::new(ScriptedNetwork::new(address, vec![io]));
        let (mut execution, mut state) = source_project_with(source, network.clone(), [provider]);
        let host = TestHost::default();
        let mut echo = Vec::new();
        let mut run = std::pin::pin!(execution.run_main(&host, &mut state, &mut echo));
        assert!(host.poll(run.as_mut()).is_pending());
        host.advance(std::time::Duration::from_millis(1));
        assert!(host.poll(run.as_mut()).is_pending());
        host.advance(std::time::Duration::from_millis(1));
        assert!(host.poll(run.as_mut()).is_pending());
        host.advance(std::time::Duration::from_millis(8));
        assert_eq!(
            host.poll(run.as_mut()).map(|result| result
                .unwrap()
                .try_into_value()
                .expect("fixture must return normally")),
            std::task::Poll::Ready(geam::Value::Nil)
        );
        let events = network.events.lock();
        assert!(!events.iter().any(|event| matches!(event, Event::Read(_))));
        assert_eq!(
            events
                .iter()
                .filter(|event| **event == Event::CloseConnection)
                .count(),
            1
        );
    }

    fn replay<'call>(
        mut call: crate::Call<'call, crate::test_support::Profile, ()>,
        socket: geam::host::HostExternal<'call, crate::schema::Socket>,
        mode: geam::provider::BigInt,
        bytes: geam::provider::BitArrayValue,
        error: bool,
    ) -> Result<geam::host::HostCallCompletion<'call, ()>, geam::host::HostCallError> {
        let key = call.external_payload(socket).key.clone();
        let owned = call.service::<crate::Component>().connection(&key).unwrap();
        let identity = call.native_value::<crate::schema::Socket>(socket);
        let mode = u32::try_from(mode).unwrap();
        // A completed IO result can reach delivery after a valid transfer has
        // paused reading. The same production guard restores delivery.
        let transfer = if mode == 2 {
            Some(
                crate::sockets::Transfer::begin(
                    Arc::clone(&owned),
                    call.require_execution_unit().unwrap().id(),
                    true,
                )
                .unwrap(),
            )
        } else {
            None
        };
        if mode == 0 {
            call.service::<crate::Component>().remove(&key);
        }
        super::deliver(
            &mut call,
            &owned,
            identity,
            if error {
                Err("econnreset")
            } else {
                Ok(bytes.bytes().to_vec())
            },
        );
        drop(transfer);
        Ok(call.return_value(()))
    }

    #[test]
    fn completed_reads_preserve_data_and_endings_during_passive_transfer_or_close() {
        use crate::{Component, schema::Socket, test_support::source_project_with};
        use geam::host::HostProviderModule;
        for mode in [0, 1, 2] {
            for (bytes, error, expected) in [
                (b"late".as_slice(), false, "Data(<<\"late\":utf8>>)"),
                (b"".as_slice(), false, "Closed"),
                (b"".as_slice(), true, "Failure(atom.create(\"econnreset\"))"),
            ] {
                let provider = HostProviderModule::new("fixture", "fixture")
                    .unwrap()
                    .with_scoped_function::<Component, (
                        Socket,
                        geam::provider::BigInt,
                        geam::provider::BitArrayValue,
                        bool,
                    ), (), _>("replay", replay)
                    .unwrap();
                let source = format!(
                    r#"
import gleam/dynamic/decode
import gleam/erlang/atom.{{type Atom}}
import gleam/erlang/process
import glisten/tcp
import glisten/socket
import glisten/socket/options
pub type Record {{ Data(BitArray) Closed Failure(Atom) }}
@external(erlang, "fixture", "replay") fn replay(value: socket.Socket, mode: Int, data: BitArray, failed: Bool) -> Nil
fn data(record) {{
  let decoder = {{ use bytes <- decode.field(2, decode.bit_array) decode.success(Data(bytes)) }}
  let assert Ok(value) = decode.run(record, decoder)
  value
}}
fn failure(record) {{
  let decoder = {{ use reason <- decode.field(2, atom.decoder()) decode.success(Failure(reason)) }}
  let assert Ok(value) = decode.run(record, decoder)
  value
}}
pub fn main() {{
  let assert Ok(listener) = tcp.listen(0, [])
  let assert Ok(connection) = tcp.accept(listener)
  let selector = process.new_selector()
    |> process.select_record(atom.create("tcp"), 2, data)
    |> process.select_record(atom.create("tcp_error"), 2, failure)
    |> process.select_record(atom.create("tcp_closed"), 1, fn(_) {{ Closed }})
  replay(connection, {mode}, {payload}, {failed})
  {assertions}
  let _ = tcp.close(connection)
  let _ = tcp.close(listener)
  Nil
}}
"#,
                    payload = if bytes.is_empty() {
                        "<<>>"
                    } else {
                        "<<\"late\":utf8>>"
                    },
                    failed = if error { "True" } else { "False" },
                    assertions = if mode == 0 {
                        "assert process.selector_receive(selector, 0) == Error(Nil) assert tcp.receive(connection, 0) == Error(socket.Closed)".to_owned()
                    } else {
                        format!(
                            "assert tcp.set_opts(connection, [options.ActiveMode(options.Once)]) == Ok(Nil) assert process.selector_receive(selector, 100) == Ok({expected}) {}",
                            if error {
                                "assert process.selector_receive(selector, 100) == Ok(Closed)"
                            } else {
                                ""
                            }
                        )
                    }
                );
                let local = "127.0.0.1:4321".parse().unwrap();
                let io = Arc::new(ScriptedConnection::new(
                    local,
                    "127.0.0.2:5678".parse().unwrap(),
                    vec![],
                ));
                let network = Arc::new(ScriptedNetwork::new(local, vec![io]));
                let (mut execution, mut state) = source_project_with(&source, network, [provider]);
                let host = TestHost::default();
                assert_eq!(
                    host.block_on(execution.run_main(&host, &mut state, &mut Vec::new()))
                        .map(|outcome| outcome
                            .try_into_value()
                            .expect("fixture must return normally"))
                        .unwrap(),
                    geam::Value::Nil
                );
            }
        }
    }

    #[test]
    fn tcp_and_tls_once_count_and_active_deliver_exact_records_and_errors() {
        for transport in ["tcp", "ssl"] {
            let address = "127.0.0.1:4321".parse().unwrap();
            let io = Arc::new(ScriptedConnection::new(
                address,
                "127.0.0.2:1234".parse().unwrap(),
                vec![
                    Ok(b"a".to_vec()),
                    Ok(b"b".to_vec()),
                    Ok(b"c".to_vec()),
                    Ok(b"d".to_vec()),
                    Err(std::io::ErrorKind::ConnectionReset.into()),
                ],
            ));
            let network = Arc::new(ScriptedNetwork::new(address, vec![io]));
            let source = format!(
                r#"
import gleam/dynamic/decode
import gleam/erlang/atom.{{type Atom}}
import gleam/erlang/process
import glisten/{transport}
import glisten/tcp as tcp_close
import glisten/socket
import glisten/socket/options
pub type Record {{ Data(BitArray) Passive Failure(Atom) Closed }}
fn data(record) {{
  let decoder = {{ use bytes <- decode.field(2, decode.bit_array) decode.success(Data(bytes)) }}
  let assert Ok(value) = decode.run(record, decoder)
  value
}}
fn failure(record) {{
  let decoder = {{ use reason <- decode.field(2, atom.decoder()) decode.success(Failure(reason)) }}
  let assert Ok(value) = decode.run(record, decoder)
  value
}}
pub fn main() {{
  let assert Ok(listener) = {transport}.listen(0, [options.CertKeyConfig(options.CertKeyFiles("cert", "key"))])
  let assert Ok(connection) = {transport}.accept(listener)
  let assert Ok(_) = {transport}.handshake(connection)
  let selector = process.new_selector()
    |> process.select_record(atom.create("{tag}"), 2, data)
    |> process.select_record(atom.create("{tag}_passive"), 1, fn(_) {{ Passive }})
    |> process.select_record(atom.create("{tag}_error"), 2, failure)
    |> process.select_record(atom.create("{tag}_closed"), 1, fn(_) {{ Closed }})
  assert {transport}.set_opts(connection, [options.ActiveMode(options.Count(0))]) == Ok(Nil)
  assert process.selector_receive(selector, 1000) == Ok(Passive)
  assert {transport}.set_opts(connection, [options.ActiveMode(options.Once)]) == Ok(Nil)
  assert process.selector_receive(selector, 1000) == Ok(Data(<<"a":utf8>>))
  let assert Ok([#(_, active)]) = {transport}.get_socket_opts(connection, [atom.create("active")])
  assert decode.run(active, decode.bool) == Ok(False)
  assert {transport}.set_opts(connection, [options.ActiveMode(options.Count(1)), options.ActiveMode(options.Count(1))]) == Ok(Nil)
  assert process.selector_receive(selector, 1000) == Ok(Data(<<"b":utf8>>))
  assert process.selector_receive(selector, 1000) == Ok(Data(<<"c":utf8>>))
  assert process.selector_receive(selector, 1000) == Ok(Passive)
  assert {transport}.set_opts(connection, [options.ActiveMode(options.Active)]) == Ok(Nil)
  assert process.selector_receive(selector, 1000) == Ok(Data(<<"d":utf8>>))
  assert process.selector_receive(selector, 1000) == Ok(Failure(atom.create("econnreset")))
  assert process.selector_receive(selector, 1000) == Ok(Closed)
  assert {transport}.receive(connection, 0) == Error(socket.Closed)
  assert {transport}.close(connection) == Ok(Nil)
  assert tcp_close.close(listener) == Ok(Nil)
  Nil
}}
"#,
                tag = if transport == "tcp" { "tcp" } else { "ssl" }
            );
            let (mut execution, mut state) = source_project(&source, network.clone());
            let host = TestHost::default();
            assert_eq!(
                host.poll(std::pin::pin!(execution.run_main(
                    &host,
                    &mut state,
                    &mut Vec::new()
                )))
                .map(|result| result
                    .unwrap()
                    .try_into_value()
                    .expect("fixture must return normally")),
                std::task::Poll::Ready(geam::Value::Nil),
                "{transport}"
            );
            assert_eq!(
                network
                    .events
                    .lock()
                    .iter()
                    .filter(|event| **event == Event::Read(65536))
                    .count(),
                5
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
    fn rearming_after_passive_eof_delivers_closed_for_tcp_and_tls() {
        for transport in ["tcp", "ssl"] {
            let address = "127.0.0.1:4321".parse().unwrap();
            let io = Arc::new(ScriptedConnection::new(
                address,
                "127.0.0.2:1234".parse().unwrap(),
                vec![Ok(vec![])],
            ));
            let network = Arc::new(ScriptedNetwork::new(address, vec![io]));
            let source = format!(
                r#"
import gleam/erlang/atom
import gleam/erlang/process
import glisten/{transport}
import glisten/tcp as tcp_close
import glisten/socket
import glisten/socket/options
pub fn main() {{
  let assert Ok(listener) = {transport}.listen(0, [options.CertKeyConfig(options.CertKeyFiles("cert", "key"))])
  let assert Ok(connection) = {transport}.accept(listener)
  let assert Ok(_) = {transport}.handshake(connection)
  assert {transport}.receive(connection, 0) == Error(socket.Closed)
  assert {transport}.set_opts(connection, [options.ActiveMode(options.Active)]) == Ok(Nil)
  let selector = process.new_selector() |> process.select_record(atom.create("{tag}_closed"), 1, fn(_) {{ Nil }})
  assert process.selector_receive(selector, 1000) == Ok(Nil)
  assert tcp_close.close(listener) == Ok(Nil)
  Nil
}}
"#,
                tag = if transport == "tcp" { "tcp" } else { "ssl" }
            );
            let (mut execution, mut state) = source_project(&source, network.clone());
            let host = TestHost::default();
            assert_eq!(
                host.poll(std::pin::pin!(execution.run_main(
                    &host,
                    &mut state,
                    &mut Vec::new()
                )))
                .map(|result| result
                    .unwrap()
                    .try_into_value()
                    .expect("fixture must return normally")),
                std::task::Poll::Ready(geam::Value::Nil)
            );
            assert_eq!(
                network
                    .events
                    .lock()
                    .iter()
                    .filter(|event| **event == Event::Read(65536))
                    .count(),
                1
            );
        }
    }
}
