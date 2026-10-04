import gleam/bit_array
import gleam/bytes_tree
import gleam/erlang/process
import gleam/int
import gleam/io
import gleam/option.{Some}
import glisten
import glisten/socket/options

pub type Control {
  UserPing
}

pub fn main() {
  let seen = process.new_subject()
  let closed = process.new_subject()
  let name = process.new_name("glisten_audit_user")
  let assert Ok(server) =
    glisten.new(
      fn(_connection: glisten.Connection(Control)) {
        let subject = process.new_subject()
        process.send(subject, UserPing)
        #(<<>>, Some(process.new_selector() |> process.select(subject)))
      },
      fn(data, message, connection) {
        case message {
          glisten.User(UserPing) -> {
            process.send(seen, "user")
            glisten.continue(data)
          }
          glisten.Packet(packet) -> {
            let data = bit_array.concat([data, packet])
            case bit_array.byte_size(data) {
              4 -> {
                let assert <<"ping":utf8>> = data
                let assert Ok(Nil) =
                  glisten.send(connection, bytes_tree.from_string("pong"))
                glisten.continue(data)
              }
              _ -> glisten.continue(data)
            }
          }
        }
      },
    )
    |> glisten.with_pool_size(1)
    |> glisten.with_listener_name(name)
    |> glisten.with_active_state(options.Active)
    |> glisten.with_close(fn(data) {
      let assert <<"ping":utf8>> = data
      process.send(closed, Nil)
    })
    |> glisten.start(0)
  let info = glisten.get_server_info(name, 5000)
  io.println("READY server " <> int.to_string(info.port))
  let assert Ok("user") = process.receive(seen, 5000)
  let assert Ok(Nil) = process.receive(closed, 5000)
  process.unlink(server.pid)
  process.kill(server.pid)
  io.println("PASS server")
}
