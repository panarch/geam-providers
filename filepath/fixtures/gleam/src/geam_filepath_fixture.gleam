import filepath

pub fn main() {
  assert filepath.join("/usr/local", "bin") == "/usr/local/bin"
  assert filepath.split("/usr/local/bin") == ["/", "usr", "local", "bin"]
  assert filepath.split_unix("C:\\work\\file.txt") == ["C:\\work\\file.txt"]
  assert filepath.split_windows("C:\\work\\file.txt")
    == ["c:/", "work", "file.txt"]
  assert filepath.base_name("/usr/local/bin") == "bin"
  let assert Ok("/usr/bin") = filepath.expand("/usr/local/../bin")
}
