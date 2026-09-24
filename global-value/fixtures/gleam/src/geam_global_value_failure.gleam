import gleam/erlang/process
import global_value

pub fn main() {
  let started = process.new_subject()
  let done = process.new_subject()
  let _ =
    process.spawn_unlinked(fn() {
      let failed: Int =
        global_value.create_with_unique_name("fixture.failed", fn() {
          process.send(started, Nil)
          panic as "initializer failed before storing"
        })
      let _ = failed
    })

  let assert Ok(Nil) = process.receive(started, within: 100)
  let _ =
    process.spawn_unlinked(fn() {
      let value =
        global_value.create_with_unique_name("fixture.failed", fn() { 2 })
      process.send(done, value)
    })
  let assert Ok(value) = process.receive(done, within: 100)
  value == 2
}
