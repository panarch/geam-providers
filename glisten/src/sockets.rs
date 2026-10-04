//! Domain-owned socket resources and the native TCP/TLS registrations.
use crate::socket::types::SocketReason;
use crate::{Domain, GlistenProfile, SocketKey, network, send as declarations};
use geam::execution::ExecutionUnit;
use geam::host::{HostProviderModule, HostRegistrationError};
use parking_lot::Mutex;
use std::io;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;

pub(crate) fn tcp_provider<Profile: GlistenProfile>()
-> Result<HostProviderModule<Profile>, HostRegistrationError> {
    declarations::tcp_provider::<Profile>().and_then(crate::native::tcp)
}
pub(crate) fn ssl_provider<Profile: GlistenProfile>()
-> Result<HostProviderModule<Profile>, HostRegistrationError> {
    declarations::ssl_provider::<Profile>().and_then(crate::native::ssl)
}
pub(crate) fn transport_provider<Profile: GlistenProfile>()
-> Result<HostProviderModule<Profile>, HostRegistrationError> {
    crate::native::transport::<Profile>()
}
pub(crate) fn handler_provider<Profile: GlistenProfile>()
-> Result<HostProviderModule<Profile>, HostRegistrationError> {
    crate::handler::provider::<Profile>()
}

pub(crate) enum Resource {
    Listener {
        owner: ExecutionUnit,
        io: Arc<dyn network::Listener>,
        settings: Settings,
        transport: Transport,
    },
    Connection(Arc<Connection>),
}

pub(crate) struct Connection {
    pub(crate) io: Arc<dyn network::Connection>,
    network: Arc<dyn network::Network>,
    pub(crate) transport: Transport,
    pub(crate) control: Mutex<Control>,
    pub(crate) changed: Notify,
    pub(crate) read: tokio::sync::Mutex<()>,
}

impl Resource {
    pub(crate) fn owner(&self) -> ExecutionUnit {
        match self {
            Self::Listener { owner, .. } => owner.clone(),
            Self::Connection(socket) => socket.control.lock().owner.clone(),
        }
    }

    pub(crate) fn close(&self) {
        match self {
            Self::Listener { io, .. } => io.close(),
            Self::Connection(socket) => socket.close(),
        }
    }
}

impl Connection {
    pub(crate) fn new(
        io: Arc<dyn network::Connection>,
        network: Arc<dyn network::Network>,
        owner: ExecutionUnit,
        settings: Settings,
        transport: Transport,
    ) -> Self {
        Self {
            io,
            network,
            transport,
            control: Mutex::new(Control {
                owner,
                settings,
                closed: false,
                ready: transport == Transport::Tcp,
                transferring: false,
                bytes: Vec::new(),
                ended: None,
            }),
            changed: Notify::new(),
            read: tokio::sync::Mutex::new(()),
        }
    }

    pub(crate) fn close(&self) {
        self.control.lock().closed = true;
        self.io.close();
        self.changed.notify_waiters();
    }
}

pub(crate) struct Control {
    pub(crate) owner: ExecutionUnit,
    pub(crate) settings: Settings,
    pub(crate) closed: bool,
    pub(crate) ready: bool,
    pub(crate) transferring: bool,
    pub(crate) bytes: Vec<u8>,
    pub(crate) ended: Option<&'static str>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Transport {
    Tcp,
    Tls,
}
impl Transport {
    pub(crate) fn tag(self) -> &'static str {
        match self {
            Self::Tcp => "tcp",
            Self::Tls => "ssl",
        }
    }
    pub(crate) fn closed_tag(self) -> &'static str {
        match self {
            Self::Tcp => "tcp_closed",
            Self::Tls => "ssl_closed",
        }
    }
    pub(crate) fn passive_tag(self) -> &'static str {
        match self {
            Self::Tcp => "tcp_passive",
            Self::Tls => "ssl_passive",
        }
    }
    pub(crate) fn error_tag(self) -> &'static str {
        match self {
            Self::Tcp => "tcp_error",
            Self::Tls => "ssl_error",
        }
    }
    pub(crate) fn matches(self, tag: &str) -> bool {
        tag == self.tag()
            || tag == self.closed_tag()
            || tag == self.passive_tag()
            || tag == self.error_tag()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Active {
    Passive,
    Once,
    Count(i16),
    All,
}

#[derive(Clone, Debug)]
pub(crate) struct Settings {
    pub io: network::ConnectionOptions,
    pub active: Active,
    pub send_timeout: Duration,
    pub send_timeout_close: bool,
}

/// One transfer phase owns suspension and restores delivery on cancellation.
pub(crate) struct Transfer {
    socket: Arc<Connection>,
}
impl Transfer {
    pub(crate) fn begin(
        socket: Arc<Connection>,
        caller: geam::execution::ExecutionUnitId,
        target_alive: bool,
    ) -> Result<Self, &'static str> {
        {
            let mut control = socket.control.lock();
            if control.closed {
                return Err("closed");
            }
            if control.owner.id() != caller {
                return Err("not_owner");
            }
            if !target_alive {
                return Err("badarg");
            }
            if control.transferring {
                return Err("einval");
            }
            control.transferring = true;
        }
        socket.changed.notify_waiters();
        Ok(Self { socket })
    }
    pub(crate) fn commit(
        &mut self,
        target: ExecutionUnit,
        alive: bool,
    ) -> Result<(), &'static str> {
        if !alive {
            return Err("badarg");
        }
        let mut control = self.socket.control.lock();
        if control.closed {
            return Err("closed");
        }
        control.owner = target;
        control.transferring = false;
        drop(control);
        self.socket.changed.notify_waiters();
        Ok(())
    }
    fn finish(&mut self) {
        self.socket.control.lock().transferring = false;
        self.socket.changed.notify_waiters();
    }
}
impl Drop for Transfer {
    fn drop(&mut self) {
        self.finish();
    }
}

