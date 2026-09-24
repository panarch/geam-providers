import logging.{Alert, Critical, Debug, Emergency, Error, Info, Notice, Warning}

pub fn log_all_levels() {
  logging.configure()
  logging.set_level(Debug)
  logging.log(Emergency, "emergency")
  logging.log(Alert, "alert")
  logging.log(Critical, "critical")
  logging.log(Error, "error")
  logging.log(Warning, "warning")
  logging.log(Notice, "notice")
  logging.log(Info, "info")
  logging.log(Debug, "debug")
}

pub fn filter_and_reset() {
  logging.configure()
  logging.log(Debug, "hidden")
  logging.set_level(Error)
  logging.log(Warning, "also hidden")
  logging.log(Error, "visible error")
  logging.configure()
  logging.log(Info, "visible after reset")
}

pub fn log_one() {
  logging.log(Info, "one")
}
