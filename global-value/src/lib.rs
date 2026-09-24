//! Native externals for the unmodified `global_value` 1.0.0 package.

use geam::execution::{ExecutionUnit, ExecutionUnitId, HostExecutionState, UnitExit};
use geam::provider::advanced::NativeValue;
use geam::provider::{BigInt, HostFailure, HostResult, Value};
use geam::{
    HostCall, HostComponentProfile, HostExecutionService, HostProfile, HostProvider,
    HostProviderComponent, HostProviderComponentInitialization, HostProviderComponentRegistration,
    HostProviderConfiguration, HostProviderInitializationError, HostProviderModule,
    HostRegistrationError, HostServiceProfile,
};
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Values and per-name transactions shared by processes in one Geam execution.
#[derive(Default)]
pub struct Cache {
    names: BTreeMap<String, BigInt>,
    next_key: BigInt,
    values: BTreeMap<BigInt, NativeValue>,
    locks: BTreeMap<BigInt, Arc<Mutex<()>>>,
}

impl HostExecutionState for Cache {
    fn started(&mut self, _unit: ExecutionUnit) {}

    fn finished(&mut self, _unit: ExecutionUnitId, _exit: &UnitExit) {}

    fn close(&mut self) {
        self.values.clear();
        self.locks.clear();
        self.names.clear();
    }
}

/// Provider component for the original `global_value` package.
pub struct Component;

/// Typed external-value stores owned by this provider.
#[derive(Default)]
pub struct Stores {
    global_value: global_value::__GeamStores,
}

impl HostProviderComponent for Component {
    const ID: &'static str = "global_value";
    type Stores = Stores;
    type RunState = ();
}

impl HostProviderComponentInitialization for Component {
    fn initialize(
        configuration: &HostProviderConfiguration,
    ) -> Result<Self::RunState, HostProviderInitializationError> {
        if let Some((name, _)) = configuration.iter().next() {
            return Err(HostProviderInitializationError::for_component::<Self>(
                format!("unknown configuration key `{name}`"),
            ));
        }
        Ok(())
    }
}

impl geam::__macro_support::ProviderPackage for Component {
    const PACKAGE: &'static str = "global_value";
}

impl<Profile: GlobalValueProfile> HostProvider<Profile> for Component {
    type State = ();

    fn project(state: &mut Profile::RunState) -> &mut Self::State {
        <Profile as HostComponentProfile<Component>>::component_state(state)
    }
}

impl HostExecutionService for Component {
    type State = Cache;

    fn initialize_service(_: &mut Self::RunState) -> Self::State {
        Cache::default()
    }
}

/// Host profiles that project the provider's stores, state, and execution cache.
pub trait GlobalValueProfile:
    HostProfile + HostComponentProfile<Component> + HostServiceProfile<Component>
{
}

impl<Profile> GlobalValueProfile for Profile where
    Profile: HostProfile + HostComponentProfile<Component> + HostServiceProfile<Component>
{
}

fn name_key(cache: &mut Cache, native: &NativeValue) -> HostResult<BigInt> {
    let name = native
        .as_string()
        .ok_or_else(|| HostFailure::new("global_value name must be a String"))?;
    if let Some(key) = cache.names.get(name.as_str()) {
        return Ok(key.clone());
    }
    cache.next_key += 1u8;
    let key = cache.next_key.clone();
    cache.names.insert(name.as_str().to_owned(), key.clone());
    Ok(key)
}

type HashInput = geam::HostTypeParameter<0>;

fn phash2<'call, Profile: GlobalValueProfile>(
    mut call: HostCall<'call, Profile, Component, BigInt>,
    value: geam::HostValue<'call, HashInput>,
) -> Result<geam::HostCallCompletion<'call, BigInt>, geam::HostCallError> {
    let native = call.native_value::<HashInput>(value);
    name_key(call.service::<Component>(), &native).map(|key| call.return_value(key))
}

