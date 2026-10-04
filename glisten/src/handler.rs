//! The unchanged handler's callback and socket-record projection boundaries.
use crate::Component;

#[geam::module(path = "glisten/internal/handler", crate_path = geam,
    profile = crate::GlistenProfile, component = crate::Component)]
mod functions {
    use crate::State;
    use geam::provider::{Call, Callback, HostResult, Value};
    #[geam::function(await)]
    async fn rescue<Item>(
        #[geam::call] call: &mut Call<State>,
        body: Callback<fn() -> Value<Item>>,
    ) -> HostResult<Result<Value<Item>, geam::gleam_stdlib::Dynamic>> {
        // The Erlang FFI catches throw only. Geam has no throw class; its source
        // and host failures, cancellation, and application exit keep their identity.
        Ok(Ok(call.invoke(&body, ()).await?))
    }
}

use crate::{GlistenProfile, schema::One};
use geam::gleam_stdlib::provider_support::Dynamic;
use geam::host::native::{NativeCall, NativeRules};
use geam::host::{
    HostCallCompletion, HostCallError, HostExternal, HostProviderModule, HostRegistrationError,
    HostType, HostTypeIndex0,
};
use geam::provider::{BitArrayValue, HostFailure};

pub(crate) fn provider<Profile: GlistenProfile>()
-> Result<HostProviderModule<Profile>, HostRegistrationError> {
    functions::__geam_module::<Profile>()
        .and_then(|module| module.with_native_function::<Component, (Dynamic,), BitArrayValue, One<BitArrayValue>, _>("socket_data", NativeRules::default(), project::<Profile, BitArrayValue>))
        .and_then(|module| module.with_native_function::<Component, (Dynamic,), crate::schema::SocketReason, One<crate::schema::SocketReason>, _>("socket_error", NativeRules::default(), project::<Profile, crate::schema::SocketReason>))
}

fn project<'call, Profile: GlistenProfile, Return: HostType>(
    mut native: NativeCall<'call, Profile, Component, Return, One<Return>>,
    record: HostExternal<'call, Dynamic>,
) -> Result<HostCallCompletion<'call, Return>, HostCallError> {
    let record = native.source::<Dynamic>(record);
    let tag = record.index(0).and_then(|value| value.as_symbol());
    if record.len() != Some(3)
        || !matches!(
            tag.as_deref(),
            Some("tcp" | "ssl" | "tcp_error" | "ssl_error")
        )
    {
        return Err(HostFailure::new("invalid glisten socket record").into());
    }
    let value = record
        .index(2)
        .and_then(|value| native.convert::<HostTypeIndex0>(&value))
        .ok_or_else(|| {
            HostFailure::new("socket record payload does not match the declared type")
        })?;
    Ok(native.finish(value))
}

#[cfg(test)]
mod tests {
    use crate::test_support::{
        execution_fixture::TestHost,
        network::{Event, ScriptedConnection, ScriptedNetwork},
        source_project,
    };
    use std::sync::Arc;
    use std::task::Poll;

    #[test]
    fn socket_record_projection_validates_tags_arity_and_requested_payload() {
        use crate::{
            Component,
            schema::One,
            test_support::{Profile, source_project_with},
        };
        use geam::gleam_stdlib::provider_support::Dynamic;
        use geam::host::{HostProviderModule, native::NativeRules};
        use geam::provider::BitArrayValue;

        for (record, expected) in [
            (
                "dynamic.array([atom.to_dynamic(atom.create(\"tcp\")), dynamic.nil(), dynamic.bit_array(<<0, 255>>)])",
                None,
            ),
            (
                "dynamic.array([atom.to_dynamic(atom.create(\"ssl\")), dynamic.nil(), dynamic.bit_array(<<0, 255>>)])",
                None,
            ),
            ("dynamic.nil()", Some("invalid glisten socket record")),
            (
                "dynamic.array([atom.to_dynamic(atom.create(\"unknown\")), dynamic.nil(), dynamic.bit_array(<<0, 255>>)])",
                Some("invalid glisten socket record"),
            ),
            (
                "dynamic.array([atom.to_dynamic(atom.create(\"tcp\")), dynamic.nil()])",
                Some("invalid glisten socket record"),
            ),
            (
                "dynamic.array([atom.to_dynamic(atom.create(\"tcp\")), dynamic.nil(), dynamic.int(42)])",
                Some("socket record payload does not match the declared type"),
            ),
        ] {
            let provider = HostProviderModule::new("fixture", "fixture").unwrap()
                .with_native_function::<Component, (Dynamic,), BitArrayValue, One<BitArrayValue>, _>("data", NativeRules::default(), super::project::<Profile, BitArrayValue>).unwrap();
            let source = format!(
                r#"
import gleam/dynamic
import gleam/erlang/atom
@external(erlang, "glisten_ffi", "socket_data")
fn data(record: dynamic.Dynamic) -> BitArray
pub fn main() {{ assert data({record}) == <<0, 255>> Nil }}
"#
            );
            let network = Arc::new(ScriptedNetwork::new(
                "127.0.0.1:4321".parse().unwrap(),
                vec![],
            ));
            let (mut execution, mut state) = source_project_with(&source, network, [provider]);
            let host = TestHost::default();
            let result = host
                .block_on(execution.run_main(&host, &mut state, &mut Vec::new()))
                .map(|outcome| {
                    outcome
                        .try_into_value()
                        .expect("fixture must return normally")
                });
            match expected {
                None => assert_eq!(result.unwrap(), geam::Value::Nil),
                Some(message) => assert!(
                    result.unwrap_err().to_string().contains(message),
                    "{record}"
                ),
            }
        }
    }

