//! Native externals for the unmodified `envoy` 1.2.0 package.

mod state;

use geam::gleam_stdlib::{Component as StdlibComponent, DictOf, GleamStdlibHostProfile, service};
use geam::host::{
    HostCall, HostCallCompletion, HostCallError, HostComponentProfile, HostConstructions,
    HostProvider, HostProviderComponent, HostProviderComponentInitialization,
    HostProviderComponentRegistration, HostProviderConfiguration, HostProviderInitializationError,
    HostProviderModule, HostRegistrationError, HostTypeIndex0, HostTypeList, HostTypeListEnd,
};
use geam::provider::{GleamError, GleamOk, GleamResult, StringValue};

pub use state::RunState;

/// The provider component for the original `envoy` package.
pub struct Component;

type TextDict = DictOf<StringValue, StringValue>;
type DictConstructions = HostTypeList<TextDict, HostTypeListEnd>;
type TextResult = GleamResult<StringValue, ()>;

impl HostProviderComponent for Component {
    const ID: &'static str = "geam_envoy";
    type Stores = ();
    type RunState = RunState;
}

impl HostProviderComponentInitialization for Component {
    fn initialize(
        configuration: &HostProviderConfiguration,
    ) -> Result<Self::RunState, HostProviderInitializationError> {
        RunState::initialize(configuration)
            .map_err(|error| HostProviderInitializationError::for_component::<Self>(error.reason()))
    }
}

impl<Profile: HostComponentProfile<Self>> HostProvider<Profile> for Component {
    type State = RunState;

    fn project(state: &mut Profile::RunState) -> &mut Self::State {
        Profile::component_state(state)
    }
}

impl<Profile> HostProviderComponentRegistration<Profile> for Component
where
    Profile: GleamStdlibHostProfile
        + HostComponentProfile<StdlibComponent<Profile::Io>>
        + HostComponentProfile<Self>,
{
    fn providers() -> Result<Vec<HostProviderModule<Profile>>, HostRegistrationError> {
        HostProviderModule::new("envoy", "envoy")
            .and_then(|module| module.with_scoped_function::<Self, (StringValue,), TextResult, _>("get", get::<Profile>))
            .and_then(|module| module.with_scoped_function::<Self, (StringValue, StringValue), (), _>("set", set::<Profile>))
            .and_then(|module| module.with_scoped_function::<Self, (StringValue,), (), _>("unset", unset::<Profile>))
            .and_then(|module| module.with_scoped_function_and_constructions::<Self, (), TextDict, DictConstructions, _>("all", all::<Profile>))
            .map(|module| vec![module])
    }
}

fn get<'call, Profile>(
    mut call: HostCall<'call, Profile, Component, TextResult>,
    name: StringValue,
) -> Result<HostCallCompletion<'call, TextResult>, HostCallError>
where
    Profile: HostComponentProfile<Component>,
{
    let value = call.state().get(name.as_str())?.map(StringValue::from);
    match value {
        Some(value) => Ok(call.return_custom::<GleamOk<StringValue, ()>>((value, ()))),
        None => Ok(call.return_custom::<GleamError<StringValue, ()>>(((), ()))),
    }
}

fn set<'call, Profile>(
    mut call: HostCall<'call, Profile, Component, ()>,
    name: StringValue,
    value: StringValue,
) -> Result<HostCallCompletion<'call, ()>, HostCallError>
where
    Profile: HostComponentProfile<Component>,
{
    call.state().set(name.as_str(), value.as_str())?;
    Ok(call.return_value(()))
}

fn unset<'call, Profile>(
    mut call: HostCall<'call, Profile, Component, ()>,
    name: StringValue,
) -> Result<HostCallCompletion<'call, ()>, HostCallError>
where
    Profile: HostComponentProfile<Component>,
{
    call.state().unset(name.as_str())?;
    Ok(call.return_value(()))
}

fn all<'call, Profile>(
    mut call: HostCall<'call, Profile, Component, TextDict>,
    constructions: HostConstructions<'call, DictConstructions>,
) -> Result<HostCallCompletion<'call, TextDict>, HostCallError>
where
    Profile: GleamStdlibHostProfile
        + HostComponentProfile<StdlibComponent<Profile::Io>>
        + HostComponentProfile<Component>,
{
    let snapshot = call.state().snapshot();
    let entries = snapshot.values().map(|entry| {
        (
            StringValue::from(entry.name.as_str()),
            StringValue::from(entry.value.as_str()),
        )
    });
    let dict = service::dict_from_entries(&mut call, constructions.at::<HostTypeIndex0>(), entries);
    Ok(call.return_value(dict))
}
