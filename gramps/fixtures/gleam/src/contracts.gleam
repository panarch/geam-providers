import gleam/bit_array
import gleam/bytes_tree
import gleam/http.{Http, Https, Other, Post} as _
import gleam/http/response
import gleam/option.{None, Some}
import gleam/string
import gramps/http
import gramps/websocket.{BinaryFrame, Complete, Control, Data, TextFrame}
import gramps/websocket/compression.{ContextTakeover}
import upgrade_echo/session

/// Shared contracts that execute on unchanged upstream Erlang and Geam.
pub fn common() -> Bool {
  let assert "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=" =
    websocket.parse_websocket_key("dGhlIHNhbXBsZSBub25jZQ==")
  let assert <<"echo":utf8>> = session.round_trip("echo")
  let assert Ok(#(request, <<"body":utf8, 255>>)) =
    http.read_request(<<
      "POST /path?q=a%20b HTTP/1.1\r\nHost: example.test\r\nX: one\r\nX: two\r\nEmpty:\r\n\r\nbody":utf8,
      255,
    >>)
  let assert Post = request.method
  let assert "/path" = request.path
  let assert Some("q=a%20b") = request.query
  let assert None = request.port
  let assert "example.test" = request.host
  let assert [
    #("empty", ""),
    #("x", "two"),
    #("x", "one"),
    #("host", "example.test"),
  ] = request.headers
  let assert Ok(#(reply, <<"next":utf8>>)) =
    http.read_response(<<"HTTP/1.1 204 No Content\r\nX: v\r\n\r\nnext":utf8>>)
  let assert 204 = reply.status
  let assert [#("x", "v")] = reply.headers
  let assert <<"HTTP/1.1 599 Unknown HTTP Status\r\nx: v\r\n\r\nbody":utf8>> =
    http.response_builder(599, [#("x", "v")])
    |> bytes_tree.append_string("body")
    |> bytes_tree.to_bit_array
  let assert Ok(#(raw, <<>>)) =
    http.read_request(<<
      "GET / HTTP/1.1\r\nX: ":utf8,
      255,
      128,
      "\r\n\r\n":utf8,
    >>)
  let assert [#("x", raw_value)] = raw.headers
  let assert <<255, 128>> = bit_array.from_string(raw_value)
  frames()
  compression_messages(False)
  compression_messages(True)
  duplex(False, True)
  duplex(True, False)
  http_packets()
  True
}

fn frames() {
  let mask = Some(<<1, 2, 3, 4>>)
  let text =
    websocket.encode_text_frame("Hello", None, mask)
    |> bytes_tree.to_bit_array
  let assert Ok(#(Complete(Data(TextFrame(<<"Hello":utf8>>))), <<9, 8>>)) =
    websocket.decode_frame(<<text:bits, 9, 8>>, None)
  let binary =
    websocket.encode_binary_frame(<<0, 255, 128>>, None, None)
    |> bytes_tree.to_bit_array
  let assert Ok(#(Complete(Data(BinaryFrame(<<0, 255, 128>>))), <<>>)) =
    websocket.decode_frame(binary, None)
  let assert Error(websocket.NeedMoreData(<<129, 5, 72>>)) =
    websocket.decode_frame(<<129, 5, 72>>, None)
  let assert Error(websocket.InvalidFrame) =
    websocket.decode_frame(<<193, 0>>, None)
  let assert Error(websocket.InvalidFrame) =
    websocket.decode_frame(<<131, 0>>, None)
  let assert Ok(#(websocket.Incomplete(Data(TextFrame(<<"a":utf8>>))), <<>>)) =
    websocket.decode_frame(<<1, 1, 97>>, None)
  let assert Ok(#(Complete(websocket.Continuation(0, <<98>>)), <<>>)) =
    websocket.decode_frame(<<128, 1, 98>>, None)
  let assert Ok(#(Complete(Control(websocket.PingFrame(<<>>))), <<>>)) =
    websocket.decode_frame(<<137, 0>>, None)
  let assert Ok(#(Complete(Control(websocket.PongFrame(<<>>))), <<>>)) =
    websocket.decode_frame(<<138, 0>>, None)
  let assert Ok(#(
    Complete(Control(websocket.CloseFrame(websocket.NotProvided))),
    <<>>,
  )) = websocket.decode_frame(<<136, 0>>, None)
  let extended = bit_array.from_string(string.repeat("x", 126))
  let frame =
    websocket.encode_binary_frame(extended, None, None)
    |> bytes_tree.to_bit_array
  let assert <<130, 126, 0, 126, _:bits>> = frame
  let assert Ok(#(Complete(Data(BinaryFrame(decoded))), <<>>)) =
    websocket.decode_frame(frame, None)
  let assert True = decoded == extended
  let long = bit_array.from_string(string.repeat("x", 65_536))
  let long_frame =
    websocket.encode_binary_frame(long, None, None)
    |> bytes_tree.to_bit_array
  let assert <<130, 127, 65_536:64, _:bits>> = long_frame
  let assert Ok(#(Complete(Data(BinaryFrame(decoded))), <<>>)) =
    websocket.decode_frame(long_frame, None)
  let assert True = decoded == long
  let assert #(
    [
      Complete(Data(TextFrame(<<"Hello":utf8>>))),
      Complete(Data(BinaryFrame(<<0, 255, 128>>))),
    ],
    <<129, 5, 72>>,
  ) =
    websocket.decode_many_frames(
      <<text:bits, binary:bits, 129, 5, 72>>,
      None,
      [],
    )
  let assert Ok([Data(TextFrame(<<"ab":utf8>>))]) =
    websocket.aggregate_frames(
      [
        websocket.Incomplete(Data(TextFrame(<<"a":utf8>>))),
        Complete(websocket.Continuation(0, <<"b":utf8>>)),
      ],
      None,
      [],
    )
  let assert Ok([websocket.Continuation(0, <<"orphan":utf8>>)]) =
    websocket.aggregate_frames(
      [Complete(websocket.Continuation(0, <<"orphan":utf8>>))],
      None,
      [],
    )
  let assert Error(Nil) =
    websocket.aggregate_frames(
      [
        websocket.Incomplete(Data(TextFrame(<<"a":utf8>>))),
        Complete(Data(TextFrame(<<"b":utf8>>))),
      ],
      None,
      [],
    )
  Nil
}

pub fn compression_messages(reset: Bool) -> Bool {
  let contexts = compression.init(ContextTakeover(reset, reset))
  let repeated = bit_array.from_string(string.repeat("hello websocket ", 1024))
  let messages = [<<>>, repeated, repeated, <<0, 255, 128, 0, 255>>]
  compression_loop(contexts.inflate, contexts.deflate, messages)
  compression.close(contexts.inflate)
  compression.close(contexts.deflate)
  True
}

/// The source assigns no_client to inflate and no_server to deflate. A peer
/// therefore swaps the two flags; each direction uses its own stream pair.
fn duplex(no_client: Bool, no_server: Bool) {
  let server = compression.init(ContextTakeover(no_client, no_server))
  let peer = compression.init(ContextTakeover(no_server, no_client))
  let repeated = bit_array.from_string(string.repeat("dictionary ", 2000))
  duplex_loop(server, peer, [<<>>, repeated, repeated, <<0, 255, 128>>])
  compression.close(server.inflate)
  compression.close(server.deflate)
  compression.close(peer.inflate)
  compression.close(peer.deflate)
}

fn duplex_loop(
  server: compression.Compression,
  peer: compression.Compression,
  messages: List(BitArray),
) {
  case messages {
    [] -> Nil
    [message, ..rest] -> {
      let assert True =
        compression.inflate(
          peer.inflate,
          compression.deflate(server.deflate, message),
        )
        == message
      let assert True =
        compression.inflate(
          server.inflate,
          compression.deflate(peer.deflate, message),
        )
        == message
      duplex_loop(server, peer, rest)
    }
  }
}

fn http_packets() {
  let assert Ok(#(first, rest)) =
    http.read_request(<<
      "GET /first HTTP/1.1\r\n\r\nGET /second?q=2 HTTP/1.0\r\nX: v\r\n\r\ntail":utf8,
    >>)
  let assert "/first" = first.path
  let assert Ok(#(second, <<"tail":utf8>>)) = http.read_request(rest)
  let assert "/second" = second.path
  let assert Some("q=2") = second.query
  let assert [#("x", "v")] = second.headers
  let assert <<"HTTP/1.1 201 Created\r\nx-test: v\r\n\r\n", 0, 255>> =
    response.new(201)
    |> response.set_header("X-Test", "v")
    |> response.set_body(bytes_tree.from_bit_array(<<0, 255>>))
    |> http.to_bytes_tree
    |> bytes_tree.to_bit_array
  let assert Error(http.HttpError("Unexpected data")) =
    http.read_response(<<"GET / HTTP/1.1\r\n\r\n":utf8>>)
  let assert Error(http.HttpError(reason)) =
    http.read_request(<<"HTTP/1.1 200 OK\r\n\r\n":utf8>>)
  let assert True =
    string.contains(reason, "Unexpected data: Ok(#(HttpResponse")
}

fn compression_loop(
  inflater: compression.Context,
  deflater: compression.Context,
  messages: List(BitArray),
) {
  case messages {
    [] -> Nil
    [message, ..rest] -> {
      let wire = compression.deflate(deflater, message)
      let decoded = compression.inflate(inflater, wire)
      let assert True = decoded == message
      compression_loop(inflater, deflater, rest)
    }
  }
}

/// Typed Geam normalizations for native values outside the original FFI's type.
pub fn normalized() -> Bool {
  let assert Ok(#(absolute, <<>>)) =
    http.read_request(<<
      "CUSTOM https://example.test:8443/a?q=1 HTTP/1.0\r\n\r\n":utf8,
    >>)
  let assert Other("CUSTOM") = absolute.method
  let assert Https = absolute.scheme
  let assert "example.test" = absolute.host
  let assert Some(8443) = absolute.port
  let assert "/a" = absolute.path
  let assert Some("q=1") = absolute.query
  let assert Error(http.More(0)) = http.read_request(<<"GET / HTTP/1.1":utf8>>)
  let assert Error(http.More(0)) =
    http.read_request(<<"GET / HTTP/1.1\r\nX: a":utf8>>)
  let assert Ok(#(plain, <<>>)) =
    http.read_request(<<"GET http://example.test HTTP/1.1\r\n\r\n":utf8>>)
  let assert Http = plain.scheme
  let assert Some(80) = plain.port
  let assert "/" = plain.path
  let assert Ok(#(query_only, <<>>)) =
    http.read_request(<<"GET http://example.test?q=1 HTTP/1.1\r\n\r\n":utf8>>)
  let assert "/" = query_only.path
  let assert Some("q=1") = query_only.query
  let partial = <<"GET /retry HTTP/1.1\r\nHost: exam":utf8>>
  let assert Error(http.More(0)) = http.read_request(partial)
  let assert Ok(#(retried, <<"body":utf8>>)) =
    http.read_request(<<partial:bits, "ple.test\r\n\r\nbody":utf8>>)
  let assert "example.test" = retried.host
  let assert "/retry" = retried.path
  let assert Ok(#(secure, <<>>)) =
    http.read_request(<<"GET https://[::1]/a HTTP/1.1\r\n\r\n":utf8>>)
  let assert Https = secure.scheme
  let assert Some(443) = secure.port
  let assert "[::1]" = secure.host
  let assert Error(http.HttpError(_)) =
    http.read_request(<<"OPTIONS * HTTP/1.1\r\n\r\n":utf8>>)
  let assert Error(http.HttpError(_)) =
    http.read_request(<<"CONNECT example.test:443 HTTP/1.1\r\n\r\n":utf8>>)
  let assert Error(http.HttpError(_)) =
    http.read_request(<<"GET ftp://example.test/ HTTP/1.1\r\n\r\n":utf8>>)
  let assert Error(http.HttpError(_)) =
    http.read_request(<<"GET / HTTP/1.1\r\nBad Header: v\r\n\r\n":utf8>>)
  let assert Error(http.HttpError(_)) =
    http.read_response(<<"HTTP/1.1 x OK\r\n\r\n":utf8>>)
  let assert Error(http.HttpError(_)) =
    http.read_request(<<"GET / HTTP/1.1\r\n\r\n":utf8, 1:1>>)
  True
}

pub fn closed() {
  let contexts = compression.init(ContextTakeover(False, False))
  compression.close(contexts.deflate)
  compression.deflate(contexts.deflate, <<"fails":utf8>>)
}

pub fn malformed() {
  let contexts = compression.init(ContextTakeover(False, False))
  compression.inflate(contexts.inflate, <<7>>)
}

pub fn partial() {
  let contexts = compression.init(ContextTakeover(False, False))
  compression.deflate(contexts.deflate, <<1:1>>)
}

pub fn client_key() -> String {
  websocket.make_client_key()
}
