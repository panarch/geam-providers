import argv
import geam_clip_fixture
import geam_clip_search
import simplifile

pub fn verify_contracts() -> Bool {
  geam_clip_fixture.verify()
}

pub fn arguments() -> List(String) {
  argv.load().arguments
}

pub fn identity() -> #(String, String) {
  let values = argv.load()
  #(values.runtime, values.program)
}

pub fn run() -> geam_clip_search.Outcome {
  geam_clip_search.run()
}

pub fn main() {
  geam_clip_search.main()
}

// Keep the application's custom value in its execution scope. These typed
// projections let the host inspect it through Gleam without a Rust-side ABI.
pub fn completed(
  outcome: geam_clip_search.Outcome,
) -> Result(List(String), Nil) {
  case outcome {
    geam_clip_search.Completed(lines) -> Ok(lines)
    _ -> Error(Nil)
  }
}

pub fn help_text(outcome: geam_clip_search.Outcome) -> Result(String, Nil) {
  case outcome {
    geam_clip_search.Help(text) -> Ok(text)
    _ -> Error(Nil)
  }
}

pub fn failure_message(
  outcome: geam_clip_search.Outcome,
) -> Result(String, Nil) {
  case outcome {
    geam_clip_search.Failed(reason) ->
      Ok(geam_clip_search.describe_error(reason))
    _ -> Error(Nil)
  }
}

pub fn missing_file(outcome: geam_clip_search.Outcome) -> Result(String, Nil) {
  case outcome {
    geam_clip_search.Failed(geam_clip_search.ReadFailure(
      path,
      simplifile.Enoent,
    )) -> Ok(path)
    _ -> Error(Nil)
  }
}

pub fn invalid_pattern(outcome: geam_clip_search.Outcome) -> Bool {
  case outcome {
    geam_clip_search.Failed(geam_clip_search.InvalidPattern(_)) -> True
    _ -> False
  }
}
