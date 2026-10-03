import gleam/option.{type Option, None, Some}

pub fn main() {
  let assert "required" = label(None)
  let assert "required" = label(Some("Error(Nil)"))
  let assert "value" = label(Some("value"))
}

pub fn label(default: Option(String)) -> String {
  case default {
    None | Some("Error(Nil)") -> "required"
    Some(value) -> value
  }
}
