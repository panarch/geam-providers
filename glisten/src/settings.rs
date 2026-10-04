//! Native socket options after the original Gleam defaults and conversion.
use crate::network::{ConnectionOptions, ListenOptions, TlsOptions};
use crate::options::types::{
    ActiveStateInput, InterfaceInput, IpAddressInput, TcpOptionInput, TlsCertsInput,
};
use crate::sockets::{Active, Settings};
use geam::provider::advanced::NativeValue;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;

impl Default for Settings {
    fn default() -> Self {
        Self {
            io: ConnectionOptions {
                reuse_address: true,
                nodelay: true,
                linger: None,
                buffer: 65536,
            },
            active: Active::Passive,
            send_timeout: Duration::from_millis(30_000),
            send_timeout_close: true,
        }
    }
}

/// Phase-local builder. Only the final options escape into the IO capability.
pub(crate) struct ListenBuilder {
    port: u16,
    settings: Settings,
    ipv6: bool,
    interface: Interface,
    backlog: i32,
    reuse_address: bool,
    credentials: Option<(String, String)>,
    alpn: Vec<Vec<u8>>,
}

enum Interface {
    Any,
    Loopback,
    Address(IpAddr),
}

impl ListenBuilder {
    pub(crate) fn new(port: u16) -> Self {
        Self {
            port,
            settings: Settings::default(),
            ipv6: false,
            interface: Interface::Any,
            backlog: 1024,
            reuse_address: true,
            credentials: None,
            alpn: Vec::new(),
        }
    }

    pub(crate) fn push(&mut self, option: &TcpOptionInput) -> Result<(), &'static str> {
        update(&mut self.settings, option)?;
        match option {
            TcpOptionInput::Ipv6 => self.ipv6 = true,
            TcpOptionInput::Ip(interface) => {
                self.interface = match interface {
                    InterfaceInput::Any => Interface::Any,
                    InterfaceInput::Loopback => Interface::Loopback,
                    InterfaceInput::Address(address) => Interface::Address(typed_address(address)?),
                }
            }
            TcpOptionInput::Backlog(value) => {
                self.backlog = bounded(value)?;
                if self.backlog < 0 {
                    return Err("badarg");
                }
            }
            TcpOptionInput::Reuseaddr(value) => self.reuse_address = *value,
            TcpOptionInput::CertKeyConfig(TlsCertsInput::CertKeyFiles { certfile, keyfile }) => {
                self.credentials =
                    Some((certfile.as_str().to_owned(), keyfile.as_str().to_owned()));
            }
            TcpOptionInput::AlpnPreferredProtocols(protocols) => {
                self.alpn.clear();
                let mut index = 0;
                while let Some(protocol) = protocols.get(index) {
                    let bytes = protocol.as_str().as_bytes();
                    if bytes.is_empty() || bytes.len() > 255 {
                        return Err("badarg");
                    }
                    self.alpn.push(bytes.to_vec());
                    index += 1;
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn finish(self, tls: bool) -> Result<(ListenOptions, Settings), &'static str> {
        let ip = match self.interface {
            Interface::Address(address) => address,
            Interface::Any if self.ipv6 => IpAddr::V6(Ipv6Addr::UNSPECIFIED),
            Interface::Any => IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            Interface::Loopback if self.ipv6 => IpAddr::V6(Ipv6Addr::LOCALHOST),
            Interface::Loopback => IpAddr::V4(Ipv4Addr::LOCALHOST),
        };
        let tls = if tls {
            let (certificate_file, private_key_file) = self.credentials.ok_or("badarg")?;
            Some(TlsOptions {
                certificate_file,
                private_key_file,
                alpn: self.alpn,
            })
        } else {
            None
        };
        Ok((
            ListenOptions {
                address: SocketAddr::new(ip, self.port),
                backlog: self.backlog,
                reuse_address: self.reuse_address,
                connection: self.settings.io.clone(),
                tls,
            },
            self.settings,
        ))
    }
}

/// Creation options cannot be silently accepted after a connection exists.
pub(crate) fn update_connection(
    settings: &mut Settings,
    option: &TcpOptionInput,
) -> Result<bool, crate::SocketReason> {
    match option {
        TcpOptionInput::Backlog(_)
        | TcpOptionInput::CertKeyConfig(_)
        | TcpOptionInput::AlpnPreferredProtocols(_)
        | TcpOptionInput::Ipv6
        | TcpOptionInput::Ip(_) => Err(crate::SocketReason::Einval),
        _ => update(settings, option).map_err(|_| crate::SocketReason::Badarg),
    }
}

/// The caller validates all options before committing this phase-local copy.
/// Count updates add to an existing count, exactly once per source option.
pub(crate) fn update(
    settings: &mut Settings,
    option: &TcpOptionInput,
) -> Result<bool, &'static str> {
    let mut passive = false;
    match option {
        TcpOptionInput::Nodelay(value) => settings.io.nodelay = *value,
        TcpOptionInput::Reuseaddr(value) => settings.io.reuse_address = *value,
        TcpOptionInput::Linger((enabled, seconds)) => {
            settings.io.linger = if *enabled {
                Some(Duration::from_secs(bounded(seconds)?))
            } else {
                None
            }
        }
        TcpOptionInput::Buffer(value) => {
            settings.io.buffer = bounded(value)?;
            if settings.io.buffer == 0 {
                return Err("badarg");
            }
        }
        TcpOptionInput::SendTimeout(value) => {
            settings.send_timeout = Duration::from_millis(bounded(value)?)
        }
        TcpOptionInput::SendTimeoutClose(value) => settings.send_timeout_close = *value,
        TcpOptionInput::ActiveMode(state) => {
            settings.active = match state {
                ActiveStateInput::Passive => Active::Passive,
                ActiveStateInput::Once => Active::Once,
                ActiveStateInput::Active => Active::All,
                ActiveStateInput::Count(value) => {
                    let delta = i32::from(bounded::<i16>(value)?);
                    let count = match settings.active {
                        Active::Count(count) => i32::from(count) + delta,
                        _ => delta,
                    };
                    if count > i32::from(i16::MAX) {
                        return Err("badarg");
                    }
                    if count <= 0 {
                        passive = true;
                        Active::Passive
                    } else {
                        Active::Count(count as i16)
                    }
                }
            }
        }
        TcpOptionInput::Backlog(_)
        | TcpOptionInput::Mode(_)
        | TcpOptionInput::CertKeyConfig(_)
        | TcpOptionInput::AlpnPreferredProtocols(_)
        | TcpOptionInput::Ipv6
        | TcpOptionInput::Ip(_) => {}
    }
    Ok(passive)
}

fn typed_address(value: &IpAddressInput) -> Result<IpAddr, &'static str> {
    match value {
        IpAddressInput::IpV4(a, b, c, d) => {
            typed_parts([a, b, c, d]).map(|parts| IpAddr::V4(Ipv4Addr::from(parts)))
        }
        IpAddressInput::IpV6(a, b, c, d, e, f, g, h) => {
            typed_parts([a, b, c, d, e, f, g, h]).map(|parts| IpAddr::V6(Ipv6Addr::from(parts)))
        }
    }
}

