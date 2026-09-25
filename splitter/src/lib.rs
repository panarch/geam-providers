use geam::provider::{EcoString, ExternalPayload, StringValue};
use memchr::memmem::Finder;
use std::ops::Range;
use std::sync::atomic::{AtomicU64, Ordering};

#[geam::provider(package = "splitter", modules = [splitter])]
pub struct Component;

#[geam::module(path = "splitter")]
mod splitter {
    use super::{AtomicU64, EcoString, ExternalPayload, Finder, Ordering, Range, StringValue};

    static NEXT_SPLITTER_ID: AtomicU64 = AtomicU64::new(1);

    #[geam::external(name = "Splitter", manual)]
    struct Splitter {
        id: Option<u64>,
        finders: Vec<Finder<'static>>,
    }

    impl Splitter {
        fn new(patterns: impl IntoIterator<Item = StringValue>) -> Self {
            let finders: Vec<_> = patterns
                .into_iter()
                .filter(|pattern| !pattern.as_str().is_empty())
                .map(|pattern| Finder::new(pattern.as_str().as_bytes()).into_owned())
                .collect();
            let id =
                (!finders.is_empty()).then(|| NEXT_SPLITTER_ID.fetch_add(1, Ordering::Relaxed));
            Self { id, finders }
        }

        fn first_match(&self, input: &str, from: usize) -> Option<Range<usize>> {
            let mut first: Option<Range<usize>> = None;
            for finder in &self.finders {
                if let Some(relative) = finder.find(&input.as_bytes()[from..]) {
                    let start = from + relative;
                    let end = start + finder.needle().len();
                    if first.as_ref().is_none_or(|previous| {
                        start < previous.start || start == previous.start && end > previous.end
                    }) {
                        first = Some(start..end);
                    }
                }
            }
            first
        }

        fn split(&self, input: StringValue) -> (StringValue, StringValue, StringValue) {
            if self.finders.is_empty() {
                return ("".into(), "".into(), input);
            }
            match self.first_match(input.as_str(), 0) {
                Some(found) => (
                    input.slice(0..found.start),
                    input.slice(found.clone()),
                    input.slice(found.end..input.len()),
                ),
                None => (input, "".into(), "".into()),
            }
        }

        fn split_before(&self, input: StringValue) -> (StringValue, StringValue) {
            if self.finders.is_empty() {
                return ("".into(), input);
            }
            match self.first_match(input.as_str(), 0) {
                Some(found) => (
                    input.slice(0..found.start),
                    input.slice(found.start..input.len()),
                ),
                None => (input, "".into()),
            }
        }

        fn split_after(&self, input: StringValue) -> (StringValue, StringValue) {
            if self.finders.is_empty() {
                return ("".into(), input);
            }
            match self.first_match(input.as_str(), 0) {
                Some(found) => (
                    input.slice(0..found.end),
                    input.slice(found.end..input.len()),
                ),
                None => (input, "".into()),
            }
        }

        fn would_split(&self, input: StringValue) -> bool {
            self.first_match(input.as_str(), 0).is_some()
        }

        fn split_all(&self, input: StringValue) -> Vec<StringValue> {
            if self.finders.is_empty() {
                return vec![input];
            }
            let mut parts = Vec::new();
            let mut from = 0;
            while let Some(found) = self.first_match(input.as_str(), from) {
                parts.push(input.slice(from..found.start));
                from = found.end;
            }
            parts.push(input.slice(from..input.len()));
            parts
        }
    }

    impl ExternalPayload for Splitter {
        fn source_equal(&self, other: &Self) -> bool {
            self.id == other.id
        }

        fn source_hash(&self) -> u64 {
            self.id.unwrap_or(0)
        }

        fn inspect(&self) -> EcoString {
            "#<splitter.Splitter>".into()
        }
    }

    #[geam::function]
    fn make(patterns: geam::List<StringValue>) -> Splitter {
        let mut index = 0;
        Splitter::new(std::iter::from_fn(|| {
            let pattern = patterns.get(index);
            index += 1;
            pattern
        }))
    }

    #[geam::function]
    fn split(splitter: &Splitter, input: StringValue) -> (StringValue, StringValue, StringValue) {
        splitter.split(input)
    }

    #[geam::function]
    fn split_before(splitter: &Splitter, input: StringValue) -> (StringValue, StringValue) {
        splitter.split_before(input)
    }

    #[geam::function]
    fn split_after(splitter: &Splitter, input: StringValue) -> (StringValue, StringValue) {
        splitter.split_after(input)
    }

    #[geam::function]
    fn would_split(splitter: &Splitter, input: StringValue) -> bool {
        splitter.would_split(input)
    }

    #[geam::function]
    fn split_all(splitter: &Splitter, input: StringValue) -> Vec<StringValue> {
        splitter.split_all(input)
    }

    #[cfg(test)]
    mod tests {
        use super::{ExternalPayload, Splitter, StringValue};

        #[test]
        fn earliest_match_wins_and_longest_match_breaks_ties() {
            let patterns = Splitter::new(["b".into(), "ab".into(), "a".into()]);
            assert_eq!(
                patterns.split("zab!".into()),
                ("z".into(), "ab".into(), "!".into())
            );
            assert_eq!(
                patterns.split_before("zab!".into()),
                ("z".into(), "ab!".into())
            );
            assert_eq!(
                patterns.split_after("zab!".into()),
                ("zab".into(), "!".into())
            );
            assert!(patterns.would_split("zab!".into()));
            assert_eq!(
                patterns.split_all("zab!a".into()),
                vec![StringValue::from("z"), "!".into(), "".into()]
            );
        }

        #[test]
        fn unmatched_and_empty_splitters_preserve_upstream_shapes() {
            let patterns = Splitter::new(["x".into()]);
            assert_eq!(
                patterns.split("abc".into()),
                ("abc".into(), "".into(), "".into())
            );
            assert_eq!(
                patterns.split_before("abc".into()),
                ("abc".into(), "".into())
            );
            assert_eq!(
                patterns.split_after("abc".into()),
                ("abc".into(), "".into())
            );
            assert!(!patterns.would_split("abc".into()));
            assert_eq!(
                patterns.split_all("abc".into()),
                vec![StringValue::from("abc")]
            );

            let empty = Splitter::new(["".into()]);
            let another_empty = Splitter::new([]);
            assert_eq!(
                empty.split("abc".into()),
                ("".into(), "".into(), "abc".into())
            );
            assert_eq!(empty.split_before("abc".into()), ("".into(), "abc".into()));
            assert_eq!(empty.split_after("abc".into()), ("".into(), "abc".into()));
            assert!(!empty.would_split("abc".into()));
            assert_eq!(
                empty.split_all("abc".into()),
                vec![StringValue::from("abc")]
            );
            assert!(empty.source_equal(&another_empty));
            assert_eq!(empty.source_hash(), another_empty.source_hash());
        }

        #[test]
        fn unicode_slices_and_opaque_identity() {
            let first = Splitter::new(["é".into()]);
            let second = Splitter::new(["é".into()]);
            assert_eq!(
                first.split("가é나".into()),
                ("가".into(), "é".into(), "나".into())
            );
            assert!(first.source_equal(&first));
            assert!(!first.source_equal(&second));
            assert_ne!(first.source_hash(), second.source_hash());
            assert_eq!(first.inspect(), "#<splitter.Splitter>");
        }
    }
}
