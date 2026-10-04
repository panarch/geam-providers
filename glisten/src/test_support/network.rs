//! Scripted host networking for deterministic native boundary and lifecycle tests.
use crate::network::{Connection, ConnectionOptions, IoFuture, ListenOptions, Listener, Network};
use parking_lot::Mutex;
use std::collections::VecDeque;
use std::future::Future;
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::Notify;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Event {
    Listen(SocketAddr),
    Accept,
    Read(usize),
    Write(Vec<u8>),
    Configure(ConnectionOptions),
    Handshake,
    ShutdownWrite,
    CloseConnection,
    CloseListener,
}

pub(crate) struct ScriptedNetwork {
    pub(crate) events: Arc<Mutex<Vec<Event>>>,
    pub(crate) listener: Arc<ScriptedListener>,
    pub(crate) listen_error: Option<io::ErrorKind>,
    pub(crate) timer: Arc<Notify>,
    pub(crate) listen_options: Mutex<Vec<ListenOptions>>,
}

impl ScriptedNetwork {
    pub(crate) fn new(address: SocketAddr, connections: Vec<Arc<ScriptedConnection>>) -> Self {
        let events = Arc::new(Mutex::new(Vec::new()));
        for connection in &connections {
            *connection.events.lock() = Arc::clone(&events);
        }
        Self {
            events: Arc::clone(&events),
            listener: Arc::new(ScriptedListener {
                address,
                connections: Mutex::new(connections.into()),
                events,
                closed: AtomicBool::new(false),
                changed: Notify::new(),
            }),
            listen_error: None,
            timer: Arc::new(Notify::new()),
            listen_options: Mutex::new(Vec::new()),
        }
    }

    /// Make a client arrive only after the source server's startup handshake.
    pub(crate) fn offer(&self, connection: Arc<ScriptedConnection>) {
        *connection.events.lock() = Arc::clone(&self.events);
        self.listener.connections.lock().push_back(connection);
        self.listener.changed.notify_waiters();
    }
}

impl Network for ScriptedNetwork {
    fn listen(&self, options: ListenOptions) -> io::Result<Arc<dyn Listener>> {
        self.events.lock().push(Event::Listen(options.address));
        self.listen_options.lock().push(options);
        match self.listen_error {
            Some(kind) => Err(io::Error::from(kind)),
            None => Ok(self.listener.clone()),
        }
    }

    fn sleep(&self, _: Duration) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        let timer = Arc::clone(&self.timer);
        Box::pin(async move { timer.notified().await })
    }
}

pub(crate) struct ScriptedListener {
    address: SocketAddr,
    connections: Mutex<VecDeque<Arc<ScriptedConnection>>>,
    events: Arc<Mutex<Vec<Event>>>,
    closed: AtomicBool,
    changed: Notify,
}

impl Listener for ScriptedListener {
    fn accept(&self) -> IoFuture<'_, Arc<dyn Connection>> {
        Box::pin(async move {
            self.events.lock().push(Event::Accept);
            loop {
                let changed = self.changed.notified();
                if self.closed.load(Ordering::Acquire) {
                    return Err(io::ErrorKind::NotConnected.into());
                }
                if let Some(connection) = self.connections.lock().pop_front() {
                    return Ok(connection as Arc<dyn Connection>);
                }
                changed.await;
            }
        })
    }
    fn address(&self) -> io::Result<SocketAddr> {
        if self.closed.load(Ordering::Acquire) {
            Err(io::ErrorKind::NotConnected.into())
        } else {
            Ok(self.address)
        }
    }
    fn close(&self) {
        if !self.closed.swap(true, Ordering::AcqRel) {
            self.events.lock().push(Event::CloseListener);
            self.changed.notify_waiters();
        }
    }
}

pub(crate) struct ScriptedConnection {
    pub(crate) local: SocketAddr,
    pub(crate) peer: SocketAddr,
    pub(crate) incoming: Mutex<VecDeque<io::Result<Vec<u8>>>>,
    pub(crate) write_error: Option<io::ErrorKind>,
    pub(crate) write_pending: AtomicBool,
    pub(crate) handshake_error: Option<io::ErrorKind>,
    pub(crate) handshake_pending: AtomicBool,
    pub(crate) protocol: Option<Vec<u8>>,
    pub(crate) configure_error: Option<io::ErrorKind>,
    pub(crate) peer_error: Option<io::ErrorKind>,
    pub(crate) local_error: Option<io::ErrorKind>,
    pub(crate) buffer_error: Option<io::ErrorKind>,
    pub(crate) shutdown_error: Option<io::ErrorKind>,
    pub(crate) events: Mutex<Arc<Mutex<Vec<Event>>>>,
    pub(crate) changed: Notify,
    closed: AtomicBool,
    handshaken: AtomicBool,
}

