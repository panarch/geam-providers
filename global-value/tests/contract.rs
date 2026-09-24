mod support {
    pub mod execution_host;
}

use geam::execution::ExecutionServices;
use geam::{
    HostComponentProfile, HostModule, HostProfile, HostProvider, HostProviderComponent,
    HostProviderComponentRegistration, HostProviderSet, HostServiceProfile, HostedExecution,
    ModuleSource, PackageSource, compile_typed_host_program, plan_host_program,
};
use geam_global_value::{Cache, Component};
use std::path::Path;

struct PlainProfile;

#[derive(Default)]
struct PlainStores {
    global_value: <Component as HostProviderComponent>::Stores,
}

impl HostProfile for PlainProfile {
    type RunState = ();
    type ExternalStores = PlainStores;
    type ExecutionState = Cache;

    fn initialize_execution(_: &mut Self::RunState) -> Self::ExecutionState {
        Cache::default()
    }
}

impl HostComponentProfile<Component> for PlainProfile {
    fn component_stores(
        stores: &Self::ExternalStores,
    ) -> &<Component as HostProviderComponent>::Stores {
        &stores.global_value
    }

    fn component_state(
        state: &mut Self::RunState,
    ) -> &mut <Component as HostProviderComponent>::RunState {
        state
    }
}

impl HostServiceProfile<Component> for PlainProfile {
    fn service(state: &mut Self::ExecutionState) -> &mut Cache {
        state
    }
}

fn global_value_package(source: impl Into<String>) -> PackageSource {
    PackageSource::new(
        "global_value",
        Vec::<String>::new(),
        [ModuleSource::new(
            "global_value",
            "src/global_value.gleam",
            source,
        )],
    )
}

fn original_package() -> PackageSource {
    global_value_package(include_str!(
        "../fixtures/gleam/build/packages/global_value/src/global_value.gleam"
    ))
}

fn plain_execution(source: &str, global_value: PackageSource) -> HostedExecution<PlainProfile> {
    let providers = <Component as HostProviderComponentRegistration<PlainProfile>>::providers()
        .expect("provider registration");
    let hosts = HostProviderSet::with_providers(Vec::<HostModule<PlainProfile>>::new(), providers)
        .expect("provider set");
    let typed = compile_typed_host_program(
        "application",
        "main",
        [
            global_value,
            PackageSource::new(
                "application",
                ["global_value"],
                [ModuleSource::new("main", "src/main.gleam", source)],
            ),
        ],
        hosts,
    )
    .expect("original source links");
    HostedExecution::try_from_module_plan(plan_host_program(typed).expect("program plans"))
        .expect("execution prepares")
}

#[test]
fn component_projects_the_callers_run_state() {
    let mut state = ();
    let projected = <Component as HostProvider<PlainProfile>>::project(&mut state);
    assert_eq!(*projected, ());
}

const REUSE: &str = r#"
import global_value

type StoredThing {
  StoredThing(Int)
}

