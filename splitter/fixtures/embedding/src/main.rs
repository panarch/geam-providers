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
        splitter: HostProviderConfiguration::empty(),
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
            let parts = scope
                .call(&functions.split_commas, ("a,,é,".into(),))
                .await
                .expect("source call");
            assert_eq!(
                (0..parts.len())
                    .map(|index| parts.read_item(index, Clone::clone).expect("list item"))
                    .collect::<Vec<_>>(),
                vec![
                    geam::StringValue::from("a"),
                    "".into(),
                    "é".into(),
                    "".into(),
                ]
            );
        }),
    )?;
    Ok(())
}
