import exception
import gleam/dynamic
import gleam/string

pub fn protect(body: fn() -> value) -> Result(value, String) {
  case exception.rescue(body) {
    Ok(value) -> Ok(value)
    Error(reason) -> Error(string.inspect(reason))
  }
}

type Wrapped {
  Wrapped(List(Int))
}

fn protect_with_cleanup(body: fn() -> value) -> Result(value, String) {
  protect(fn() {
    let value = body()
    assert True == exception.defer(fn() { value }, fn() { True })
    let assert Error(exception.Errored(_)) =
      exception.rescue(fn() {
        exception.on_crash(fn() { value }, fn() { panic })
      })
    use <- exception.defer(fn() { Nil })
    use <- exception.on_crash(fn() { panic as "cleanup on success" })
    value
  })
}

pub fn verify_values() -> Bool {
  assert Ok(5) == protect_with_cleanup(fn() { 5 })
  assert Ok(1.5) == protect_with_cleanup(fn() { 1.5 })
  assert Ok("retained text") == protect_with_cleanup(fn() { "retained text" })
  assert Ok(Nil) == protect_with_cleanup(fn() { Nil })
  assert Ok(True) == protect_with_cleanup(fn() { True })
  assert Ok(<<1, 2, 3>>) == protect_with_cleanup(fn() { <<1, 2, 3>> })
  assert Ok([1, 2, 3]) == protect_with_cleanup(fn() { [1, 2, 3] })
  assert Ok(#("text", 8)) == protect_with_cleanup(fn() { #("text", 8) })
  assert Ok(Ok(["nested"])) == protect_with_cleanup(fn() { Ok(["nested"]) })
  assert Ok(Wrapped([4, 5])) == protect_with_cleanup(fn() { Wrapped([4, 5]) })
  let assert [codepoint] = string.to_utf_codepoints("한")
  assert Ok(codepoint) == protect_with_cleanup(fn() { codepoint })
  let assert Ok(callback) =
    protect_with_cleanup(fn() { fn(value) { value + 1 } })
  assert callback(4) == 5
  let dynamic_value = dynamic.int(17)
  let assert Ok(retained) = protect_with_cleanup(fn() { dynamic_value })
  assert dynamic.classify(retained) == "Int"
  assert string.inspect(retained) == "17"
  assert ["deferred"] == exception.defer(fn() { Nil }, fn() { ["deferred"] })
  assert #(7, "kept")
    == exception.on_crash(fn() { panic as "cleanup on success" }, fn() {
      #(7, "kept")
    })
  True
}
