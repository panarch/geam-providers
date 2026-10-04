import argv

pub fn main() {
  let assert ["", "space value", "--", "-dash", "한글"] = argv.load().arguments
}
