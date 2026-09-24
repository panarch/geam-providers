use geam::provider::{BigInt, Call, Callback, ExternalPayload, HostResult, StringValue};
use regex::{Regex, RegexBuilder};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[geam::provider(package = "gleam_regexp", modules = [regexp])]
pub struct Component;

#[geam::module(path = "gleam/regexp")]
mod regexp {
    use super::{
        BigInt, Call, Callback, DefaultHasher, ExternalPayload, Hash, Hasher, HostResult, Regex,
        RegexBuilder, StringValue,
    };

    #[geam::external(name = "Regexp", manual)]
    struct Regexp {
        pattern: StringValue,
        case_insensitive: bool,
        multi_line: bool,
        regex: Regex,
    }

    impl ExternalPayload for Regexp {
        fn source_equal(&self, other: &Self) -> bool {
            self.pattern == other.pattern
                && self.case_insensitive == other.case_insensitive
                && self.multi_line == other.multi_line
        }

        fn source_hash(&self) -> u64 {
            let mut hasher = DefaultHasher::new();
            self.pattern.hash(&mut hasher);
            self.case_insensitive.hash(&mut hasher);
            self.multi_line.hash(&mut hasher);
            hasher.finish()
        }

        fn inspect(&self) -> geam::provider::EcoString {
            format!(
                "Regexp({:?}, case_insensitive: {}, multi_line: {})",
                self.pattern, self.case_insensitive, self.multi_line,
            )
            .into()
        }
    }

    #[allow(dead_code)]
    #[geam::custom(input = OptionsInput)]
    enum Options {
        Options {
            case_insensitive: bool,
            multi_line: bool,
        },
    }

    #[derive(Debug)]
    #[geam::custom]
    enum CompileError {
        CompileError {
            error: StringValue,
            byte_index: BigInt,
        },
    }

    #[geam::custom]
    enum Match {
        Match {
            content: StringValue,
            submatches: Vec<Option<StringValue>>,
        },
    }

    #[geam::function]
    fn do_compile(pattern: StringValue, options: OptionsInput) -> Result<Regexp, CompileError> {
        let OptionsInput::Options {
            case_insensitive,
            multi_line,
        } = options;
        if let Err(error) = regex_syntax::ast::parse::Parser::new().parse(pattern.as_str()) {
            let byte_index = if matches!(error.kind(), regex_syntax::ast::ErrorKind::ClassUnclosed)
            {
                pattern.len()
            } else {
                error.span().start.offset
            };
            return Err(CompileError::CompileError {
                error: error.kind().to_string().into(),
                byte_index: BigInt::from(byte_index),
            });
        }
        match RegexBuilder::new(pattern.as_str())
            .case_insensitive(case_insensitive)
            .multi_line(multi_line)
            .build()
        {
            Ok(regex) => Ok(Regexp {
                pattern,
                case_insensitive,
                multi_line,
                regex,
            }),
            Err(error) => Err(CompileError::CompileError {
                error: error.to_string().into(),
                byte_index: BigInt::from(0),
            }),
        }
    }

    #[geam::function]
    fn do_check(regexp: &Regexp, string: StringValue) -> bool {
        regexp.regex.is_match(string.as_str())
    }

    #[geam::function]
    fn do_split(regexp: &Regexp, string: StringValue) -> Vec<StringValue> {
        split_value(&regexp.regex, string)
    }

    fn split_value(regex: &Regex, string: StringValue) -> Vec<StringValue> {
        let mut pieces = Vec::new();
        let mut previous_end = 0;
        for captures in regex.captures_iter(string.as_str()) {
            let matched = captures.get_match();
            pieces.push(string.slice(previous_end..matched.start()));
            for group in captures.iter().skip(1) {
                pieces.push(match group {
                    Some(group) => string.slice(group.range()),
                    None => StringValue::from(""),
                });
            }
            previous_end = matched.end();
        }
        pieces.push(string.slice(previous_end..string.len()));
        pieces
    }

    #[geam::function]
    fn do_scan(regexp: &Regexp, string: StringValue) -> Vec<Match> {
        regexp
            .regex
            .captures_iter(string.as_str())
            .map(|captures| match_value(&string, &captures))
            .collect()
    }

    #[geam::function]
    fn replace(regexp: &Regexp, string: StringValue, substitute: StringValue) -> StringValue {
        match regexp
            .regex
            .replace_all(string.as_str(), substitute.as_str())
        {
            std::borrow::Cow::Borrowed(_) => string,
            std::borrow::Cow::Owned(replaced) => replaced.into(),
        }
    }

    #[geam::function(await)]
    async fn match_map(
        #[geam::call] call: &mut Call<()>,
        regexp: &Regexp,
        string: StringValue,
        substitute: Callback<fn(Match) -> StringValue>,
    ) -> HostResult<StringValue> {
        let regex = regexp.with(|value| value.regex.clone());
        let mut output = String::new();
        let mut matched = false;
        let mut previous_end = 0;
        for captures in regex.captures_iter(string.as_str()) {
            let full = captures.get_match();
            if !matched {
                output.reserve(string.len());
                matched = true;
            }
            output.push_str(&string.as_str()[previous_end..full.start()]);
            let replacement = call
                .invoke(&substitute, (match_value(&string, &captures),))
                .await?;
            output.push_str(replacement.as_str());
            previous_end = full.end();
        }
        if matched {
            output.push_str(&string.as_str()[previous_end..]);
            Ok(output.into())
        } else {
            Ok(string)
        }
    }

