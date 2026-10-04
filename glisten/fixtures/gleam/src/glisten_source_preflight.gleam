import gleam/option.{None}
import glisten

pub fn main() {
  assert glisten.ip_address_to_string(glisten.IpV4(127, 0, 0, 1)) == "127.0.0.1"
  let builder =
    glisten.new(
      fn(_connection) { #(Nil, None) },
      fn(state, _message, _connection) { glisten.continue(state) },
    )
  let _ = glisten.bind(builder, "localhost")
  let _ = glisten.bind(builder, "127.0.0.2")
  let _ = glisten.bind(builder, "::1")
  Nil
}
