import callback_fixture
import case_binding_bool_fixture
import case_binding_fixture
import case_binding_result_control
import case_binding_tuple_control
import clip
import clip/arg
import clip/help
import clip/opt
import clip_contracts
import guard_literal_fixture
import guard_local_fixture
import prepared_option_fixture
import prepared_product_fixture
import prepared_repeat_fixture

pub fn main() {
  let assert True = verify()
}

pub fn verify() -> Bool {
  case_binding_bool_fixture.main()
  case_binding_fixture.main()
  case_binding_tuple_control.main()
  case_binding_result_control.main()
  guard_local_fixture.main()
  guard_literal_fixture.main()
  prepared_product_fixture.main()
  prepared_repeat_fixture.main()
  prepared_option_fixture.main()
  let assert True = clip_contracts.verify()
  let assert True = callback_fixture.verify()
  let command =
    clip.command2()
    |> clip.arg(arg.new("file"))
    |> clip.opt(opt.new("limit") |> opt.int)
    |> clip.help(
      help.custom(fn(info) {
        let assert [named] = info.named
        "help: " <> named.name
      }),
    )
  let assert Ok(#("notes.txt", 7)) =
    clip.run(command, ["--limit", "7", "notes.txt"])
  let assert Error("help: limit") = clip.run(command, ["--help"])
  True
}