impl Domain {
    pub(crate) fn connection(&self, key: &SocketKey) -> Option<Arc<Connection>> {
        match self.resources.get(key) {
            Some(Resource::Connection(socket)) if !socket.control.lock().closed => {
                Some(Arc::clone(socket))
            }
            _ => None,
        }
    }
}

pub(crate) async fn send(socket: &Connection, bytes: &[u8]) -> Result<(), SocketReason> {
    let settings = socket.control.lock().settings.clone();
    let write = socket.io.write(bytes);
    tokio::select! {
            biased;
            result = write => result.map_err(reason),
            () = socket.network.sleep(settings.send_timeout) => {
                if settings.send_timeout_close { socket.close(); }
                Err(SocketReason::Timeout)
            }
    }
}

pub(crate) fn reason(error: io::Error) -> SocketReason {
    io_reason(error.kind()).0
}

pub(crate) fn native_reason(error: io::Error) -> &'static str {
    io_reason(error.kind()).1
}

// Typed source results and native mailbox records share one IO classification.
fn io_reason(kind: io::ErrorKind) -> (SocketReason, &'static str) {
    use io::ErrorKind;
    match kind {
        ErrorKind::TimedOut => (SocketReason::Timeout, "timeout"),
        ErrorKind::BrokenPipe | ErrorKind::UnexpectedEof | ErrorKind::NotConnected => {
            (SocketReason::Closed, "closed")
        }
        ErrorKind::ConnectionAborted => (SocketReason::Econnaborted, "econnaborted"),
        ErrorKind::ConnectionRefused => (SocketReason::Econnrefused, "econnrefused"),
        ErrorKind::ConnectionReset => (SocketReason::Econnreset, "econnreset"),
        ErrorKind::AddrInUse => (SocketReason::Eaddrinuse, "eaddrinuse"),
        ErrorKind::AddrNotAvailable => (SocketReason::Eaddrnotavail, "eaddrnotavail"),
        ErrorKind::PermissionDenied => (SocketReason::Eacces, "eacces"),
        ErrorKind::InvalidInput | ErrorKind::InvalidData => (SocketReason::Badarg, "badarg"),
        ErrorKind::WouldBlock => (SocketReason::Eagain, "eagain"),
        ErrorKind::Interrupted => (SocketReason::Eintr, "eintr"),
        ErrorKind::NotFound => (SocketReason::Enoent, "enoent"),
        _ => (SocketReason::Eio, "eio"),
    }
}

#[cfg(test)]
mod tests {
    use super::{native_reason, reason};
    use crate::SocketReason;
    use std::io::{Error, ErrorKind};
    use std::sync::Arc;

