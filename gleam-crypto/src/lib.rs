//! Native functions for the unmodified `gleam_crypto` 1.6.0 package.

use geam::provider::{BigInt, BitArrayValue, HostFailure};

/// The host-owned source of cryptographically secure random bytes.
pub trait Entropy: Send {
    fn fill(&mut self, bytes: &mut [u8]) -> Result<(), HostFailure>;
}

/// Operating-system entropy used by standalone Geam runs.
pub struct SystemEntropy;

impl Entropy for SystemEntropy {
    fn fill(&mut self, bytes: &mut [u8]) -> Result<(), HostFailure> {
        getrandom::fill(bytes).map_err(os_entropy_failure)
    }
}

fn os_entropy_failure(error: impl std::fmt::Display) -> HostFailure {
    HostFailure::new(format!("OS entropy unavailable: {error}"))
}

/// Execution-local provider state. An embedding host may supply its own entropy.
pub struct RunState {
    entropy: Box<dyn Entropy>,
}

impl RunState {
    pub fn with_entropy(entropy: impl Entropy + 'static) -> Self {
        Self {
            entropy: Box::new(entropy),
        }
    }

    fn random_bytes(&mut self, length: BigInt) -> Result<BitArrayValue, HostFailure> {
        let length = usize::try_from(length).map_err(|_| {
            HostFailure::new("random byte count must be nonnegative and fit in host memory")
        })?;
        if length == 0 {
            return Ok(BitArrayValue::from_bytes(Vec::new()));
        }
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(length)
            .map_err(|error| HostFailure::new(format!("random byte allocation failed: {error}")))?;
        bytes.resize(length, 0);
        self.entropy.fill(&mut bytes)?;
        Ok(BitArrayValue::from_bytes(bytes))
    }
}

impl Default for RunState {
    fn default() -> Self {
        Self::with_entropy(SystemEntropy)
    }
}

#[geam::provider(package = "gleam_crypto", state = RunState, modules = [crypto])]
pub struct Component;

#[geam::module(path = "gleam/crypto")]
mod crypto {
    use super::RunState;
    use geam::provider::{
        BigInt, BitArrayValue, Call, EcoString, ExternalPayload, HostFailure, HostResult,
    };
    use md5::Md5;
    use sha1::Sha1;
    use sha2::{Digest, Sha224, Sha256, Sha384, Sha512};
    use std::borrow::Cow;
    use std::sync::atomic::{AtomicU64, Ordering};

    // A source context is identity-bearing even when its digest state matches
    // another context. The sequence also distinguishes contexts across runs.
    static NEXT_HASHER_ID: AtomicU64 = AtomicU64::new(1);

    #[allow(dead_code)]
    #[geam::custom(input = HashAlgorithmInput)]
    enum HashAlgorithm {
        Sha224,
        Sha256,
        Sha384,
        Sha512,
        Md5,
        Sha1,
    }

    #[derive(Clone)]
    enum HashState {
        Sha224(Sha224),
        Sha256(Sha256),
        Sha384(Sha384),
        Sha512(Sha512),
        Md5(Md5),
        Sha1(Sha1),
    }

    impl HashState {
        fn new(algorithm: &HashAlgorithmInput) -> Self {
            match algorithm {
                HashAlgorithmInput::Sha224 => Self::Sha224(Sha224::new()),
                HashAlgorithmInput::Sha256 => Self::Sha256(Sha256::new()),
                HashAlgorithmInput::Sha384 => Self::Sha384(Sha384::new()),
                HashAlgorithmInput::Sha512 => Self::Sha512(Sha512::new()),
                HashAlgorithmInput::Md5 => Self::Md5(Md5::new()),
                HashAlgorithmInput::Sha1 => Self::Sha1(Sha1::new()),
            }
        }

