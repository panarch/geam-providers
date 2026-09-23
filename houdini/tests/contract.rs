// The generated project() uses the embedding crate's manifest path; this test
// opens that same original Gleam project from the provider crate's manifest path.
#[allow(dead_code)]
#[path = "../fixtures/embedding/src/geam_bindings.rs"]
mod geam_bindings;

use geam::embedding::{FunctionDeclaration, HostedModuleBuilder, HostedProject, StringValue};
use geam::{
    HostProvider, HostProviderComponentRegistration, ModuleSource, PackageSource,
    compile_typed_host_program,
};

#[test]
fn registration_exposes_the_original_module_and_functions() {
    let providers = <geam_houdini::Component as HostProviderComponentRegistration<
        geam_bindings::Profile,
    >>::providers()
    .expect("static Houdini registration");
    assert_eq!(providers.len(), 1);
    assert_eq!(providers[0].package(), "houdini");
    assert_eq!(providers[0].module(), "houdini/internal/escape_erl");
    let names: Vec<_> = providers[0]
        .functions()
        .map(|schema| schema.name().as_str())
        .collect();
    assert_eq!(names, ["coerce", "slice"]);

    let mut state = geam_bindings::RunStateInputs {
        houdini: geam::HostProviderConfiguration::empty(),
    }
    .initialize()
    .expect("stateless provider initializes");
    let projected =
        <geam_houdini::Component as HostProvider<geam_bindings::Profile>>::project(&mut state);
    assert_eq!(*projected, ());
}

#[test]
fn original_houdini_runs_through_the_public_host_boundary() {
    let executor = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("single-threaded test executor");
    let host = geam::execution::TokioHost::new(executor.handle().clone());
    let project = HostedProject::new(
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
        houdini: geam::HostProviderConfiguration::empty(),
    }
    .initialize()
    .expect("provider initializes");
    let mut echo = Vec::new();

    executor
        .block_on(
            module.with_execution(&host, &mut state, &mut echo, async |scope| {
                assert!(
                    scope
                        .call(&functions.verify, ())
                        .await
                        .expect("original source contract")
                );
                for (input, expected) in [
                    ("&한글<", "&amp;한글&lt;"),
                    ("", ""),
                    ("abcdefgh<ijklmnop", "abcdefgh&lt;ijklmnop"),
                ] {
                    let actual = scope
                        .call(&functions.escape, (input.into(),))
                        .await
                        .expect("public escape call");
                    assert_eq!(actual.as_str(), expected);
                }
            }),
        )
        .expect("host execution completes");
}

#[test]
fn malformed_native_inputs_return_host_errors() {
    // This minimal source exposes Houdini's private declarations to exercise
    // caller-owned invalid input. The original Hex package remains unchanged.
    let program = compile_typed_host_program(
        "houdini",
        "houdini/internal/escape_erl",
        [PackageSource::new(
            "houdini",
            Vec::<String>::new(),
            [ModuleSource::new(
                "houdini/internal/escape_erl",
                "src/houdini/internal/escape_erl.gleam",
                r#"
@external(erlang, "houdini_ffi", "coerce")
pub fn coerce(value: a) -> b

@external(erlang, "binary", "part")
pub fn slice(value: BitArray, from: Int, size: Int) -> BitArray

pub fn bad_coerce() -> String { coerce(<<255>>) }
pub fn bad_slice() -> BitArray { slice(<<1, 2>>, -1, 1) }
"#,
            )],
        )],
        geam_bindings::host_providers().expect("Houdini provider registration"),
    )
    .expect("malformed-call source compiles");
    let (mut bindings, bad_coerce) = HostedModuleBuilder::new(program)
        .expect("host program plans")
        .function::<(), StringValue>(FunctionDeclaration::new("bad_coerce"))
        .expect("coerce source signature binds");
    let bad_slice = bindings
        .function::<(), geam::provider::BitArrayValue>(FunctionDeclaration::new("bad_slice"))
        .expect("slice source signature binds");
    let mut module = bindings.seal().expect("host module seals");
    let mut state = geam_bindings::RunStateInputs {
        houdini: geam::HostProviderConfiguration::empty(),
    }
    .initialize()
    .expect("provider initializes");
    let executor = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("single-threaded test executor");
    let host = geam::execution::TokioHost::new(executor.handle().clone());
    let mut echo = Vec::new();
    let (coerce_error, slice_error) = executor
        .block_on(
            module.with_execution(&host, &mut state, &mut echo, async |scope| {
                let coerce_error = scope
                    .call(&bad_coerce, ())
                    .await
                    .expect_err("invalid UTF-8 cannot become a String")
                    .to_string();
                let slice_error = scope
                    .call(&bad_slice, ())
                    .await
                    .expect_err("negative byte offset is invalid")
                    .to_string();
                (coerce_error, slice_error)
            }),
        )
        .expect("host execution completes");
    assert_eq!(
        coerce_error,
        "host function houdini::houdini/internal/escape_erl.coerce failed: houdini coerce: invalid conversion"
    );
    assert_eq!(
        slice_error,
        "host function houdini::houdini/internal/escape_erl.slice failed: houdini binary:part/3: invalid byte range"
    );
    assert!(echo.is_empty());
}
