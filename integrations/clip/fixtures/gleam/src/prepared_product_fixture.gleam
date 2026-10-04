import gleam/option.{type Option, None, Some}

pub type Repeat {
  NoRepeat
  ManyRepeat
  Many1Repeat
}

pub fn main() {
  let assert "many" = label(ManyRepeat, None)
  let assert "many1" = label(Many1Repeat, Some("value"))
  let assert "required" = label(NoRepeat, None)
  let assert "Error(Nil)" = label(NoRepeat, Some("Error(Nil)"))
  let assert "value" = label(NoRepeat, Some("value"))
}

pub fn label(repeat: Repeat, default: Option(String)) -> String {
  case repeat, default {
    ManyRepeat, _ -> "many"
    Many1Repeat, _ -> "many1"
    NoRepeat, None -> "required"
    NoRepeat, Some(value) -> value
  }
}
