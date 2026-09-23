#[allow(dead_code)]
mod geam_bindings;

use geam::HostProviderConfiguration;
use geam::embedding::HostedModuleBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let executor = tokio::runtime::Builder::new_current_thread().build()?;
    let host = geam::execution::TokioHost::new(executor.handle().clone());
    let program = geam_bindings::project::<Vec<geam::gleam_stdlib::IoOutput>>().compile()?;
    let (bindings, functions) = geam_bindings::bind(HostedModuleBuilder::new(program)?)?;
    let mut module = bindings.seal()?;
    let mut state = geam_bindings::RunStateInputs {
        stdlib: geam::gleam_stdlib::GleamStdlibRunState::from_seed([0; 32]),
        gleam_crypto: HostProviderConfiguration::empty(),
    }
    .initialize()?;
    let mut echo = Vec::new();

    executor.block_on(
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
                .expect_err("partial hash input fails");
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
    )?;
    Ok(())
}
