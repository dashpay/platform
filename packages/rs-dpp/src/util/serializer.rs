use crate::errors::ProtocolError;
use byteorder::{LittleEndian, WriteBytesExt};
// ciborium::value

pub const MAX_ENCODED_KBYTE_LENGTH: usize = 16;

pub fn value_to_cbor(
    value: serde_json::Value,
    protocol_version: Option<u32>,
) -> Result<Vec<u8>, ProtocolError> {
    let mut buffer: Vec<u8> = Vec::new();
    if let Some(protocol_version) = protocol_version {
        buffer
            .write_u32::<LittleEndian>(protocol_version)
            .map_err(|e| ProtocolError::EncodingError(e.to_string()))?
    }
    let size_with_protocol = buffer.len();

    ciborium::ser::into_writer(&value, &mut buffer)
        .map_err(|e| ProtocolError::EncodingError(e.to_string()))?;

    if (buffer.len() - size_with_protocol) >= MAX_ENCODED_KBYTE_LENGTH * 1024 {
        return Err(ProtocolError::MaxEncodedBytesReachedError(
            MAX_ENCODED_KBYTE_LENGTH,
        ));
    }

    Ok(buffer)
}
