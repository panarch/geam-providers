mod geam_bindings;

use geam::HostProviderConfiguration;
use geam::embedding::HostedModuleBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let executor = tokio::runtime::Builder::new_current_thread().build()?;
    let host = geam::execution::TokioHost::new(executor.handle().clone());
    let program = geam_bindings::project().compile()?;
    let builder = HostedModuleBuilder::new(program)?;
    let (bindings, functions) = geam_bindings::bind(builder)?;
    let mut module = bindings.seal()?;
    let mut state = geam_bindings::RunStateInputs {
        houdini: HostProviderConfiguration::empty(),
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
            let escaped = scope
                .call(&functions.escape, ("&한글<".into(),))
                .await
                .expect("public escape call");
            assert_eq!(escaped.as_str(), "&amp;한글&lt;");
        }),
    )?;
    Ok(())
}
