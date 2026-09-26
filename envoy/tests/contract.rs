// The generated project() uses the embedding crate's manifest path; this test
// opens that same original Gleam project from the provider crate's manifest path.
#[allow(dead_code)]
#[path = "../fixtures/embedding/src/geam_bindings.rs"]
mod geam_bindings;

use geam::HostProviderComponentRegistration;
use geam::embedding::{HostedModuleBuilder, HostedProject};
use geam::gleam_stdlib::{GleamStdlibRunState, IoOutput};
use std::collections::BTreeMap;

type Io = Vec<IoOutput>;

fn configured_environment() -> geam::HostProviderConfiguration {
    let entries =
        geam::HostProviderConfiguration::new(BTreeMap::from([("INITIAL".into(), "ready".into())]));
    geam::HostProviderConfiguration::new(BTreeMap::from([("values".into(), entries.into())]))
}

#[test]
fn registration_matches_the_four_original_externals() {
    let providers = <geam_envoy::Component as HostProviderComponentRegistration<
        geam_bindings::Profile<Io>,
    >>::providers()
    .expect("static envoy registration");
    assert_eq!(providers.len(), 1);
    assert_eq!(providers[0].package(), "envoy");
    assert_eq!(providers[0].module(), "envoy");
    let names: Vec<_> = providers[0]
        .functions()
        .map(|schema| schema.name().as_str())
        .collect();
    assert_eq!(names, ["get", "set", "unset", "all"]);

    geam_bindings::RunStateInputs {
        stdlib: GleamStdlibRunState::from_seed([0; 32]),
        envoy: configured_environment(),
    }
    .initialize()
    .expect("configured provider initializes");

    let invalid = geam_bindings::RunStateInputs::<Io> {
        stdlib: GleamStdlibRunState::from_seed([0; 32]),
        envoy: geam::HostProviderConfiguration::new(BTreeMap::from([(
            "unknown".into(),
            true.into(),
        )])),
    }
    .initialize()
    .err()
    .expect("invalid provider configuration fails initialization");
    assert!(
        invalid
            .to_string()
            .contains("unknown envoy configuration key")
    );
}

#[test]
fn unchanged_gleam_source_observes_result_and_dict_after_mutations() {
    let executor = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("single-threaded test executor");
    let host = geam::execution::TokioHost::new(executor.handle().clone());
    let project = HostedProject::new(
        concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/embedding/gleam"),
        geam_bindings::ROOT_MODULE,
        geam_bindings::host_providers::<Io>,
    );
    let program = project.compile().expect("original Hex package compiles");
    let (bindings, functions) =
        geam_bindings::bind(HostedModuleBuilder::new(program).expect("host program plans"))
            .expect("generated source signatures bind");
    let mut module = bindings.seal().expect("host module seals");
    let mut state = geam_bindings::RunStateInputs {
        stdlib: GleamStdlibRunState::from_seed([0; 32]),
        envoy: configured_environment(),
    }
    .initialize()
    .expect("provider initializes");
    let mut echo = Vec::new();

    executor
        .block_on(
            module.with_execution(&host, &mut state, &mut echo, async |scope| {
                assert!(
                    scope
                        .call(&functions.verify, ())
                        .await
                        .expect("source contract")
                );
                scope
                    .call(&functions.write, ("PERSIST".into(), "one".into()))
                    .await
                    .expect("source set");
            }),
        )
        .expect("first execution completes");
    executor
        .block_on(
            module.with_execution(&host, &mut state, &mut echo, async |scope| {
                assert!(
                    scope
                        .call(&functions.contains, ("PERSIST".into(), "one".into()))
                        .await
                        .expect("source get and all")
                );
                assert!(
                    scope
                        .call(&functions.clear, ("PERSIST".into(),))
                        .await
                        .expect("source unset")
                );
            }),
        )
        .expect("reused state completes");
    executor
        .block_on(
            module.with_execution(&host, &mut state, &mut echo, async |scope| {
                let invalid_get = scope
                    .call(&functions.contains, ("A=B".into(), "x".into()))
                    .await
                    .expect_err("invalid get name");
                assert!(
                    invalid_get
                        .to_string()
                        .contains("environment variable name contains `=`")
                );
                let invalid_set = scope
                    .call(&functions.write, ("A=B".into(), "x".into()))
                    .await
                    .expect_err("invalid set name");
                assert!(
                    invalid_set
                        .to_string()
                        .contains("environment variable name contains `=`")
                );
                let invalid_value = scope
                    .call(&functions.write, ("A".into(), "x\0y".into()))
                    .await
                    .expect_err("invalid set value");
                assert!(
                    invalid_value
                        .to_string()
                        .contains("environment variable value contains NUL")
                );
                let invalid_unset = scope
                    .call(&functions.clear, ("A=B".into(),))
                    .await
                    .expect_err("invalid unset name");
                assert!(
                    invalid_unset
                        .to_string()
                        .contains("environment variable name contains `=`")
                );
            }),
        )
        .expect("invalid inputs are reported at the host boundary");
    assert!(echo.is_empty());
}