#[geam::module(
    path = "global_value",
    crate_path = geam,
    profile = crate::GlobalValueProfile,
    component = crate::Component,
    stores = global_value,
)]
mod global_value {
    use super::{Arc, BigInt, Component, HostResult, Mutex, NativeValue, Value};
    use geam::provider::{Call, Callback};

    // The original Pid and Node only identify the local transaction. Our lock is per name.
    #[geam::external(name = "Pid")]
    #[derive(PartialEq, Eq, Hash)]
    pub struct Pid;

    #[geam::external(name = "Node")]
    #[derive(PartialEq, Eq, Hash)]
    pub struct Node;

    #[geam::external(name = "DoNotLeak")]
    #[derive(PartialEq, Eq, Hash)]
    pub struct DoNotLeak;

    #[allow(dead_code)]
    #[geam::custom(input = HeaderInput)]
    enum Header {
        GleamGlobalValue,
    }

    #[geam::function(profile = Profile)]
    fn current_process() -> Pid {
        Pid
    }

    #[geam::function(profile = Profile)]
    fn current_node() -> Node {
        Node
    }

    #[geam::function(await, profile = Profile)]
    async fn transaction<Item>(
        #[geam::call] call: &mut Call<()>,
        id: (BigInt, &Pid),
        callback: Callback<fn() -> Value<Item>>,
        _nodes: geam::List<Node>,
    ) -> HostResult<Value<Item>> {
        let key = id.0;
        let lock = call
            .with_call(move |call| {
                call.service::<Component>()
                    .locks
                    .entry(key)
                    .or_insert_with(|| Arc::new(Mutex::new(())))
                    .clone()
            })
            .await?;
        let guard = lock.lock_owned().await;
        let result = call.invoke(&callback, ()).await;
        drop(guard);
        result
    }

    #[geam::function(profile = Profile)]
    fn persistent_term_put<Item>(
        #[geam::call] call: &mut Call<()>,
        key: BigInt,
        value: Value<(HeaderInput, Item)>,
    ) -> DoNotLeak {
        // The typed HeaderInput validates the original tag; retain only the value field.
        let tuple = value.into_host(call.host_call());
        let (_, (item, ())) = call.host_call().tuple_values(tuple);
        let native: NativeValue = call
            .host_call()
            .native_value::<geam::HostTypeParameter<0>>(item);
        call.service::<Component>().values.insert(key, native);
        DoNotLeak
    }
}

struct GetErrorSchema;
struct InvalidStoredFormatDefinition;
struct DoesNotExistDefinition;

impl geam::HostCustomConstructorDefinition for InvalidStoredFormatDefinition {
    const NAME: &'static str = "InvalidStoredFormat";
    type Fields = geam::HostCustomFieldListEnd;
}

impl geam::HostCustomConstructorDefinition for DoesNotExistDefinition {
    const NAME: &'static str = "DoesNotExist";
    type Fields = geam::HostCustomFieldListEnd;
}

impl geam::HostCustomSchema for GetErrorSchema {
    const PACKAGE: &'static str = "global_value";
    const MODULE: &'static str = "global_value";
    const NAME: &'static str = "GetError";
    const PARAMETER_COUNT: usize = 0;
    type Constructors = geam::HostCustomConstructorList<
        InvalidStoredFormatDefinition,
        geam::HostCustomConstructorList<DoesNotExistDefinition, geam::HostCustomConstructorListEnd>,
    >;
}

type Item = geam::HostTypeParameter<0>;
type GetError = geam::HostCustomType<GetErrorSchema>;
type GetResult = geam_core::provider::ProviderResult<Item, GetError>;
type Targets = geam::HostTypeList<Item, geam::HostTypeList<GetError, geam::HostTypeListEnd>>;
type ErrorIndex = geam::HostTypeIndexNext<geam::HostTypeIndex0>;
type ErrorDoesNotExist = geam::HostCustomConstructorAt<
    GetError,
    geam::HostCustomIndexNext<geam::HostCustomIndex0>,
    DoesNotExistDefinition,
