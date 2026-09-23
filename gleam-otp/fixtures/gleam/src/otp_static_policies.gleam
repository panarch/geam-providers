import gleam/erlang/process
import gleam/list
import gleam/otp/actor
import gleam/otp/static_supervisor as static
import gleam/otp/supervision

fn await_exit(pid: process.Pid) {
  let monitor = process.monitor(pid)
  let _ =
    process.new_selector()
    |> process.select_specific_monitor(monitor, fn(down) { down })
    |> process.selector_receive_forever
  let assert False = process.is_alive(pid)
}

fn child(started, number, restart) {
  supervision.worker(fn() {
    let assert Ok(actor.Started(pid, inbox)) =
      actor.new(Nil)
      |> actor.on_message(fn(_, _: Nil) { actor.stop() })
      |> actor.start
    process.send(started, #(number, pid, inbox))
    Ok(actor.Started(pid, Nil))
  })
  |> supervision.restart(restart)
}

fn expect_started(started, number) {
  let #(tag, pid, _) = process.receive_forever(started)
  let assert True = tag == number
  pid
}

fn restart_group(strategy: static.Strategy, expected: List(Int)) {
  let started = process.new_subject()
  let assert Ok(actor.Started(supervisor, _)) =
    static.new(strategy)
    |> static.add(child(started, 1, supervision.Permanent))
    |> static.add(child(started, 2, supervision.Permanent))
    |> static.add(child(started, 3, supervision.Temporary))
    |> static.start
  let first = expect_started(started, 1)
  let second = expect_started(started, 2)
  let third = expect_started(started, 3)
  process.kill(second)
  let restarted =
    expected
    |> list.map(fn(number) { expect_started(started, number) })
  let assert Error(Nil) = process.receive(started, 0)
  case strategy {
    static.OneForOne -> {
      let assert True = process.is_alive(first)
      let assert True = process.is_alive(third)
    }
    static.OneForAll -> {
      let assert False = process.is_alive(first)
      let assert False = process.is_alive(third)
    }
    static.RestForOne -> {
      let assert True = process.is_alive(first)
      let assert False = process.is_alive(third)
    }
  }
  let assert False = process.is_alive(second)
  let assert True = list.all(restarted, process.is_alive)
  process.unlink(supervisor)
  process.kill(supervisor)
  await_exit(supervisor)
  let assert False = process.is_alive(first)
  let assert False = list.any(restarted, process.is_alive)
}

fn inactive_transient_restarts_with_group(strategy: static.Strategy) {
  let started = process.new_subject()
  let assert Ok(actor.Started(supervisor, _)) =
    static.new(strategy)
    |> static.add(child(started, 1, supervision.Permanent))
    |> static.add(child(started, 2, supervision.Transient))
    |> static.start
  let first = expect_started(started, 1)
  let assert Ok(#(2, transient, inbox)) = process.receive(started, 0)
  process.unlink(supervisor)
  process.send(inbox, Nil)
  await_exit(transient)
  let assert True = process.is_alive(supervisor)
  process.kill(first)
  let replacement = expect_started(started, 1)
  let resumed = expect_started(started, 2)
  let assert True = replacement != first
  let assert True = resumed != transient
  let assert True = process.is_alive(resumed)
  process.kill(supervisor)
  await_exit(supervisor)
  let assert False = process.is_alive(replacement)
  let assert False = process.is_alive(resumed)
}

fn significant_child(started, number) {
  child(started, number, supervision.Temporary)
  |> supervision.significant(True)
}

fn auto_shutdown_any(abnormal: Bool) {
  let started = process.new_subject()
  let assert Ok(actor.Started(supervisor, _)) =
    static.new(static.OneForOne)
    |> static.auto_shutdown(static.AnySignificant)
    |> static.add(significant_child(started, 1))
    |> static.add(significant_child(started, 2))
    |> static.start
  let assert Ok(#(1, first, inbox)) = process.receive(started, 0)
  let second = expect_started(started, 2)
  process.unlink(supervisor)
  case abnormal {
    True -> process.kill(first)
    False -> process.send(inbox, Nil)
  }
  await_exit(supervisor)
  let assert False = process.is_alive(first)
  let assert False = process.is_alive(second)
}

fn transient_significant_restarts_before_normal_shutdown() {
  let started = process.new_subject()
  let significant =
    child(started, 1, supervision.Transient)
    |> supervision.significant(True)
  let assert Ok(actor.Started(supervisor, _)) =
    static.new(static.OneForOne)
    |> static.auto_shutdown(static.AnySignificant)
    |> static.add(significant)
    |> static.start
  let assert Ok(#(1, first, _)) = process.receive(started, 0)
  process.unlink(supervisor)
  process.kill(first)
  let #(tag, replacement, inbox) = process.receive_forever(started)
  let assert True = tag == 1
  let assert True = replacement != first
  let assert True = process.is_alive(supervisor)
  process.send(inbox, Nil)
  await_exit(supervisor)
  let assert False = process.is_alive(replacement)
}

fn auto_shutdown_all() {
  let started = process.new_subject()
  let assert Ok(actor.Started(supervisor, _)) =
    static.new(static.OneForOne)
    |> static.auto_shutdown(static.AllSignificant)
    |> static.add(significant_child(started, 1))
    |> static.add(significant_child(started, 2))
    |> static.start
  let assert Ok(#(1, first, first_inbox)) = process.receive(started, 0)
  let assert Ok(#(2, second, second_inbox)) = process.receive(started, 0)
  process.unlink(supervisor)
  process.send(first_inbox, Nil)
  await_exit(first)
  let assert True = process.is_alive(supervisor)
  let assert True = process.is_alive(second)
  process.send(second_inbox, Nil)
  await_exit(supervisor)
  let assert Error(Nil) = process.receive(started, 0)
  Nil
}

fn invalid_significance() {
  let started = process.new_subject()
  let permanent =
    child(started, 1, supervision.Permanent)
    |> supervision.significant(True)
  let assert Error(actor.InitFailed(
    "significant child requires automatic shutdown",
  )) =
    static.new(static.OneForOne)
    |> static.add(permanent)
    |> static.start
  let assert Error(actor.InitFailed(
    "significant child cannot restart permanently",
  )) =
    static.new(static.OneForOne)
    |> static.auto_shutdown(static.AnySignificant)
    |> static.add(permanent)
    |> static.start
  let assert Error(Nil) = process.receive(started, 0)
  Nil
}

pub fn main() {
  restart_group(static.OneForOne, [2])
  restart_group(static.OneForAll, [1, 2])
  restart_group(static.RestForOne, [2])
  inactive_transient_restarts_with_group(static.OneForAll)
  inactive_transient_restarts_with_group(static.RestForOne)
  auto_shutdown_any(False)
  auto_shutdown_any(True)
  transient_significant_restarts_before_normal_shutdown()
  auto_shutdown_all()
  invalid_significance()
  Nil
}
