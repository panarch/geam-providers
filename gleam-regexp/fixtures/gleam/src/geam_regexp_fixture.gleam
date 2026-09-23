import gleam/option.{None, Some}
import gleam/regexp

pub fn main() {
  let assert Ok(words) = regexp.from_string("(\\w+)")
  let assert Ok(same_words) = regexp.from_string("(\\w+)")
  assert words == same_words
  assert regexp.check(words, "hello")
  assert !regexp.check(words, "---")
  assert regexp.scan(words, "hi, joe")
    == [
      regexp.Match("hi", [Some("hi")]),
      regexp.Match("joe", [Some("joe")]),
    ]

  let assert Ok(optional) = regexp.from_string("(a)?b")
  assert regexp.scan(optional, "ab b")
    == [
      regexp.Match("ab", [Some("a")]),
      regexp.Match("b", []),
    ]

  let assert Ok(parts) = regexp.from_string(",\\s*")
  assert regexp.split(parts, "a, b,c") == ["a", "b", "c"]

  let assert Ok(names) = regexp.from_string("(\\w+)-(\\d+)")
  assert regexp.replace(names, "foo-12 bar-3", "$2:$1") == "12:foo 3:bar"
  assert regexp.match_map(words, "hi, joe", fn(found) {
      "[" <> found.content <> "]"
    })
    == "[hi], [joe]"
  assert regexp.match_map(words, "---", fn(_) { "unused" }) == "---"

  let assert Ok(lines) =
    regexp.compile(
      "^foo",
      with: regexp.Options(case_insensitive: True, multi_line: True),
    )
  assert regexp.check(lines, "bar\nFOO")

  let assert Ok(empty) = regexp.from_string("(a*)b")
  assert regexp.scan(empty, "b") == [regexp.Match("b", [])]
  let assert Ok(empty_before) = regexp.from_string("(a*)(b)")
  assert regexp.scan(empty_before, "b")
    == [
      regexp.Match("b", [None, Some("b")]),
    ]
  let assert Ok(edges) = regexp.from_string("^|$")
  assert regexp.scan(edges, "a") == [regexp.Match("", []), regexp.Match("", [])]
  assert regexp.match_map(edges, "a", fn(_) { "|" }) == "|a|"

  let assert Error(regexp.CompileError(_, byte_index)) =
    regexp.from_string("[0-9")
  assert byte_index == 4
  let assert Error(regexp.CompileError(_, _)) = regexp.from_string("(?=a)")
}
