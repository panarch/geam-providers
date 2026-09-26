import gleam/io
import operating_system

pub fn main() {
  io.println(operating_system.name())
}
