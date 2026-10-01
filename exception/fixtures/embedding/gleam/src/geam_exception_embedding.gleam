import exception
import exception_consumer
import gleam/dynamic
import gleam/int
import gleam/io
import gleam/string

pub fn verify() -> Bool {
  assert exception_consumer.verify_values()
  assert Ok(7) == exception.rescue(fn() { 7 })
  let assert Error(exception.Errored(reason)) = exception.rescue(fn() { panic })
  assert dynamic.classify(reason) == "Atom"
  assert Ok(5) == exception_consumer.protect(fn() { 5 })
  let assert Error(message) = exception_consumer.protect(fn() { panic })
  assert string.contains(message, "Errored")

  assert 9
    == exception.defer(fn() { io.println("cleanup success") }, fn() { 9 })
  let assert Error(exception.Errored(_)) =
    exception.rescue(fn() {
      use <- exception.defer(fn() { io.println("cleanup failure") })
      use <- exception.on_crash(fn() { io.println("crash cleanup") })
      panic
    })
  assert 11
    == exception.on_crash(fn() { io.println("unexpected cleanup") }, fn() { 11 })
  True
}

pub fn cleanup_failure_wins() -> Bool {
  let assert Error(exception.Errored(success_reason)) =
    exception.rescue(fn() {
      exception.defer(fn() { panic as "success cleanup failed" }, fn() { 7 })
    })
  let assert Error(exception.Errored(defer_reason)) =
    exception.rescue(fn() {
      exception.defer(fn() { panic as "defer cleanup failed" }, fn() {
        panic as "defer body failed"
      })
    })
  let assert Error(exception.Errored(crash_reason)) =
    exception.rescue(fn() {
      exception.on_crash(fn() { panic as "crash cleanup failed" }, fn() {
        panic as "crash body failed"
      })
    })
  string.contains(string.inspect(success_reason), "success cleanup failed")
  && string.contains(string.inspect(defer_reason), "defer cleanup failed")
  && string.contains(string.inspect(crash_reason), "crash cleanup failed")
}

pub fn native_failure_is_caught() -> Bool {
  let assert Ok(huge) = int.parse("1" <> string.repeat("0", 400))
  let assert Error(exception.Errored(reason)) =
    exception.rescue(fn() { int.to_float(huge) })
  string.contains(
    string.inspect(reason),
    "Int cannot be represented as a finite Float",
  )
}

pub fn nested_cleanup_order() -> Bool {
  let value = {
    use <- exception.defer(fn() { io.println("success outer") })
    use <- exception.defer(fn() { io.println("success inner") })
    3
  }
  let assert Error(exception.Errored(_)) =
    exception.rescue(fn() {
      use <- exception.defer(fn() { io.println("failure outer") })
      use <- exception.defer(fn() { io.println("failure inner") })
      panic
    })
  value == 3
}
