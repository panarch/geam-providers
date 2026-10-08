//! Native functions for the unchanged `gramps` 6.0.1 Gleam package.

mod compression;
mod http;
mod websocket;

pub use compression::Domain;

use geam::gleam_erlang::GleamErlangHostProfile;
use geam::gleam_stdlib::GleamStdlibHostProfile;
use geam::host::{
    HostComponentProfile, HostExecutionService, HostProvider, HostProviderComponent,
    HostProviderComponentInitialization, HostProviderComponentRegistration,
    HostProviderConfiguration, HostProviderInitializationError, HostProviderModule,
    HostRegistrationError, HostServiceProfile,
};

/// The independently selected gramps provider and compression execution service.
pub struct Component;

/// Provider-owned storage for logical compression handles.
#[derive(Default)]
pub struct Stores {
    compression: compression::bindings::__GeamStores,
}

impl geam::__macro_support::ProviderPackage for Component {
    const PACKAGE: &'static str = "gramps";
}

impl HostProviderComponent for Component {
    const ID: &'static str = "gramps";
    type Stores = Stores;
    type RunState = ();
}

impl HostProviderComponentInitialization for Component {
    fn initialize(
        configuration: &HostProviderConfiguration,
    ) -> Result<(), HostProviderInitializationError> {
        if let Some((key, _)) = configuration.iter().next() {
            return Err(HostProviderInitializationError::for_component::<Self>(
                format!("unknown configuration key `{key}`"),
            ));
        }
        Ok(())
    }
}

/// Static projection of the original stdlib/Erlang bindings and stream service.
pub trait GrampsProfile:
    GleamStdlibHostProfile
    + GleamErlangHostProfile
    + HostComponentProfile<Component>
    + HostServiceProfile<Component>
{
}
impl<Profile> GrampsProfile for Profile where
    Profile: GleamStdlibHostProfile
        + GleamErlangHostProfile
        + HostComponentProfile<Component>
        + HostServiceProfile<Component>
{
}

impl<Profile: GrampsProfile> HostProvider<Profile> for Component {
    type State = ();
    fn project(state: &mut Profile::RunState) -> &mut () {
        <Profile as HostComponentProfile<Component>>::component_state(state)
    }
}

impl HostExecutionService for Component {
    type State = Domain;
    fn initialize_service(_: &mut ()) -> Domain {
        Domain::default()
    }
}

impl<Profile: GrampsProfile> HostProviderComponentRegistration<Profile> for Component {
    fn providers() -> Result<Vec<HostProviderModule<Profile>>, HostRegistrationError> {
        [
            websocket::provider::<Profile>,
            compression::provider::<Profile>,
            http::provider::<Profile>,
        ]
        .into_iter()
        .map(|register| register())
        .collect()
    }
}

type Call<'call, Profile, Return> = geam::HostCall<'call, Profile, Component, Return>;

#[cfg(test)]
mod tests {
    use super::Component;
    use crate::test_support::Profile;
    use geam::{
        HostProvider, HostProviderComponentInitialization, HostProviderComponentRegistration,
        HostProviderConfiguration,
    };

    #[test]
    fn configuration_and_the_thirteen_original_native_declarations_are_exact() {
        assert_eq!(
            Component::initialize(&HostProviderConfiguration::empty()),
            Ok(())
        );
        let mut state = crate::test_support::state(Default::default());
        assert_eq!(
            *<Component as HostProvider<Profile>>::project(&mut state),
            ()
        );
        let config = HostProviderConfiguration::new(std::collections::BTreeMap::from([(
            "runtime".into(),
            "ambient".into(),
        )]));
        let error = Component::initialize(&config).unwrap_err();
        assert_eq!(error.component_id(), "gramps");
        assert_eq!(error.reason(), "unknown configuration key `runtime`");
        let providers =
            <Component as HostProviderComponentRegistration<Profile>>::providers().unwrap();
        assert_eq!(
            providers
                .iter()
                .map(|module| (
                    module.package().as_str(),
                    module.module().as_str(),
                    module
                        .functions()
                        .map(|function| function.name().as_str())
                        .collect::<Vec<_>>()
                ))
                .collect::<Vec<_>>(),
            vec![
                (
                    "gramps",
                    "gramps/websocket",
                    vec!["crypto_exor", "crypto_hash", "base64_encode"]
                ),
                (
                    "gramps",
                    "gramps/websocket/compression",
                    vec![
                        "do_inflate",
                        "do_deflate",
                        "do_close",
                        "inflate_reset",
                        "deflate_reset",
                        "open",
                        "inflate_init",
                        "deflate_init",
                        "set_controlling_process"
                    ]
                ),
                ("gramps", "gramps/http", vec!["decode_packet"]),
            ]
        );
    }
}

#[cfg(test)]
extern crate geam as geam_core;
#[cfg(test)]
#[path = "../tests/original_gramps.rs"]
mod original_gramps;
#[cfg(test)]
mod test_support;
