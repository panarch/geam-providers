//! Explicit socket IO capabilities. No operation selects an ambient runtime.

use futures_util::TryFutureExt;
use parking_lot::Mutex;
use rustls::ServerConfig;
use socket2::{Domain, Protocol, SockRef, Type};
use std::future::Future;
use std::io;
use std::net::{IpAddr, Shutdown, SocketAddr, TcpStream as ControlSocket};
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadHalf, WriteHalf};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Notify, oneshot};
use tokio_rustls::TlsAcceptor;
use tokio_rustls::server::TlsStream;

/// A socket operation that borrows only its host-owned IO capability.
pub type IoFuture<'a, T> = Pin<Box<dyn Future<Output = io::Result<T>> + Send + 'a>>;

/// Options supplied explicitly when the source creates a listener.
#[derive(Clone, Debug)]
pub struct ListenOptions {
    pub address: SocketAddr,
    pub backlog: i32,
    pub reuse_address: bool,
    pub connection: ConnectionOptions,
    pub tls: Option<TlsOptions>,
}

/// TCP options shared by accepted connections and source `set_opts` calls.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectionOptions {
    pub reuse_address: bool,
    pub nodelay: bool,
    pub linger: Option<Duration>,
    pub buffer: usize,
}

/// Server credentials and ALPN supplied by the source's TLS options.
#[derive(Clone, Debug)]
pub struct TlsOptions {
    pub certificate_file: String,
    pub private_key_file: String,
    pub alpn: Vec<Vec<u8>>,
}

/// The networking capability selected by the embedding caller or component config.
pub trait Network: Send + Sync {
    fn listen(&self, options: ListenOptions) -> io::Result<Arc<dyn Listener>>;
    fn sleep(&self, duration: Duration) -> Pin<Box<dyn Future<Output = ()> + Send>>;
}

/// A listener belongs to the provider execution domain, independently of aliases.
pub trait Listener: Send + Sync {
    fn accept(&self) -> IoFuture<'_, Arc<dyn Connection>>;
    fn address(&self) -> io::Result<SocketAddr>;
    fn close(&self);
}

/// Full-duplex IO and socket controls used by the provider's connection owner.
pub trait Connection: Send + Sync {
    fn read(&self, limit: usize) -> IoFuture<'_, Vec<u8>>;
    fn write<'a>(&'a self, bytes: &'a [u8]) -> IoFuture<'a, ()>;
    fn handshake(&self) -> IoFuture<'_, ()>;
    fn peer_address(&self) -> io::Result<SocketAddr>;
    fn local_address(&self) -> io::Result<SocketAddr>;
    fn configure(&self, options: &ConnectionOptions) -> io::Result<()>;
    fn receive_buffer(&self) -> io::Result<usize>;
    fn negotiated_protocol(&self) -> Option<Vec<u8>>;
    fn shutdown_write(&self) -> IoFuture<'_, ()>;
    fn close(&self);
}

/// Owns one reactor thread, created before source execution. Socket operations
/// register with this reactor even when polled by a different execution host.
pub struct TokioNetwork {
    reactor: Arc<Reactor>,
}
impl TokioNetwork {
    pub fn new() -> io::Result<Self> {
        Self::start(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_io()
                .enable_time()
                .build()
        })
    }
    // Runtime construction belongs to the worker; its result precedes source
    // execution. Dropping the stop sender also stops an incomplete capability.
    fn start(create_runtime: fn() -> io::Result<tokio::runtime::Runtime>) -> io::Result<Self> {
        let (ready, receive_ready) = std::sync::mpsc::sync_channel(1);
        let (stop, stopped) = oneshot::channel();
        std::thread::Builder::new()
            .name("geam-glisten-io".into())
            .spawn(move || match create_runtime() {
                Ok(runtime) => {
                    let _ = ready.send(Ok(runtime.handle().clone()));
                    runtime.block_on(async {
                        let _ = stopped.await;
                    });
                }
                Err(error) => {
                    let _ = ready.send(Err(error));
                }
            })
            .and_then(|worker| {
                receive_ready
                    .recv()
                    .map_err(io::Error::other)
                    .and_then(std::convert::identity)
                    .map(|handle| Self {
                        reactor: Arc::new(Reactor {
                            handle,
                            stop: Some(stop),
                            worker: Some(worker),
                        }),
                    })
            })
    }
}
impl Network for TokioNetwork {
    fn sleep(&self, duration: Duration) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        let _entered = self.reactor.handle.enter();
        Box::pin(tokio::time::sleep(duration))
    }
    fn listen(&self, options: ListenOptions) -> io::Result<Arc<dyn Listener>> {
        self.bind(options)
            .map(|listener| listener as Arc<dyn Listener>)
    }
}