impl ScriptedConnection {
    pub(crate) fn new(
        local: SocketAddr,
        peer: SocketAddr,
        incoming: Vec<io::Result<Vec<u8>>>,
    ) -> Self {
        Self {
            local,
            peer,
            incoming: Mutex::new(incoming.into()),
            write_error: None,
            write_pending: AtomicBool::new(false),
            handshake_error: None,
            handshake_pending: AtomicBool::new(false),
            protocol: None,
            configure_error: None,
            peer_error: None,
            local_error: None,
            buffer_error: None,
            shutdown_error: None,
            events: Mutex::new(Arc::default()),
            changed: Notify::new(),
            closed: AtomicBool::new(false),
            handshaken: AtomicBool::new(false),
        }
    }

    fn record(&self, event: Event) {
        self.events.lock().lock().push(event);
    }
}

impl Connection for ScriptedConnection {
    fn read(&self, limit: usize) -> IoFuture<'_, Vec<u8>> {
        Box::pin(async move {
            self.record(Event::Read(limit));
            loop {
                let changed = self.changed.notified();
                if self.closed.load(Ordering::Acquire) {
                    return Err(io::ErrorKind::NotConnected.into());
                }
                let value = self.incoming.lock().pop_front();
                if let Some(value) = value {
                    let mut bytes = value?;
                    if bytes.len() > limit {
                        let rest = bytes.split_off(limit);
                        self.incoming.lock().push_front(Ok(rest));
                    }
                    return Ok(bytes);
                }
                changed.await;
            }
        })
    }
    fn write<'a>(&'a self, bytes: &'a [u8]) -> IoFuture<'a, ()> {
        Box::pin(async move {
            self.record(Event::Write(bytes.to_vec()));
            loop {
                let changed = self.changed.notified();
                if self.closed.load(Ordering::Acquire) {
                    return Err(io::ErrorKind::NotConnected.into());
                }
                if !self.write_pending.load(Ordering::Acquire) {
                    break;
                }
                changed.await;
            }
            match self.write_error {
                Some(kind) => Err(kind.into()),
                None => Ok(()),
            }
        })
    }
    fn handshake(&self) -> IoFuture<'_, ()> {
        Box::pin(async move {
            self.record(Event::Handshake);
            loop {
                let changed = self.changed.notified();
                if self.closed.load(Ordering::Acquire) {
                    return Err(io::ErrorKind::NotConnected.into());
                }
                if !self.handshake_pending.load(Ordering::Acquire) {
                    break;
                }
                changed.await;
            }
            match self.handshake_error {
                Some(kind) => Err(kind.into()),
                None => {
                    self.handshaken.store(true, Ordering::Release);
                    Ok(())
                }
            }
        })
    }
    fn peer_address(&self) -> io::Result<SocketAddr> {
        self.peer_error
            .map_or(Ok(self.peer), |error| Err(error.into()))
    }
    fn local_address(&self) -> io::Result<SocketAddr> {
        self.local_error
            .map_or(Ok(self.local), |error| Err(error.into()))
    }
    fn configure(&self, options: &ConnectionOptions) -> io::Result<()> {
        self.record(Event::Configure(options.clone()));
        self.configure_error
            .map_or(Ok(()), |error| Err(error.into()))
    }
    fn receive_buffer(&self) -> io::Result<usize> {
        self.buffer_error
            .map_or(Ok(8192), |error| Err(error.into()))
    }
    fn negotiated_protocol(&self) -> Option<Vec<u8>> {
        if self.handshaken.load(Ordering::Acquire) {
            self.protocol.clone()
        } else {
            None
        }
    }
    fn shutdown_write(&self) -> IoFuture<'_, ()> {
        Box::pin(async move {
            self.record(Event::ShutdownWrite);
            self.shutdown_error
                .map_or(Ok(()), |error| Err(error.into()))
        })
    }
    fn close(&self) {
        if !self.closed.swap(true, Ordering::AcqRel) {
            self.record(Event::CloseConnection);
            self.changed.notify_waiters();
        }
    }
}
