use crate::Component;

pub(super) fn provider<Profile: crate::GrampsProfile>()
-> Result<geam::HostProviderModule<Profile>, geam::HostRegistrationError> {
    bindings::__geam_module::<Profile>()
}

#[geam::module(path = "gramps/websocket", crate_path = geam, profile = crate::GrampsProfile,
    component = crate::Component)]
pub mod bindings {
    use base64::{Engine, engine::general_purpose::STANDARD};
    use geam::provider::{BitArrayValue, HostFailure, HostResult, StringValue};
    use sha1::{Digest, Sha1};

    #[allow(dead_code)]
    #[geam::custom(input = ShaHashInput)]
    enum ShaHash {
        Sha,
    }

    #[geam::function]
    fn crypto_exor(a: BitArrayValue, b: BitArrayValue) -> HostResult<BitArrayValue> {
        if !a.bit_len().is_multiple_of(8) || !b.bit_len().is_multiple_of(8) {
            return Err(HostFailure::new("WebSocket XOR input must contain complete bytes").into());
        }
        if a.bit_len() != b.bit_len() {
            return Err(HostFailure::new("WebSocket XOR inputs must have equal lengths").into());
        }
        Ok(BitArrayValue::from_bytes(
            a.bytes()
                .iter()
                .zip(b.bytes())
                .map(|(a, b)| a ^ b)
                .collect(),
        ))
    }

    #[geam::function]
    fn crypto_hash(_: ShaHashInput, data: StringValue) -> StringValue {
        StringValue::from_bytes(Sha1::digest(data.as_bytes()).to_vec())
    }

    #[geam::function]
    fn base64_encode(data: StringValue) -> StringValue {
        STANDARD.encode(data.as_bytes()).into()
    }

    #[cfg(test)]
    mod tests {
        use super::{ShaHashInput, base64_encode, crypto_exor, crypto_hash};
        use geam::{BitArrayValue, StringValue};

        #[test]
        fn sha1_preserves_all_twenty_raw_bytes_before_padded_base64() {
            let digest = crypto_hash(ShaHashInput::Sha, "abc".into());
            assert_eq!(
                digest.as_bytes(),
                &[
                    0xa9, 0x99, 0x3e, 0x36, 0x47, 0x06, 0x81, 0x6a, 0xba, 0x3e, 0x25, 0x71, 0x78,
                    0x50, 0xc2, 0x6c, 0x9c, 0xd0, 0xd8, 0x9d
                ]
            );
            assert_eq!(
                base64_encode(digest).as_bytes(),
                b"qZk+NkcGgWq6PiVxeFDCbJzQ2J0="
            );
            assert_eq!(
                base64_encode(StringValue::from_bytes(vec![255, 0, 128])).as_bytes(),
                b"/wCA"
            );
            assert_eq!(base64_encode("".into()).as_bytes(), b"");
        }

        #[test]
        fn xor_keeps_input_aliases_and_rejects_length_and_both_alignment_failures() {
            let a = BitArrayValue::from_bytes(vec![0xff, 0, 128]);
            let b = BitArrayValue::from_bytes(vec![0, 255, 128]);
            assert_eq!(
                crypto_exor(a.clone(), b.clone()).unwrap().bytes(),
                &[255, 255, 0]
            );
            assert_eq!(a.bytes(), &[255, 0, 128]);
            assert_eq!(b.bytes(), &[0, 255, 128]);
            assert_eq!(
                crypto_exor(a.clone(), BitArrayValue::from_bytes(vec![]))
                    .unwrap_err()
                    .to_string(),
                "WebSocket XOR inputs must have equal lengths"
            );
            let partial = BitArrayValue::try_from_parts(vec![128], 1).unwrap();
            for (a, b) in [(partial.clone(), a.clone()), (a, partial)] {
                assert_eq!(
                    crypto_exor(a, b).unwrap_err().to_string(),
                    "WebSocket XOR input must contain complete bytes"
                );
            }
            assert_eq!(
                crypto_exor(
                    BitArrayValue::from_bytes(vec![]),
                    BitArrayValue::from_bytes(vec![])
                )
                .unwrap()
                .bytes(),
                &[] as &[u8]
            );
        }

        #[test]
        fn original_xor_is_also_callable_as_a_computed_source_function() {
            let additional = r#"
pub fn probe_computed_xor() {
  list.fold([<<255,3,128>>,<<1,2,4>>],<<0,1,128>>,crypto_exor)
}
"#;
            let (mut execution, mut state) = crate::test_support::source_project(
                "import gramps/websocket\npub fn main() { websocket.probe_computed_xor() }",
                &[("gramps/websocket", additional)],
                [],
            );
            assert_eq!(
                crate::test_support::execution_host::run(
                    &mut execution,
                    &mut state,
                    &mut Vec::new()
                )
                .unwrap(),
                geam::Value::BitArray(BitArrayValue::from_bytes(vec![254, 0, 4]))
            );
        }

        #[test]
        fn original_xor_failure_crosses_the_typed_native_adapter_without_changing_its_reason() {
            let additional = "pub fn probe_xor(a: BitArray,b: BitArray) { crypto_exor(a,b) }";
            for (arguments, reason) in [
                ("<<0>>,<<>>", "equal lengths"),
                ("<<1:1>>,<<0>>", "complete bytes"),
                ("<<0>>,<<1:1>>", "complete bytes"),
            ] {
                let source = format!(
                    "import gramps/websocket\npub fn main() {{ websocket.probe_xor({arguments}) }}"
                );
                let (mut execution, mut state) = crate::test_support::source_project(
                    &source,
                    &[("gramps/websocket", additional)],
                    [],
                );
                let error = crate::test_support::execution_host::run(
                    &mut execution,
                    &mut state,
                    &mut Vec::new(),
                )
                .unwrap_err();
                assert!(error.to_string().contains(reason), "{error}");
            }
        }

        #[test]
        fn original_client_key_uses_the_composed_crypto_entropy_and_preserves_its_failure() {
            struct Sequence;
            impl geam_crypto::Entropy for Sequence {
                fn fill(&mut self, bytes: &mut [u8]) -> Result<(), geam::HostFailure> {
                    assert_eq!(bytes.len(), 16);
                    for (index, byte) in bytes.iter_mut().enumerate() {
                        *byte = index as u8;
                    }
                    Ok(())
                }
            }
            struct Failed;
            impl geam_crypto::Entropy for Failed {
                fn fill(&mut self, _: &mut [u8]) -> Result<(), geam::HostFailure> {
                    Err(geam::HostFailure::new(
                        "injected client-key entropy failure",
                    ))
                }
            }
            use crate::test_support::{execution_host::TestHost, source_project};
            let source = "import gramps/websocket\npub fn main() { websocket.make_client_key() }";
            let (mut execution, mut state) = source_project(source, &[], []);
            let host = TestHost::default();
            state.crypto = geam_crypto::RunState::with_entropy(Sequence);
            assert_eq!(
                host.block_on(execution.run_main(&host, &mut state, &mut Vec::new()))
                    .unwrap()
                    .try_into_value()
                    .unwrap(),
                geam::Value::String("AAECAwQFBgcICQoLDA0ODw==".into())
            );
            state.crypto = geam_crypto::RunState::with_entropy(Failed);
            assert!(
                host.block_on(execution.run_main(&host, &mut state, &mut Vec::new()))
                    .unwrap_err()
                    .to_string()
                    .contains("injected client-key entropy failure")
            );
        }
    }
}
