import term_size

pub fn get_size() -> Result(#(Int, Int), Nil) {
  term_size.get()
}

pub fn row_count() -> Result(Int, Nil) {
  term_size.rows()
}

pub fn column_count() -> Result(Int, Nil) {
  term_size.columns()
}