impl TokioNetwork {
    fn bind(&self, options: ListenOptions) -> io::Result<Arc<TokioListener>> {
        options
            .tls
            .as_ref()
            .map(server_config)
            .transpose()
            .and_then(|tls| {
                socket2::Socket::new(
                    if options.address.is_ipv6() {
                        Domain::IPV6
                    } else {
                        Domain::IPV4
                    },
                    Type::STREAM,
                    Some(Protocol::TCP),
                )
                .and_then(|socket| {
                    socket
                        .set_reuse_address(options.reuse_address)
                        .and_then(|()| socket.set_nonblocking(true))
                        .and_then(|()| socket.bind(&options.address.into()))
                        .and_then(|()| socket.listen(options.backlog))
                        .and_then(|()| {
                            let _entered = self.reactor.handle.enter();
                            TcpListener::from_std(socket.into())
                        })
                })
                .map(|listener| {
                    Arc::new(TokioListener {
                        listener: Mutex::new(Some(Arc::new(listener))),
                        reactor: Arc::clone(&self.reactor),
                        connection: options.connection,
                        tls,
                        closed: Arc::new(Closed::default()),
                    })
                })
            })
    }
}

struct Reactor {
    handle: tokio::runtime::Handle,
    stop: Option<oneshot::Sender<()>>,
    worker: Option<JoinHandle<()>>,
}

impl Drop for Reactor {
    fn drop(&mut self) {
        let _ = self.stop.take().map(|stop| stop.send(()));
        let _ = self.worker.take().map(JoinHandle::join);
    }
}

#[derive(Default)]
struct Closed {
    value: AtomicBool,
    changed: Notify,
}

impl Closed {
    fn close(&self) {
        self.value.store(true, Ordering::Release);
        self.changed.notify_waiters();
    }

    async fn wait(&self) {
        loop {
            let changed = self.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            if self.value.load(Ordering::Acquire) {
                return;
            }
            changed.await;
        }
    }
}

fn closed() -> io::Error {
    io::Error::new(io::ErrorKind::NotConnected, "glisten socket is closed")
}

struct TokioListener {
    listener: Mutex<Option<Arc<TcpListener>>>,
    reactor: Arc<Reactor>,
    connection: ConnectionOptions,
    tls: Option<Arc<ServerConfig>>,
    closed: Arc<Closed>,
}

impl Listener for TokioListener {
    fn accept(&self) -> IoFuture<'_, Arc<dyn Connection>> {
        Box::pin(async {
            self.accept_owned()
                .await
                .map(|connection| connection as Arc<dyn Connection>)
        })
    }
    fn address(&self) -> io::Result<SocketAddr> {
        self.listener
            .lock()
            .as_ref()
            .ok_or_else(closed)?
            .local_addr()
    }

    fn close(&self) {
        self.closed.close();
        self.listener.lock().take();
    }
}

impl Drop for TokioListener {
    fn drop(&mut self) {
        self.close();
    }
}

struct TokioConnection {
    control: ControlSocket,
    phase: Mutex<Phase>,
    closed: Closed,
    _reactor: Arc<Reactor>,
}

