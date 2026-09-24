#[allow(dead_code)]
mod geam_bindings;

use geam::HostProviderConfiguration;
use geam::embedding::{BigInt, HostedModuleBuilder};
use geam::gleam_stdlib::{GleamStdlibRunState, IoOutput};
use geam::gleam_time::SystemTimeSource;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let executor = tokio::runtime::Builder::new_current_thread().build()?;
    let host = geam::execution::TokioHost::new(executor.handle().clone());
    let program = geam_bindings::project::<Vec<IoOutput>, SystemTimeSource>().compile()?;
    let (bindings, functions) = geam_bindings::bind(HostedModuleBuilder::new(program)?)?;
    let mut module = bindings.seal()?;
    let mut state = geam_bindings::RunStateInputs {
        stdlib: GleamStdlibRunState::from_seed([0; 32]),
        time: SystemTimeSource,
        birl: HostProviderConfiguration::empty(),
        gleam_regexp: HostProviderConfiguration::empty(),
    }
    .initialize()?;
    let mut echo = Vec::new();

    executor.block_on(
        module.with_execution(&host, &mut state, &mut echo, async |scope| {
            assert_eq!(
                scope
                    .call(&functions.weekday_at, (BigInt::from(0), "Z".into()))
                    .await
                    .expect("epoch weekday"),
                "Thursday"
            );
            let _ = scope
                .call(&functions.observed_timezone, ())
                .await
                .expect("system timezone");
            let _ = scope
                .call(&functions.observed_offset, ())
                .await
                .expect("system offset");
            let _ = scope
                .call(&functions.sampled_difference, ())
                .await
                .expect("monotonic difference");
        }),
    )?;
    Ok(())
}
