import birl
import gleam/option
import gleam/order
import gleam/time/duration

pub fn weekday_at(micros: Int, offset: String) -> String {
  let assert Ok(value) = birl.set_offset(birl.from_unix_micro(micros), offset)
  birl.weekday_to_string(birl.weekday(value))
}

pub fn sampled_order_is_lt() -> Bool {
  let first = birl.utc_now()
  let second = birl.utc_now()
  birl.compare(first, second) == order.Lt
}

pub fn sampled_difference() -> #(Int, Int) {
  let first = birl.utc_now()
  let second = birl.utc_now()
  birl.difference(second, first)
  |> duration.to_seconds_and_nanoseconds
}

pub fn observed_timezone() -> option.Option(String) {
  birl.now()
  |> birl.get_timezone
}

pub fn observed_offset() -> String {
  birl.now()
  |> birl.get_offset
}

pub fn observed_iso8601() -> String {
  birl.now()
  |> birl.to_iso8601
}
