//! Native HTTP provider for the original `gleam_httpc` 5.0.0 package.

mod httpc;
mod schema;
pub mod transport;

use geam::gleam_erlang::GleamErlangHostProfile;
use geam::host::{
    HostComponentProfile, HostProvider, HostProviderComponent, HostProviderComponentInitialization,
    HostProviderComponentRegistration, HostProviderConfiguration, HostProviderInitializationError,
    HostProviderModule, HostRegistrationError,
};
use std::marker::PhantomData;
use std::sync::Arc;
use transport::{HttpTransport, Transport};

/// The independently selected `gleam_httpc` provider component.
pub struct Component<Profile>(PhantomData<fn() -> Profile>);

pub struct State {
    transport: Arc<dyn Transport>,
}

impl State {
    /// Replaces the HTTP capability before running a hosted source execution.
    /// The caller retains ownership of the capability through the supplied `Arc`.
    pub fn set_transport(&mut self, transport: Arc<dyn Transport>) {
        self.transport = transport;
    }
}

impl<Profile: Send + Sync + 'static> HostProviderComponent for Component<Profile> {
    const ID: &'static str = "gleam_httpc";
    type Stores = ();
    type RunState = State;
}

impl<Profile: Send + Sync + 'static> HostProviderComponentInitialization for Component<Profile> {
    fn initialize(
        configuration: &HostProviderConfiguration,
    ) -> Result<State, HostProviderInitializationError> {
        for (key, _) in configuration.iter() {
            if key.as_str() != "root_certificate_pem" {
                return Err(HostProviderInitializationError::for_component::<Self>(
                    format!("unknown configuration key `{key}`"),
                ));
            }
        }
        let root = configuration.get("root_certificate_pem");
        let root = root
            .map(|value| {
                value.as_string().map(|text| text.as_str()).ok_or_else(|| {
                    HostProviderInitializationError::for_component::<Self>(
                        "configuration key `root_certificate_pem` must be a String",
                    )
                })
            })
            .transpose()?;
        let transport = HttpTransport::new(root).map_err(|error| {
            HostProviderInitializationError::for_component::<Self>(format!(
                "invalid root certificate or HTTP client: {error}"
            ))
        })?;
        Ok(State {
            transport: Arc::new(transport),
        })
    }
}

pub trait HttpcProfile:
    GleamErlangHostProfile + HostComponentProfile<Component<Self>> + Send + Sync
{
}

impl<Profile> HttpcProfile for Profile where
    Profile: GleamErlangHostProfile + HostComponentProfile<Component<Self>> + Send + Sync
{
}

impl<Profile: HttpcProfile> HostProvider<Profile> for Component<Profile> {
    type State = State;

    fn project(state: &mut Profile::RunState) -> &mut State {
        <Profile as HostComponentProfile<Self>>::component_state(state)
    }
}

impl<Profile: HttpcProfile> HostProviderComponentRegistration<Profile> for Component<Profile> {
    fn providers() -> Result<Vec<HostProviderModule<Profile>>, HostRegistrationError> {
        httpc::provider::<Profile>().map(|module| vec![module])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use geam::HostProviderConfigurationValue;
    use std::collections::BTreeMap;

    type Profile = ();

    #[test]
    fn configuration_rejects_unknown_keys_and_non_string_certificate() {
        let unknown = HostProviderConfiguration::new(BTreeMap::from([(
            "unknown".into(),
            HostProviderConfigurationValue::Bool(true),
        )]));
        let error = Component::<Profile>::initialize(&unknown).err().unwrap();
        assert!(
            error
                .to_string()
                .contains("unknown configuration key `unknown`")
        );

        let non_string = HostProviderConfiguration::new(BTreeMap::from([(
            "root_certificate_pem".into(),
            HostProviderConfigurationValue::Bool(true),
        )]));
        let error = Component::<Profile>::initialize(&non_string).err().unwrap();
        assert!(error.to_string().contains("must be a String"));

        let invalid_pem = HostProviderConfiguration::new(BTreeMap::from([(
            "root_certificate_pem".into(),
            HostProviderConfigurationValue::from("not a certificate"),
        )]));
        let error = Component::<Profile>::initialize(&invalid_pem)
            .err()
            .unwrap();
        assert!(error.to_string().contains("contains no certificate"));

        let malformed_pem = HostProviderConfiguration::new(BTreeMap::from([(
            "root_certificate_pem".into(),
            HostProviderConfigurationValue::from(
                "-----BEGIN CERTIFICATE-----\n%%%\n-----END CERTIFICATE-----",
            ),
        )]));
        let error = Component::<Profile>::initialize(&malformed_pem)
            .err()
            .unwrap();
        assert!(error.to_string().contains("invalid root certificate"));

        let invalid_der = HostProviderConfiguration::new(BTreeMap::from([(
            "root_certificate_pem".into(),
            HostProviderConfigurationValue::from(
                "-----BEGIN CERTIFICATE-----\nAQID\n-----END CERTIFICATE-----",
            ),
        )]));
        let error = Component::<Profile>::initialize(&invalid_der)
            .err()
            .unwrap();
        assert!(error.to_string().contains("invalid root certificate"));
    }

    #[test]
    fn configured_ca_initializes_the_provider_state() {
        let certificate = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()]).unwrap();
        let pem = certificate.cert.pem();
        let configuration = HostProviderConfiguration::new(BTreeMap::from([(
            "root_certificate_pem".into(),
            HostProviderConfigurationValue::from(pem.as_str()),
        )]));
        let state = Component::<Profile>::initialize(&configuration).unwrap();
        assert_eq!(Arc::strong_count(&state.transport), 1);
    }
}
