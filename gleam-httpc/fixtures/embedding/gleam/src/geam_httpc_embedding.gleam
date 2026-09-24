import gleam/http
import gleam/http/request
import gleam/httpc

pub fn verify(url: String) -> Bool {
  let assert Ok(request) = request.to(url)
  case httpc.send(request) {
    Ok(response) -> response.status == 200 && response.body == "hello"
    Error(_) -> False
  }
}

pub fn methods(url: String) -> Bool {
  let assert Ok(base) = request.to(url)
  let assert Ok(get) = httpc.send(request.set_body(base, "ignored"))
  let assert Ok(head) = httpc.send(request.set_method(base, http.Head))
  let assert Ok(options) = httpc.send(request.set_method(base, http.Options))
  let assert Ok(post) =
    httpc.send(
      base |> request.set_method(http.Post) |> request.set_body("post"),
    )
  let assert Ok(put) = httpc.send(request.set_method(base, http.Put))
  let assert Ok(delete) = httpc.send(request.set_method(base, http.Delete))
  let assert Ok(trace) = httpc.send(request.set_method(base, http.Trace))
  let assert Ok(connect) = httpc.send(request.set_method(base, http.Connect))
  let assert Ok(patch) = httpc.send(request.set_method(base, http.Patch))
  let assert Ok(other) =
    httpc.send(request.set_method(base, http.Other("REPORT")))
  get.status == 200
  && head.status == 200
  && options.status == 200
  && post.status == 200
  && put.status == 200
  && delete.status == 200
  && trace.status == 200
  && connect.status == 200
  && patch.status == 200
  && other.status == 200
}

pub fn options_and_headers(url: String) -> Bool {
  let assert Ok(base) = request.to(url)
  let request =
    base
    |> request.set_method(http.Post)
    |> request.set_body("payload")
    |> request.set_header("user-agent", "fixture-agent")
    |> request.set_header("content-type", "text/plain")
    |> request.prepend_header("x-duplicate", "first")
    |> request.prepend_header("x-duplicate", "second")
  let configuration =
    httpc.configure()
    |> httpc.verify_tls(False)
    |> httpc.follow_redirects(True)
    |> httpc.timeout(123)
  let assert Ok(reply) = httpc.dispatch(configuration, request)
  reply.status == 200
  && reply.body == "hello"
  && reply.headers == [#("x-duplicate", "first"), #("x-duplicate", "second")]
}

pub fn binary_response(url: String) -> Bool {
  let assert Ok(base) = request.to(url)
  let request =
    base
    |> request.set_method(http.Post)
    |> request.set_body(<<0, 255>>)
  let assert Ok(reply) = httpc.send_bits(request)
  reply.body == <<0, 255>>
}

pub fn invalid_utf8(url: String) -> Bool {
  let assert Ok(request) = request.to(url)
  case httpc.send(request) {
    Error(httpc.InvalidUtf8Response) -> True
    _ -> False
  }
}

pub fn timeout(url: String) -> Bool {
  let assert Ok(request) = request.to(url)
  case httpc.send(request) {
    Error(httpc.ResponseTimeout) -> True
    _ -> False
  }
}

pub fn posix_error(url: String) -> Bool {
  let assert Ok(request) = request.to(url)
  case httpc.send(request) {
    Error(httpc.FailedToConnect(
      httpc.Posix("econnrefused"),
      httpc.Posix("enetunreach"),
    )) -> True
    _ -> False
  }
}

pub fn tls_error(url: String) -> Bool {
  let assert Ok(request) = request.to(url)
  case httpc.send(request) {
    Error(httpc.FailedToConnect(
      httpc.TlsAlert("unknown_ca", "certificate rejected"),
      httpc.TlsAlert("unknown_ca", "certificate rejected"),
    )) -> True
    _ -> False
  }
}

pub fn negative_timeout(url: String) -> Bool {
  let assert Ok(request) = request.to(url)
  let _ = httpc.dispatch(httpc.timeout(httpc.configure(), -1), request)
  True
}

pub fn negative_timeout_body(url: String) -> Bool {
  let assert Ok(request) = request.to(url)
  let request =
    request
    |> request.set_method(http.Post)
    |> request.set_body("payload")
  let _ = httpc.dispatch(httpc.timeout(httpc.configure(), -1), request)
  True
}

pub fn unaligned_request(url: String) -> Bool {
  let assert Ok(request) = request.to(url)
  let request =
    request
    |> request.set_method(http.Post)
    |> request.set_body(<<1:1>>)
  let _ = httpc.send_bits(request)
  True
}

pub fn invalid_method(url: String) -> Bool {
  let assert Ok(request) = request.to(url)
  let _ = httpc.send(request.set_method(request, http.Other("")))
  True
}