    #[test]
    fn socket_error_projection_uses_the_declared_reason_family_and_rejects_other_payloads() {
        use crate::{
            Component,
            schema::{One, SocketReason},
            test_support::{Profile, source_project_with},
        };
        use geam::gleam_stdlib::provider_support::Dynamic;
        use geam::host::{HostProviderModule, native::NativeRules};
        for (record, expected) in [
            (
                "dynamic.array([atom.to_dynamic(atom.create(\"tcp_error\")), dynamic.nil(), atom.to_dynamic(atom.create(\"econnreset\"))])",
                None,
            ),
            (
                "dynamic.array([atom.to_dynamic(atom.create(\"ssl_error\")), dynamic.nil(), atom.to_dynamic(atom.create(\"econnreset\"))])",
                None,
            ),
            ("dynamic.nil()", Some("invalid glisten socket record")),
            (
                "dynamic.array([atom.to_dynamic(atom.create(\"unknown\")), dynamic.nil(), atom.to_dynamic(atom.create(\"econnreset\"))])",
                Some("invalid glisten socket record"),
            ),
            (
                "dynamic.array([atom.to_dynamic(atom.create(\"tcp_error\")), dynamic.nil(), dynamic.bit_array(<<>>)])",
                Some("socket record payload does not match the declared type"),
            ),
        ] {
            let provider = HostProviderModule::new("fixture", "fixture")
                .unwrap()
                .with_native_function::<Component, (Dynamic,), SocketReason, One<SocketReason>, _>(
                    "reason",
                    NativeRules::default(),
                    super::project::<Profile, SocketReason>,
                )
                .unwrap();
            let source = format!(
                r#"
import gleam/dynamic
import gleam/erlang/atom
import glisten/socket
@external(erlang, "glisten_ffi", "socket_data") fn reason(record: dynamic.Dynamic) -> socket.SocketReason
pub fn main() {{ assert reason({record}) == socket.Econnreset Nil }}
"#
            );
            let network = Arc::new(ScriptedNetwork::new(
                "127.0.0.1:4321".parse().unwrap(),
                vec![],
            ));
            let (mut execution, mut state) = source_project_with(&source, network, [provider]);
            let host = TestHost::default();
            let result = host
                .block_on(execution.run_main(&host, &mut state, &mut Vec::new()))
                .map(|outcome| {
                    outcome
                        .try_into_value()
                        .expect("fixture must return normally")
                });
            match expected {
                None => assert_eq!(result.unwrap(), geam::Value::Nil),
                Some(message) => assert!(result.unwrap_err().to_string().contains(message)),
            }
        }
    }

