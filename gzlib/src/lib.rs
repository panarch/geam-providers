use geam::provider::{BigInt, BitArrayValue};

#[geam::provider(package = "gzlib", modules = [gzlib])]
pub struct Component;

#[geam::module(path = "gzlib")]
mod gzlib {
    use super::{BigInt, BitArrayValue};
    use crc32fast::Hasher;
    use miniz_oxide::deflate::compress_to_vec_zlib;
    use miniz_oxide::inflate::decompress_to_vec_zlib;

    const DEFAULT_LEVEL: u8 = 6;

    #[geam::function]
    fn compress(data: BitArrayValue) -> Result<BitArrayValue, ()> {
        compress_at_level(data, DEFAULT_LEVEL)
    }

    #[geam::function]
    fn compress_custom(data: BitArrayValue, level: BigInt) -> Result<BitArrayValue, ()> {
        let level = u8::try_from(&level).map_err(|_| ())?;
        if level > 9 {
            return Err(());
        }
        compress_at_level(data, level)
    }

    #[geam::function]
    fn uncompress(data: BitArrayValue) -> Result<BitArrayValue, ()> {
        let output = decompress_to_vec_zlib(aligned_bytes(&data)?).map_err(|_| ())?;
        Ok(BitArrayValue::from_bytes(output))
    }

    #[geam::function]
    fn crc32(data: BitArrayValue) -> Result<BigInt, ()> {
        let bytes = aligned_bytes(&data)?;
        Ok(BigInt::from(crc32fast::hash(bytes)))
    }

    #[geam::function]
    fn crc32_continue(crc: BigInt, data: BitArrayValue) -> Result<BigInt, ()> {
        let crc = u32::try_from(&crc).map_err(|_| ())?;
        let bytes = aligned_bytes(&data)?;
        let mut hasher = Hasher::new_with_initial(crc);
        hasher.update(bytes);
        Ok(BigInt::from(hasher.finalize()))
    }

    fn compress_at_level(data: BitArrayValue, level: u8) -> Result<BitArrayValue, ()> {
        let bytes = aligned_bytes(&data)?;
        Ok(BitArrayValue::from_bytes(compress_to_vec_zlib(
            bytes, level,
        )))
    }

    fn aligned_bytes(value: &BitArrayValue) -> Result<&[u8], ()> {
        if value.bit_len().is_multiple_of(8) {
            Ok(value.bytes())
        } else {
            Err(())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::{compress, compress_custom, crc32, crc32_continue, uncompress};
        use geam::provider::{BigInt, BitArrayValue};

        fn bits(bytes: &[u8]) -> BitArrayValue {
            BitArrayValue::from_bytes(bytes.to_vec())
        }

        fn unaligned() -> BitArrayValue {
            BitArrayValue::try_from_parts(vec![0xa0], 3).expect("three bits fit in one byte")
        }

        #[test]
        fn compression_levels_emit_decodable_zlib_streams() {
            let input = bits(b"a repeated value a repeated value");
            for level in [0, 1, 6, 9] {
                let compressed = compress_custom(input.clone(), BigInt::from(level))
                    .expect("valid compression level");
                assert_eq!(uncompress(compressed), Ok(input.clone()));
            }
            let default = compress(input.clone()).expect("default compression");
            assert_eq!(uncompress(default), Ok(input));
            let empty = bits(b"");
            let empty_stream = compress(empty.clone()).expect("empty input is compressible");
            assert_eq!(uncompress(empty_stream), Ok(empty));
        }

        #[test]
        fn invalid_levels_and_non_byte_aligned_inputs_return_nil_errors() {
            let input = bits(b"abc");
            for level in [-1, 10, 256] {
                assert_eq!(compress_custom(input.clone(), BigInt::from(level)), Err(()));
            }
            let partial = unaligned();
            assert_eq!(compress(partial.clone()), Err(()));
            assert_eq!(compress_custom(partial.clone(), BigInt::from(6)), Err(()));
            assert_eq!(uncompress(partial.clone()), Err(()));
            assert_eq!(crc32(partial.clone()), Err(()));
            assert_eq!(crc32_continue(BigInt::from(0), partial), Err(()));
        }

        #[test]
        fn decompression_rejects_invalid_or_incomplete_streams() {
            let original = [120, 156, 75, 76, 74, 6, 0, 2, 77, 1, 39];
            assert_eq!(uncompress(bits(&original)), Ok(bits(b"abc")));
            assert_eq!(uncompress(bits(&original[..original.len() - 1])), Err(()));
            assert_eq!(uncompress(bits(b"abc")), Err(()));
            assert_eq!(uncompress(bits(b"")), Err(()));
            let mut corrupt = original;
            corrupt[original.len() - 1] ^= 1;
            assert_eq!(uncompress(bits(&corrupt)), Err(()));
            let mut trailing = original.to_vec();
            trailing.extend_from_slice(b"extra");
            assert_eq!(uncompress(bits(&trailing)), Ok(bits(b"abc")));
        }

        #[test]
        fn crc32_matches_standard_and_continued_checksums() {
            assert_eq!(crc32(bits(b"123456789")), Ok(BigInt::from(0xcbf4_3926u32)));
            let first = crc32(bits(b"1234")).expect("first chunk checksum");
            assert_eq!(
                crc32_continue(first, bits(b"56789")),
                Ok(BigInt::from(0xcbf4_3926u32))
            );
            assert_eq!(crc32(bits(b"")), Ok(BigInt::from(0)));
            assert_eq!(
                crc32_continue(BigInt::from(u32::MAX), bits(b"")),
                Ok(BigInt::from(u32::MAX))
            );
            for invalid in [-1_i64, u32::MAX as i64 + 1] {
                assert_eq!(crc32_continue(BigInt::from(invalid), bits(b"")), Err(()));
            }
        }
    }
}
