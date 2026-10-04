import gleam/bit_array
import gleam/bytes_tree
import gleam/erlang/process
import gleam/int
import gleam/io
import gleam/option.{None}
import glisten
import glisten/socket/options

pub fn verify(certfile: String, keyfile: String) {
  let closed = process.new_subject()
  let name = process.new_name("glisten_tls_contract_listener")
  let assert Ok(server) =
    glisten.new(
      fn(connection) {
        let assert Ok(info) = glisten.get_connection_info(connection)
        let assert True = info.port > 0
        #(<<>>, None)
      },
      fn(data, message: glisten.Message(Nil), connection) {
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
    |> glisten.with_pool_size(1)
    |> glisten.with_listener_name(name)
    |> glisten.with_close(fn(data) {
      let assert <<"ping":utf8>> = data
      process.send(closed, Nil)
    })
    |> glisten.with_active_state(options.Count(2))
    |> glisten.with_http2()
    |> glisten.with_tls(certfile: certfile, keyfile: keyfile)
    |> glisten.start(0)
  let info = glisten.get_server_info(name, 5000)
  let assert glisten.IpV4(127, 0, 0, 1) = info.ip_address
  io.println("READY tls " <> int.to_string(info.port))
  let assert Ok(Nil) = process.receive(closed, 5000)
  process.unlink(server.pid)
  process.kill(server.pid)
  io.println("PASS tls")
}