fn typed_parts<T: Default + Copy + for<'a> TryFrom<&'a geam::provider::BigInt>, const N: usize>(
    parts: [&geam::provider::BigInt; N],
) -> Result<[T; N], &'static str> {
    let mut result = [T::default(); N];
    for (part, converted) in parts.into_iter().zip(&mut result) {
        *converted = bounded(part)?;
    }
    Ok(result)
}

pub(crate) fn number<T: for<'a> TryFrom<&'a geam::provider::BigInt>>(
    value: &NativeValue,
) -> Result<T, &'static str> {
    bounded(&value.as_int().ok_or("badarg")?)
}

fn bounded<T: for<'a> TryFrom<&'a geam::provider::BigInt>>(
    value: &geam::provider::BigInt,
) -> Result<T, &'static str> {
    T::try_from(value).map_err(|_| "badarg")
}

pub(crate) fn address(value: &NativeValue) -> Result<IpAddr, &'static str> {
    if value.kind() != geam::provider::advanced::NativeKind::Tuple {
        return Err("badarg");
    }
    // Validate the generic FFI's complete tuple shape at this boundary. No
    // fallible element lookup follows an earlier arity validation.
    match std::array::from_fn::<_, 9, _>(|index| value.index(index)) {
        [
            Some(a),
            Some(b),
            Some(c),
            Some(d),
            None,
            None,
            None,
            None,
            None,
        ] => native_parts([a, b, c, d]).map(|parts| IpAddr::V4(Ipv4Addr::from(parts))),
        [
            Some(a),
            Some(b),
            Some(c),
            Some(d),
            Some(e),
            Some(f),
            Some(g),
            Some(h),
            None,
        ] => native_parts([a, b, c, d, e, f, g, h]).map(|parts| IpAddr::V6(Ipv6Addr::from(parts))),
        _ => Err("badarg"),
    }
}

