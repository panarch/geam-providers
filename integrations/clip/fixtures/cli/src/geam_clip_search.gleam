import argv
import clip
import clip/arg
import clip/flag
import clip/help
import clip/opt
import filepath
import gleam/int
import gleam/io
import gleam/list
import gleam/regexp
import gleam/result
import gleam/string
import simplifile

pub type Outcome {
  Completed(lines: List(String))
  Help(text: String)
  Failed(reason: SearchError)
}

pub type SearchError {
  InvalidArguments(message: String)
  UnexpectedArguments(arguments: List(String))
  InvalidPattern(details: regexp.CompileError)
  ReadFailure(path: String, reason: simplifile.FileError)
  EmptyFilePath
}

type Mode {
  Search
  Count
}

type Request {
  Request(mode: Mode, pattern: String, ignore_case: Bool, files: List(String))
}

const root_help = "Usage: clip-search <search|count> --pattern PATTERN [-i] FILE...\n\nCommands:\n  search  Print matching lines\n  count   Print matching line counts\n\nUse --help after a command for its options."

const search_help = "Usage: clip-search search --pattern PATTERN [-i] FILE...\n\nOptions:\n  --pattern, -p PATTERN  Regular expression\n  --ignore-case, -i     Ignore letter case\n  --help, -h            Print this help\n\nUse -- before file names starting with a dash."

const count_help = "Usage: clip-search count --pattern PATTERN [-i] FILE...\n\nOptions:\n  --pattern, -p PATTERN  Regular expression\n  --ignore-case, -i     Ignore letter case\n  --help, -h            Print this help\n\nUse -- before file names starting with a dash."

pub fn main() {
  case run() {
    Completed(_) | Help(_) -> Nil
    Failed(ReadFailure(_, _)) -> exit(1)
    Failed(_) -> exit(2)
  }
}

@external(erlang, "erlang", "halt")
fn exit(status: Int) -> Nil

/// Execute the same argv-driven application in a process or Rust embedding.
pub fn run() -> Outcome {
  let outcome = evaluate(argv.load().arguments)
  case outcome {
    Completed(lines) -> list.each(lines, io.println)
    Help(text) -> io.println(text)
    Failed(reason) -> io.println_error(describe_error(reason))
  }
  outcome
}

pub fn evaluate(arguments: List(String)) -> Outcome {
  case clip.parse(parser(), arguments) {
    Error(message) -> {
      case
        message == root_help || message == search_help || message == count_help
      {
        True -> Help(message)
        False -> Failed(InvalidArguments(message))
      }
    }
    Ok(#(_, [_, ..] as remaining)) -> Failed(UnexpectedArguments(remaining))
    Ok(#(request, [])) -> {
      let options =
        regexp.Options(case_insensitive: request.ignore_case, multi_line: False)
      case regexp.compile(request.pattern, options) {
        Error(details) -> Failed(InvalidPattern(details))
        Ok(pattern) -> {
          case read_files(request.mode, pattern, request.files) {
            Ok(lines) -> Completed(lines)
            Error(reason) -> Failed(reason)
          }
        }
      }
    }
  }
}

fn parser() -> clip.Command(Request) {
  clip.subcommands([
    #("search", file_command(Search, search_help)),
    #("count", file_command(Count, count_help)),
  ])
  |> clip.help(help.custom(fn(_) { root_help }))
}

fn file_command(mode: Mode, help_text: String) -> clip.Command(Request) {
  let build = {
    use pattern <- clip.parameter
    use ignore_case <- clip.parameter
    use files <- clip.parameter
    Request(mode, pattern, ignore_case, files)
  }
  clip.command(build)
  |> clip.opt(opt.new("pattern") |> opt.short("p"))
  |> clip.flag(flag.new("ignore-case") |> flag.short("i"))
  |> clip.arg_many1(arg.new("file"))
  |> clip.help(help.custom(fn(_) { help_text }))
}

fn read_files(
  mode: Mode,
  pattern: regexp.Regexp,
  paths: List(String),
) -> Result(List(String), SearchError) {
  case paths {
    [] -> Ok([])
    ["", ..] -> Error(EmptyFilePath)
    [path, ..rest] -> {
      // filepath 1.1.2's directory_name only parses forward slashes.
      // Preserve paths containing native Windows separators for the filesystem.
      let read_path = case string.contains(path, "\\") {
        True -> path
        False ->
          filepath.join(filepath.directory_name(path), filepath.base_name(path))
      }
      use text <- result.try(
        simplifile.read(read_path)
        |> result.map_error(fn(reason) { ReadFailure(path, reason) }),
      )
      let matches =
        lines(text)
        |> list.index_map(fn(line, index) { #(index + 1, line) })
        |> list.filter(fn(line) { regexp.check(pattern, line.1) })
      let output = case mode {
        Search ->
          list.map(matches, fn(line) {
            path <> ":" <> int.to_string(line.0) <> ":" <> line.1
          })
        Count -> [path <> ":" <> int.to_string(list.length(matches))]
      }
      use following <- result.try(read_files(mode, pattern, rest))
      Ok(list.append(output, following))
    }
  }
}

fn lines(text: String) -> List(String) {
  case text {
    "" -> []
    _ -> {
      let split = string.split(text, "\n")
      let split = case string.ends_with(text, "\n") {
        True -> list.take(split, list.length(split) - 1)
        False -> split
      }
      list.map(split, fn(line) {
        case string.ends_with(line, "\r") {
          True -> string.drop_end(line, 1)
          False -> line
        }
      })
    }
  }
}

pub fn describe_error(error: SearchError) -> String {
  case error {
    InvalidArguments(message) -> "arguments: " <> message
    UnexpectedArguments(arguments) ->
      "unexpected arguments: " <> string.join(arguments, " ")
    InvalidPattern(_) -> "invalid regular expression"
    ReadFailure(path, reason) ->
      "cannot read " <> path <> ": " <> simplifile.describe_error(reason)
    EmptyFilePath -> "file path must not be empty"
  }
}
