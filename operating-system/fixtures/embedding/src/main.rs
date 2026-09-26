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
        operating_system: HostProviderConfiguration::empty(),
    }
    .initialize()?;
    let mut echo = Vec::new();
    let expected = match std::env::consts::OS {
        "windows" => "windows_nt",
        "macos" => "darwin",
        "solaris" | "illumos" => "sunos",
        other => other,
    };

    executor.block_on(
        module.with_execution(&host, &mut state, &mut echo, async |scope| {
            let actual = scope
                .call(&functions.name, ())
                .await
                .expect("original operating_system.name contract");
            assert_eq!(actual.as_str(), expected);
        }),
    )?;
    Ok(())
}
