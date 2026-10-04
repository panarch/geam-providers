//! Ordinary consumption of the stdlib's retained BytesTree input.

#[geam::module(path = "glisten/tcp", crate_path = geam, profile = crate::GlistenProfile,
    component = crate::Component)]
pub(crate) mod tcp {
    use crate::{Component, State, socket::types, sockets};
    use geam::gleam_stdlib::service;
    use geam::provider::{Call, HostResult};

    #[geam::function(await)]
    async fn send(
        #[geam::call] call: &mut Call<State>,
        socket: &types::Socket,
        packet: service::BytesTreeInput,
    ) -> HostResult<Result<(), types::SocketReason>> {
        let key = socket.with(|socket| socket.key.clone());
        let socket = call
            .with_call(move |call| call.service::<Component>().connection(&key))
            .await?;
        Ok(match socket {
            Some(socket) if socket.transport == sockets::Transport::Tcp => {
                let bytes = packet.to_bit_array();
                sockets::send(&socket, bytes.bytes()).await
            }
            Some(_) => Err(types::SocketReason::Badarg),
            None => Err(types::SocketReason::Closed),
        })
    }
}

#[geam::module(path = "glisten/ssl", crate_path = geam, profile = crate::GlistenProfile,
    component = crate::Component)]
pub(crate) mod ssl {
    use crate::{Component, State, socket::types, sockets};
    use geam::gleam_stdlib::service;
    use geam::provider::{Call, HostResult};

    #[geam::function(await)]
    async fn send(
        #[geam::call] call: &mut Call<State>,
        socket: &types::Socket,
        packet: service::BytesTreeInput,
    ) -> HostResult<Result<(), types::SocketReason>> {
        let key = socket.with(|socket| socket.key.clone());
        let socket = call
            .with_call(move |call| call.service::<Component>().connection(&key))
            .await?;
        Ok(match socket {
            Some(socket) if socket.transport == sockets::Transport::Tls => {
                let bytes = packet.to_bit_array();
                sockets::send(&socket, bytes.bytes()).await
            }
            Some(_) => Err(types::SocketReason::Badarg),
            None => Err(types::SocketReason::Closed),
        })
    }
}

pub(crate) fn tcp_provider<Profile: crate::GlistenProfile>()
-> Result<geam::HostProviderModule<Profile>, geam::HostRegistrationError> {
    tcp::__geam_module::<Profile>()
}
pub(crate) fn ssl_provider<Profile: crate::GlistenProfile>()
-> Result<geam::HostProviderModule<Profile>, geam::HostRegistrationError> {
    ssl::__geam_module::<Profile>()
}
