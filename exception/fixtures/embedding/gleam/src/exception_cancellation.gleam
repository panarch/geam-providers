import exception
import gleam/io
import gleam/string

@external(erlang, "exception_test_native", "cancel")
fn cancel() -> Nil

pub fn rescue_cancel() -> Nil {
  let _ = exception.rescue(fn() { cancel() })
  io.println("cancellation was rescued")
}

pub fn defer_cancel() -> Nil {
  exception.defer(fn() { io.println("defer cleanup") }, fn() { cancel() })
}

pub fn on_crash_cancel() -> Nil {
  exception.on_crash(fn() { io.println("crash cleanup") }, fn() { cancel() })
}

pub fn defer_cancel_cleanup_failure() -> Nil {
  let assert Error(exception.Errored(reason)) =
    exception.rescue(fn() {
      exception.defer(fn() { panic as "cancel cleanup failed" }, fn() {
        cancel()
      })
    })
  assert string.contains(string.inspect(reason), "cancel cleanup failed")
  Nil
}
