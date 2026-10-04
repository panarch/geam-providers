import gleam/bytes_tree
import gleam/dynamic/decode
import gleam/erlang/atom
import gleam/int
import gleam/io
import glisten/socket.{Timeout}
import glisten/socket/options.{Backlog, Buffer, Ip, Loopback, Nodelay, Reuseaddr}
import glisten/tcp
import glisten/transport

pub fn main() {
  let assert Ok(listener) = tcp.listen(0, [Ip(Loopback)])
  let assert Error(Timeout) = tcp.accept_timeout(listener, 0)
  let assert Ok(#(_, port)) = tcp.sockname(listener)
  let assert True = port > 0
  io.println("READY tcp " <> int.to_string(port))
  let assert Ok(socket) = tcp.accept_timeout(listener, 5000)
  let alias = socket
  let assert Ok(socket) = tcp.handshake(socket)
  let assert Ok(#(_, peer_port)) = tcp.peername(socket)
  let assert True = peer_port > 0
  let assert Ok([#(_, nodelay)]) =
    tcp.get_socket_opts(socket, [atom.create("nodelay")])
  let assert Ok(True) = decode.run(nodelay, decode.bool)
  let assert Ok(Nil) =
    tcp.set_opts(socket, [Buffer(4096), Nodelay(False), Reuseaddr(False)])
  let assert Ok([#(_, buffer), #(_, nodelay), #(_, reuseaddr)]) =
    tcp.get_socket_opts(socket, [
      atom.create("buffer"),
      atom.create("nodelay"),
      atom.create("reuseaddr"),
    ])
  let assert Ok(4096) = decode.run(buffer, decode.int)
  let assert Ok(False) = decode.run(nodelay, decode.bool)
  let assert Ok(False) = decode.run(reuseaddr, decode.bool)
  let assert Error(_) = tcp.set_opts(socket, [Backlog(1)])
  let assert Ok(#(_, peer_port_again)) =
    transport.peername(transport.Tcp, socket)
  let assert True = peer_port == peer_port_again
  let assert Ok(<<"ping":utf8>>) = tcp.receive_timeout(socket, 4, 5000)
  let tree =
    bytes_tree.concat([
      bytes_tree.from_string("po"),
      bytes_tree.concat([
        bytes_tree.from_string("n"),
        bytes_tree.from_string("g"),
      ]),
    ])
  let assert Ok(Nil) = tcp.send(alias, tree)
  let assert <<"pong":utf8>> = bytes_tree.to_bit_array(tree)
  let assert Ok(Nil) = tcp.shutdown(socket)
  let assert Ok(Nil) = tcp.close(alias)
  let assert Ok(Nil) = tcp.close(listener)
  io.println("PASS tcp")
}
