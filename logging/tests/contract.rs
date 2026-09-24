// Generated bindings compile this fixture's original Gleam package at the
// provider crate's manifest path instead of the embedding crate's path.
#[allow(dead_code)]
#[path = "../fixtures/embedding/src/geam_bindings.rs"]
mod geam_bindings;

use geam::HostComponentProfile;
use geam::embedding::{HostedModuleBuilder, HostedProject};
use geam_logging::RunState;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
struct RecordingWriter(Arc<Mutex<Vec<u8>>>);

impl RecordingWriter {
    fn text(&self) -> String {
        String::from_utf8(self.0.lock().expect("recorded bytes").clone()).expect("UTF-8 log output")
    }
}

impl Write for RecordingWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0
            .lock()
            .expect("recorded bytes")
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct FailingWriter;

impl Write for FailingWriter {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::Error::other("injected output failure"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn source_project() -> HostedProject<geam_bindings::Profile> {
    HostedProject::new(
        concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/embedding/gleam"),
        geam_bindings::ROOT_MODULE,
        geam_bindings::host_providers,
    )
}

fn run_state(
    writer: impl Write + Send + 'static,
    no_color: Option<&str>,
) -> geam_bindings::RunState {
    let mut state = geam_bindings::RunStateInputs {
        logging: geam::HostProviderConfiguration::empty(),
    }
    .initialize()
    .expect("provider initializes");
    *<geam_bindings::Profile as HostComponentProfile<geam_logging::Component>>::component_state(
        &mut state,
    ) = RunState::with_writer(writer, no_color, None);
    state
}

#[test]
fn original_package_filters_and_formats_all_levels_through_geam() {
    let executor = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("test executor");
    let host = geam::execution::TokioHost::new(executor.handle().clone());
    let program = source_project()
        .compile()
        .expect("original Hex package compiles");
    let (bindings, functions) =
        geam_bindings::bind(HostedModuleBuilder::new(program).expect("program plans"))
            .expect("source signatures bind");
    let mut module = bindings.seal().expect("module seals");
    let output = RecordingWriter::default();
    let mut state = run_state(output.clone(), Some("1"));
    let mut echo = Vec::new();

    executor
        .block_on(
            module.with_execution(&host, &mut state, &mut echo, async |scope| {
                scope
                    .call(&functions.log_all_levels, ())
                    .await
                    .expect("all original levels");
                scope
                    .call(&functions.filter_and_reset, ())
                    .await
                    .expect("original level transitions");
            }),
        )
        .expect("source execution completes");

    assert_eq!(
        output.text(),
        "EMRG emergency\nALRT alert\nCRIT critical\nEROR error\nWARN warning\nNTCE notice\nINFO info\nDEBG debug\nEROR visible error\nINFO visible after reset\n"
    );

    let colored = RecordingWriter::default();
    let mut colored_state = run_state(colored.clone(), None);
    executor
        .block_on(
            module.with_execution(&host, &mut colored_state, &mut echo, async |scope| {
                scope
                    .call(&functions.log_one, ())
                    .await
                    .expect("independent run logs");
            }),
        )
        .expect("second run completes");
    assert_eq!(colored.text(), "\x1b[1;34mINFO\x1b[0m one\n");

    let mut failing_state = run_state(FailingWriter, Some("1"));
    executor
        .block_on(
            module.with_execution(&host, &mut failing_state, &mut echo, async |scope| {
                let failure = scope
                    .call(&functions.log_one, ())
                    .await
                    .expect_err("host output failure reaches caller");
                assert!(failure.to_string().contains("injected output failure"));
            }),
        )
        .expect("failed call stays in hosted execution");
}
