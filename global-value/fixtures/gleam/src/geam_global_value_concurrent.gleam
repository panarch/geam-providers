import gleam/erlang/process
import global_value

pub fn main() {
  let replies = process.new_subject()
  let _ =
    process.spawn_unlinked(fn() {
      let value =
        global_value.create_with_unique_name("fixture.concurrent", fn() {
          process.sleep(1)
          process.self()
        })
      process.send(replies, value)
    })
  let _ =
    process.spawn_unlinked(fn() {
      let value =
        global_value.create_with_unique_name("fixture.concurrent", fn() {
          process.sleep(1)
          process.self()
        })
      process.send(replies, value)
    })
  let assert Ok(first) = process.receive(replies, within: 1000)
  let assert Ok(second) = process.receive(replies, within: 1000)
  first == second
}