>;
type ErrorInvalidFormat =
    geam::HostCustomConstructorAt<GetError, geam::HostCustomIndex0, InvalidStoredFormatDefinition>;

fn persistent_term_get<'call, Profile: GlobalValueProfile>(
    mut call: geam::host::native::NativeCall<'call, Profile, Component, GetResult, Targets>,
    key: BigInt,
) -> Result<geam::HostCallCompletion<'call, GetResult>, geam::HostCallError> {
    let stored = call.call().service::<Component>().values.get(&key).cloned();
    let exists = stored.is_some();
    if let Some(value) = stored
        && let Some(value) = call.convert::<geam::HostTypeIndex0>(&value)
    {
        let (call, _) = call.into_call();
        return Ok(
            call.return_custom::<geam_core::provider::ProviderOk<Item, GetError>>((value, ()))
        );
    }

    let (mut call, constructions) = call.into_call();
    let error = if exists {
        call.construct_custom::<ErrorInvalidFormat>(constructions.at::<ErrorIndex>(), ())
    } else {
        call.construct_custom::<ErrorDoesNotExist>(constructions.at::<ErrorIndex>(), ())
    };
    Ok(call.return_custom::<geam_core::provider::ProviderError<Item, GetError>>((error, ())))
}

impl<Profile: GlobalValueProfile> HostProviderComponentRegistration<Profile> for Component {
    fn providers() -> Result<Vec<HostProviderModule<Profile>>, HostRegistrationError> {
        global_value::__geam_module::<Profile>()
            .and_then(|module| {
                module.with_scoped_function::<Component, (HashInput,), BigInt, _>(
                    "phash2",
                    phash2::<Profile>,
                )
            })
            .and_then(|module| {
                module.with_native_function::<Component, (BigInt,), GetResult, Targets, _>(
                    "persistent_term_get",
                    geam::host::native::NativeRules::default(),
                    persistent_term_get::<Profile>,
                )
            })
            .map(|module| vec![module])
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Cache, Component, HostExecutionService, HostExecutionState,
        HostProviderComponentInitialization, HostProviderConfiguration, NativeValue, name_key,
    };

    #[test]
    fn component_accepts_no_configuration_keys() {
        Component::initialize(&HostProviderConfiguration::empty()).expect("empty configuration");
        let configuration = HostProviderConfiguration::new(std::collections::BTreeMap::from([(
            "unexpected".into(),
            true.into(),
        )]));
        let error = Component::initialize(&configuration).expect_err("unknown configuration");
        assert_eq!(error.component_id(), "global_value");
        assert_eq!(error.reason(), "unknown configuration key `unexpected`");
    }

    #[test]
    fn name_requires_a_source_string() {
        let failure = name_key(
            &mut Cache::default(),
            &NativeValue::symbol("a native symbol"),
        )
        .expect_err("a symbol is not a Gleam String");
        assert!(
            failure
                .to_string()
                .contains("global_value name must be a String")
        );
    }

    #[test]
    fn closing_a_domain_releases_its_names_and_locks() {
        let mut cache = Cache::default();
        cache.names.insert("one".to_owned(), 1u8.into());
        cache
            .values
            .insert(1u8.into(), NativeValue::symbol("retained value"));
        cache
            .locks
            .insert(1u8.into(), std::sync::Arc::new(tokio::sync::Mutex::new(())));
        cache.close();
        assert!(cache.names.is_empty());
        assert!(cache.locks.is_empty());
        assert!(cache.values.is_empty());
    }

    #[test]
    fn execution_service_starts_with_empty_storage() {
        let service = Component::initialize_service(&mut ());
        assert!(service.names.is_empty());
        assert!(service.values.is_empty());
        assert!(service.locks.is_empty());
    }
}
