import argv

pub fn main() {
  let first = argv.load()
  assert first == argv.load()
  assert first.runtime != ""
  assert first.program != ""

  case first.arguments {
    [] -> Nil
    ["--flag", "", "key=value", "한글"] -> Nil
    _ -> panic as "unexpected application arguments"
  }
}