    #[test]
    fn unchanged_tcp_and_tls_handlers_preserve_callback_state_and_close_after_eof() {
        for tls in [false, true] {
            let address = "127.0.0.1:4321".parse().unwrap();
            let io = Arc::new(ScriptedConnection::new(
                address,
                "127.0.0.2:1234".parse().unwrap(),
                vec![Ok(b"ping".to_vec()), Ok(vec![])],
            ));
            let network = Arc::new(ScriptedNetwork::new(address, vec![]));
            let source = format!(
                r#"
import gleam/bytes_tree
import gleam/erlang/process
import gleam/option.{{None}}
import glisten
import glisten/tcp
import glisten/socket
pub fn main() {{
  let initialised = process.new_subject()
  let completed = process.new_subject()
  let assert Ok(server) = glisten.new(
    fn(connection) {{
      process.send(initialised, connection.socket)
      #(<<>>, None)
    }},
    fn(state, message, connection) {{
      let assert glisten.Packet(packet) = message
      assert state == <<>>
      assert packet == <<"ping":utf8>>
      assert glisten.send(connection, bytes_tree.from_string("pong")) == Ok(Nil)
      glisten.continue(packet)
    }},
  )
  |> glisten.with_pool_size(1)
  {tls}
  |> glisten.with_close(fn(state) {{ assert state == <<"ping":utf8>> process.send(completed, state) }})
  |> glisten.start(0)
  let assert Ok(alias) = process.receive(initialised, 1000)
  assert process.receive(completed, 1000) == Ok(<<"ping":utf8>>)
  assert tcp.receive(alias, 0) == Error(socket.Closed)
  process.unlink(server.pid)
  process.kill(server.pid)
  Nil
}}
"#,
                tls = if tls {
                    "|> glisten.with_tls(\"cert\", \"key\")"
                } else {
                    ""
                }
            );
            let (mut execution, mut state) = source_project(&source, network.clone());
            let host = TestHost::default();
            let mut echo = Vec::new();
            let mut running = std::pin::pin!(execution.run_main(&host, &mut state, &mut echo));
            assert!(host.poll(running.as_mut()).is_pending());
            network.offer(io);
            assert_eq!(
                host.poll(running.as_mut()).map(|result| result
                    .unwrap()
                    .try_into_value()
                    .expect("fixture must return normally")),
                Poll::Ready(geam::Value::Nil),
                "TLS={tls}"
            );
            assert!(
                network
                    .events
                    .lock()
                    .contains(&Event::Write(b"pong".to_vec()))
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
    }

    #[test]
    fn application_exit_in_rescue_closes_tcp_and_tls_servers_with_pending_io() {
        use crate::test_support::{Profile, source_project_with};
        use geam::execution::{ExecutionOutcome, ExitStatus};
        use geam::host::{HostCallCompletion, HostCallError, HostProviderModule};

        fn exit_application<'call>(
            call: crate::Call<'call, Profile, ()>,
        ) -> Result<HostCallCompletion<'call, ()>, HostCallError> {
            call.exit(ExitStatus::new(7))
        }

        for tls in [false, true] {
            let provider = HostProviderModule::new("fixture", "fixture")
                .unwrap()
                .with_scoped_function::<crate::Component, (), (), _>(
                    "exit_application",
                    exit_application,
                )
                .unwrap();
            let source = format!(
                r#"
import gleam/erlang/process
import gleam/option.{{None}}
import glisten
@external(erlang, "fixture", "exit_application") fn exit_application() -> Nil
pub fn main() {{
  let initialised = process.new_subject()
  let assert Ok(_server) = glisten.new(
    fn(_connection) {{ process.send(initialised, Nil) #(7, None) }},
    fn(state, message, _connection) {{
      assert state == 7
      let assert glisten.Packet(<<"exit":utf8>>) = message
      exit_application()
      panic as "application exit must leave the callback"
    }},
  ) |> glisten.with_pool_size(2) {tls} |> glisten.start(0)
  assert process.receive(initialised, 1000) == Ok(Nil)
  assert process.receive(initialised, 1000) == Ok(Nil)
  process.sleep(60_000)
  panic as "application exit must leave the main unit"
}}
"#,
                tls = if tls {
                    "|> glisten.with_tls(\"cert\", \"key\")"
                } else {
                    ""
                }
            );
            let address = "127.0.0.1:4321".parse().unwrap();
            let initial_network = Arc::new(ScriptedNetwork::new(address, vec![]));
            let (mut execution, mut state) =
                source_project_with(&source, initial_network, [provider]);
            let host = TestHost::default();
            // Reuse the same module and host after Exited; each run owns a fresh domain.
            for _ in 0..2 {
                let first = Arc::new(ScriptedConnection::new(
                    address,
                    "127.0.0.2:1234".parse().unwrap(),
                    vec![],
                ));
                let second = Arc::new(ScriptedConnection::new(
                    address,
                    "127.0.0.3:5678".parse().unwrap(),
                    vec![],
                ));
                let network = Arc::new(ScriptedNetwork::new(address, vec![]));
                state.provider = crate::State::with_network(network.clone());
                let mut echo = Vec::new();
                let mut running = Box::pin(execution.run_main(&host, &mut state, &mut echo));
                assert!(host.poll(running.as_mut()).is_pending());
                network.offer(first.clone());
                network.offer(second.clone());
                assert!(host.poll(running.as_mut()).is_pending());
                assert_eq!(
                    network
                        .events
                        .lock()
                        .iter()
                        .filter(|event| matches!(event, Event::Read(_)))
                        .count(),
                    2,
                    "both active readers must be waiting before exit"
                );
                assert!(
                    network
                        .events
                        .lock()
                        .iter()
                        .filter(|event| **event == Event::Accept)
                        .count()
                        > 2
                );
                first.incoming.lock().push_back(Ok(b"exit".to_vec()));
                first.changed.notify_waiters();
                assert_eq!(
                    host.poll(running.as_mut()).map(Result::unwrap),
                    Poll::Ready(ExecutionOutcome::Exited(ExitStatus::new(7))),
                    "TLS={tls}"
                );
                drop(running);
                assert!(echo.is_empty());
                assert_eq!(
                    network
                        .events
                        .lock()
                        .iter()
                        .filter(|event| **event == Event::CloseConnection)
                        .count(),
                    2
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
                let events = network.events.lock().len();
                second.incoming.lock().push_back(Ok(b"exit".to_vec()));
                second.changed.notify_waiters();
                host.step();
                assert_eq!(
                    network.events.lock().len(),
                    events,
                    "no IO worker remains after Exited"
                );
            }
        }
    }

    #[test]
    fn unchanged_handler_normal_stop_abnormal_stop_panic_and_io_error_release_the_owner() {
        for (callback, input, expected) in [
            (
                "glisten.stop()",
                Ok(b"data".to_vec()),
                "assert reason == process.Normal",
            ),
            (
                "glisten.stop_abnormal(\"application stop\")",
                Ok(b"data".to_vec()),
                "let assert process.Abnormal(value) = reason assert decode.run(value, decode.string) == Ok(\"application stop\")",
            ),
            (
                "panic as \"callback failure\"",
                Ok(b"data".to_vec()),
                "let assert process.Abnormal(value) = reason assert string.contains(string.inspect(value), \"callback failure\")",
            ),
            (
                "panic as \"callback must not receive an IO error\"",
                Err(std::io::ErrorKind::ConnectionReset.into()),
                "let assert process.Abnormal(value) = reason assert decode.run(value, decode.string) == Ok(\"Received socket error Econnreset\")",
            ),
        ] {
            let address = "127.0.0.1:4321".parse().unwrap();
            let io = Arc::new(ScriptedConnection::new(
                address,
                "127.0.0.2:1234".parse().unwrap(),
                vec![],
            ));
            let network = Arc::new(ScriptedNetwork::new(address, vec![]));
            let source = format!(
                r#"
import gleam/dynamic/decode
import gleam/erlang/process
import gleam/option.{{None}}
import gleam/string
import glisten
import glisten/tcp
import glisten/socket
pub fn main() {{
  let initialised = process.new_subject()
  let assert Ok(server) = glisten.new(
    fn(connection) {{ process.send(initialised, #(connection.socket, process.self())) #(7, None) }},
    fn(state, message, _connection) {{
      assert state == 7
      let assert glisten.Packet(<<"data":utf8>>) = message
      {callback}
    }},
  ) |> glisten.with_pool_size(1) |> glisten.start(0)
  let assert Ok(#(alias, owner)) = process.receive(initialised, 1000)
  let _monitor = process.monitor(owner)
  let selector = process.new_selector() |> process.select_monitors(fn(down) {{ down.reason }})
  let assert Ok(reason) = process.selector_receive(selector, 1000)
  {expected}
  assert tcp.receive(alias, 0) == Error(socket.Closed)
  process.unlink(server.pid)
  process.kill(server.pid)
  Nil
}}
"#
            );
            let (mut execution, mut state) = source_project(&source, network.clone());
            let host = TestHost::default();
            let mut echo = Vec::new();
            let mut running = std::pin::pin!(execution.run_main(&host, &mut state, &mut echo));
            assert!(host.poll(running.as_mut()).is_pending());
            network.offer(io.clone());
            assert!(host.poll(running.as_mut()).is_pending());
            assert!(
                network
                    .events
                    .lock()
                    .iter()
                    .any(|event| matches!(event, Event::Read(_)))
            );
            io.incoming.lock().push_back(input);
            io.changed.notify_waiters();
            assert_eq!(
                host.poll(running.as_mut()).map(|result| result
                    .unwrap()
                    .try_into_value()
                    .expect("fixture must return normally")),
                Poll::Ready(geam::Value::Nil),
                "{callback}"
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
}
