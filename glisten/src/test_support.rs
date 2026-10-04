//! An explicit execution profile for original-source and owner tests.
#[path = "../tests/support/execution_host.rs"]
pub(crate) mod execution_fixture;
pub(crate) mod network;
use crate::{Component, Domain};
use geam::execution::ExecutionServices;
use geam::gleam_erlang::{Configuration, ErlangExecution, GleamErlangHostProfile};
use geam::gleam_stdlib::{GleamStdlibRunState, GleamStdlibStores, IoOutput};
use geam::{
    HostComponentProfile, HostExecutionService, HostProfile, HostProviderComponent,
    HostProviderComponentRegistration, HostProviderSet, HostServiceProfile, HostedExecution,
    compile_typed_host_project, plan_host_program,
};

pub(crate) struct Profile;
type Otp = geam_otp::Component<Profile>;
#[derive(Default)]
pub(crate) struct Stores {
    stdlib: GleamStdlibStores,
    erlang: geam::gleam_erlang::Stores<Profile>,
    provider: <Component as HostProviderComponent>::Stores,
    otp: <Otp as HostProviderComponent>::Stores,
    logging: <geam_logging::Component as HostProviderComponent>::Stores,
    argv: <geam_argv::Component as HostProviderComponent>::Stores,
}
pub(crate) struct State {
    pub(crate) stdlib: GleamStdlibRunState,
    erlang: Configuration,
    pub(crate) provider: crate::State,
    otp: geam_otp::State,
    logging: geam_logging::RunState,
    argv: geam_argv::RunState,
}

impl HostProfile for Profile {
    type RunState = State;
    type ExternalStores = Stores;
    type ExecutionState = ExecutionServices<ErlangExecution, Domain>;
    fn initialize_execution(state: &mut State) -> Self::ExecutionState {
        ExecutionServices {
            first:
                <geam::gleam_erlang::Component<Profile> as HostExecutionService>::initialize_service(
                    &mut state.erlang,
                ),
            rest: Component::initialize_service(&mut state.provider),
        }
    }
}
impl geam::gleam_stdlib::GleamStdlibHostProfile for Profile {
    type Io = Vec<IoOutput>;
}
macro_rules! component {
    ($component:ty, $field:ident) => {
        impl HostComponentProfile<$component> for Profile {
            fn component_stores(stores: &Stores) -> &<$component as HostProviderComponent>::Stores {
                &stores.$field
            }
            fn component_state(
                state: &mut State,
            ) -> &mut <$component as HostProviderComponent>::RunState {
                &mut state.$field
            }
        }
    };
}
component!(geam::gleam_stdlib::Component, stdlib);
component!(geam::gleam_erlang::Component<Profile>, erlang);
component!(Component, provider);
component!(Otp, otp);
component!(geam_logging::Component, logging);
component!(geam_argv::Component, argv);
impl HostServiceProfile<Component> for Profile {
    fn service(state: &mut Self::ExecutionState) -> &mut Domain {
        &mut state.rest
    }
}
impl GleamErlangHostProfile for Profile {
    fn erlang_execution(state: &mut Self::ExecutionState) -> &mut ErlangExecution {
        &mut state.first
    }
}

pub(crate) fn typed_project(entry: &str) -> geam::HostedTypedProgram<Profile> {
    let root = camino::Utf8Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/gleam");
    let providers = providers([]);
    compile_typed_host_project(&root, entry, providers).unwrap()
}

pub(crate) fn project(entry: &str) -> (HostedExecution<Profile>, State) {
    let typed = typed_project(entry);
    let resources = typed.package_resources().clone();
    let execution =
        HostedExecution::try_from_module_plan(plan_host_program(typed).unwrap()).unwrap();
    let state = State {
        stdlib: GleamStdlibRunState::from_seed([0; 32]),
        erlang: Configuration { resources },
        provider: <Component as geam::HostProviderComponentInitialization>::initialize(
            &geam::HostProviderConfiguration::empty(),
        )
        .unwrap(),
        otp: Default::default(),
        logging: geam_logging::RunState::with_writer(Vec::<u8>::new(), Some("true"), None),
        argv: geam_argv::RunState::new("/host/runtime".into(), "/host/project".into(), Vec::new()),
    };
    (execution, state)
}

