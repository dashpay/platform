pub mod de;
pub mod enc;
mod features;

use bincode::config::Config;
use bincode::de::read::Reader;
use bincode::de::{read, Decoder, DecoderImpl};
use bincode::enc::write::Writer;
use bincode::enc::{write, EncoderImpl};
pub use enc::PlatformVersionEncode;
pub use features::platform_encode_to_vec;

pub use de::PlatformVersionedBorrowDecode;
pub use de::PlatformVersionedDecode;

pub use bincode::de::BorrowDecode;
pub use bincode::de::Decode;
pub use bincode::enc::Encode;
use bincode::error;
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
pub fn decode_from_slice<D: PlatformVersionedDecode, C: Config>(
    src: &[u8],
    config: C,
    platform_version: &PlatformVersion,
) -> Result<D, error::DecodeError> {
    let reader = read::SliceReader::new(src);
    let mut decoder = DecoderImpl::<_, C>::new(reader, config);
    D::platform_versioned_decode(&mut decoder, platform_version)
}

/// Attempt to decode a given type `D` from the given slice. Returns the decoded output and the amount of bytes read.
///
/// See the [config] module for more information on configurations.
///
/// [config]: config/index.html
pub fn borrow_decode_from_slice<'a, D: PlatformVersionedBorrowDecode<'a>, C: Config>(
    src: &'a [u8],
    config: C,
    platform_version: &PlatformVersion,
) -> Result<D, error::DecodeError> {
    let reader = read::SliceReader::new(src);
    let mut decoder = DecoderImpl::<_, C>::new(reader, config);
    D::platform_versioned_borrow_decode(&mut decoder, platform_version)
}

/// Attempt to decode a given type `D` from the given [Reader].
///
/// See the [config] module for more information on configurations.
///
/// [config]: config/index.html
pub fn decode_from_reader<D: PlatformVersionedDecode, R: Reader, C: Config>(
    reader: R,
    config: C,
    platform_version: &PlatformVersion,
) -> Result<D, error::DecodeError> {
    let mut decoder = DecoderImpl::<_, C>::new(reader, config);
    D::platform_versioned_decode(&mut decoder, platform_version)
}
