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
        term_size: HostProviderConfiguration::empty(),
    }
    .initialize()?;
    let mut echo = Vec::new();

    executor.block_on(
        module.with_execution(&host, &mut state, &mut echo, async |scope| {
            let _ = scope
                .call(&functions.get_size, ())
                .await
                .expect("source get call");
            let _ = scope
                .call(&functions.row_count, ())
                .await
                .expect("source rows call");
            let _ = scope
                .call(&functions.column_count, ())
                .await
                .expect("source columns call");
        }),
    )?;
    assert!(echo.is_empty());
    Ok(())
}
