//! Native functions for the unchanged `glisten` 9.0.1 Gleam package.

mod active;
mod handler;
mod native;
pub mod network;
mod options;
mod schema;
mod send;
mod settings;
mod socket;
mod sockets;
pub use socket::types::SocketReason;
#[cfg(test)]
extern crate geam as geam_core;
#[cfg(test)]
#[path = "../tests/original_glisten.rs"]
mod original_glisten;
#[cfg(test)]
mod test_support;

use geam::execution::{ExecutionUnit, ExecutionUnitId, HostExecutionState, UnitExit};
use geam::gleam_erlang::GleamErlangHostProfile;
use geam::host::{
    HostComponentProfile, HostExecutionService, HostExternalStore, HostProvider,
    HostProviderComponent, HostProviderComponentInitialization, HostProviderComponentRegistration,
    HostProviderConfiguration, HostProviderInitializationError, HostProviderModule,
    HostRegistrationError, HostServiceProfile,
};
use geam::provider::BigInt;
use std::collections::BTreeMap;
use std::sync::Arc;

/// The independently selected glisten provider and execution service.
pub struct Component;

#[derive(Default)]
pub struct Stores {
    socket: socket::types::__GeamStores,
    options: HostExternalStore<options::ErlangOption>,
}

/// Caller-owned networking capability, selected before source execution.
pub struct State {
    network: Arc<dyn network::Network>,
}

impl State {
    pub fn with_network(network: Arc<dyn network::Network>) -> Self {
        Self { network }
    }
}

impl geam::__macro_support::ProviderPackage for Component {
    const PACKAGE: &'static str = "glisten";
}

impl HostProviderComponent for Component {
    const ID: &'static str = "glisten";
    type Stores = Stores;
    type RunState = State;
}

impl HostProviderComponentInitialization for Component {
    fn initialize(
        configuration: &HostProviderConfiguration,
    ) -> Result<State, HostProviderInitializationError> {
        initialize_network(configuration, || {
            network::TokioNetwork::new().map(|network| State::with_network(Arc::new(network)))
        })
    }
}

fn initialize_network(
    configuration: &HostProviderConfiguration,
    create: impl FnOnce() -> std::io::Result<State>,
) -> Result<State, HostProviderInitializationError> {
    if let Some((key, _)) = configuration.iter().next() {
        return Err(HostProviderInitializationError::for_component::<Component>(
            format!("unknown configuration key `{key}`"),
        ));
    }
    create().map_err(|error| {
        HostProviderInitializationError::for_component::<Component>(error.to_string())
    })
}

pub trait GlistenProfile:
    GleamErlangHostProfile + HostComponentProfile<Component> + HostServiceProfile<Component>
{
}
impl<Profile> GlistenProfile for Profile where
    Profile:
        GleamErlangHostProfile + HostComponentProfile<Component> + HostServiceProfile<Component>
{
}

impl<Profile: GlistenProfile> HostProvider<Profile> for Component {
    type State = State;
    fn project(state: &mut Profile::RunState) -> &mut State {
        <Profile as HostComponentProfile<Component>>::component_state(state)
    }
}

impl HostExecutionService for Component {
    type State = Domain;
    fn initialize_service(_: &mut State) -> Domain {
        Domain::default()
    }
}

impl<Profile: GlistenProfile> HostProviderComponentRegistration<Profile> for Component {
    fn providers() -> Result<Vec<HostProviderModule<Profile>>, HostRegistrationError> {
        [
            socket::provider::<Profile>,
            options::provider::<Profile>,
            options::address_provider::<Profile>,
            sockets::tcp_provider::<Profile>,
            sockets::ssl_provider::<Profile>,
            sockets::transport_provider::<Profile>,
            sockets::handler_provider::<Profile>,
        ]
        .into_iter()
        .map(|register| register())
        .collect()
    }
}

/// Handles retain identity only. The execution domain owns live IO resources.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct SocketKey {
    creator: ExecutionUnitId,
    serial: BigInt,
}

#[derive(Default)]
pub struct Domain {
    next: BigInt,
    resources: BTreeMap<SocketKey, sockets::Resource>,
}