pub fn main() {
  let first = global_value.create_with_unique_name("shared", fn() { #(41, "same") })
  let second = global_value.create_with_unique_name("shared", fn() { panic as "initialized twice" })
  let other = global_value.create_with_unique_name("other", fn() { #(7, "other") })
  let empty = global_value.create_with_unique_name("", fn() { 1 })
  let unicode = global_value.create_with_unique_name("한글🌟", fn() { 2 })
  let cached: Result(String, Nil) = global_value.create_with_unique_name("result", fn() { Ok("cached") })
  let cached_again: Result(String, Nil) = global_value.create_with_unique_name("result", fn() { panic as "result initialized twice" })
  let callback = global_value.create_with_unique_name("function", fn() { fn(value: Int) { value + 5 } })
  let callback_again = global_value.create_with_unique_name("function", fn() { panic as "function initialized twice" })
  let stored_thing = global_value.create_with_unique_name("opaque", fn() { StoredThing(8) })
  let stored_thing_again = global_value.create_with_unique_name("opaque", fn() { panic as "opaque initialized twice" })
  first == second && first == #(41, "same") && other == #(7, "other") && empty == 1 && unicode == 2 && cached == cached_again && cached == Ok("cached") && callback(1) == 6 && callback_again(2) == 7 && stored_thing == stored_thing_again && stored_thing == StoredThing(8)
}
"#;

#[test]
fn original_source_reuses_distinct_generic_values() {
    let mut execution = plain_execution(REUSE, original_package());
    let mut echo = Vec::new();
    let result = support::execution_host::run(&mut execution, &mut (), &mut echo)
        .expect("source run completes");
    assert_eq!(result, geam::Value::Bool(true));
    assert!(echo.is_empty());
}

const WRONG_TYPE: &str = r#"
import global_value

pub fn main() {
  let _ = global_value.create_with_unique_name("wrong.type", fn() { 1 })
  global_value.create_with_unique_name("wrong.type", fn() { "text" })
}
"#;

#[test]
fn original_source_reports_reused_name_with_incompatible_type() {
    let mut execution = plain_execution(WRONG_TYPE, original_package());
    let mut echo = Vec::new();
    let error = support::execution_host::run(&mut execution, &mut (), &mut echo)
        .expect_err("a reused name cannot restore the wrong type");
    assert!(
        error
            .to_string()
            .contains("global_value error: the name was already taken")
    );
}

#[test]
fn private_generic_hash_rejects_non_string_without_storing_it() {
    // This adds only a test entry point to the original source; production fixtures are untouched.
    let source = format!(
        "{}\npub fn non_string_hash_probe() -> Int {{ phash2(1) }}\n",
        include_str!("../fixtures/gleam/build/packages/global_value/src/global_value.gleam")
    );
    let mut execution = plain_execution(
        "import global_value\npub fn main() { global_value.non_string_hash_probe() }",
        global_value_package(source),
    );
    let mut echo = Vec::new();
    let error = support::execution_host::run(&mut execution, &mut (), &mut echo)
        .expect_err("the generic native boundary rejects a non-string name");
    assert!(
        error
            .to_string()
            .contains("global_value name must be a String")
    );
}

struct ConcurrentProfile;

#[derive(Default)]
struct ConcurrentStores {
    stdlib: geam::gleam_stdlib::GleamStdlibStores,
    erlang: geam::gleam_erlang::Stores<ConcurrentProfile>,
    global_value: <Component as HostProviderComponent>::Stores,
}

struct ConcurrentState {
    stdlib: geam::gleam_stdlib::GleamStdlibRunState,
    erlang: geam::gleam_erlang::Configuration,
    global_value: (),
}

type ConcurrentServices =
    ExecutionServices<geam::gleam_erlang::ErlangExecution, ExecutionServices<Cache, ()>>;

impl HostProfile for ConcurrentProfile {
    type RunState = ConcurrentState;
    type ExternalStores = ConcurrentStores;
    type ExecutionState = ConcurrentServices;

    fn initialize_execution(state: &mut Self::RunState) -> Self::ExecutionState {
        ExecutionServices {
            first: <geam::gleam_erlang::Component<Self> as geam::HostExecutionService>::initialize_service(
                &mut state.erlang,
            ),
            rest: ExecutionServices {
                first: Cache::default(),
                rest: (),
            },
        }
    }
}

impl geam::gleam_stdlib::GleamStdlibHostProfile for ConcurrentProfile {
    type Io = Vec<geam::gleam_stdlib::IoOutput>;
}

impl HostComponentProfile<geam::gleam_stdlib::Component> for ConcurrentProfile {
    fn component_stores(stores: &Self::ExternalStores) -> &geam::gleam_stdlib::GleamStdlibStores {
        &stores.stdlib
    }

    fn component_state(state: &mut Self::RunState) -> &mut geam::gleam_stdlib::GleamStdlibRunState {
        &mut state.stdlib
    }
}

impl HostComponentProfile<geam::gleam_erlang::Component<Self>> for ConcurrentProfile {
    fn component_stores(stores: &Self::ExternalStores) -> &geam::gleam_erlang::Stores<Self> {
        &stores.erlang
    }

    fn component_state(state: &mut Self::RunState) -> &mut geam::gleam_erlang::Configuration {
        &mut state.erlang
    }
}

impl HostComponentProfile<Component> for ConcurrentProfile {
    fn component_stores(
        stores: &Self::ExternalStores,
    ) -> &<Component as HostProviderComponent>::Stores {
        &stores.global_value
    }

    fn component_state(
        state: &mut Self::RunState,
    ) -> &mut <Component as HostProviderComponent>::RunState {
        &mut state.global_value
    }
}

impl HostServiceProfile<geam::gleam_erlang::Component<Self>> for ConcurrentProfile {
    fn service(state: &mut Self::ExecutionState) -> &mut geam::gleam_erlang::ErlangExecution {
        &mut state.first
    }
}

impl geam::gleam_erlang::GleamErlangHostProfile for ConcurrentProfile {
    fn erlang_execution(
        state: &mut Self::ExecutionState,
    ) -> &mut geam::gleam_erlang::ErlangExecution {
        &mut state.first
    }
}

impl HostServiceProfile<Component> for ConcurrentProfile {
    fn service(state: &mut Self::ExecutionState) -> &mut Cache {
        &mut state.rest.first
    }
}

fn package_from_dir(name: &str, dependencies: Vec<&str>, root: &Path) -> PackageSource {
    fn visit(root: &Path, dir: &Path, files: &mut Vec<ModuleSource>) {
        for entry in std::fs::read_dir(dir).expect("Gleam source directory") {
            let path = entry.expect("Gleam source entry").path();
            if path.is_dir() {
                visit(root, &path, files);
            } else if path
                .extension()
                .is_some_and(|extension| extension == "gleam")
            {
                let relative = path.strip_prefix(root).expect("source relative path");
                let module = relative
                    .with_extension("")
                    .to_string_lossy()
                    .replace('\\', "/");
                let source_path = format!("src/{}", relative.display());
                files.push(ModuleSource::new(
                    module,
                    source_path,
                    std::fs::read_to_string(path).expect("Gleam source text"),
                ));
            }
        }
    }
    let mut files = Vec::new();
    visit(root, root, &mut files);
    PackageSource::new(name.to_owned(), dependencies, files)
}

fn process_execution(
    module_name: &str,
    source: &'static str,
) -> HostedExecution<ConcurrentProfile> {
    let mut providers =
        geam::gleam_stdlib::host_providers::<ConcurrentProfile>().expect("stdlib providers");
    providers.extend(
        geam::gleam_erlang::host_providers::<ConcurrentProfile>().expect("erlang providers"),
    );
    providers.extend(
        <Component as HostProviderComponentRegistration<ConcurrentProfile>>::providers()
            .expect("global_value providers"),
    );
    let hosts =
        HostProviderSet::with_providers(Vec::<HostModule<ConcurrentProfile>>::new(), providers)
            .expect("all providers");
    let root = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures/gleam/build/packages"
    ));
    let typed = compile_typed_host_program(
        "application",
        module_name,
        [
            package_from_dir("gleam_stdlib", vec![], &root.join("gleam_stdlib/src")),
            package_from_dir(
                "gleam_erlang",
                vec!["gleam_stdlib"],
                &root.join("gleam_erlang/src"),
            ),
            original_package(),
            PackageSource::new(
                "application",
                ["global_value", "gleam_erlang", "gleam_stdlib"],
                [ModuleSource::new(
                    module_name,
                    format!("src/{module_name}.gleam"),
                    source,
                )],
            ),
        ],
        hosts,
    )
    .expect("original packages link");
    HostedExecution::<ConcurrentProfile>::try_from_module_plan(
        plan_host_program(typed).expect("program plans"),
    )
    .expect("execution prepares")
}

#[test]
fn original_source_initializes_once_across_concurrent_processes() {
    let mut execution = process_execution(
        "geam_global_value_concurrent",
        include_str!("../fixtures/gleam/src/geam_global_value_concurrent.gleam"),
    );
    let host = support::execution_host::TestHost::default();
    let mut state = ConcurrentState {
        stdlib: geam::gleam_stdlib::GleamStdlibRunState::from_seed([0; 32]),
        erlang: geam::gleam_erlang::Configuration::default(),
        global_value: (),
    };
    let mut echo = Vec::new();
    let mut driver = Box::pin(execution.run_main(&host, &mut state, &mut echo));
    assert!(host.poll(driver.as_mut()).is_pending());
    host.advance(std::time::Duration::from_millis(2));
    let result = host.poll(driver.as_mut());
    assert_eq!(
        result.map(Result::unwrap),
        std::task::Poll::Ready(geam::Value::Bool(true))
    );
}

#[test]
fn cancelled_initialiser_releases_name_for_another_process() {
    let mut execution = process_execution(
        "geam_global_value_cancel",
        include_str!("../fixtures/gleam/src/geam_global_value_cancel.gleam"),
    );
    let host = support::execution_host::TestHost::default();
    let mut state = ConcurrentState {
        stdlib: geam::gleam_stdlib::GleamStdlibRunState::from_seed([0; 32]),
        erlang: geam::gleam_erlang::Configuration::default(),
        global_value: (),
    };
    let mut echo = Vec::new();
    let mut driver = Box::pin(execution.run_main(&host, &mut state, &mut echo));
    let result = host.poll(driver.as_mut());
    assert_eq!(
        result.map(|result| result.expect("cancelled name can be reinitialised")),
        std::task::Poll::Ready(geam::Value::Bool(true))
    );
}

#[test]
fn failed_initialiser_releases_name_for_another_process() {
    let mut execution = process_execution(
        "geam_global_value_failure",
        include_str!("../fixtures/gleam/src/geam_global_value_failure.gleam"),
    );
    let host = support::execution_host::TestHost::default();
    let mut state = ConcurrentState {
        stdlib: geam::gleam_stdlib::GleamStdlibRunState::from_seed([0; 32]),
        erlang: geam::gleam_erlang::Configuration::default(),
        global_value: (),
    };
    let mut echo = Vec::new();
    let mut driver = Box::pin(execution.run_main(&host, &mut state, &mut echo));
    let result = host.poll(driver.as_mut());
    assert_eq!(
        result.map(|result| result.expect("failed name can be reinitialised")),
        std::task::Poll::Ready(geam::Value::Bool(true))
    );
}
