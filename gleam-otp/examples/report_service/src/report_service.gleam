import gleam/erlang/process
import gleam/int
import gleam/io
import gleam/otp/actor
import gleam/otp/factory_supervisor as factory
import gleam/otp/static_supervisor as static
import gleam/otp/supervision

pub type Request {
  Generate(Int, process.Subject(Result(String, String)))
}

pub opaque type Service {
  Service(
    supervisor: process.Pid,
    name: process.Name(factory.Message(Int, String)),
    factories: process.Subject(process.Pid),
    dispatchers: process.Subject(#(process.Pid, process.Subject(Request))),
    factory_pid: process.Pid,
    dispatcher_pid: process.Pid,
    inbox: process.Subject(Request),
  )
}

fn start_report(value: Int) -> actor.StartResult(String) {
  case value > 0 {
    False -> Error(actor.InitFailed("positive input required"))
    True -> {
      let assert Ok(actor.Started(pid, _)) =
        actor.new(Nil)
        |> actor.on_message(fn(state, _: Nil) { actor.continue(state) })
        |> actor.start
      Ok(actor.Started(pid, "report " <> int.to_string(value * 2)))
    }
  }
}

fn start_dispatcher(name, started) {
  let assert Ok(actor.Started(pid, inbox)) =
    actor.new(name)
    |> actor.on_message(fn(name, message: Request) {
      case message {
        Generate(value, reply) -> {
          let outcome = case
            factory.start_child(factory.get_by_name(name), value)
          {
            Ok(actor.Started(_, report)) -> Ok(report)
            Error(actor.InitFailed(reason)) -> Error(reason)
            Error(_) -> Error("report worker could not start")
          }
          process.send(reply, outcome)
          actor.continue(name)
        }
      }
    })
    |> actor.start
  process.send(started, #(pid, inbox))
  Ok(actor.Started(pid, inbox))
}

fn await_exit(pid: process.Pid) {
  let monitor = process.monitor(pid)
  let _ =
    process.new_selector()
    |> process.select_specific_monitor(monitor, fn(down) { down })
    |> process.selector_receive_forever
  let assert False = process.is_alive(pid)
}

pub fn start() -> Service {
  let name = process.new_name("report_workers")
  let factories = process.new_subject()
  let dispatchers = process.new_subject()
  let factory_child =
    supervision.supervisor(fn() {
      case
        factory.worker_child(start_report)
        |> factory.named(name)
        |> factory.start
      {
        Ok(actor.Started(pid, handle)) -> {
          process.send(factories, pid)
          Ok(actor.Started(pid, handle))
        }
        Error(error) -> Error(error)
      }
    })
  let dispatcher_child =
    supervision.worker(fn() { start_dispatcher(name, dispatchers) })
  let assert Ok(actor.Started(supervisor, _)) =
    static.new(static.RestForOne)
    |> static.add(factory_child)
    |> static.add(dispatcher_child)
    |> static.start
  let first_factory = process.receive_forever(factories)
  let #(first_dispatcher, first_inbox) = process.receive_forever(dispatchers)
  Service(
    supervisor: supervisor,
    name: name,
    factories: factories,
    dispatchers: dispatchers,
    factory_pid: first_factory,
    dispatcher_pid: first_dispatcher,
    inbox: first_inbox,
  )
}

pub fn submit(service: Service, value: Int) -> Result(String, String) {
  process.call_forever(service.inbox, fn(reply) { Generate(value, reply) })
}

pub fn worker_count(service: Service) -> Int {
  factory.count_children(factory.get_by_name(service.name))
}

pub fn restart_factory(service: Service) -> Service {
  process.kill(service.factory_pid)
  let second_factory = process.receive_forever(service.factories)
  let #(second_dispatcher, second_inbox) =
    process.receive_forever(service.dispatchers)
  let assert True = service.factory_pid != second_factory
  let assert True = service.dispatcher_pid != second_dispatcher
  let assert False = process.is_alive(service.dispatcher_pid)
  Service(
    ..service,
    factory_pid: second_factory,
    dispatcher_pid: second_dispatcher,
    inbox: second_inbox,
  )
}

pub fn shutdown(service: Service) {
  process.unlink(service.supervisor)
  process.kill(service.supervisor)
  await_exit(service.supervisor)
  let assert False = process.is_alive(service.factory_pid)
  let assert False = process.is_alive(service.dispatcher_pid)
  Nil
}

pub fn main() {
  let service = start()
  let assert Ok(first) = submit(service, 7)
  io.println("request 7 -> " <> first)
  let assert Ok(second) = submit(service, 9)
  io.println("request 9 -> " <> second)
  let assert Error(reason) = submit(service, 0)
  io.println("request 0 -> " <> reason)
  let assert 2 = worker_count(service)

  let service = restart_factory(service)
  let assert 0 = worker_count(service)
  let assert Ok(after_restart) = submit(service, 11)
  io.println("after restart, request 11 -> " <> after_restart)
  let assert 1 = worker_count(service)

  shutdown(service)
  Nil
}
