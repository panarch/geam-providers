import platform

pub fn main() {
  assert platform.runtime() == platform.Erlang

  let expected_arch = case platform.os() {
    platform.Darwin -> platform.Arm64
    platform.Linux -> platform.X64
    _ -> panic as "This fixture expects macOS ARM64 or Linux x86_64"
  }
  assert platform.arch() == expected_arch
}