impl TokioListener {
    async fn accept_owned(&self) -> io::Result<Arc<TokioConnection>> {
        let listener = self.listener.lock().clone().ok_or_else(closed);
        let listener = match listener {
            Ok(listener) => listener,
            Err(error) => return Err(error),
        };
        let accepted = tokio::select! {
            biased;
            _ = self.closed.wait() => return Err(closed()),
            accepted = std::future::poll_fn(|context| {
                // Tokio registers an accepted stream during poll_accept.
                // Select the capability's reactor for that poll, without
                // keeping an enter guard alive across an await.
                let _entered = self.reactor.handle.enter();
                listener.poll_accept(context)
            }) => accepted,
        };
        accepted.and_then(|(stream, _)| {
            stream.into_std().and_then(|standard| {
                standard.try_clone().and_then(|control| {
                    let _entered = self.reactor.handle.enter();
                    TcpStream::from_std(standard).and_then(|stream| {
                        let phase = match &self.tls {
                            Some(config) => Phase::PendingTls {
                                stream,
                                config: Arc::clone(config),
                            },
                            None => {
                                Phase::Ready(Arc::new(IoHalves::new(Stream::Tcp(stream), None)))
                            }
                        };
                        let connection = Arc::new(TokioConnection {
                            control,
                            phase: Mutex::new(phase),
                            closed: Closed::default(),
                            _reactor: Arc::clone(&self.reactor),
                        });
                        connection.configure(&self.connection).map(|()| connection)
                    })
                })
            })
        })
    }
}

enum Phase {
    PendingTls {
        stream: TcpStream,
        config: Arc<ServerConfig>,
    },
    Ready(Arc<IoHalves>),
    Closed,
}

struct IoHalves {
    read: tokio::sync::Mutex<ReadHalf<Stream>>,
    write: tokio::sync::Mutex<WriteHalf<Stream>>,
    alpn: Option<Vec<u8>>,
}

impl IoHalves {
    fn new(stream: Stream, alpn: Option<Vec<u8>>) -> Self {
        let (read, write) = tokio::io::split(stream);
        Self {
            read: tokio::sync::Mutex::new(read),
            write: tokio::sync::Mutex::new(write),
            alpn,
        }
    }
}

impl TokioConnection {
    fn ready(&self) -> io::Result<Arc<IoHalves>> {
        match &*self.phase.lock() {
            Phase::Ready(halves) => Ok(Arc::clone(halves)),
            Phase::PendingTls { .. } => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "TLS handshake is required",
            )),
            Phase::Closed => Err(closed()),
        }
    }
}

impl TokioConnection {
    fn install_tls(&self, halves: Arc<IoHalves>) -> io::Result<()> {
        let mut phase = self.phase.lock();
        if self.closed.value.load(Ordering::Acquire) {
            return Err(closed());
        }
        *phase = Phase::Ready(halves);
        Ok(())
    }
}

