import platform

pub fn verify(expected_os: String, expected_arch: String) -> Bool {
  let os = case expected_os {
    "darwin" -> platform.Darwin
    "linux" -> platform.Linux
    "win32" -> platform.Win32
    other -> platform.OtherOs(other)
  }
  let arch = case expected_arch {
    "aarch64" -> platform.Arm64
    "x86_64" | "x64" -> platform.X64
    "ia32" -> platform.X86
    other -> platform.OtherArch(other)
  }

  assert platform.runtime() == platform.Erlang
  assert platform.os() == os
  assert platform.arch() == arch
  True
}
