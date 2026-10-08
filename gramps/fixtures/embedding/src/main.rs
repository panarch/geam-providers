mod checks;
// Geam verifies these generated files with `geam embedding check`.
#[allow(dead_code)]
#[rustfmt::skip]
mod geam_bindings;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    checks::run()
}

#[cfg(test)]
mod tests {
    #[test]
    fn live_and_prepared_scopes_repeat_and_recover_without_leaking_contexts() {
        super::checks::run().unwrap();
    }
}
