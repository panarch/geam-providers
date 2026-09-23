#[allow(dead_code)]
#[path = "../fixtures/embedding/src/geam_bindings.rs"]
mod geam_bindings;

use geam::HostComponentProfile;
use geam::embedding::{HostedModuleBuilder, HostedProject};
use geam_crypto::{Entropy, RunState};

struct SequenceEntropy;

impl Entropy for SequenceEntropy {
    fn fill(&mut self, bytes: &mut [u8]) -> Result<(), geam::HostFailure> {
        for (index, byte) in bytes.iter_mut().enumerate() {
            *byte = index as u8;
        }
        Ok(())
    }
}

struct FailedEntropy;

impl Entropy for FailedEntropy {
    fn fill(&mut self, _: &mut [u8]) -> Result<(), geam::HostFailure> {
        Err(geam::HostFailure::new("injected entropy failure"))
    }
}

#[test]
fn original_gleam_package_runs_through_public_host_boundary() {
    let executor = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("single-threaded test executor");
    let host = geam::execution::TokioHost::new(executor.handle().clone());
    let project = HostedProject::new(
        concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/embedding/gleam"),
        geam_bindings::ROOT_MODULE,
        geam_bindings::host_providers::<Vec<geam::gleam_stdlib::IoOutput>>,
    );
    let program = project.compile().expect("unmodified Hex package compiles");
    let (bindings, functions) =
        geam_bindings::bind(HostedModuleBuilder::new(program).expect("host program plans"))
            .expect("generated source signatures bind");
    let mut module = bindings.seal().expect("host module seals");
    let mut state = geam_bindings::RunStateInputs {
        stdlib: geam::gleam_stdlib::GleamStdlibRunState::from_seed([0; 32]),
        gleam_crypto: geam::HostProviderConfiguration::empty(),
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
                assert_eq!(
                    scope
                        .call(&functions.random_four, ())
                        .await
                        .expect("OS entropy")
                        .bytes()
                        .len(),
                    4,
                );
                let failure = scope
                    .call(&functions.random_negative, ())
                    .await
                    .expect_err("negative byte count fails");
                assert!(
                    failure
                        .to_string()
                        .contains("random byte count must be nonnegative")
                );
                let failure = scope
                    .call(&functions.partial_hash, ())
                    .await
                    .expect_err("partial hash fails");
                assert!(
                    failure
                        .to_string()
                        .contains("crypto input must contain complete bytes")
                );
                let failure = scope
                    .call(&functions.partial_hmac_data, ())
                    .await
                    .expect_err("partial HMAC data fails");
                assert!(
                    failure
                        .to_string()
                        .contains("crypto input must contain complete bytes")
                );
                let failure = scope
                    .call(&functions.partial_hmac_key, ())
                    .await
                    .expect_err("partial HMAC key fails");
                assert!(
                    failure
                        .to_string()
                        .contains("crypto input must contain complete bytes")
                );
            }),
        )
        .expect("host execution completes");

    let mut deterministic = geam_bindings::RunStateInputs {
        stdlib: geam::gleam_stdlib::GleamStdlibRunState::from_seed([0; 32]),
        gleam_crypto: geam::HostProviderConfiguration::empty(),
    }
    .initialize()
    .expect("deterministic run initializes");
    *<geam_bindings::Profile<Vec<geam::gleam_stdlib::IoOutput>> as HostComponentProfile<
        geam_crypto::Component,
    >>::component_state(&mut deterministic) = RunState::with_entropy(SequenceEntropy);
    executor
        .block_on(
            module.with_execution(&host, &mut deterministic, &mut echo, async |scope| {
                assert_eq!(
                    scope
                        .call(&functions.random_four, ())
                        .await
                        .expect("injected bytes")
                        .bytes(),
                    &[0, 1, 2, 3],
                );
            }),
        )
        .expect("deterministic source execution completes");

    let mut failed = geam_bindings::RunStateInputs {
        stdlib: geam::gleam_stdlib::GleamStdlibRunState::from_seed([0; 32]),
        gleam_crypto: geam::HostProviderConfiguration::empty(),
    }
    .initialize()
    .expect("failing run initializes");
    *<geam_bindings::Profile<Vec<geam::gleam_stdlib::IoOutput>> as HostComponentProfile<
        geam_crypto::Component,
    >>::component_state(&mut failed) = RunState::with_entropy(FailedEntropy);
    executor
        .block_on(
            module.with_execution(&host, &mut failed, &mut echo, async |scope| {
                let failure = scope
                    .call(&functions.random_four, ())
                    .await
                    .expect_err("entropy fails");
                assert!(failure.to_string().contains("injected entropy failure"));
            }),
        )
        .expect("failure reaches embedding caller");
}