impl Domain {
    fn insert(&mut self, creator: &ExecutionUnit, resource: sockets::Resource) -> SocketKey {
        let key = SocketKey {
            creator: creator.id(),
            serial: self.next.clone(),
        };
        self.next += 1;
        self.resources.insert(key.clone(), resource);
        key
    }

    fn remove(&mut self, key: &SocketKey) {
        if let Some(resource) = self.resources.remove(key) {
            resource.close();
        }
    }
}

impl HostExecutionState for Domain {
    fn started(&mut self, _: ExecutionUnit) {}
    fn finished(&mut self, unit: ExecutionUnitId, _: &UnitExit) {
        self.resources.retain(|_, resource| {
            if resource.owner().id() == unit {
                resource.close();
                false
            } else {
                true
            }
        });
    }
    fn close(&mut self) {
        for (_, resource) in std::mem::take(&mut self.resources) {
            resource.close();
        }
    }
}

type Call<'call, Profile, Return> = geam::HostCall<'call, Profile, Component, Return>;
type A = geam::HostTypeParameter<0>;
type B = geam::HostTypeParameter<1>;

#[cfg(test)]
mod tests {
    use super::Component;
    use geam::host::{HostProviderComponentInitialization, HostProviderConfiguration};
    use std::collections::BTreeMap;

    #[test]
    fn configuration_rejects_unknown_keys_before_creating_io_resources() {
        let configuration =
            HostProviderConfiguration::new(BTreeMap::from([("runtime".into(), "ambient".into())]));
        let error = Component::initialize(&configuration).err().unwrap();
        assert_eq!(error.component_id(), "glisten");
        assert_eq!(error.reason(), "unknown configuration key `runtime`");
    }

    #[test]
    fn initializer_preserves_the_network_dependency_failure_identity() {
        let error = super::initialize_network(&HostProviderConfiguration::empty(), || {
            Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "IO reactor unavailable",
            ))
        })
        .err()
        .unwrap();
        assert_eq!(error.component_id(), "glisten");
        assert_eq!(error.reason(), "IO reactor unavailable");
    }

    #[test]
    fn closing_the_execution_service_releases_all_live_resources_and_invalidates_aliases() {
        use crate::test_support::{
            Profile,
            execution_fixture::TestHost,
            network::{Event, ScriptedConnection, ScriptedNetwork},
            source_project_with,
        };
        use geam::execution::HostExecutionState;
        use geam::host::{HostCallCompletion, HostCallError, HostProviderModule};
        use std::sync::Arc;
        fn close<'call>(
            mut call: super::Call<'call, Profile, ()>,
        ) -> Result<HostCallCompletion<'call, ()>, HostCallError> {
            assert_eq!(call.service::<Component>().resources.len(), 2);
            call.service::<Component>().close();
            assert!(call.service::<Component>().resources.is_empty());
            Ok(call.return_value(()))
        }
        let provider = HostProviderModule::new("fixture", "fixture")
            .unwrap()
            .with_scoped_function::<Component, (), (), _>("close_domain", close)
            .unwrap();
        let local = "127.0.0.1:4321".parse().unwrap();
        let io = Arc::new(ScriptedConnection::new(
            local,
            "127.0.0.2:5678".parse().unwrap(),
            vec![],
        ));
        let network = Arc::new(ScriptedNetwork::new(local, vec![io]));
        let source = r#"
import gleam/erlang/process
import glisten/tcp
import glisten/socket
@external(erlang, "fixture", "close_domain") fn close_domain() -> Nil
pub fn main() {
  let assert Ok(listener) = tcp.listen(0, [])
  let assert Ok(connection) = tcp.accept(listener)
  close_domain()
  assert tcp.receive(connection, 0) == Error(socket.Closed)
  assert tcp.accept(listener) == Error(socket.Closed)
  process.sleep(1)
  Nil
}
"#;
        let (mut execution, mut state) = source_project_with(source, network.clone(), [provider]);
        let host = TestHost::default();
        let mut echo = Vec::new();
        let mut running = Box::pin(execution.run_main(&host, &mut state, &mut echo));
        assert!(host.poll(running.as_mut()).is_pending());
        host.advance(std::time::Duration::from_millis(1));
        assert_eq!(
            host.poll(running.as_mut()).map(|result| result
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
