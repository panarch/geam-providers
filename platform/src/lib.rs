use geam::provider::StringValue;

#[geam::provider(package = "platform", modules = [platform])]
pub struct Component;

#[geam::module(path = "platform")]
mod platform {
    use super::StringValue;

    #[geam::function]
    fn runtime_() -> StringValue {
        "erlang".into()
    }

    #[geam::function]
    fn os_() -> StringValue {
        os_name(std::env::consts::OS).into()
    }

    fn os_name(target_os: &str) -> &str {
        match target_os {
            "windows" => "win32",
            "macos" => "darwin",
            "solaris" | "illumos" => "sunos",
            "aix" | "freebsd" | "linux" | "openbsd" | "netbsd" | "dragonfly" => target_os,
            _ => "unknown",
        }
    }

    #[geam::function]
    fn arch_() -> StringValue {
        arch_name(
            std::env::consts::OS,
            std::env::consts::ARCH,
            std::mem::size_of::<usize>(),
        )
        .into()
    }

    fn arch_name<'a>(target_os: &str, target_arch: &'a str, word_size: usize) -> &'a str {
        if target_os == "windows" {
            if word_size == 4 { "ia32" } else { "x64" }
        } else {
            target_arch
        }
    }

    #[cfg(test)]
    mod tests {
        use super::{arch_, arch_name, os_, os_name, runtime_};
        use geam::provider::StringValue;

        #[test]
        fn original_ffi_returns_strings_with_erlang_runtime() {
            let runtime: StringValue = runtime_();
            let os: StringValue = os_();
            let arch: StringValue = arch_();

            assert_eq!(runtime.as_str(), "erlang");
            assert_eq!(os.as_str(), os_name(std::env::consts::OS));
            assert_eq!(
                arch.as_str(),
                arch_name(
                    std::env::consts::OS,
                    std::env::consts::ARCH,
                    std::mem::size_of::<usize>()
                )
            );
        }

        #[test]
        fn erlang_os_names_cover_supported_unix_windows_and_unknown() {
            for (target, expected) in [
                ("windows", "win32"),
                ("macos", "darwin"),
                ("solaris", "sunos"),
                ("illumos", "sunos"),
                ("aix", "aix"),
                ("freebsd", "freebsd"),
                ("linux", "linux"),
                ("openbsd", "openbsd"),
                ("netbsd", "netbsd"),
                ("dragonfly", "dragonfly"),
                ("unsupported", "unknown"),
            ] {
                assert_eq!(os_name(target), expected, "target OS: {target}");
            }
        }

        #[test]
        fn unix_architecture_uses_target_arch_and_windows_uses_word_size() {
            assert_eq!(arch_name("macos", "aarch64", 8), "aarch64");
            assert_eq!(arch_name("linux", "x86_64", 8), "x86_64");
            assert_eq!(arch_name("linux", "riscv64", 8), "riscv64");
            assert_eq!(arch_name("windows", "aarch64", 4), "ia32");
            assert_eq!(arch_name("windows", "aarch64", 8), "x64");
        }
    }
}