    fn match_value(string: &StringValue, captures: &regex::Captures<'_>) -> Match {
        let full = captures.get_match();
        let mut submatches = captures
            .iter()
            .skip(1)
            .map(|capture| {
                capture
                    .filter(|capture| !capture.is_empty())
                    .map(|capture| string.slice(capture.range()))
            })
            .collect::<Vec<_>>();
        while submatches.last().is_some_and(Option::is_none) {
            submatches.pop();
        }
        Match::Match {
            content: string.slice(full.range()),
            submatches,
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn compile(pattern: &str) -> Regexp {
            do_compile(
                pattern.into(),
                OptionsInput::Options {
                    case_insensitive: false,
                    multi_line: false,
                },
            )
            .expect("fixture pattern compiles")
        }

        #[test]
        fn compilation_options_and_errors() {
            let plain = compile("^foo");
            assert!(!plain.regex.is_match("bar\nFOO"));
            let configured = do_compile(
                "^foo".into(),
                OptionsInput::Options {
                    case_insensitive: true,
                    multi_line: true,
                },
            )
            .expect("configured expression compiles");
            assert!(configured.regex.is_match("bar\nFOO"));

            let CompileError::CompileError { byte_index, .. } = do_compile(
                "[0-9".into(),
                OptionsInput::Options {
                    case_insensitive: false,
                    multi_line: false,
                },
            )
            .err()
            .expect("unclosed class is rejected");
            assert_eq!(byte_index, BigInt::from(4));

            let CompileError::CompileError { byte_index, .. } = do_compile(
                "(".into(),
                OptionsInput::Options {
                    case_insensitive: false,
                    multi_line: false,
                },
            )
            .err()
            .expect("unclosed group is rejected");
            assert_eq!(byte_index, BigInt::from(0));

            let CompileError::CompileError { byte_index, .. } = do_compile(
                r"\p{NotAUnicodeClass}".into(),
                OptionsInput::Options {
                    case_insensitive: false,
                    multi_line: false,
                },
            )
            .err()
            .expect("unknown Unicode property is rejected");
            assert_eq!(byte_index, BigInt::from(0));
        }

        #[test]
        fn external_value_equality_hash_and_inspection_use_source_configuration() {
            let same = compile("a+");
            let equal = compile("a+");
            let other_pattern = compile("b+");
            let other_options = do_compile(
                "a+".into(),
                OptionsInput::Options {
                    case_insensitive: true,
                    multi_line: false,
                },
            )
            .expect("configured expression compiles");
            let other_multi_line = do_compile(
                "a+".into(),
                OptionsInput::Options {
                    case_insensitive: false,
                    multi_line: true,
                },
            )
            .expect("multi-line expression compiles");

            assert!(same.source_equal(&equal));
            assert_eq!(same.source_hash(), equal.source_hash());
            assert!(!same.source_equal(&other_pattern));
            assert!(!same.source_equal(&other_options));
            assert!(!same.source_equal(&other_multi_line));
            assert_ne!(same.source_hash(), other_options.source_hash());
            assert_ne!(same.source_hash(), other_multi_line.source_hash());
            assert_eq!(
                same.inspect().as_str(),
                "Regexp(\"a+\", case_insensitive: false, multi_line: false)"
            );
        }

        #[test]
        fn scans_unicode_optional_and_empty_captures() {
            let text: StringValue = "é-12, b-3".into();
            let regex = compile(r"(\w+)-(\d+)");
            let matches = regex
                .regex
                .captures_iter(text.as_str())
                .map(|captures| match_value(&text, &captures))
                .collect::<Vec<_>>();
            assert_eq!(matches.len(), 2);
            let Match::Match {
                content: first,
                submatches: first_parts,
            } = &matches[0];
            let Match::Match {
                content: second,
                submatches: second_parts,
            } = &matches[1];
            assert_eq!(first.as_str(), "é-12");
            assert_eq!(first_parts, &[Some("é".into()), Some("12".into())]);
            assert_eq!(second.as_str(), "b-3");
            assert_eq!(second_parts, &[Some("b".into()), Some("3".into())]);

            let text: StringValue = "b".into();
            let regex = compile(r"(a*)(b)");
            let captures = regex.regex.captures(text.as_str()).expect("one match");
            let Match::Match {
                content,
                submatches,
            } = match_value(&text, &captures);
            assert_eq!(content.as_str(), "b");
            assert_eq!(submatches, &[None, Some("b".into())]);

            let regex = compile(r"(a)?b");
            let captures = regex.regex.captures(text.as_str()).expect("one match");
            let Match::Match { submatches, .. } = match_value(&text, &captures);
            assert!(submatches.is_empty());
            assert!(!compile("z+").regex.is_match(text.as_str()));
        }

        #[test]
        fn split_keeps_captured_delimiters_and_empty_optional_groups() {
            assert_eq!(
                split_value(&compile("([+-])").regex, "-01:00".into()),
                vec!["", "-", "01:00"]
            );
            assert_eq!(
                split_value(&compile("(a)?b").regex, "1b2ab3".into()),
                vec!["1", "", "2", "a", "3"]
            );
        }
    }
}
