import gleam/erlang/atom
import gleam/erlang/process
import gleam/otp/actor
import gleam/otp/factory_supervisor as factory
import gleam/otp/static_supervisor as static
import gleam/otp/supervision

fn start_child() {
  actor.new(Nil)
  |> actor.on_message(fn(state, _: Nil) { actor.continue(state) })
  |> actor.start
}

fn await_exit(pid: process.Pid) {
  let monitor = process.monitor(pid)
  let _ =
    process.new_selector()
    |> process.select_specific_monitor(monitor, fn(down) { down })
    |> process.selector_receive_forever
  let assert False = process.is_alive(pid)
}

pub fn main() {
  let started = process.new_subject()
  let group_child =
    supervision.worker(fn() {
      let assert Ok(actor.Started(pid, inbox)) = start_child()
      process.send(started, pid)
      Ok(actor.Started(pid, inbox))
    })
  let assert Ok(actor.Started(group, _)) =
    static.new(static.RestForOne)
    |> static.add(group_child)
    |> static.add(group_child)
    |> static.start
  let assert Ok(first) = process.receive(started, 0)
  let assert Ok(sibling) = process.receive(started, 0)
  let assert Ok(actor.Started(static, _)) =
    static.new(static.OneForOne)
    |> static.add(supervision.worker(start_child))
    |> static.start
  let assert Ok(actor.Started(factory_pid, factory)) =
    factory.worker_child(fn(_: Nil) { start_child() }) |> factory.start
  let assert Ok(actor.Started(child, _)) = factory.start_child(factory, Nil)
  process.unlink(static)
  process.unlink(factory_pid)
  process.unlink(group)
  // The host advances its monotonic clock to the last representable second.
  process.sleep(1)
  process.send_abnormal_exit(static, atom.create("shutdown"))
  process.send_abnormal_exit(factory_pid, atom.create("shutdown"))
  process.kill(first)
  await_exit(static)
  await_exit(factory_pid)
  await_exit(group)
  let assert False = process.is_alive(child)
  let assert False = process.is_alive(sibling)
  Nil
}
