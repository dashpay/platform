use crate::errors::ProtocolError;
use base64;
use bs58;

pub enum Encoding {
    Base58,
    Base64,
}

pub fn decode(encoded_value: &str, encoding: Encoding) -> Result<Vec<u8>, ProtocolError> {
    match encoding {
        Encoding::Base58 => Ok(bs58::decode(encoded_value)
            .into_vec()
            .map_err(|e| ProtocolError::StringDecodeError(e.to_string()))?),
        Encoding::Base64 => Ok(base64::decode(encoded_value)
            .map_err(|e| ProtocolError::StringDecodeError(e.to_string()))?),
    }
}

pub fn encode(value: &[u8], encoding: Encoding) -> String {
    match encoding {
        Encoding::Base58 => bs58::encode(value).into_string(),
        Encoding::Base64 => base64::encode(value),
    }
}
