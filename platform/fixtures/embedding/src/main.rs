mod geam_bindings;

use geam::HostProviderConfiguration;
use geam::embedding::HostedModuleBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let executor = tokio::runtime::Builder::new_current_thread().build()?;
    let host = geam::execution::TokioHost::new(executor.handle().clone());
    let program = geam_bindings::project().compile()?;
    let (bindings, functions) = geam_bindings::bind(HostedModuleBuilder::new(program)?)?;
    let mut module = bindings.seal()?;
    let mut state = geam_bindings::RunStateInputs {
        platform: HostProviderConfiguration::empty(),
    }
    .initialize()?;
    let mut echo = Vec::new();

    let (os, arch) = if cfg!(target_os = "macos") {
        ("darwin", "aarch64")
    } else {
        ("linux", "x86_64")
    };
    executor.block_on(
        module.with_execution(&host, &mut state, &mut echo, async |scope| {
            assert!(
                scope
                    .call(&functions.verify, (os.into(), arch.into()))
                    .await
                    .expect("original platform contract")
            );
        }),
    )?;
    Ok(())
}
