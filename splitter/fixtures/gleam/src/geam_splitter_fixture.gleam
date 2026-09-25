import splitter

pub fn main() {
  let lines = splitter.new(["\r\n", "\n"])
  assert splitter.split(lines, "first\r\nsecond")
    == #("first", "\r\n", "second")
  assert splitter.split_before(lines, "first\r\nsecond")
    == #("first", "\r\nsecond")
  assert splitter.split_after(lines, "first\r\nsecond")
    == #("first\r\n", "second")
  assert splitter.would_split(lines, "first\nsecond")
  assert splitter.split_all(lines, "first\r\nsecond\n")
    == ["first", "second", ""]

  let overlapping = splitter.new(["a", "ab"])
  assert splitter.split(overlapping, "zab!") == #("z", "ab", "!")
  assert splitter.split_all(overlapping, "zab!a") == ["z", "!", ""]
  let ordered = splitter.new(["ab", "a"])
  assert splitter.split(ordered, "zab!") == #("z", "ab", "!")
  assert overlapping != ordered
  let first_position = splitter.new(["b", "a"])
  assert splitter.split(first_position, "ab") == #("", "a", "b")
  let duplicate = splitter.new(["a", "a"])
  assert splitter.split_all(duplicate, "aa") == ["", "", ""]

  let comma = splitter.new([","])
  assert splitter.split_all(comma, "a,,b,") == ["a", "", "b", ""]
  assert splitter.split_all(comma, "") == [""]
  assert splitter.split(comma, "plain") == #("plain", "", "")
  assert splitter.split_before(comma, "plain") == #("plain", "")
  assert splitter.split_after(comma, "plain") == #("plain", "")
  assert !splitter.would_split(comma, "plain")

  let unicode = splitter.new(["é"])
  assert splitter.split(unicode, "가é나") == #("가", "é", "나")
  assert splitter.split_before(unicode, "가é나") == #("가", "é나")
  assert splitter.split_after(unicode, "가é나") == #("가é", "나")
  assert splitter.split_all(unicode, "éé") == ["", "", ""]

  let empty = splitter.new([])
  assert empty == splitter.new([""])
  assert splitter.split(empty, "abc") == #("", "", "abc")
  assert splitter.split_before(empty, "abc") == #("", "abc")
  assert splitter.split_after(empty, "abc") == #("", "abc")
  assert !splitter.would_split(empty, "abc")
  assert splitter.split_all(empty, "abc") == ["abc"]
  assert splitter.split(splitter.new(["", ","]), "a,b") == #("a", ",", "b")
}
