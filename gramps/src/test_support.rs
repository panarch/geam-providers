//! Source-backed contract setup, shared by owner and public-boundary tests.

pub(crate) mod execution_host;

use geam::host::{HostProviderModule, HostProviderSet};
use geam::{
    HostedExecution, ModuleSource, PackageSource, compile_typed_host_program, plan_host_program,
};
use std::path::Path;

use crate::Component;
use crate::compression::tests::{AuditedDomain, LifecycleAudit};
use geam::execution::ExecutionServices;
use geam::gleam_erlang::{Configuration, ErlangExecution, GleamErlangHostProfile};
use geam::gleam_stdlib::{GleamStdlibRunState, GleamStdlibStores, IoOutput};
use geam::{
    HostComponentProfile, HostExecutionService, HostProfile, HostProviderComponent,
    HostProviderComponentRegistration, HostServiceProfile,
};

pub(crate) struct Profile;
#[derive(Default)]
pub(crate) struct Stores {
    stdlib: GleamStdlibStores,
    erlang: geam::gleam_erlang::Stores<Profile>,
    gramps: crate::Stores,
    crypto: <geam_crypto::Component as HostProviderComponent>::Stores,
}
pub(crate) struct State {
    stdlib: GleamStdlibRunState,
    erlang: Configuration,
    gramps: (),
    pub(crate) crypto: geam_crypto::RunState,
    pub(crate) audit: LifecycleAudit,
}
impl HostProfile for Profile {
    type RunState = State;
    type ExternalStores = Stores;
    type ExecutionState = ExecutionServices<ErlangExecution, AuditedDomain>;
    fn initialize_execution(state: &mut State) -> Self::ExecutionState {
        ExecutionServices {
            first:
                <geam::gleam_erlang::Component<Profile> as HostExecutionService>::initialize_service(
                    &mut state.erlang,
                ),
            rest: AuditedDomain {
                domain: Component::initialize_service(&mut state.gramps),
                audit: std::sync::Arc::clone(&state.audit),
            },
        }
    }
}
impl geam::gleam_stdlib::GleamStdlibHostProfile for Profile {
    type Io = Vec<IoOutput>;
}
impl GleamErlangHostProfile for Profile {
    fn erlang_execution(state: &mut Self::ExecutionState) -> &mut ErlangExecution {
        &mut state.first
    }
}
impl HostServiceProfile<Component> for Profile {
    fn service(state: &mut Self::ExecutionState) -> &mut crate::Domain {
        &mut state.rest.domain
    }
}
macro_rules! component {
    ($component:ty,$field:ident) => {
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
component!(Component, gramps);
component!(geam_crypto::Component, crypto);

pub(crate) fn providers(
    extra: impl IntoIterator<Item = HostProviderModule<Profile>>,
) -> HostProviderSet<Profile> {
    let mut providers = geam::gleam_stdlib::host_providers::<Profile>().unwrap();
    providers.extend(geam::gleam_erlang::host_providers::<Profile>().unwrap());
    providers
        .extend(<Component as HostProviderComponentRegistration<Profile>>::providers().unwrap());
    providers.extend(
        <geam_crypto::Component as HostProviderComponentRegistration<Profile>>::providers()
            .unwrap(),
    );
    providers.extend(extra);
    HostProviderSet::from_providers(providers).unwrap()
}

pub(crate) fn state(
    resources: std::collections::BTreeMap<geam::provider::EcoString, std::path::PathBuf>,
) -> State {
    State {
        stdlib: GleamStdlibRunState::from_seed([0; 32]),
        erlang: Configuration { resources },
        gramps: (),
        crypto: geam_crypto::RunState::default(),
        audit: Default::default(),
    }
}

/// The source files remain unchanged. Optional in-memory additions expose private
/// native callers for owner tests without changing their original declarations.
pub(crate) fn source_project(
    source: &str,
    additions: &[(&str, &str)],
    extra: impl IntoIterator<Item = HostProviderModule<Profile>>,
) -> (HostedExecution<Profile>, State) {
    let package_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/gleam/build/packages");
    let mut packages = Vec::new();
    for (name, dependencies) in [
        ("gleam_stdlib", vec![]),
        ("gleam_erlang", vec!["gleam_stdlib"]),
        ("gleam_http", vec!["gleam_stdlib"]),
        ("gleam_crypto", vec!["gleam_stdlib"]),
        (
            "gramps",
            vec!["gleam_stdlib", "gleam_erlang", "gleam_http", "gleam_crypto"],
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
                    let mut text = std::fs::read_to_string(&path).unwrap();
                    for (target, additional) in additions {
                        if name == "gramps" && *target == module {
                            text.push_str(additional);
                        }
                    }
                    modules.push(ModuleSource::new(module, path.to_str().unwrap(), text));
                }
            }
        }
        packages.push(PackageSource::new(name, dependencies, modules));
    }
    packages.push(PackageSource::new(
        "fixture",
        [
            "gramps",
            "gleam_stdlib",
            "gleam_erlang",
            "gleam_http",
            "gleam_crypto",
        ],
        [ModuleSource::new("fixture", "src/fixture.gleam", source)],
    ));
    let typed =
        compile_typed_host_program("fixture", "fixture", packages, providers(extra)).unwrap();
    let state = state(typed.package_resources().clone());
    (
        HostedExecution::try_from_module_plan(plan_host_program(typed).unwrap()).unwrap(),
        state,
    )
}
