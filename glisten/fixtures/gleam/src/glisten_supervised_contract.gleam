import gleam/bit_array
import gleam/bytes_tree
import gleam/erlang/process
import gleam/int
import gleam/io
import gleam/option.{None}
import gleam/otp/static_supervisor as supervisor
import glisten

pub fn main() {
  let closed = process.new_subject()
  let name = process.new_name("glisten_supervised_ipv6_listener")
  let child =
    glisten.new(
      fn(_connection) { #(<<>>, None) },
      fn(data, message, connection) {
        case message {
          glisten.Packet(packet) -> {
            let data = bit_array.concat([data, packet])
            case bit_array.byte_size(data) {
              4 -> {
                let assert <<"ping":utf8>> = data
                let assert Ok(Nil) =
                  glisten.send(connection, bytes_tree.from_string("pong"))
                glisten.continue(data)
              }
              size -> {
                let assert True = size < 4
                glisten.continue(data)
              }
            }
          }
          glisten.User(_) -> panic as "unexpected user message"
        }
      },
    )
    |> glisten.bind("::1")
    |> glisten.with_ipv6()
    |> glisten.with_pool_size(2)
    |> glisten.with_listener_name(name)
    |> glisten.with_close(fn(data) {
      let assert <<"ping":utf8>> = data
      process.send(closed, Nil)
    })
    |> glisten.supervised(0)
  let assert Ok(parent) =
    supervisor.new(supervisor.OneForOne)
    |> supervisor.add(child)
    |> supervisor.start
  let info = glisten.get_server_info(name, 5000)
  let assert glisten.IpV6(0, 0, 0, 0, 0, 0, 0, 1) = info.ip_address
  io.println("READY ipv6 " <> int.to_string(info.port))
  let assert Ok(Nil) = process.receive(closed, 5000)
  process.unlink(parent.pid)
  process.kill(parent.pid)
  io.println("PASS ipv6")
}
