import gzlib

pub fn main() {
  let data = <<49, 50, 51, 52, 53, 54, 55, 56, 57>>
  let assert Ok(compressed) = gzlib.compress(data)
  assert gzlib.uncompress(compressed) == Ok(data)
  let assert Ok(empty_stream) = gzlib.compress(<<>>)
  assert gzlib.uncompress(empty_stream) == Ok(<<>>)

  let original_stream = <<120, 156, 75, 76, 74, 6, 0, 2, 77, 1, 39>>
  let assert Ok(uncompressed) = gzlib.uncompress(original_stream)
  assert uncompressed == <<97, 98, 99>>
  assert gzlib.uncompress(<<original_stream:bits, 1, 2>>) == Ok(<<97, 98, 99>>)
  assert gzlib.uncompress(<<120, 156, 75, 76, 74, 6, 0, 2, 77, 1>>)
    == Error(Nil)
  assert gzlib.uncompress(<<120, 156, 75, 76, 74, 6, 0, 2, 77, 1, 38>>)
    == Error(Nil)

  let assert Ok(stored) = gzlib.compress_custom(data, level: 0)
  let assert Ok(best) = gzlib.compress_custom(data, level: 9)
  assert gzlib.uncompress(stored) == Ok(data)
  assert gzlib.uncompress(best) == Ok(data)
  assert gzlib.compress_custom(data, level: -1) == Error(Nil)
  assert gzlib.compress_custom(data, level: 10) == Error(Nil)
  assert gzlib.compress(<<5:3>>) == Error(Nil)
  assert gzlib.uncompress(<<>>) == Error(Nil)
  assert gzlib.uncompress(<<5:3>>) == Error(Nil)

  assert gzlib.crc32(data) == Ok(3_421_780_262)
  let assert Ok(first) = gzlib.crc32(<<49, 50, 51, 52>>)
  assert gzlib.crc32_continue(first, <<53, 54, 55, 56, 57>>)
    == Ok(3_421_780_262)
  assert gzlib.crc32_continue(4_294_967_295, <<>>) == Ok(4_294_967_295)
  assert gzlib.crc32_continue(4_294_967_296, <<>>) == Error(Nil)
  assert gzlib.crc32_continue(-1, <<>>) == Error(Nil)
  assert gzlib.crc32(<<5:3>>) == Error(Nil)
  assert gzlib.crc32_continue(0, <<5:3>>) == Error(Nil)
}