        fn update(&mut self, bytes: &[u8]) {
            match self {
                Self::Sha224(state) => state.update(bytes),
                Self::Sha256(state) => state.update(bytes),
                Self::Sha384(state) => state.update(bytes),
                Self::Sha512(state) => state.update(bytes),
                Self::Md5(state) => state.update(bytes),
                Self::Sha1(state) => state.update(bytes),
            }
        }

        fn digest(&self) -> Vec<u8> {
            match self.clone() {
                Self::Sha224(state) => state.finalize().to_vec(),
                Self::Sha256(state) => state.finalize().to_vec(),
                Self::Sha384(state) => state.finalize().to_vec(),
                Self::Sha512(state) => state.finalize().to_vec(),
                Self::Md5(state) => state.finalize().to_vec(),
                Self::Sha1(state) => state.finalize().to_vec(),
            }
        }
    }

    #[geam::external(name = "Hasher", manual)]
    struct Hasher {
        id: u64,
        state: HashState,
    }

    impl Hasher {
        fn new(state: HashState) -> Self {
            Self {
                id: NEXT_HASHER_ID.fetch_add(1, Ordering::Relaxed),
                state,
            }
        }

        fn updated(&self, bytes: &[u8]) -> Self {
            let mut state = self.state.clone();
            state.update(bytes);
            Self::new(state)
        }

        fn digest(&self) -> BitArrayValue {
            BitArrayValue::from_bytes(self.state.digest())
        }
    }

    impl ExternalPayload for Hasher {
        fn source_equal(&self, other: &Self) -> bool {
            self.id == other.id
        }

        fn source_hash(&self) -> u64 {
            self.id
        }

        fn inspect(&self) -> EcoString {
            "#<gleam_crypto.Hasher>".into()
        }
    }

    #[geam::function]
    fn new_hasher(algorithm: HashAlgorithmInput) -> Hasher {
        Hasher::new(HashState::new(&algorithm))
    }

    #[geam::function]
    fn hash_chunk(hasher: &Hasher, chunk: BitArrayValue) -> HostResult<Hasher> {
        let bytes = aligned_bytes(&chunk)?;
        Ok(hasher.updated(bytes))
    }

    #[geam::function]
    fn digest(hasher: &Hasher) -> BitArrayValue {
        hasher.digest()
    }

    #[geam::function]
    fn hmac(
        data: BitArrayValue,
        algorithm: HashAlgorithmInput,
        key: BitArrayValue,
    ) -> HostResult<BitArrayValue> {
        let message = aligned_bytes(&data)?;
        let key = aligned_bytes(&key)?;
        let block_size = match &algorithm {
            HashAlgorithmInput::Sha224
            | HashAlgorithmInput::Sha256
            | HashAlgorithmInput::Md5
            | HashAlgorithmInput::Sha1 => 64,
            HashAlgorithmInput::Sha384 | HashAlgorithmInput::Sha512 => 128,
        };
        let effective_key = if key.len() > block_size {
            let mut state = HashState::new(&algorithm);
            state.update(key);
            Cow::Owned(state.digest())
        } else {
            Cow::Borrowed(key)
        };
        let mut pad = vec![0x36; block_size];
        for (slot, byte) in pad.iter_mut().zip(effective_key.iter()) {
            *slot ^= byte;
        }
        let mut inner = HashState::new(&algorithm);
        inner.update(&pad);
        inner.update(message);
        let inner_digest = inner.digest();

        pad.fill(0x5c);
        for (slot, byte) in pad.iter_mut().zip(effective_key.iter()) {
            *slot ^= byte;
        }
        let mut outer = HashState::new(&algorithm);
        outer.update(&pad);
        outer.update(&inner_digest);
        Ok(BitArrayValue::from_bytes(outer.digest()))
    }

    #[geam::function]
    fn strong_random_bytes(
        #[geam::call] call: &mut Call<RunState>,
        length: BigInt,
    ) -> HostResult<BitArrayValue> {
        call.state_mut().random_bytes(length).map_err(Into::into)
    }

