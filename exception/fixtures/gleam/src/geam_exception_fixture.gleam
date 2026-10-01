import exception
import exception_consumer
import gleam/io

pub fn main() {
  assert exception_consumer.verify_values()
  assert Ok(7) == exception.rescue(fn() { 7 })
  let assert Error(exception.Errored(_)) = exception.rescue(fn() { panic })
  assert Ok(5) == exception_consumer.protect(fn() { 5 })
  let assert Error(_) = exception_consumer.protect(fn() { panic })

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
}
