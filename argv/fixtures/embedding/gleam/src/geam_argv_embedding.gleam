import argv

pub fn verify(
  runtime: String,
  program: String,
  first: String,
  second: String,
) -> Bool {
  let value = argv.load()
  assert value == argv.load()
  assert value.runtime == runtime
  assert value.program == program
  assert value.arguments == [first, second]
  True
}

pub fn verify_empty(runtime: String, program: String) -> Bool {
  let value = argv.load()
  assert value.runtime == runtime
  assert value.program == program
  assert value.arguments == []
  True
}
