// Open the original Gleam project from this crate, as the generated embedding
// consumer does from its own manifest directory.
#[allow(dead_code)]
#[path = "../fixtures/embedding/src/geam_bindings.rs"]
mod geam_bindings;

use geam::HostProviderComponentRegistration;

#[test]
fn registration_matches_the_original_external() {
    let providers = <geam_term_size::Component as HostProviderComponentRegistration<
        geam_bindings::Profile,
    >>::providers()
    .expect("static term_size registration");
    assert_eq!(providers.len(), 1);
    assert_eq!(providers[0].package(), "term_size");
    assert_eq!(providers[0].module(), "term_size");
    let names: Vec<_> = providers[0]
        .functions()
        .map(|schema| schema.name().as_str())
        .collect();
    assert_eq!(names, ["get"]);

    geam_bindings::RunStateInputs {
        term_size: geam::HostProviderConfiguration::empty(),
    }
    .initialize()
    .expect("stateless provider initializes");
}

#[cfg(unix)]
mod terminal_contract {
    use super::geam_bindings;
    use geam::HostProviderConfiguration;
    use geam::embedding::{BigInt, HostedModuleBuilder};
    use nix::pty::{OpenptyResult, Winsize, openpty};
    use std::io::{BufRead, BufReader, Read, Write};
    use std::os::fd::AsRawFd;
    use std::process::{Command, Stdio};
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn original_gleam_functions_report_missing_tty() {
        run_with_streams("none", None, None, None);
    }

    #[test]
    fn original_gleam_functions_follow_stream_priority() {
        run_with_streams("stdout", Some((37, 101)), Some((43, 109)), Some((59, 127)));
        run_with_streams("stderr", None, Some((43, 109)), Some((59, 127)));
        run_with_streams("stdin", None, None, Some((59, 127)));
    }

    #[test]
    fn original_gleam_functions_see_resized_tty_in_same_process() {
        let OpenptyResult { master, slave } = sized_pty((43, 109));
        let mut command = child_command("resize");
        command.stderr(Stdio::from(slave));
        let mut child = command.spawn().expect("spawn hosted source under PTY");
        let stdout = child.stdout.take().expect("child stdout pipe");
        let (sender, ready) = mpsc::channel();
        let output_reader = std::thread::spawn(move || {
            let mut transcript = Vec::new();
            for line in BufReader::new(stdout).lines() {
                match line {
                    Ok(line) if line.contains("READY") => {
                        let _ = sender.send(());
                        transcript.push(line);
                    }
                    Ok(line) => transcript.push(line),
                    Err(_) => break,
                }
            }
            transcript.join("\n")
        });

        if ready.recv_timeout(Duration::from_secs(60)).is_err() {
            let _ = child.kill();
            let _ = child.wait();
            let transcript = output_reader.join().expect("child output reader exits");
            panic!("hosted source did not finish its initial PTY query: {transcript}");
        }

        let changed = winsize((47, 113));
        let result =
            unsafe { nix::libc::ioctl(master.as_raw_fd(), nix::libc::TIOCSWINSZ, &changed) };
        assert_eq!(result, 0, "resize PTY before the second source call");
        child
            .stdin
            .take()
            .expect("child synchronization pipe")
            .write_all(b"g")
            .expect("release child after resize");
        let status = child.wait().expect("resized child exits");
        let transcript = output_reader.join().expect("child output reader exits");
        assert!(status.success(), "resized child failed: {transcript}");
    }

