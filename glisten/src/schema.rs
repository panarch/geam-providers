//! Static nominal declarations from unchanged glisten 9.0.1.
use geam::host::{HostExternalSchema, HostExternalType, HostTypeList, HostTypeListEnd};

pub(crate) type One<A> = HostTypeList<A, HostTypeListEnd>;
pub(crate) type Two<A, B> = HostTypeList<A, One<B>>;
pub(crate) type Three<A, B, C> = HostTypeList<A, Two<B, C>>;
pub(crate) type Four<A, B, C, D> = HostTypeList<A, Three<B, C, D>>;

macro_rules! external {
    ($schema:ident: $alias:ident in $module:literal) => {
        pub(crate) struct $schema;
        pub(crate) type $alias = HostExternalType<$schema>;
        impl HostExternalSchema for $schema {
            const PACKAGE: &'static str = "glisten";
            const MODULE: &'static str = $module;
            const NAME: &'static str = stringify!($alias);
            const PARAMETER_COUNT: usize = 0;
        }
    };
}

// These aliases use our own macro declarations, not a second nominal table.
pub(crate) type SocketSchema =
    <crate::socket::types::Socket as geam::__macro_support::ProviderExternalDeclaration>::Schema;
pub(crate) type ListenSocketSchema = <crate::socket::types::ListenSocket as geam::__macro_support::ProviderExternalDeclaration>::Schema;
pub(crate) type Socket = HostExternalType<SocketSchema>;
pub(crate) type ListenSocket = HostExternalType<ListenSocketSchema>;
pub(crate) type SocketReason =
    <crate::socket::types::SocketReason as geam::__macro_support::ProviderValue>::Host;
external!(ErlangTcpOptionSchema: ErlangTcpOption in "glisten/socket/options");

pub(crate) type TcpOption =
    <crate::options::types::TcpOption as geam::__macro_support::ProviderValue>::Host;
pub(crate) type IpAddress =
    <crate::options::types::IpAddress as geam::__macro_support::ProviderValue>::Host;
