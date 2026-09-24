// The provider crate's owner tests import the generated bindings by path.
// Rust resolves `mod program` beside that imported file in this context.
include!("geam_bindings/program.rs");
