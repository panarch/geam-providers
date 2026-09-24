import global_value

pub fn main() {
  let first =
    global_value.create_with_unique_name("fixture.shared", fn() {
      #(41, "same")
    })
  let second =
    global_value.create_with_unique_name("fixture.shared", fn() {
      panic as "shared value initialized twice"
    })
  let other = global_value.create_with_unique_name("fixture.other", fn() { 7 })
  let result: Result(String, Nil) =
    global_value.create_with_unique_name("fixture.result", fn() { Ok("cached") })
  let callback =
    global_value.create_with_unique_name("fixture.function", fn() {
      fn(value: Int) { value + 5 }
    })

  let assert True = first == second
  let assert True = first == #(41, "same")
  let assert True = other == 7
  let assert True = result == Ok("cached")
  let assert True = callback(2) == 7
}
