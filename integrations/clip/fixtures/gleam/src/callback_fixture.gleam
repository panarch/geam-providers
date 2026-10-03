import clip
import clip/arg
import clip/opt
import gleam/io

pub fn verify() -> Bool {
  let mapped =
    arg.new("trace")
    |> arg.map(fn(value) {
      io.println("arg.map")
      value <> ":first"
    })
    |> arg.try_map(fn(value) {
      io.println("arg.try_map")
      Ok(value <> ":second")
    })
  let command = clip.command1() |> clip.arg(mapped)
  let assert Ok("one:first:second") = clip.run(command, ["one"])
  let assert Ok("two:first:second") = clip.run(command, ["two"])
  let named =
    opt.new("trace")
    |> opt.map(fn(value) {
      io.println("opt.map")
      value <> ":first"
    })
    |> opt.try_map(fn(value) {
      io.println("opt.try_map")
      Ok(value <> ":second")
    })
  let assert Ok("option:first:second") =
    clip.run(clip.command1() |> clip.opt(named), ["--trace", "option"])
  let assert Ok(4) =
    clip.run(
      clip.apply(
        clip.return(fn(value) {
          io.println("apply")
          value + 1
        }),
        clip.return(3),
      ),
      [],
    )
  let failed_arg =
    arg.new("trace")
    |> arg.try_map(fn(_) {
      io.println("arg.fail")
      Error("argument failure")
    })
    |> arg.map(fn(_) { panic as "must not execute after argument failure" })
  let assert Error("argument failure") =
    clip.run(clip.command1() |> clip.arg(failed_arg), ["value"])
  let failed_opt =
    opt.new("trace")
    |> opt.try_map(fn(_) {
      io.println("opt.fail")
      Error("option failure")
    })
    |> opt.map(fn(_) { panic as "must not execute after option failure" })
  let assert Error("option failure") =
    clip.run(clip.command1() |> clip.opt(failed_opt), ["--trace", "value"])
  True
}
