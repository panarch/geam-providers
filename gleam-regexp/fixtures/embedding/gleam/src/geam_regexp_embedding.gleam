import gleam/option.{None, Some}
import gleam/regexp

pub fn decorate(pattern: String, input: String) -> Result(String, String) {
  case regexp.from_string(pattern) {
    Ok(compiled) ->
      Ok(
        regexp.match_map(compiled, input, fn(found) {
          "[" <> found.content <> "]"
        }),
      )
    Error(regexp.CompileError(message, _)) -> Error(message)
  }
}

pub fn verify() -> Bool {
  let assert Ok(words) = regexp.from_string("(\\w+)")
  let assert Ok(same_words) = regexp.from_string("(\\w+)")
  let assert Ok(other_words) =
    regexp.compile(
      "(\\w+)",
      with: regexp.Options(case_insensitive: True, multi_line: False),
    )
  assert words == same_words
  assert words != other_words
  assert regexp.check(words, "é")
  assert !regexp.check(words, "---")
  assert regexp.scan(words, "é, joe")
    == [
      regexp.Match("é", [Some("é")]),
      regexp.Match("joe", [Some("joe")]),
    ]

  let assert Ok(optional) = regexp.from_string("(a)?b")
  assert regexp.scan(optional, "ab b")
    == [
      regexp.Match("ab", [Some("a")]),
      regexp.Match("b", []),
    ]
  let assert Ok(empty_before) = regexp.from_string("(a*)(b)")
  assert regexp.scan(empty_before, "b")
    == [
      regexp.Match("b", [None, Some("b")]),
    ]
  assert regexp.scan(words, "---") == []
  let assert Ok(edges) = regexp.from_string("^|$")
  assert regexp.scan(edges, "a") == [regexp.Match("", []), regexp.Match("", [])]
  assert regexp.match_map(edges, "a", fn(_) { "|" }) == "|a|"

  let assert Ok(comma) = regexp.from_string(",\\s*")
  assert regexp.split(comma, ",é,b,") == ["", "é", "b", ""]
  assert regexp.split(comma, "plain") == ["plain"]
  let assert Ok(optional) = regexp.from_string("(a)?b")
  assert regexp.split(optional, "1b2ab3") == ["1", "", "2", "a", "3"]

  let assert Ok(names) = regexp.from_string("(\\w+)-(\\d+)")
  assert regexp.replace(names, "é-12 b-3", "$2:$1") == "12:é 3:b"
  assert regexp.replace(names, "plain", "unused") == "plain"
  let assert Ok(named) = regexp.from_string("(?P<word>\\w+)")
  assert regexp.replace(named, "foo", "${word}!") == "foo!"
  assert regexp.replace(named, "foo", "$$") == "$"
  assert regexp.match_map(words, "é, joe", fn(found) {
      "[" <> found.content <> "]"
    })
    == "[é], [joe]"
  assert regexp.match_map(words, "---", fn(_) {
      panic as "callback must not run without a match"
    })
    == "---"

  let assert Ok(lines) =
    regexp.compile(
      "^foo",
      with: regexp.Options(case_insensitive: True, multi_line: True),
    )
  assert regexp.check(lines, "bar\nFOO")
  let assert Ok(single_line) =
    regexp.compile(
      "^foo",
      with: regexp.Options(case_insensitive: True, multi_line: False),
    )
  assert !regexp.check(single_line, "bar\nFOO")

  let assert Error(regexp.CompileError(message, byte_index)) =
    regexp.from_string("[0-9")
  assert message != ""
  assert byte_index == 4
  let assert Error(regexp.CompileError(_, _)) = regexp.from_string("(?=a)")
  True
}

pub fn callback_failure() -> String {
  let assert Ok(words) = regexp.from_string("\\w+")
  regexp.match_map(words, "one two", fn(found) {
    case found.content {
      "one" -> "first"
      _ -> panic as "regexp callback failed"
    }
  })
}
