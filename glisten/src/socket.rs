//! Original socket declarations. The macro owns their sole static schema and stores.
use crate::Component;

#[geam::module(path = "glisten/socket", crate_path = geam, profile = crate::GlistenProfile,
    component = crate::Component, stores = socket)]
pub mod types {
    use crate::SocketKey;
    use geam::provider::{EcoString, ExternalPayload};
    use std::hash::{Hash, Hasher};

    #[geam::external(name = "Socket", manual)]
    pub struct Socket {
        pub(crate) key: SocketKey,
    }

    #[geam::external(name = "ListenSocket", manual)]
    pub struct ListenSocket {
        pub(crate) key: SocketKey,
    }

    #[geam::custom]
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum SocketReason {
        Closed,
        Timeout,
        Badarg,
        Terminated,
        Eaddrinuse,
        Eaddrnotavail,
        Eafnosupport,
        Ealready,
        Econnaborted,
        Econnrefused,
        Econnreset,
        Edestaddrreq,
        Ehostdown,
        Ehostunreach,
        Einprogress,
        Eisconn,
        Emsgsize,
        Enetdown,
        Enetunreach,
        Enopkg,
        Enoprotoopt,
        Enotconn,
        Enotty,
        Enotsock,
        Eproto,
        Eprotonosupport,
        Eprototype,
        Esocktnosupport,
        Etimedout,
        Ewouldblock,
        Exbadport,
        Exbadseq,
        Eacces,
        Eagain,
        Ebadf,
        Ebadmsg,
        Ebusy,
        Edeadlk,
        Edeadlock,
        Edquot,
        Eexist,
        Efault,
        Efbig,
        Eftype,
        Eintr,
        Einval,
        Eio,
        Eisdir,
        Eloop,
        Emfile,
        Emlink,
        Emultihop,
        Enametoolong,
        Enfile,
        Enobufs,
        Enodev,
        Enolck,
        Enolink,
        Enoent,
        Enomem,
        Enospc,
        Enosr,
        Enostr,
        Enosys,
        Enotblk,
        Enotdir,
        Enotsup,
        Enxio,
        Eopnotsupp,
        Eoverflow,
        Eperm,
        Epipe,
        Erange,
        Erofs,
        Espipe,
        Esrch,
        Estale,
        Etxtbsy,
        Exdev,
    }

    macro_rules! identity {
        ($payload:ty) => {
            impl ExternalPayload for $payload {
                fn source_equal(&self, other: &Self) -> bool {
                    self.key == other.key
                }
                fn source_hash(&self) -> u64 {
                    let mut hasher = std::collections::hash_map::DefaultHasher::new();
                    self.key.hash(&mut hasher);
                    hasher.finish()
                }
                fn inspect(&self) -> EcoString {
                    format!("glisten socket {:?}/{}", self.key.creator, self.key.serial).into()
                }
            }
        };
    }
    identity!(Socket);
    identity!(ListenSocket);
}

pub(crate) fn provider<Profile: crate::GlistenProfile>()
-> Result<geam::HostProviderModule<Profile>, geam::HostRegistrationError> {
    types::__geam_module::<Profile>().and_then(|module| {
        module.with_resumable_callable::<Component, crate::active::Reader, (), _>(
            crate::active::reader::<Profile>,
        )
    })
}

// Reuse the stores and schemas derived at our own Socket declarations.
impl<Profile: crate::GlistenProfile> geam::HostExternalBinding<Profile, crate::schema::SocketSchema>
    for Component
{
    type Storage = types::__GeamExternalStorage0;
}
impl<Profile: crate::GlistenProfile>
    geam::HostExternalBinding<Profile, crate::schema::ListenSocketSchema> for Component
{
    type Storage = types::__GeamExternalStorage1;
}
