import envoy
import gleam/dict

pub fn verify() -> Bool {
  let assert Ok("ready") = envoy.get("INITIAL")
  let assert Error(Nil) = envoy.get("MISSING")

  let before = envoy.all()
  let assert Ok("ready") = dict.get(before, "INITIAL")
  let assert Error(Nil) = dict.get(before, "ADDED")

  envoy.set("ADDED", "한글=🙂")
  let assert Ok("한글=🙂") = envoy.get("ADDED")
  let after = envoy.all()
  let assert Ok("한글=🙂") = dict.get(after, "ADDED")
  let assert Error(Nil) = dict.get(before, "ADDED")

  envoy.unset("ADDED")
  let assert Error(Nil) = envoy.get("ADDED")
  let assert Error(Nil) = dict.get(envoy.all(), "ADDED")
  True
}

pub fn write(name: String, value: String) {
  envoy.set(name, value)
}

pub fn contains(name: String, expected: String) -> Bool {
  envoy.get(name) == Ok(expected) && dict.get(envoy.all(), name) == Ok(expected)
}

pub fn clear(name: String) -> Bool {
  envoy.unset(name)
  envoy.get(name) == Error(Nil) && dict.get(envoy.all(), name) == Error(Nil)
}