    fn transfer_stage<'call>(
        mut call: crate::Call<'call, crate::test_support::Profile, ()>,
        socket: geam::host::HostExternal<'call, crate::schema::Socket>,
        pid: geam::host::HostExternal<'call, geam::gleam_erlang::Pid>,
        mode: geam::provider::BigInt,
    ) -> Result<geam::host::HostCallCompletion<'call, ()>, geam::host::HostCallError> {
        let key = call.external_payload(socket).key.clone();
        let owned = call.service::<crate::Component>().connection(&key).unwrap();
        let caller = call.require_execution_unit().unwrap();
        let target = geam::gleam_erlang::service::Processes::new(&mut call).pid(pid);
        let mode = u32::try_from(mode).unwrap();
        if mode == 1 {
            owned.close();
            assert_eq!(
                super::Transfer::begin(Arc::clone(&owned), caller.id(), true).err(),
                Some("closed")
            );
        } else {
            let mut transfer =
                super::Transfer::begin(Arc::clone(&owned), caller.id(), true).unwrap();
            match mode {
                0 => assert_eq!(
                    super::Transfer::begin(Arc::clone(&owned), caller.id(), true).err(),
                    Some("einval")
                ),
                2 => {
                    owned.close();
                    assert_eq!(transfer.commit(target, true), Err("closed"));
                }
                3 => {
                    geam::gleam_erlang::service::Processes::new(&mut call).kill(&target);
                    let alive =
                        geam::gleam_erlang::service::Processes::new(&mut call).is_alive(&target);
                    assert!(!alive);
                    assert_eq!(transfer.commit(target, alive), Err("badarg"));
                }
                4 => assert_eq!(transfer.commit(target, true), Ok(())),
                _ => {}
            }
            drop(transfer);
            assert!(!owned.control.lock().transferring);
        }
        Ok(call.return_value(()))
    }

    #[test]
    fn transfer_phases_reject_closed_or_busy_resources_dead_targets_and_restore_on_cancel() {
        use crate::{
            Component,
            schema::Socket,
            test_support::{
                execution_fixture::TestHost,
                network::{ScriptedConnection, ScriptedNetwork},
                source_project_with,
            },
        };
        use geam::host::HostProviderModule;
        use std::sync::Arc;
        for mode in 0..=5 {
            let provider = HostProviderModule::new("fixture", "fixture").unwrap()
                .with_scoped_function::<Component, (Socket, geam::gleam_erlang::Pid, geam::provider::BigInt), (), _>("stage", transfer_stage).unwrap();
            let source = format!(
                r#"
import gleam/erlang/process
import glisten/socket
import glisten/tcp
@external(erlang, "fixture", "stage") fn stage(socket: socket.Socket, target: process.Pid, mode: Int) -> Nil
pub fn main() {{
  let assert Ok(listener) = tcp.listen(0, [])
  let assert Ok(connection) = tcp.accept(listener)
  let target = process.spawn_unlinked(fn() {{ process.sleep_forever() }})
  stage(connection, target, {mode})
  let _ = tcp.close(connection)
  let _ = tcp.close(listener)
  process.kill(target)
  Nil
}}
"#
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

    #[test]
    fn typed_results_and_active_records_classify_the_same_io_failures() {
        for (kind, expected, symbol) in [
            (ErrorKind::TimedOut, SocketReason::Timeout, "timeout"),
            (ErrorKind::BrokenPipe, SocketReason::Closed, "closed"),
            (ErrorKind::UnexpectedEof, SocketReason::Closed, "closed"),
            (ErrorKind::NotConnected, SocketReason::Closed, "closed"),
            (
                ErrorKind::ConnectionAborted,
                SocketReason::Econnaborted,
                "econnaborted",
            ),
            (
                ErrorKind::ConnectionRefused,
                SocketReason::Econnrefused,
                "econnrefused",
            ),
            (
                ErrorKind::ConnectionReset,
                SocketReason::Econnreset,
                "econnreset",
            ),
            (ErrorKind::AddrInUse, SocketReason::Eaddrinuse, "eaddrinuse"),
            (
                ErrorKind::AddrNotAvailable,
                SocketReason::Eaddrnotavail,
                "eaddrnotavail",
            ),
            (ErrorKind::PermissionDenied, SocketReason::Eacces, "eacces"),
            (ErrorKind::InvalidInput, SocketReason::Badarg, "badarg"),
            (ErrorKind::InvalidData, SocketReason::Badarg, "badarg"),
            (ErrorKind::WouldBlock, SocketReason::Eagain, "eagain"),
            (ErrorKind::Interrupted, SocketReason::Eintr, "eintr"),
            (ErrorKind::NotFound, SocketReason::Enoent, "enoent"),
            (ErrorKind::Other, SocketReason::Eio, "eio"),
        ] {
            assert_eq!(reason(Error::from(kind)), expected);
            assert_eq!(native_reason(Error::from(kind)), symbol);
        }
    }
}
