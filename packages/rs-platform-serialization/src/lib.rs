pub mod de;
pub mod enc;
mod features;

use bincode::config::Config;
use bincode::de::read::Reader;
use bincode::de::{read, DecoderImpl};
use bincode::enc::write::Writer;
use bincode::enc::{write, EncoderImpl};
pub use enc::PlatformVersionEncode;
pub use features::platform_encode_to_vec;

/// Alias for the decoding context used across this crate.
pub type BincodeContext = ();

pub use de::DefaultBorrowDecode;
pub use de::DefaultDecode;
pub use de::PlatformVersionedBorrowDecode;
pub use de::PlatformVersionedDecode;

pub use bincode::enc::Encode;
pub use bincode::error;
pub use de::BorrowDecode;
pub use de::Decode;
use platform_version::version::PlatformVersion;

extern crate alloc;
extern crate std;

/// Encode the given value into the given slice. Returns the amount of bytes that have been written.
///
/// See the [config] module for more information on configurations.
///
/// [config]: config/index.html
pub fn platform_encode_into_slice<E: PlatformVersionEncode, C: Config>(
    val: E,
    dst: &mut [u8],
    config: C,
    platform_version: &PlatformVersion,
) -> Result<usize, error::EncodeError> {
    let writer = write::SliceWriter::new(dst);
    let mut encoder = EncoderImpl::<_, C>::new(writer, config);
    val.platform_encode(&mut encoder, platform_version)?;
    Ok(encoder.into_writer().bytes_written())
}

/// Encode the given value into a custom [Writer].
///
/// See the [config] module for more information on configurations.
///
/// [config]: config/index.html
pub fn encode_into_writer<E: PlatformVersionEncode, W: Writer, C: Config>(
    val: E,
    writer: W,
    config: C,
    platform_version: &PlatformVersion,
) -> Result<(), error::EncodeError> {
    let mut encoder = EncoderImpl::<_, C>::new(writer, config);
    val.platform_encode(&mut encoder, platform_version)
}

/// Attempt to decode a given type `D` from the given slice. Returns the decoded output and the amount of bytes read.
///
/// See the [config] module for more information on configurations.
///
/// [config]: config/index.html
pub fn platform_versioned_decode_from_slice<D: PlatformVersionedDecode, C: Config>(
    src: &[u8],
    config: C,
    platform_version: &PlatformVersion,
) -> Result<D, error::DecodeError> {
    let reader = read::SliceReader::new(src);
    let mut decoder = DecoderImpl::<_, C, crate::BincodeContext>::new(reader, config, ());
    D::platform_versioned_decode(&mut decoder, platform_version)
}

/// Attempt to decode a given type `D` from the given slice. Returns the decoded output and the amount of bytes read.
///
/// See the [config] module for more information on configurations.
///
/// [config]: config/index.html
pub fn platform_versioned_borrow_decode_from_slice<
    'a,
    D: PlatformVersionedBorrowDecode<'a>,
    C: Config,
>(
    src: &'a [u8],
    config: C,
    platform_version: &PlatformVersion,
) -> Result<D, error::DecodeError> {
    let reader = read::SliceReader::new(src);
    let mut decoder = DecoderImpl::<_, C, crate::BincodeContext>::new(reader, config, ());
    D::platform_versioned_borrow_decode(&mut decoder, platform_version)
}

/// Attempt to decode a given type `D` from the given [Reader].
///
/// See the [config] module for more information on configurations.
///
/// [config]: config/index.html
pub fn platform_versioned_decode_from_reader<D: PlatformVersionedDecode, R: Reader, C: Config>(
    reader: R,
    config: C,
    platform_version: &PlatformVersion,
) -> Result<D, error::DecodeError> {
    let mut decoder = DecoderImpl::<_, C, crate::BincodeContext>::new(reader, config, ());
    D::platform_versioned_decode(&mut decoder, platform_version)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bincode::config;
    use platform_version::version::PlatformVersion;

    fn cfg() -> impl Config {
        config::standard().with_big_endian().with_no_limit()
    }

    fn pv() -> &'static PlatformVersion {
        PlatformVersion::first()
    }

    // -----------------------------------------------------------------------
    // Top-level encode/decode round-trip functions
    // -----------------------------------------------------------------------

    #[test]
    fn encode_into_slice_and_decode_round_trip() {
        let value: u32 = 42;
        let mut buf = [0u8; 64];
        let written = platform_encode_into_slice(value, &mut buf, cfg(), pv()).unwrap();
        assert!(written > 0);

        let decoded: u32 =
            platform_versioned_decode_from_slice(&buf[..written], cfg(), pv()).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn encode_into_slice_too_small_returns_error() {
        let value: u64 = u64::MAX;
        let mut buf = [0u8; 1]; // too small for a u64
        let result = platform_encode_into_slice(value, &mut buf, cfg(), pv());
        assert!(result.is_err());
    }

    #[test]
    fn encode_into_writer_round_trip() {
        let value: u16 = 1234;
        let mut writer = enc::VecWriter::default();
        encode_into_writer(value, &mut writer, cfg(), pv()).unwrap();

        let encoded = platform_encode_to_vec(value, cfg(), pv()).unwrap();
        let decoded: u16 = platform_versioned_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn borrow_decode_from_slice_str() {
        let value = "hello world";
        let encoded = platform_encode_to_vec(value, cfg(), pv()).unwrap();

        let decoded: &str =
            platform_versioned_borrow_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn decode_from_reader() {
        let value: i64 = -99999;
        let encoded = platform_encode_to_vec(value, cfg(), pv()).unwrap();

        let reader = bincode::de::read::SliceReader::new(&encoded);
        let decoded: i64 = platform_versioned_decode_from_reader(reader, cfg(), pv()).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn decode_from_slice_empty_input_fails() {
        let result = platform_versioned_decode_from_slice::<u32, _>(&[], cfg(), pv());
        assert!(result.is_err());
    }

    #[test]
    fn decode_from_slice_truncated_input_fails() {
        let encoded = platform_encode_to_vec(42u32, cfg(), pv()).unwrap();
        // only give half the bytes
        let result = platform_versioned_decode_from_slice::<u32, _>(
            &encoded[..encoded.len() / 2],
            cfg(),
            pv(),
        );
        assert!(result.is_err());
    }
}
