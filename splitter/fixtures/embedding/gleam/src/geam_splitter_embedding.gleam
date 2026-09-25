import splitter

pub fn verify() -> Bool {
  let lines = splitter.new(["\r\n", "\n"])
  assert splitter.split(lines, "가\r\n나") == #("가", "\r\n", "나")
  assert splitter.split_before(lines, "가\r\n나") == #("가", "\r\n나")
  assert splitter.split_after(lines, "가\r\n나") == #("가\r\n", "나")
  assert splitter.would_split(lines, "가\n나")
  assert splitter.split_all(lines, "가\r\n나\n") == ["가", "나", ""]
  assert splitter.split(splitter.new(["b", "a"]), "ab") == #("", "a", "b")
  assert splitter.split_all(splitter.new(["a", "a"]), "aa") == ["", "", ""]

  let empty = splitter.new([])
  assert splitter.split(empty, "abc") == #("", "", "abc")
  assert splitter.split_all(empty, "abc") == ["abc"]
  assert splitter.split_all(splitter.new([","]), "") == [""]
  True
}

pub fn split_commas(input: String) -> List(String) {
  splitter.split_all(splitter.new([","]), input)
}
