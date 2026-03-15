use bincode::enc::Encoder;
use bincode::error::EncodeError;
use bincode::{enc, Encode};
use platform_version::version::PlatformVersion;

mod impls;

#[derive(Default)]
pub(crate) struct VecWriter {
    inner: Vec<u8>,
}

impl VecWriter {
    /// Create a new vec writer with the given capacity
    #[allow(dead_code)]
    #[deprecated(note = "This function is marked as unused.")]
    #[allow(deprecated)]
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            inner: Vec::with_capacity(cap),
        }
    }
    // May not be used in all feature combinations
    #[allow(dead_code)]
    #[deprecated(note = "This function is marked as unused.")]
    #[allow(deprecated)]
    pub(crate) fn collect(self) -> Vec<u8> {
        self.inner
    }
}

impl enc::write::Writer for VecWriter {
    #[inline(always)]
    fn write(&mut self, bytes: &[u8]) -> Result<(), EncodeError> {
        self.inner.extend_from_slice(bytes);
        Ok(())
    }
}

pub trait PlatformVersionEncode {
    /// Encode a given type.
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        platform_version: &PlatformVersion,
    ) -> Result<(), EncodeError>;
}

/// Encode the variant of the given option. Will not encode the option itself.
#[inline]
pub(crate) fn encode_option_variant<E: Encoder, T>(
    encoder: &mut E,
    value: &Option<T>,
) -> Result<(), EncodeError> {
    match value {
        None => 0u8.encode(encoder),
        Some(_) => 1u8.encode(encoder),
    }
}

/// Encodes the length of any slice, container, etc into the given encoder
#[inline]
pub(crate) fn encode_slice_len<E: Encoder>(encoder: &mut E, len: usize) -> Result<(), EncodeError> {
    (len as u64).encode(encoder)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bincode::config;

    fn cfg() -> impl bincode::config::Config {
        config::standard().with_big_endian().with_no_limit()
    }

    #[test]
    fn encode_option_variant_none() {
        let value: Option<u32> = None;
        let mut writer = VecWriter::default();
        let mut encoder = bincode::enc::EncoderImpl::new(&mut writer, cfg());
        encode_option_variant(&mut encoder, &value).unwrap();
        drop(encoder);
        assert_eq!(writer.inner, &[0u8]);
    }

    #[test]
    fn encode_option_variant_some() {
        let value: Option<u32> = Some(42);
        let mut writer = VecWriter::default();
        let mut encoder = bincode::enc::EncoderImpl::new(&mut writer, cfg());
        encode_option_variant(&mut encoder, &value).unwrap();
        drop(encoder);
        assert_eq!(writer.inner, &[1u8]);
    }

    #[test]
    fn encode_slice_len_encodes_as_u64() {
        let mut writer = VecWriter::default();
        let mut encoder = bincode::enc::EncoderImpl::new(&mut writer, cfg());
        encode_slice_len(&mut encoder, 5).unwrap();
        drop(encoder);
        // 5 as u64 in big-endian varint
        let expected = bincode::encode_to_vec(5u64, cfg()).unwrap();
        assert_eq!(writer.inner, expected);
    }

    #[test]
    fn vec_writer_write_impl() {
        let mut writer = VecWriter::default();
        bincode::enc::write::Writer::write(&mut writer, b"hello").unwrap();
        bincode::enc::write::Writer::write(&mut writer, b" world").unwrap();
        assert_eq!(writer.inner, b"hello world");
    }
}
