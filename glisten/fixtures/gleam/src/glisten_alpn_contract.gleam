import gleam/bytes_tree
import gleam/erlang/process
import gleam/int
import gleam/io
import glisten/socket/options
import glisten/ssl

pub fn verify(certfile: String, keyfile: String) {
  // TLS ListenSocket has no public close function. Its owner exits normally,
  // and the monitor confirms cleanup before the fixture proceeds.
  let owner = process.spawn_unlinked(fn() { run_contract(certfile, keyfile) })
  let monitor = process.monitor(owner)
  let selector =
    process.new_selector()
    |> process.select_specific_monitor(monitor, fn(down) {
      let assert process.ProcessDown(_, _, reason) = down
      reason
    })
  let assert Ok(process.Normal) = process.selector_receive(selector, 10_000)
  Nil
}

fn run_contract(certfile: String, keyfile: String) {
  let cert = options.CertKeyConfig(options.CertKeyFiles(certfile, keyfile))
  let alpn = options.AlpnPreferredProtocols(["h2", "http/1.1"])
  let assert Ok(listener) = ssl.listen(0, [cert, alpn])
  let assert Ok(#(_, port)) = ssl.sockname(listener)
  io.println("READY tls_none " <> int.to_string(port))
  let assert Ok(socket) = ssl.accept_timeout(listener, 5000)
  let assert Ok(socket) = ssl.handshake(socket)
  let assert Ok(Nil) =
    ssl.set_opts(socket, [
      options.Reuseaddr(False),
      options.Mode(options.Binary),
    ])
  let assert Error(_) = ssl.set_opts(socket, [options.Backlog(1)])
  // The original Erlang FFI returns this String error as a charlist.
  // The typed Rust owner test checks the normalized String value exactly.
  let assert Error(_) = ssl.negotiated_protocol(socket)
  let assert Ok(<<"ping":utf8>>) = ssl.receive_timeout(socket, 4, 5000)
  let assert Ok(Nil) = ssl.send(socket, bytes_tree.from_string("pong"))
  let assert Ok(Nil) = ssl.close(socket)
  io.println("PASS tls_none")

  let assert Ok(listener) = ssl.listen(0, [cert, alpn])
  let assert Ok(#(_, port)) = ssl.sockname(listener)
  io.println("READY tls_mismatch " <> int.to_string(port))
  let assert Ok(socket) = ssl.accept_timeout(listener, 5000)
  let assert Error(_) = ssl.handshake(socket)
  let assert Ok(Nil) = ssl.close(socket)
  io.println("PASS tls_mismatch")
}