impl Connection for TokioConnection {
    fn read(&self, limit: usize) -> IoFuture<'_, Vec<u8>> {
        Box::pin(async move {
            let halves = self.ready()?;
            tokio::select! {
                biased;
                _ = self.closed.wait() => Err(closed()),
                result = async {
                    let mut read = halves.read.lock().await;
                    let mut bytes = vec![0; limit];
                    read.read(&mut bytes).await.map(|length| { bytes.truncate(length); bytes })
                } => result,
            }
        })
    }

    fn write<'a>(&'a self, bytes: &'a [u8]) -> IoFuture<'a, ()> {
        Box::pin(async move {
            let halves = self.ready()?;
            tokio::select! {
                biased;
                _ = self.closed.wait() => Err(closed()),
                result = async {
                    let mut write = halves.write.lock().await;
                    write.write_all(bytes).await.map(|()| write)
                }.and_then(|mut write| async move { write.flush().await }) => result,
            }
        })
    }

    fn handshake(&self) -> IoFuture<'_, ()> {
        Box::pin(async move {
            let (stream, config) = {
                let mut phase = self.phase.lock();
                match std::mem::replace(&mut *phase, Phase::Closed) {
                    Phase::Ready(halves) => {
                        *phase = Phase::Ready(halves);
                        return Ok(());
                    }
                    Phase::Closed => return Err(closed()),
                    Phase::PendingTls { stream, config } => (stream, config),
                }
            };
            let mut guard = HandshakeGuard {
                connection: self,
                complete: false,
            };
            let handshake = TlsAcceptor::from(config).accept(stream);
            let stream = tokio::select! {
                biased;
                _ = self.closed.wait() => return Err(closed()),
                stream = handshake => stream?,
            };
            let alpn = stream.get_ref().1.alpn_protocol().map(<[u8]>::to_vec);
            self.install_tls(Arc::new(IoHalves::new(Stream::Tls(Box::new(stream)), alpn)))
                .map(|()| {
                    guard.complete = true;
                })
        })
    }

    fn peer_address(&self) -> io::Result<SocketAddr> {
        self.control.peer_addr()
    }
    fn local_address(&self) -> io::Result<SocketAddr> {
        self.control.local_addr()
    }

    fn configure(&self, options: &ConnectionOptions) -> io::Result<()> {
        let socket = SockRef::from(&self.control);
        socket
            .set_reuse_address(options.reuse_address)
            .and_then(|()| self.control.set_nodelay(options.nodelay))
            .and_then(|()| socket.set_linger(options.linger))
            .and_then(|()| socket.set_recv_buffer_size(options.buffer))
    }

    fn receive_buffer(&self) -> io::Result<usize> {
        SockRef::from(&self.control).recv_buffer_size()
    }

    fn negotiated_protocol(&self) -> Option<Vec<u8>> {
        match &*self.phase.lock() {
            Phase::Ready(halves) => halves.alpn.clone(),
            Phase::PendingTls { .. } | Phase::Closed => None,
        }
    }

    fn shutdown_write(&self) -> IoFuture<'_, ()> {
        Box::pin(async move {
            let halves = self.ready()?;
            tokio::select! {
                biased;
                _ = self.closed.wait() => Err(closed()),
                result = async { halves.write.lock().await.shutdown().await } => result,
            }
        })
    }

    fn close(&self) {
        self.closed.close();
        *self.phase.lock() = Phase::Closed;
        let _ = self.control.shutdown(Shutdown::Both);
    }
}

impl Drop for TokioConnection {
    fn drop(&mut self) {
        self.close();
    }
}

struct HandshakeGuard<'a> {
    connection: &'a TokioConnection,
    complete: bool,
}
impl Drop for HandshakeGuard<'_> {
    fn drop(&mut self) {
        if !self.complete {
            self.connection.close();
        }
    }
}

enum Stream {
    Tcp(TcpStream),
    Tls(Box<TlsStream<TcpStream>>),
}

impl AsyncRead for Stream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buffer: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        match self.get_mut() {
            Self::Tcp(stream) => Pin::new(stream).poll_read(cx, buffer),
            Self::Tls(stream) => Pin::new(stream.as_mut()).poll_read(cx, buffer),
        }
    }
}

impl AsyncWrite for Stream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        bytes: &[u8],
    ) -> std::task::Poll<io::Result<usize>> {
        match self.get_mut() {
            Self::Tcp(stream) => Pin::new(stream).poll_write(cx, bytes),
            Self::Tls(stream) => Pin::new(stream.as_mut()).poll_write(cx, bytes),
        }
    }
    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        match self.get_mut() {
            Self::Tcp(stream) => Pin::new(stream).poll_flush(cx),
            Self::Tls(stream) => Pin::new(stream.as_mut()).poll_flush(cx),
        }
    }
    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        match self.get_mut() {
            Self::Tcp(stream) => Pin::new(stream).poll_shutdown(cx),
            Self::Tls(stream) => Pin::new(stream.as_mut()).poll_shutdown(cx),
        }
    }
}

