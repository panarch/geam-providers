import filepath

pub fn verify(expected_windows: Bool) -> Bool {
  let path = "C:\\work\\file.txt"
  let unix_segments = [path]
  let windows_segments = ["c:/", "work", "file.txt"]

  assert filepath.split_unix(path) == unix_segments
  assert filepath.split_windows(path) == windows_segments
  assert filepath.split(path)
    == case expected_windows {
      True -> windows_segments
      False -> unix_segments
    }

  assert filepath.base_name(path)
    == case expected_windows {
      True -> "file.txt"
      False -> path
    }

  let assert Ok(expanded) = filepath.expand("docs\\temp\\..\\README.md")
  assert expanded
    == case expected_windows {
      True -> "docs/README.md"
      False -> "docs\\temp\\..\\README.md"
    }

  True
}
