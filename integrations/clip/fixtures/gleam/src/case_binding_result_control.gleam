pub type Request {
  Request(enabled: Bool)
}

pub type Options {
  Options(enabled: Bool)
}

pub type Outcome {
  Done(enabled: Bool)
  Failed(message: String)
}

pub fn main() {
  let assert Done(True) = evaluate(Ok(Request(True)))
  let assert Done(False) = evaluate(Ok(Request(False)))
  let assert Failed("invalid") = evaluate(Error("invalid"))
}

pub fn evaluate(input: Result(Request, String)) -> Outcome {
  case input {
    Error(message) -> Failed(message)
    Ok(request) -> {
      let options = Options(request.enabled)
      Done(options.enabled)
    }
  }
}
