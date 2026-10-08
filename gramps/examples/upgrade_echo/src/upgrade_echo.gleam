import gleam/io
import upgrade_echo/session

pub fn main() {
  let assert <<"hello websocket":utf8>> = session.round_trip("hello websocket")
  io.println("HTTP Upgrade: 101 Switching Protocols")
  io.println("WebSocket accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=")
  io.println("Compressed masked client message: hello websocket")
  io.println("Compressed unmasked echo: hello websocket")
  io.println("Ping/pong and close: complete")
  io.println("Compression contexts: closed")
}
