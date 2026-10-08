import contracts
import gramps/websocket/compression.{ContextTakeover}
import upgrade_echo/session

pub fn verify() -> Bool {
  let assert True = contracts.common()
  contracts.normalized()
}

pub fn round_trip(data: String) -> BitArray {
  session.round_trip(data)
}

pub fn closed() -> BitArray {
  contracts.closed()
}

pub fn malformed() -> BitArray {
  contracts.malformed()
}

pub fn partial() -> BitArray {
  contracts.partial()
}

pub fn client_key() -> String {
  contracts.client_key()
}

/// Actual provider paths used by the external Erlang compression peer.
pub fn compress(messages: List(BitArray), reset: Bool) -> List(BitArray) {
  let contexts = compression.init(ContextTakeover(reset, reset))
  let output = compress_loop(contexts.deflate, messages)
  compression.close(contexts.inflate)
  compression.close(contexts.deflate)
  output
}

fn compress_loop(
  context: compression.Context,
  messages: List(BitArray),
) -> List(BitArray) {
  case messages {
    [] -> []
    [message, ..rest] -> [
      compression.deflate(context, message),
      ..compress_loop(context, rest)
    ]
  }
}

pub fn decompress(messages: List(BitArray), reset: Bool) -> List(BitArray) {
  let contexts = compression.init(ContextTakeover(reset, reset))
  let output = decompress_loop(contexts.inflate, messages)
  compression.close(contexts.inflate)
  compression.close(contexts.deflate)
  output
}

fn decompress_loop(
  context: compression.Context,
  messages: List(BitArray),
) -> List(BitArray) {
  case messages {
    [] -> []
    [message, ..rest] -> [
      compression.inflate(context, message),
      ..decompress_loop(context, rest)
    ]
  }
}
