import gleam/bit_array
import gleam/bytes_tree
import gleam/http/response
import gleam/list
import gleam/option.{None, Some}
import gramps/http
import gramps/websocket.{Complete, Control, Data, TextFrame}
import gramps/websocket/compression.{ContextTakeover}

/// Processes a fixed Upgrade exchange and one compressed client message.
/// The application supplies bytes; this module does not own a network transport.
pub fn round_trip(payload: String) -> BitArray {
  let request_bytes = <<
    "GET /echo HTTP/1.1\r\nHost: example.test\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Extensions: permessage-deflate; client_no_context_takeover; server_no_context_takeover\r\n\r\n":utf8,
  >>
  let assert Ok(#(request, <<>>)) = http.read_request(request_bytes)
  let assert "/echo" = request.path
  let assert Ok(key) = list.key_find(request.headers, "sec-websocket-key")
  let accept = websocket.parse_websocket_key(key)
  let headers = [
    #("Upgrade", "websocket"),
    #("Connection", "Upgrade"),
    #("Sec-WebSocket-Accept", accept),
    #(
      "Sec-WebSocket-Extensions",
      "permessage-deflate; client_no_context_takeover; server_no_context_takeover",
    ),
  ]
  let response_bytes =
    response.Response(101, headers, bytes_tree.new())
    |> http.to_bytes_tree
    |> bytes_tree.to_bit_array
  let assert Ok(#(reply, <<>>)) = http.read_response(response_bytes)
  let assert 101 = reply.status
  let assert Ok("s3pPLMBiTxaQ9kYGzzhZRbK+xOo=") =
    list.key_find(reply.headers, "sec-websocket-accept")

  let server = compression.init(ContextTakeover(True, True))
  let peer = compression.init(ContextTakeover(True, True))
  let client_frame =
    websocket.encode_text_frame(
      payload,
      Some(peer.deflate),
      Some(<<1, 2, 3, 4>>),
    )
    |> bytes_tree.to_bit_array
  let assert Ok(#(Complete(Data(TextFrame(received))), <<>>)) =
    websocket.decode_frame(client_frame, Some(server.inflate))
  let assert Ok(text) = bit_array.to_string(received)
  let echo_frame =
    websocket.encode_text_frame(text, Some(server.deflate), None)
    |> bytes_tree.to_bit_array
  let assert Ok(#(Complete(Data(TextFrame(echoed))), <<>>)) =
    websocket.decode_frame(echo_frame, Some(peer.inflate))

  let ping =
    websocket.encode_ping_frame(<<"probe":utf8>>, Some(<<1, 2, 3, 4>>))
    |> bytes_tree.to_bit_array
  let assert Ok(#(Complete(Control(websocket.PingFrame(ping_payload))), <<>>)) =
    websocket.decode_frame(ping, None)
  let pong =
    websocket.encode_pong_frame(ping_payload, None) |> bytes_tree.to_bit_array
  let assert Ok(#(
    Complete(Control(websocket.PongFrame(<<"probe":utf8>>))),
    <<>>,
  )) = websocket.decode_frame(pong, None)
  let close =
    websocket.encode_close_frame(websocket.Normal(<<"bye":utf8>>), None)
    |> bytes_tree.to_bit_array
  let assert Ok(#(
    Complete(Control(websocket.CloseFrame(websocket.Normal(<<"bye":utf8>>)))),
    <<>>,
  )) = websocket.decode_frame(close, None)
  compression.close(server.inflate)
  compression.close(server.deflate)
  compression.close(peer.inflate)
  compression.close(peer.deflate)
  echoed
}