fn providers(
    extra: impl IntoIterator<Item = geam::host::HostProviderModule<Profile>>,
) -> HostProviderSet<Profile> {
    let mut providers = geam::gleam_stdlib::host_providers::<Profile>().unwrap();
    providers.extend(geam::gleam_erlang::host_providers::<Profile>().unwrap());
    providers
        .extend(<Component as HostProviderComponentRegistration<Profile>>::providers().unwrap());
    providers.extend(<Otp as HostProviderComponentRegistration<Profile>>::providers().unwrap());
    providers.extend(
        <geam_logging::Component as HostProviderComponentRegistration<Profile>>::providers()
            .unwrap(),
    );
    providers.extend(
        <geam_argv::Component as HostProviderComponentRegistration<Profile>>::providers().unwrap(),
    );
    providers.extend(extra);
    HostProviderSet::from_providers(providers).unwrap()
}

pub(crate) fn source_project(
    source: &str,
    network: std::sync::Arc<dyn crate::network::Network>,
) -> (HostedExecution<Profile>, State) {
    source_project_with(source, network, [])
}

pub(crate) fn source_project_with(
    source: &str,
    network: std::sync::Arc<dyn crate::network::Network>,
    extra: impl IntoIterator<Item = geam::host::HostProviderModule<Profile>>,
) -> (HostedExecution<Profile>, State) {
    let package_root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/gleam/build/packages");
    let mut packages = Vec::new();
    for (name, dependencies) in [
        ("argv", vec![]),
        ("gleam_stdlib", vec![]),
        ("gleam_erlang", vec!["gleam_stdlib"]),
        ("gleam_otp", vec!["gleam_stdlib", "gleam_erlang"]),
        ("logging", vec!["gleam_stdlib", "gleam_erlang"]),
        (
            "glisten",
            vec!["gleam_stdlib", "gleam_erlang", "gleam_otp", "logging"],
        ),
    ] {
        let root = package_root.join(name).join("src");
        let mut pending = vec![root.clone()];
        let mut modules = Vec::new();
        while let Some(directory) = pending.pop() {
            for entry in std::fs::read_dir(directory).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    pending.push(path);
                } else if path.extension().and_then(|value| value.to_str()) == Some("gleam") {
                    let module = path
                        .strip_prefix(&root)
                        .unwrap()
                        .with_extension("")
                        .to_str()
                        .unwrap()
                        .replace('\\', "/");
                    modules.push(geam::ModuleSource::new(
                        module,
                        path.to_str().unwrap(),
                        std::fs::read_to_string(&path).unwrap(),
                    ));
                }
            }
        }
        packages.push(geam::PackageSource::new(name, dependencies, modules));
    }
    packages.push(geam::PackageSource::new(
        "fixture",
        vec!["glisten", "gleam_stdlib", "gleam_erlang", "gleam_otp"],
        [geam::ModuleSource::new(
            "fixture",
            "src/fixture.gleam",
            source,
        )],
    ));
    let typed = geam::compile_typed_host_program::<Profile>(
        "fixture",
        "fixture",
        packages,
        providers(extra),
    )
    .unwrap();
    let resources = typed.package_resources().clone();
    let execution =
        HostedExecution::try_from_module_plan(plan_host_program(typed).unwrap()).unwrap();
    let state = State {
        stdlib: GleamStdlibRunState::from_seed([0; 32]),
        erlang: Configuration { resources },
        provider: crate::State::with_network(network),
        otp: Default::default(),
        logging: geam_logging::RunState::with_writer(Vec::<u8>::new(), Some("true"), None),
        argv: geam_argv::RunState::new("/host/runtime".into(), "/host/project".into(), Vec::new()),
    };
    (execution, state)
}
