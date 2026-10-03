import gleam/option.{type Option, Some}

pub fn main() {
  let assert True = matches(Some(7))
  let assert False = matches(Some(8))
}

fn matches(expected: Option(Int)) -> Bool {
  case expected {
    candidate if Some(7) == candidate -> True
    _ -> False
  }
}
