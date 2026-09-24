import logging.{Debug, Error, Info}

pub fn main() {
  logging.configure()
  logging.log(Info, "hello")
  logging.log(Debug, "hidden")
  logging.set_level(Debug)
  logging.log(Debug, "trace")
  logging.set_level(Error)
  logging.log(Info, "hidden again")
  logging.log(Error, "problem")
}