fn server_config(options: &TlsOptions) -> io::Result<Arc<ServerConfig>> {
    let mut certificates = std::io::BufReader::new(std::fs::File::open(&options.certificate_file)?);
    let certificates = rustls_pemfile::certs(&mut certificates).collect::<io::Result<Vec<_>>>()?;
    let mut key = std::io::BufReader::new(std::fs::File::open(&options.private_key_file)?);
    let key = rustls_pemfile::private_key(&mut key)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "TLS private key is missing"))?;
    let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
    ServerConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13, &rustls::version::TLS12])
        .map_err(io::Error::other)
        .and_then(|builder| {
            builder
                .with_no_client_auth()
                .with_single_cert(certificates, key)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))
        })
        .map(|mut config| {
            config.alpn_protocols = options.alpn.clone();
            Arc::new(config)
        })
}

pub(crate) fn ip_address(address: IpAddr) -> Vec<u16> {
    match address {
        IpAddr::V4(address) => address.octets().map(u16::from).to_vec(),
        IpAddr::V6(address) => address.segments().to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::{Connection as _, Listener as _};
    use super::{ConnectionOptions, ListenOptions, Network, TlsOptions, TokioNetwork};
    use std::io::{Read, Write};
    use std::net::{Ipv4Addr, SocketAddr, TcpStream};
    use std::sync::Arc;
    use std::task::Poll;
    use std::time::Duration;

    fn options(tls: bool) -> ListenOptions {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/certs");
        ListenOptions {
            address: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
            backlog: 8,
            reuse_address: false,
            connection: ConnectionOptions {
                reuse_address: true,
                nodelay: true,
                linger: None,
                buffer: 4096,
            },
            tls: tls.then(|| TlsOptions {
                certificate_file: root.join("cert.pem").to_string_lossy().into_owned(),
                private_key_file: root.join("key.pem").to_string_lossy().into_owned(),
                alpn: vec![b"h2".to_vec(), b"http/1.1".to_vec()],
            }),
        }
    }

    fn caller() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap()
    }

    #[test]
    fn reactor_initialization_reports_worker_dependency_failure_before_use() {
        let error = TokioNetwork::start(|| {
            Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "reactor unavailable",
            ))
        })
        .err()
        .unwrap();
        assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
        assert_eq!(error.to_string(), "reactor unavailable");
    }

    #[test]
    fn ipv6_listener_and_unassigned_interface_use_the_same_explicit_capability() {
        let network = TokioNetwork::new().unwrap();
        let mut selected = options(false);
        selected.address = "[::1]:0".parse().unwrap();
        let listener = network.listen(selected).unwrap();
        let mut client = TcpStream::connect(listener.address().unwrap()).unwrap();
        let caller = caller();
        let accepted = caller.block_on(listener.accept()).unwrap();
        assert!(accepted.local_address().unwrap().is_ipv6());
        client.write_all(b"v6").unwrap();
        assert_eq!(caller.block_on(accepted.read(2)).unwrap(), b"v6");
        accepted.close();
        listener.close();
        let mut selected = options(false);
        selected.address = "192.0.2.1:0".parse().unwrap();
        assert_eq!(
            network.listen(selected).err().unwrap().kind(),
            std::io::ErrorKind::AddrNotAvailable
        );
    }

    #[test]
    fn closing_a_connection_interrupts_already_pending_read_write_and_shutdown() {
        let network = TokioNetwork::new().unwrap();
        let listener = network.listen(options(false)).unwrap();
        let _client = TcpStream::connect(listener.address().unwrap()).unwrap();
        let caller = caller();
        let connection = caller.block_on(listener.accept()).unwrap();
        let mut read = connection.read(1);
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        assert!(read.as_mut().poll(&mut context).is_pending());
        connection.close();
        assert_eq!(
            caller.block_on(read).unwrap_err().kind(),
            std::io::ErrorKind::NotConnected
        );
        listener.close();

        let listener = network.listen(options(false)).unwrap();
        let _client = TcpStream::connect(listener.address().unwrap()).unwrap();
        let connection = caller.block_on(listener.accept()).unwrap();
        let bytes = vec![42; 16 * 1024 * 1024];
        let mut write = connection.write(&bytes);
        assert!(write.as_mut().poll(&mut context).is_pending());
        let mut shutdown = connection.shutdown_write();
        assert!(shutdown.as_mut().poll(&mut context).is_pending());
        connection.close();
        assert_eq!(
            caller.block_on(write).unwrap_err().kind(),
            std::io::ErrorKind::NotConnected
        );
        assert_eq!(
            caller.block_on(shutdown).unwrap_err().kind(),
            std::io::ErrorKind::NotConnected
        );
        listener.close();
    }

    #[test]
    fn explicit_close_interrupts_an_already_pending_tls_handshake() {
        let network = TokioNetwork::new().unwrap();
        let listener = network.listen(options(true)).unwrap();
        let _client = TcpStream::connect(listener.address().unwrap()).unwrap();
        let caller = caller();
        let connection = caller.block_on(listener.accept()).unwrap();
        let mut handshake = connection.handshake();
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        assert!(handshake.as_mut().poll(&mut context).is_pending());
        connection.close();
        assert_eq!(
            caller.block_on(handshake).unwrap_err().kind(),
            std::io::ErrorKind::NotConnected
        );
        listener.close();
    }

    #[test]
    fn accepted_connections_use_the_capability_reactor_without_caller_io() {
        let network = TokioNetwork::new().unwrap();
        let listener = network
            .bind(ListenOptions {
                address: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
                backlog: 8,
                reuse_address: false,
                connection: ConnectionOptions {
                    reuse_address: true,
                    nodelay: true,
                    linger: None,
                    buffer: 4096,
                },
                tls: None,
            })
            .unwrap();
        // Only the explicitly selected Network owns an IO-enabled runtime.
        let caller = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap();
        let mut client = TcpStream::connect(listener.address().unwrap()).unwrap();
        let connection = caller.block_on(listener.accept_owned()).unwrap();
        assert!(
            socket2::SockRef::from(&connection.control)
                .reuse_address()
                .unwrap()
        );
        connection
            .configure(&ConnectionOptions {
                reuse_address: false,
                nodelay: false,
                linger: None,
                buffer: 4096,
            })
            .unwrap();
        assert!(
            !socket2::SockRef::from(&connection.control)
                .reuse_address()
                .unwrap()
        );
        assert!(!connection.control.nodelay().unwrap());
        assert_eq!(
            connection.local_address().unwrap(),
            listener.address().unwrap()
        );
        assert_eq!(
            connection.peer_address().unwrap(),
            client.local_addr().unwrap()
        );
        assert!(connection.receive_buffer().unwrap() > 0);
        assert!(connection.negotiated_protocol().is_none());
        caller.block_on(connection.handshake()).unwrap();
        client.write_all(b"ping").unwrap();
        assert_eq!(caller.block_on(connection.read(4)).unwrap(), b"ping");
        caller.block_on(connection.write(b"pong")).unwrap();
        let mut reply = [0; 4];
        client.read_exact(&mut reply).unwrap();
        assert_eq!(&reply, b"pong");
        caller.block_on(connection.shutdown_write()).unwrap();
        assert_eq!(client.read(&mut reply).unwrap(), 0);
        connection.close();
        assert_eq!(
            caller.block_on(connection.read(1)).unwrap_err().kind(),
            std::io::ErrorKind::NotConnected
        );
        assert_eq!(
            caller
                .block_on(connection.write(b"closed"))
                .unwrap_err()
                .kind(),
            std::io::ErrorKind::NotConnected
        );
        assert_eq!(
            caller.block_on(connection.handshake()).unwrap_err().kind(),
            std::io::ErrorKind::NotConnected
        );
        assert_eq!(
            caller
                .block_on(connection.shutdown_write())
                .unwrap_err()
                .kind(),
            std::io::ErrorKind::NotConnected
        );
        listener.close();
        assert_eq!(
            listener.address().unwrap_err().kind(),
            std::io::ErrorKind::NotConnected
        );
        assert_eq!(
            caller.block_on(listener.accept()).err().unwrap().kind(),
            std::io::ErrorKind::NotConnected
        );
    }

    #[test]
    fn tls_handshake_and_duplex_io_use_only_the_selected_reactor() {
        let network = TokioNetwork::new().unwrap();
        let listener = network.bind(options(true)).unwrap();
        let address = listener.address().unwrap();
        let peer = std::thread::spawn(move || {
            let certificate =
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/certs/cert.pem");
            let mut file = std::io::BufReader::new(std::fs::File::open(certificate).unwrap());
            let mut roots = rustls::RootCertStore::empty();
            for certificate in rustls_pemfile::certs(&mut file) {
                roots.add(certificate.unwrap()).unwrap();
            }
            let mut config = rustls::ClientConfig::builder_with_provider(Arc::new(
                rustls::crypto::aws_lc_rs::default_provider(),
            ))
            .with_protocol_versions(&[&rustls::version::TLS13, &rustls::version::TLS12])
            .unwrap()
            .with_root_certificates(roots)
            .with_no_client_auth();
            config.alpn_protocols = vec![b"http/1.1".to_vec(), b"h2".to_vec()];
            let socket = TcpStream::connect(address).unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            socket
                .set_write_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut tls = rustls::StreamOwned::new(
                rustls::ClientConnection::new(
                    Arc::new(config),
                    rustls::pki_types::ServerName::try_from("localhost").unwrap(),
                )
                .unwrap(),
                socket,
            );
            tls.write_all(b"ping").unwrap();
            tls.flush().unwrap();
            let mut reply = [0; 4];
            tls.read_exact(&mut reply).unwrap();
            assert_eq!(reply, *b"pong");
            assert_eq!(tls.conn.alpn_protocol(), Some(b"h2".as_slice()));
            tls.conn.send_close_notify();
            tls.flush().unwrap();
        });
        let caller = caller();
        let connection = caller.block_on(listener.accept_owned()).unwrap();
        assert!(connection.negotiated_protocol().is_none());
        assert_eq!(
            caller.block_on(connection.read(4)).unwrap_err().kind(),
            std::io::ErrorKind::InvalidInput
        );
        assert_eq!(
            caller
                .block_on(connection.write(b"early"))
                .unwrap_err()
                .kind(),
            std::io::ErrorKind::InvalidInput
        );
        assert_eq!(
            caller
                .block_on(connection.shutdown_write())
                .unwrap_err()
                .kind(),
            std::io::ErrorKind::InvalidInput
        );
        caller.block_on(connection.handshake()).unwrap();
        caller.block_on(connection.handshake()).unwrap();
        assert_eq!(connection.negotiated_protocol(), Some(b"h2".to_vec()));
        assert_eq!(caller.block_on(connection.read(4)).unwrap(), b"ping");
        caller.block_on(connection.write(b"pong")).unwrap();
        peer.join().unwrap();
        assert_eq!(caller.block_on(connection.read(1)).unwrap(), b"");
        caller.block_on(connection.shutdown_write()).unwrap();
        let completed_tls = connection.ready().unwrap();
        connection.close();
        assert_eq!(
            connection.install_tls(completed_tls).unwrap_err().kind(),
            std::io::ErrorKind::NotConnected
        );
        assert!(connection.negotiated_protocol().is_none());
        listener.close();
    }

    #[test]
    fn cancelling_or_failing_tls_handshake_closes_the_connection() {
        for cancel in [true, false] {
            let network = TokioNetwork::new().unwrap();
            let listener = network.listen(options(true)).unwrap();
            let mut client = TcpStream::connect(listener.address().unwrap()).unwrap();
            client
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let caller = caller();
            let connection = caller.block_on(listener.accept()).unwrap();
            if cancel {
                let mut handshake = connection.handshake();
                caller.block_on(std::future::poll_fn(|context| {
                    assert!(handshake.as_mut().poll(context).is_pending());
                    Poll::Ready(())
                }));
                drop(handshake);
                assert_eq!(client.read(&mut [0; 1]).unwrap(), 0);
            } else {
                client.write_all(b"this is not a TLS client hello").unwrap();
                assert!(caller.block_on(connection.handshake()).is_err());
            }
            assert_eq!(
                caller.block_on(connection.read(1)).unwrap_err().kind(),
                std::io::ErrorKind::NotConnected
            );
            assert!(connection.negotiated_protocol().is_none());
            listener.close();
        }
    }

    #[test]
    fn listener_close_cancels_pending_accept_and_capability_timer_runs_without_ambient_io() {
        let network = TokioNetwork::new().unwrap();
        let listener = network.listen(options(false)).unwrap();
        let caller = caller();
        let mut accept = listener.accept();
        caller.block_on(std::future::poll_fn(|context| {
            assert!(accept.as_mut().poll(context).is_pending());
            Poll::Ready(())
        }));
        listener.close();
        assert_eq!(
            caller.block_on(accept).err().unwrap().kind(),
            std::io::ErrorKind::NotConnected
        );
        let sleep = network.sleep(Duration::ZERO);
        caller.block_on(sleep);
    }

    #[test]
    fn invalid_credentials_and_conflicting_binds_return_io_errors() {
        let network = TokioNetwork::new().unwrap();
        let first = network.listen(options(false)).unwrap();
        let mut conflicting = options(false);
        conflicting.address = first.address().unwrap();
        assert_eq!(
            network.listen(conflicting).err().unwrap().kind(),
            std::io::ErrorKind::AddrInUse
        );
        first.close();
        let mut missing = options(true);
        missing.tls.as_mut().unwrap().certificate_file += ".missing";
        assert_eq!(
            network.listen(missing).err().unwrap().kind(),
            std::io::ErrorKind::NotFound
        );
        let mut missing_key = options(true);
        missing_key.tls.as_mut().unwrap().private_key_file += ".missing";
        assert_eq!(
            network.listen(missing_key).err().unwrap().kind(),
            std::io::ErrorKind::NotFound
        );
        let mut invalid_key = options(true);
        let tls = invalid_key.tls.as_mut().unwrap();
        tls.private_key_file = tls.certificate_file.clone();
        assert_eq!(
            network.listen(invalid_key).err().unwrap().kind(),
            std::io::ErrorKind::InvalidInput
        );
    }

    #[test]
    fn malformed_pem_and_empty_certificates_keep_the_input_failure_boundary() {
        let root = std::env::temp_dir().join(format!("geam-glisten-pem-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let invalid = root.join("invalid.pem");
        let empty = root.join("empty.pem");
        std::fs::write(&empty, b"").unwrap();
        let network = TokioNetwork::new().unwrap();
        for (kind, label) in [
            ("CERTIFICATE", "certificate"),
            ("PRIVATE KEY", "private key"),
        ] {
            std::fs::write(
                &invalid,
                format!("-----BEGIN {kind}-----\n!\n-----END {kind}-----\n"),
            )
            .unwrap();
            let mut selected = options(true);
            if kind == "CERTIFICATE" {
                selected.tls.as_mut().unwrap().certificate_file =
                    invalid.to_string_lossy().into_owned();
            } else {
                selected.tls.as_mut().unwrap().private_key_file =
                    invalid.to_string_lossy().into_owned();
            }
            assert!(network.listen(selected).is_err(), "{label}");
        }
        let mut selected = options(true);
        selected.tls.as_mut().unwrap().certificate_file = empty.to_string_lossy().into_owned();
        assert_eq!(
            network.listen(selected).err().unwrap().kind(),
            std::io::ErrorKind::InvalidInput
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn peer_reset_is_returned_by_native_read_and_write_without_reclassifying_it() {
        let network = TokioNetwork::new().unwrap();
        let listener = network.listen(options(false)).unwrap();
        let client = TcpStream::connect(listener.address().unwrap()).unwrap();
        let caller = caller();
        let connection = caller.block_on(listener.accept()).unwrap();
        socket2::SockRef::from(&client)
            .set_linger(Some(Duration::ZERO))
            .unwrap();
        drop(client);
        assert!(caller.block_on(connection.read(1)).is_err());
        assert!(caller.block_on(connection.write(b"after reset")).is_err());
        connection.close();
        listener.close();
    }
}
