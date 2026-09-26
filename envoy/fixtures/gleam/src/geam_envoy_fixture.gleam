import envoy
import gleam/dict

pub fn main() {
  let name = "GEAM_ENVOY_FIXTURE_VALUE"
  envoy.unset(name)
  let assert Error(Nil) = envoy.get(name)
  let before = envoy.all()
  let assert Error(Nil) = dict.get(before, name)

  envoy.set(name, "fixture-value")
  let assert Ok("fixture-value") = envoy.get(name)
  let after = envoy.all()
  let assert Ok("fixture-value") = dict.get(after, name)
  let assert Error(Nil) = dict.get(before, name)

  envoy.set(name, "")
  let assert Ok("") = envoy.get(name)
  envoy.unset(name)
  let assert Error(Nil) = envoy.get(name)
  let assert Error(Nil) = dict.get(envoy.all(), name)
}