    // This libtest function is re-entered as a child with controlled stdio.
    // In the ordinary parent test run it only provides the child entry point.
    #[test]
    fn source_contract_child() {
        let Ok(scenario) = std::env::var("GEAM_TERM_SIZE_SCENARIO") else {
            return;
        };
        let (initial, resized) = match scenario.as_str() {
            "none" => (None, None),
            "stdout" => (Some((37, 101)), None),
            "stderr" => (Some((43, 109)), None),
            "stdin" => (Some((59, 127)), None),
            "resize" => (Some((43, 109)), Some((47, 113))),
            other => panic!("unexpected child scenario: {other}"),
        };

        let executor = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("single-threaded source executor");
        let host = geam::execution::TokioHost::new(executor.handle().clone());
        let project = geam::embedding::HostedProject::new(
            concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/embedding/gleam"),
            geam_bindings::ROOT_MODULE,
            geam_bindings::host_providers,
        );
        let program = project.compile().expect("original Hex package compiles");
        let (bindings, functions) =
            geam_bindings::bind(HostedModuleBuilder::new(program).expect("host program plans"))
                .expect("generated source signatures bind");
        let mut module = bindings.seal().expect("host module seals");
        let mut state = geam_bindings::RunStateInputs {
            term_size: HostProviderConfiguration::empty(),
        }
        .initialize()
        .expect("provider initializes");
        let mut echo = Vec::new();

        executor
            .block_on(
                module.with_execution(&host, &mut state, &mut echo, async |scope| {
                    assert_eq!(
                        scope
                            .call(&functions.get_size, ())
                            .await
                            .expect("source get call"),
                        expected_size(initial)
                    );
                    assert_eq!(
                        scope
                            .call(&functions.row_count, ())
                            .await
                            .expect("source rows call"),
                        initial.map(|(rows, _)| BigInt::from(rows)).ok_or(())
                    );
                    assert_eq!(
                        scope
                            .call(&functions.column_count, ())
                            .await
                            .expect("source columns call"),
                        initial.map(|(_, columns)| BigInt::from(columns)).ok_or(())
                    );

                    if let Some(expected) = resized {
                        println!("READY");
                        std::io::stdout().flush().expect("flush readiness marker");
                        let mut release = [0];
                        std::io::stdin()
                            .read_exact(&mut release)
                            .expect("wait for PTY resize");
                        assert_eq!(release, *b"g");
                        assert_eq!(
                            scope
                                .call(&functions.get_size, ())
                                .await
                                .expect("source get after resize"),
                            expected_size(Some(expected))
                        );
                    }
                }),
            )
            .expect("host execution completes");
        assert!(echo.is_empty());
    }

    fn expected_size(size: Option<(u16, u16)>) -> Result<(BigInt, BigInt), ()> {
        size.map(|(rows, columns)| (BigInt::from(rows), BigInt::from(columns)))
            .ok_or(())
    }

    fn child_command(scenario: &str) -> Command {
        let mut command = Command::new(std::env::current_exe().expect("current contract test"));
        command
            .args([
                "--exact",
                "terminal_contract::source_contract_child",
                "--nocapture",
            ])
            .env("GEAM_TERM_SIZE_SCENARIO", scenario)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        command
    }

    fn run_with_streams(
        scenario: &str,
        stdout: Option<(u16, u16)>,
        stderr: Option<(u16, u16)>,
        stdin: Option<(u16, u16)>,
    ) {
        let mut command = child_command(scenario);
        let mut masters = Vec::new();
        if let Some(size) = stdout {
            let OpenptyResult { master, slave } = sized_pty(size);
            command.stdout(Stdio::from(slave));
            masters.push(master);
        }
        if let Some(size) = stderr {
            let OpenptyResult { master, slave } = sized_pty(size);
            command.stderr(Stdio::from(slave));
            masters.push(master);
        }
        if let Some(size) = stdin {
            let OpenptyResult { master, slave } = sized_pty(size);
            command.stdin(Stdio::from(slave));
            masters.push(master);
        }
        let output = command
            .output()
            .expect("run hosted source with controlled stdio");
        assert!(
            output.status.success(),
            "{scenario} child failed: stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn sized_pty((rows, columns): (u16, u16)) -> OpenptyResult {
        openpty(Some(&winsize((rows, columns))), None).expect("open sized PTY")
    }

    fn winsize((rows, columns): (u16, u16)) -> Winsize {
        Winsize {
            ws_row: rows,
            ws_col: columns,
            ws_xpixel: 0,
            ws_ypixel: 0,
        }
    }
}
