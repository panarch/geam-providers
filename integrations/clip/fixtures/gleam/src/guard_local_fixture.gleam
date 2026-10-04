import gleam/option.{type Option, Some}

pub fn main() {
  let assert True = matches(7, Some(7))
  let assert False = matches(7, Some(8))
}

pub fn matches(value: Int, expected: Option(Int)) -> Bool {
  case expected {
    candidate if Some(value) == candidate -> True
    _ -> False
  }
}
