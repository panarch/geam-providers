import gleam/http/request
import gleam/http/response
import gleam/httpc

pub fn main() {
  check_http()
  Nil
}

fn check_http() {
  let assert Ok(request) = request.to("http://127.0.0.1:38199/hello")
  let assert Ok(reply) = httpc.send(request)
  let assert 200 = reply.status
  let assert "hello" = reply.body
  let assert Ok("yes") = response.get_header(reply, "x-fixture")
  Nil
}

pub fn check_redirect() {
  let assert Ok(redirect) = request.to("http://127.0.0.1:38199/redirect")
  let assert Ok(stopped) = httpc.send(redirect)
  let assert 302 = stopped.status
  let assert Ok(followed) =
    httpc.dispatch(httpc.follow_redirects(httpc.configure(), True), redirect)
  let assert "hello" = followed.body
  Nil
}

pub fn check_binary() {
  let assert Ok(invalid) = request.to("http://127.0.0.1:38199/invalid-utf8")
  let assert Error(httpc.InvalidUtf8Response) = httpc.send(invalid)
  let assert Ok(raw) = httpc.send_bits(request.map(invalid, fn(_) { <<>> }))
  let assert <<255>> = raw.body
  let assert Ok(configured) =
    httpc.dispatch_bits(httpc.configure(), request.map(invalid, fn(_) { <<>> }))
  let assert <<255>> = configured.body
  Nil
}

pub fn check_tls() {
  let assert Ok(secure) = request.to("https://localhost:38200/hello")
  let assert Ok(secure_reply) = httpc.send(secure)
  let assert "hello" = secure_reply.body
  let insecure = httpc.verify_tls(httpc.configure(), False)
  let assert Ok(unverified) = httpc.dispatch(insecure, secure)
  let assert "hello" = unverified.body
  Nil
}