    fn aligned_bytes(value: &BitArrayValue) -> Result<&[u8], HostFailure> {
        if !value.bit_len().is_multiple_of(8) {
            return Err(HostFailure::new("crypto input must contain complete bytes"));
        }
        Ok(value.bytes())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn hex(bytes: &[u8]) -> String {
            bytes.iter().map(|byte| format!("{byte:02x}")).collect()
        }

        #[test]
        fn all_digest_and_hmac_algorithms_match_independent_vectors() {
            let vectors = [
                (
                    HashAlgorithmInput::Sha224,
                    "23097d223405d8228642a477bda255b32aadbce4bda0b3f7e36c9da7",
                    "88ff8b54675d39b8f72322e65ff945c52d96379988ada25639747e69",
                ),
                (
                    HashAlgorithmInput::Sha256,
                    "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
                    "f7bc83f430538424b13298e6aa6fb143ef4d59a14946175997479dbc2d1a3cd8",
                ),
                (
                    HashAlgorithmInput::Sha384,
                    "cb00753f45a35e8bb5a03d699ac65007272c32ab0eded1631a8b605a43ff5bed8086072ba1e7cc2358baeca134c825a7",
                    "d7f4727e2c0b39ae0f1e40cc96f60242d5b7801841cea6fc592c5d3e1ae50700582a96cf35e1e554995fe4e03381c237",
                ),
                (
                    HashAlgorithmInput::Sha512,
                    "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f",
                    "b42af09057bac1e2d41708e48a902e09b5ff7f12ab428a4fe86653c73dd248fb82f948a549f7b791a5b41915ee4d1ec3935357e4e2317250d0372afa2ebeeb3a",
                ),
                (
                    HashAlgorithmInput::Sha1,
                    "a9993e364706816aba3e25717850c26c9cd0d89d",
                    "de7c9b85b8b78aa6bc8a7a36f70a90701c9db4d9",
                ),
                (
                    HashAlgorithmInput::Md5,
                    "900150983cd24fb0d6963f7d28e17f72",
                    "80070713463e7749b90c2dc24911e275",
                ),
            ];
            for (algorithm, expected_digest, expected_hmac) in vectors {
                let initial = HashState::new(&algorithm);
                let mut updated = initial.clone();
                updated.update(b"abc");
                assert_eq!(hex(&updated.digest()), expected_digest);
                assert_eq!(
                    hex(&initial.digest()),
                    hex(&HashState::new(&algorithm).digest())
                );
                let mac = hmac(
                    BitArrayValue::from_bytes(
                        b"The quick brown fox jumps over the lazy dog".to_vec(),
                    ),
                    algorithm,
                    BitArrayValue::from_bytes(b"key".to_vec()),
                )
                .expect("byte-aligned HMAC input");
                assert_eq!(hex(mac.bytes()), expected_hmac);
            }
        }

        #[test]
        fn streaming_states_branch_and_digest_without_consuming_their_parent() {
            let base = new_hasher(HashAlgorithmInput::Sha256);
            let left = base.updated(b"a");
            let right = base.updated(b"b");
            let complete = left.updated(b"bc");
            let expected = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
            assert_eq!(hex(complete.digest().bytes()), expected);
            assert_eq!(hex(complete.digest().bytes()), expected);
            assert_ne!(left.digest().bytes(), right.digest().bytes());
            assert!(!base.source_equal(&left));
            assert!(!left.source_equal(&right));
            assert!(base.source_equal(&base));
            assert_eq!(base.source_hash(), base.source_hash());
            assert_ne!(base.source_hash(), left.source_hash());
            assert_eq!(base.inspect().as_str(), "#<gleam_crypto.Hasher>");
        }

        #[test]
        fn hmac_hashes_long_keys_and_rejects_partial_bytes() {
            let long = BitArrayValue::from_bytes(vec![b'x'; 200]);
            let mac = hmac(
                BitArrayValue::from_bytes(b"abc".to_vec()),
                HashAlgorithmInput::Sha256,
                long.clone(),
            )
            .expect("long key HMAC");
            assert_eq!(
                hex(mac.bytes()),
                "343c0393c4ae8ef0398a29a5155358e5cad0b9fcb70afdb8cd9d0e83abdd9378"
            );
            let mac = hmac(
                BitArrayValue::from_bytes(b"abc".to_vec()),
                HashAlgorithmInput::Sha512,
                long,
            )
            .expect("long key SHA-512 HMAC");
            assert_eq!(
                hex(mac.bytes()),
                "7b7b9d1ebab49cfdc2a5b55d6252f60130cbd3b781544dfb192c4454791fc0dba924a2c646a4a3b5cd41737a60d8209c84d72414bcccc4cb401db0751811f89a"
            );

            let partial =
                BitArrayValue::try_from_parts(vec![0b1010_0000], 4).expect("four-bit fixture");
            assert_eq!(
                aligned_bytes(&partial)
                    .expect_err("partial chunk fails")
                    .to_string(),
                "crypto input must contain complete bytes"
            );
            assert_eq!(
                hmac(
                    partial.clone(),
                    HashAlgorithmInput::Sha256,
                    BitArrayValue::from_bytes(vec![])
                )
                .expect_err("partial data fails")
                .to_string(),
                "crypto input must contain complete bytes"
            );
            assert_eq!(
                hmac(
                    BitArrayValue::from_bytes(vec![]),
                    HashAlgorithmInput::Sha256,
                    partial
                )
                .expect_err("partial key fails")
                .to_string(),
                "crypto input must contain complete bytes"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct SequenceEntropy;

    impl Entropy for SequenceEntropy {
        fn fill(&mut self, bytes: &mut [u8]) -> Result<(), HostFailure> {
            for (index, byte) in bytes.iter_mut().enumerate() {
                *byte = index as u8;
            }
            Ok(())
        }
    }

    struct FailedEntropy;

    impl Entropy for FailedEntropy {
        fn fill(&mut self, _: &mut [u8]) -> Result<(), HostFailure> {
            Err(HostFailure::new("entropy fixture failed"))
        }
    }

    #[test]
    fn host_entropy_is_injected_and_failures_remain_visible() {
        let mut state = RunState::with_entropy(SequenceEntropy);
        assert_eq!(
            state
                .random_bytes(BigInt::from(4))
                .expect("four bytes")
                .bytes(),
            &[0, 1, 2, 3]
        );
        assert!(
            state
                .random_bytes(BigInt::from(0))
                .expect("zero bytes")
                .bytes()
                .is_empty()
        );
        assert_eq!(
            state
                .random_bytes(BigInt::from(-1))
                .expect_err("negative count fails")
                .to_string(),
            "random byte count must be nonnegative and fit in host memory"
        );
        assert!(
            state
                .random_bytes(BigInt::from(usize::MAX))
                .expect_err("oversized allocation fails")
                .to_string()
                .starts_with("random byte allocation failed:")
        );
        let mut failed = RunState::with_entropy(FailedEntropy);
        assert!(
            failed
                .random_bytes(BigInt::from(0))
                .expect("empty request needs no entropy")
                .bytes()
                .is_empty()
        );
        assert_eq!(
            failed
                .random_bytes(BigInt::from(4))
                .expect_err("entropy failure propagates")
                .to_string(),
            "entropy fixture failed"
        );
    }

    #[test]
    fn default_state_uses_os_entropy() {
        let mut state = RunState::default();
        assert_eq!(
            state
                .random_bytes(BigInt::from(16))
                .expect("OS entropy available")
                .bytes()
                .len(),
            16
        );
        assert_eq!(
            os_entropy_failure("test OS error").to_string(),
            "OS entropy unavailable: test OS error"
        );
    }
}
