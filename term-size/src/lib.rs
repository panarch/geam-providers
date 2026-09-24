use geam::provider::BigInt;
use terminal_size::{Height, Width, terminal_size};

#[geam::provider(package = "term_size", modules = [term_size])]
pub struct Component;

#[geam::module(path = "term_size")]
mod term_size {
    use super::{BigInt, Height, Width, terminal_size};

    #[geam::function]
    fn get() -> Result<(BigInt, BigInt), ()> {
        size_from_host(terminal_size())
    }

    fn size_from_host(size: Option<(Width, Height)>) -> Result<(BigInt, BigInt), ()> {
        match size {
            Some((Width(columns), Height(rows))) => Ok((BigInt::from(rows), BigInt::from(columns))),
            None => Err(()),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::size_from_host;
        use geam::provider::BigInt;
        use terminal_size::{Height, Width};

        #[test]
        fn host_dimensions_become_rows_then_columns() {
            assert_eq!(
                size_from_host(Some((Width(101), Height(37)))),
                Ok((BigInt::from(37), BigInt::from(101)))
            );
        }

        #[test]
        fn missing_host_terminal_is_a_source_error() {
            assert_eq!(size_from_host(None), Err(()));
        }
    }
}
