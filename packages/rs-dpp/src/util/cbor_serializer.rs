use integer_encoding::VarIntWriter;
use serde::ser;

use crate::errors::ProtocolError;

// ciborium::value

pub const MAX_ENCODED_KBYTE_LENGTH: usize = 16;

pub fn serializable_value_to_cbor<T: ?Sized + ser::Serialize>(
    value: &T,
    version: Option<u32>,
) -> Result<Vec<u8>, ProtocolError> {
    let mut buffer: Vec<u8> = Vec::new();
    if let Some(version) = version {
        buffer
            .write_varint(version)
            .map_err(|e| ProtocolError::EncodingError(e.to_string()))?;
    }
    let size_with_protocol = buffer.len();

    ciborium::ser::into_writer(value, &mut buffer)
        .map_err(|e| ProtocolError::EncodingError(e.to_string()))?;

    if (buffer.len() - size_with_protocol) >= MAX_ENCODED_KBYTE_LENGTH * 1024 {
        return Err(ProtocolError::MaxEncodedBytesReachedError {
            size_hit: buffer.len(),
            max_size_kbytes: MAX_ENCODED_KBYTE_LENGTH,
        });
    }

    Ok(buffer)
}
