//! Native `name/0` for the unmodified `operating_system` 1.0.1 package.

use geam::provider::StringValue;

#[geam::provider(package = "operating_system", modules = [operating_system])]
pub struct Component;

#[geam::module(path = "operating_system")]
mod operating_system {
    use super::StringValue;

    #[geam::function]
    fn name() -> StringValue {
        source_os_name(std::env::consts::OS).into()
    }

    fn source_os_name(target_os: &str) -> &str {
        match target_os {
            "windows" => "windows_nt",
            "macos" => "darwin",
            "solaris" | "illumos" => "sunos",
            other => other,
        }
    }

    #[cfg(test)]
    mod tests {
        use super::{name, source_os_name};

        #[test]
        fn maps_rust_targets_to_original_os_names() {
            for (target, expected) in [
                ("windows", "windows_nt"),
                ("macos", "darwin"),
                ("linux", "linux"),
                ("solaris", "sunos"),
                ("illumos", "sunos"),
                ("freebsd", "freebsd"),
            ] {
                assert_eq!(source_os_name(target), expected, "target OS: {target}");
            }
        }

        #[test]
        fn reports_the_current_native_target() {
            assert_eq!(name().as_str(), source_os_name(std::env::consts::OS));
        }
    }
}
