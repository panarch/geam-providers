mod checks;
#[allow(dead_code)]
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
