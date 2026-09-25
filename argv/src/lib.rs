//! Native external for the unmodified `argv` 1.1.0 package.

use geam::provider::{Call, Configuration, InitializationError, StringValue};
use std::ffi::OsString;
use std::io;

/// The command-line values visible to one Geam provider run.
///
/// Embedding hosts can supply their own program identity and arguments without
/// changing process-global state. The default initializer captures the host
/// process at provider initialization.
pub struct RunState {
    runtime: StringValue,
    program: StringValue,
    arguments: Vec<StringValue>,
}

impl RunState {
    /// Use an explicit host snapshot for this provider run.
    pub fn new(runtime: String, program: String, arguments: Vec<String>) -> Self {
        Self {
            runtime: runtime.into(),
            program: program.into(),
            arguments: arguments.into_iter().map(Into::into).collect(),
        }
    }

    fn load(&self) -> (StringValue, StringValue, Vec<StringValue>) {
        (
            self.runtime.clone(),
            self.program.clone(),
            self.arguments.clone(),
        )
    }
}

trait ProcessSource {
    fn executable(&self) -> io::Result<OsString>;
    fn directory(&self) -> io::Result<OsString>;
    fn arguments(&self) -> Vec<OsString>;
}

struct SystemProcess;

impl ProcessSource for SystemProcess {
    fn executable(&self) -> io::Result<OsString> {
        std::env::current_exe().map(|path| path.into_os_string())
    }

    fn directory(&self) -> io::Result<OsString> {
        std::env::current_dir().map(|path| path.into_os_string())
    }

    fn arguments(&self) -> Vec<OsString> {
        std::env::args_os().skip(1).collect()
    }
}

fn capture(source: &impl ProcessSource) -> Result<RunState, InitializationError> {
    let runtime = source.executable().map_err(|error| {
        InitializationError::new(format!("reading runtime executable: {error}"))
    })?;
    let program = source
        .directory()
        .map_err(|error| InitializationError::new(format!("reading working directory: {error}")))?;
    let arguments = source.arguments();
    Ok(RunState::new(
        string_from_os(runtime),
        string_from_os(program),
        arguments.into_iter().map(string_from_os).collect(),
    ))
}

fn string_from_os(value: OsString) -> String {
    value.to_string_lossy().into_owned()
}

fn initialize(configuration: &Configuration) -> Result<RunState, InitializationError> {
    if let Some((key, _)) = configuration.iter().next() {
        return Err(InitializationError::new(format!(
            "unknown argv configuration key `{key}`"
        )));
    }
    capture(&SystemProcess)
}

#[geam::provider(
    package = "argv",
    state = RunState,
    initialize = initialize,
    modules = [argv],
)]
pub struct Component;

#[geam::module(path = "argv")]
mod argv {
    use super::{Call, RunState, StringValue};

    #[geam::function]
    fn r#do(
        #[geam::call] call: &mut Call<RunState>,
    ) -> (StringValue, StringValue, Vec<StringValue>) {
        call.state_mut().load()
    }
}

#[cfg(test)]
mod tests {
    use super::{ProcessSource, RunState, capture, initialize, string_from_os};
    use geam::provider::Configuration;
    use std::collections::BTreeMap;
    use std::ffi::OsString;
    use std::io;

    struct FixtureSource {
        runtime: Option<OsString>,
        program: Option<OsString>,
        arguments: Vec<OsString>,
    }

    impl ProcessSource for FixtureSource {
        fn executable(&self) -> io::Result<OsString> {
            self.runtime
                .clone()
                .ok_or_else(|| io::Error::other("executable unavailable"))
        }

        fn directory(&self) -> io::Result<OsString> {
            self.program
                .clone()
                .ok_or_else(|| io::Error::other("directory unavailable"))
        }

        fn arguments(&self) -> Vec<OsString> {
            self.arguments.clone()
        }
    }

    fn values(state: &RunState) -> (String, String, Vec<String>) {
        let (runtime, program, arguments) = state.load();
        (
            runtime.as_str().to_owned(),
            program.as_str().to_owned(),
            arguments
                .into_iter()
                .map(|argument| argument.as_str().to_owned())
                .collect(),
        )
    }

    #[test]
    fn source_snapshot_keeps_runtime_program_and_exact_argument_order() {
        let source = FixtureSource {
            runtime: Some("/bin/host".into()),
            program: Some("/project".into()),
            arguments: vec![
                "--flag".into(),
                "".into(),
                "key=value".into(),
                "한글".into(),
            ],
        };
        let state = capture(&source).expect("valid source");
        let expected = (
            "/bin/host".to_owned(),
            "/project".to_owned(),
            vec![
                "--flag".into(),
                "".into(),
                "key=value".into(),
                "한글".into(),
            ],
        );
        assert_eq!(values(&state), expected);
        assert_eq!(values(&state), expected);
    }

    #[test]
    fn source_failures_identify_executable_and_directory() {
        let source = FixtureSource {
            runtime: None,
            program: Some("/project".into()),
            arguments: vec![],
        };
        let error = capture(&source).err().expect("missing executable");
        assert!(error.reason().contains("reading runtime executable"));

        let source = FixtureSource {
            runtime: Some("/bin/host".into()),
            program: None,
            arguments: vec![],
        };
        let error = capture(&source).err().expect("missing directory");
        assert!(error.reason().contains("reading working directory"));
    }

    #[test]
    fn configuration_only_accepts_system_default() {
        let state = initialize(&Configuration::empty()).expect("system process");
        let (runtime, program, arguments) = values(&state);
        assert_eq!(
            runtime,
            std::env::current_exe()
                .expect("test executable")
                .to_string_lossy()
        );
        assert_eq!(
            program,
            std::env::current_dir()
                .expect("test directory")
                .to_string_lossy()
        );
        assert_eq!(
            arguments,
            std::env::args_os()
                .skip(1)
                .map(string_from_os)
                .collect::<Vec<_>>()
        );
        let configuration = Configuration::new(BTreeMap::from([("program".into(), "x".into())]));
        let error = initialize(&configuration)
            .err()
            .expect("unknown configuration key");
        assert!(error.reason().contains("program"));
    }

    #[test]
    fn explicit_state_preserves_host_identity_and_arguments() {
        let state = RunState::new("runtime".into(), "program".into(), vec!["a".into()]);
        assert_eq!(
            values(&state),
            ("runtime".into(), "program".into(), vec!["a".into()])
        );
    }

    #[cfg(unix)]
    #[test]
    fn invalid_unix_bytes_use_lossy_replacement_without_panicking() {
        use std::os::unix::ffi::OsStringExt;

        assert_eq!(
            string_from_os(OsString::from_vec(b"before-\xff-after".to_vec())),
            "before-�-after"
        );
    }

    #[cfg(windows)]
    #[test]
    fn invalid_windows_utf16_uses_lossy_replacement_without_panicking() {
        use std::os::windows::ffi::OsStringExt;

        assert_eq!(string_from_os(OsString::from_wide(&[0xd800])), "�");
    }
}
