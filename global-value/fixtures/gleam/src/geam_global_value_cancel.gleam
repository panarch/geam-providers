import gleam/erlang/process
import global_value

pub fn main() {
  let started = process.new_subject()
  let done = process.new_subject()
  let first =
    process.spawn_unlinked(fn() {
      let _ =
        global_value.create_with_unique_name("fixture.cancelled", fn() {
          process.send(started, Nil)
          process.sleep(1000)
          1
        })
    })

  let assert Ok(Nil) = process.receive(started, within: 100)
  process.kill(first)

  let _ =
    process.spawn_unlinked(fn() {
      let value =
        global_value.create_with_unique_name("fixture.cancelled", fn() { 2 })
      process.send(done, value)
    })
  let assert Ok(value) = process.receive(done, within: 100)
  value == 2
}
