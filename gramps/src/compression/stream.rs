//! Stateful raw DEFLATE. The original Gleam code owns message suffixes and resets.
use geam::provider::{HostFailure, HostResult};
use miniz_oxide::deflate::{
    core::{CompressorOxide, create_comp_flags_from_zip_params},
    stream::deflate,
};
use miniz_oxide::inflate::stream::{InflateState, inflate};
use miniz_oxide::{DataFormat, MZError, MZFlush};

pub(super) struct Stream {
    backend: Backend,
    failed: bool,
}

enum Backend {
    Inflate(Box<InflateState>),
    Deflate(Box<CompressorOxide>),
}

impl Stream {
    pub(super) fn inflater() -> Self {
        Self {
            backend: Backend::Inflate(InflateState::new_boxed(DataFormat::Raw)),
            failed: false,
        }
    }

    pub(super) fn deflater() -> Self {
        Self {
            backend: Backend::Deflate(Box::new(CompressorOxide::new(
                create_comp_flags_from_zip_params(6, -15, 0),
            ))),
            failed: false,
        }
    }

    pub(super) fn process(&mut self, input: &[u8], inflate_mode: bool) -> HostResult<Vec<u8>> {
        if self.failed {
            return Err(HostFailure::new(
                "compression stream needs reset after a failed operation",
            )
            .into());
        }
        if !matches!(
            (&self.backend, inflate_mode),
            (Backend::Inflate(_), true) | (Backend::Deflate(_), false)
        ) {
            return Err(HostFailure::new(
                "compression operation does not match the initialized stream",
            )
            .into());
        }
        let result = self.process_chunks(input, &mut [0; 8192]);
        if result.is_err() {
            self.failed = true;
        }
        result
    }

    fn process_chunks(&mut self, input: &[u8], buffer: &mut [u8]) -> HostResult<Vec<u8>> {
        let mut consumed = 0;
        let mut output = Vec::new();
        while !self.advance(input, &mut consumed, buffer, &mut output)? {}
        Ok(output)
    }

    /// One backend step either completes, consumes/emits bytes, or fails.
    /// Reservation and progress are one transition; the drain loop propagates
    /// that transition's failure without choosing another failure owner.
    fn advance(
        &mut self,
        input: &[u8],
        consumed: &mut usize,
        buffer: &mut [u8],
        output: &mut Vec<u8>,
    ) -> HostResult<bool> {
        let (result, inflate_mode) = match &mut self.backend {
            Backend::Inflate(state) => (
                inflate(state, &input[*consumed..], buffer, MZFlush::None),
                true,
            ),
            Backend::Deflate(state) => (
                deflate(state, &input[*consumed..], buffer, MZFlush::Sync),
                false,
            ),
        };
        *consumed += result.bytes_consumed;
        // Buf after consuming all input means that an inflater has nothing more to emit.
        if let Err(error) = result.status
            && !(inflate_mode && error == MZError::Buf && *consumed == input.len())
        {
            return Err(HostFailure::new(format!("raw DEFLATE backend failed: {error:?}")).into());
        }
        reserve(output, result.bytes_written).and_then(|()| {
            output.extend_from_slice(&buffer[..result.bytes_written]);
            if *consumed == input.len() && result.bytes_written < buffer.len() {
                return Ok(true);
            }
            if result.bytes_consumed == 0 && result.bytes_written == 0 {
                return Err(HostFailure::new("raw DEFLATE backend made no progress").into());
            }
            Ok(false)
        })
    }

    pub(super) fn reset(&mut self, inflate_mode: bool) -> HostResult<()> {
        match &mut self.backend {
            Backend::Inflate(state) if inflate_mode => state.reset(DataFormat::Raw),
            Backend::Deflate(state) if !inflate_mode => state.reset(),
            _ => {
                return Err(HostFailure::new(
                    "compression reset does not match the initialized stream",
                )
                .into());
            }
        }
        self.failed = false;
        Ok(())
    }
}

fn reserve(output: &mut Vec<u8>, additional: usize) -> HostResult<()> {
    output.try_reserve(additional).map_err(|error| {
        HostFailure::new(format!("compression output allocation failed: {error}")).into()
    })
}

#[cfg(test)]
mod tests {
    use super::{Stream, reserve};

    #[test]
    fn small_buffers_drain_partial_input_and_output_with_takeover_and_resets() {
        for reset in [false, true] {
            let mut compressor = Stream::deflater();
            let mut inflater = Stream::inflater();
            for message in [
                vec![],
                b"hello websocket".repeat(12000),
                b"hello websocket".repeat(12000),
                (0..100_000).map(|n| (n % 256) as u8).collect(),
            ] {
                let bytes = compressor.process_chunks(&message, &mut [0; 7]).unwrap();
                assert!(bytes.ends_with(&[0, 0, 255, 255]));
                assert_eq!(
                    inflater.process_chunks(&bytes, &mut [0; 127]).unwrap(),
                    message
                );
                if reset {
                    compressor.reset(false).unwrap();
                    inflater.reset(true).unwrap();
                }
            }
        }
    }

    #[test]
    fn independent_fixed_and_stored_blocks_and_backend_failure_recovery() {
        // RFC 1951 stored block "abc" and fixed Huffman block "Hello".
        for (bytes, expected) in [
            (&[1, 3, 0, 252, 255, 97, 98, 99][..], &b"abc"[..]),
            (&[243, 72, 205, 201, 201, 7, 0][..], &b"Hello"[..]),
        ] {
            assert_eq!(Stream::inflater().process(bytes, true).unwrap(), expected);
        }
        let mut state = Stream::inflater();
        assert!(
            state
                .process(&[7, 0, 0, 0, 255, 255], true)
                .unwrap_err()
                .to_string()
                .contains("backend failed")
        );
        assert_eq!(
            state.process(&[], true).unwrap_err().to_string(),
            "compression stream needs reset after a failed operation"
        );
        state.reset(true).unwrap();
        assert_eq!(state.process(&[], true).unwrap(), Vec::<u8>::new());
        assert!(
            state
                .process(&[], false)
                .unwrap_err()
                .to_string()
                .contains("does not match")
        );
        assert!(
            state
                .reset(false)
                .unwrap_err()
                .to_string()
                .contains("does not match")
        );
        let mut deflater = Stream::deflater();
        assert!(deflater.process(&[], true).is_err());
        assert!(deflater.reset(true).is_err());
        deflater.reset(false).unwrap();
        assert_eq!(deflater.process(&[], false).unwrap(), &[0, 0, 0, 255, 255]);
        assert!(
            reserve(&mut Vec::new(), usize::MAX)
                .unwrap_err()
                .to_string()
                .contains("allocation failed")
        );
    }

    #[test]
    fn a_finished_stream_rejects_trailing_input_without_looping_and_reset_recovers() {
        let mut inflater = Stream::inflater();
        let error = inflater
            .process(&[1, 3, 0, 252, 255, 97, 98, 99, 42], true)
            .unwrap_err();
        assert_eq!(error.to_string(), "raw DEFLATE backend made no progress");
        assert_eq!(
            inflater.process(&[], true).unwrap_err().to_string(),
            "compression stream needs reset after a failed operation"
        );
        inflater.reset(true).unwrap();
        assert_eq!(
            inflater
                .process(&[1, 3, 0, 252, 255, 97, 98, 99], true)
                .unwrap(),
            b"abc"
        );
    }
}
