import global_value

pub fn first(value: Int) -> Int {
  global_value.create_with_unique_name("embedding.shared", fn() { value })
}

pub fn second() -> Int {
  global_value.create_with_unique_name("embedding.shared", fn() {
    panic as "embedding value initialized twice"
  })
}

pub fn other() -> Int {
  global_value.create_with_unique_name("embedding.other", fn() { 7 })
}
