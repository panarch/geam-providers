import birl
import gleam/io
import gleam/time/duration

pub fn main() {
  let assert birl.Thu = birl.weekday(birl.unix_epoch())
  let assert birl.Wed = birl.weekday(birl.from_unix_micro(-1))
  let assert Ok(before_epoch) = birl.set_offset(birl.unix_epoch(), "-01:00")
  let assert birl.Wed = birl.weekday(before_epoch)

  let first = birl.monotonic_now()
  let local = birl.now()
  let utc = birl.utc_now()
  let last = birl.monotonic_now()
  let assert True = first <= last
  let assert "Z" = birl.get_offset(utc)
  let #(seconds, nanoseconds) =
    birl.difference(utc, local)
    |> duration.to_seconds_and_nanoseconds
  let assert True = seconds >= 0 && nanoseconds >= 0

  io.println("birl fixture ok")
}
