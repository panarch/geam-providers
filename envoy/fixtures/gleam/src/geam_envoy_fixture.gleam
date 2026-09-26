import envoy
import gleam/dict

pub fn main() {
  let name = "GEAM_ENVOY_FIXTURE_VALUE"
  envoy.unset(name)
  let assert Error(Nil) = envoy.get(name)
  let before = envoy.all()
  let assert Error(Nil) = dict.get(before, name)

  envoy.set(name, "한국어=🙂")
  let assert Ok("한국어=🙂") = envoy.get(name)
  let after = envoy.all()
  let assert Ok("한국어=🙂") = dict.get(after, name)
  let assert Error(Nil) = dict.get(before, name)

  envoy.set(name, "")
  let assert Ok("") = envoy.get(name)
  envoy.unset(name)
  let assert Error(Nil) = envoy.get(name)
  let assert Error(Nil) = dict.get(envoy.all(), name)
}