fn native_parts<T: Default + Copy + for<'a> TryFrom<&'a geam::provider::BigInt>, const N: usize>(
    parts: [NativeValue; N],
) -> Result<[T; N], &'static str> {
    let mut result = [T::default(); N];
    for (part, converted) in parts.iter().zip(&mut result) {
        *converted = number(part)?;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use crate::network::Network;
    use crate::test_support::{
        execution_fixture::TestHost, network::ScriptedNetwork, source_project,
    };
    use std::net::{Ipv4Addr, SocketAddr};
    use std::sync::Arc;
    use std::time::Duration;

    #[test]
    fn original_tcp_and_tls_options_select_explicit_listener_capabilities() {
        for (options, address, tls) in [
            ("options.Ip(options.Any)", "0.0.0.0", false),
            ("options.Ip(options.Loopback)", "127.0.0.1", false),
            (
                "options.Ip(options.Address(options.IpV4(192, 0, 2, 1)))",
                "192.0.2.1",
                false,
            ),
            ("options.Ipv6, options.Ip(options.Any)", "::", false),
            ("options.Ipv6, options.Ip(options.Loopback)", "::1", false),
            (
                "options.Ip(options.Address(options.IpV6(0, 0, 0, 0, 0, 0, 0, 1)))",
                "::1",
                false,
            ),
            (
                "options.Ip(options.Loopback), options.CertKeyConfig(options.CertKeyFiles(\"cert.pem\", \"key.pem\")), options.AlpnPreferredProtocols([\"h2\", \"http/1.1\"])",
                "127.0.0.1",
                true,
            ),
        ] {
            let scripted = Arc::new(ScriptedNetwork::new(
                SocketAddr::from((Ipv4Addr::LOCALHOST, 4321)),
                vec![],
            ));
            let network: Arc<dyn Network> = scripted.clone();
            let source = format!(
                r#"
import glisten/{transport}
import glisten/socket/options
pub fn main() {{
  let assert Ok(_) = {transport}.listen(1234, [{options}, options.Backlog(17),
    options.Nodelay(False), options.Reuseaddr(False), options.Linger(#(True, 2)),
    options.SendTimeout(19), options.SendTimeoutClose(False), options.Buffer(1024)])
  Nil
}}
"#,
                transport = if tls { "ssl" } else { "tcp" }
            );
            let (mut execution, mut state) = source_project(&source, network);
            let host = TestHost::default();
            assert_eq!(
                host.block_on(execution.run_main(&host, &mut state, &mut Vec::new()))
                    .map(|outcome| outcome
                        .try_into_value()
                        .expect("fixture must return normally"))
                    .unwrap(),
                geam::Value::Nil
            );
            let captured = scripted.listen_options.lock();
            assert_eq!(captured.len(), 1);
            let selected = &captured[0];
            assert_eq!(
                selected.address,
                SocketAddr::new(address.parse().unwrap(), 1234)
            );
            assert_eq!(selected.backlog, 17);
            assert!(!selected.reuse_address);
            assert!(!selected.connection.nodelay);
            assert!(!selected.connection.reuse_address);
            assert_eq!(selected.connection.linger, Some(Duration::from_secs(2)));
            assert_eq!(selected.connection.buffer, 1024);
            if tls {
                let selected = selected.tls.as_ref().unwrap();
                assert_eq!(selected.certificate_file, "cert.pem");
                assert_eq!(selected.private_key_file, "key.pem");
                assert_eq!(selected.alpn, [b"h2".to_vec(), b"http/1.1".to_vec()]);
            } else {
                assert!(selected.tls.is_none());
            }
        }
    }

    #[test]
    fn source_representable_invalid_options_fail_before_host_io() {
        for expression in [
            "tcp.listen(-1, [])",
            "tcp.listen(65536, [])",
            "tcp.listen(0, [options.Backlog(-1)])",
            "tcp.listen(0, [options.Backlog(2147483648)])",
            "tcp.listen(0, [options.Buffer(0)])",
            "tcp.listen(0, [options.Buffer(-1)])",
            "tcp.listen(0, [options.SendTimeout(-1)])",
            "tcp.listen(0, [options.Linger(#(True, -1))])",
            "tcp.listen(0, [options.Ip(options.Address(options.IpV4(256, 0, 0, 1)))])",
            "tcp.listen(0, [options.Ip(options.Address(options.IpV6(65536, 0, 0, 0, 0, 0, 0, 1)))])",
            "tcp.listen(0, [options.ActiveMode(options.Count(32768))])",
            "tcp.listen(0, [options.ActiveMode(options.Count(-32769))])",
            "tcp.listen(0, [options.AlpnPreferredProtocols([\"\"])])",
        ] {
            let scripted = Arc::new(ScriptedNetwork::new(
                SocketAddr::from((Ipv4Addr::LOCALHOST, 4321)),
                vec![],
            ));
            let network: Arc<dyn Network> = scripted.clone();
            let source = format!(
                "import glisten/tcp\nimport glisten/socket\nimport glisten/socket/options\npub fn main() {{ let assert Error(socket.Badarg) = {expression} Nil }}"
            );
            let (mut execution, mut state) = source_project(&source, network);
            let host = TestHost::default();
            assert_eq!(
                host.block_on(execution.run_main(&host, &mut state, &mut Vec::new()))
                    .map(|outcome| outcome
                        .try_into_value()
                        .expect("fixture must return normally"))
                    .unwrap(),
                geam::Value::Nil,
                "{expression}"
            );
            assert!(scripted.events.lock().is_empty(), "{expression}");
        }
    }
}
