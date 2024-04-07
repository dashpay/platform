use crate::Error;
use base64;
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use bs58;

pub enum Encoding {
    Base58,
    Base64,
    Hex,
}

pub fn decode(encoded_value: &str, encoding: Encoding) -> Result<Vec<u8>, Error> {
    match encoding {
        Encoding::Base58 => Ok(bs58::decode(encoded_value)
            .into_vec()
            .map_err(|e| Error::StringDecodingError(e.to_string()))?),
        Encoding::Base64 => Ok(BASE64_STANDARD
            .decode(encoded_value)
            .map_err(|e| Error::StringDecodingError(e.to_string()))?),
        Encoding::Hex => Ok(
            hex::decode(encoded_value).map_err(|e| Error::StringDecodingError(e.to_string()))?
        ),
    }
}

pub fn encode(value: &[u8], encoding: Encoding) -> String {
    match encoding {
        Encoding::Base58 => bs58::encode(value).into_string(),
        Encoding::Base64 => BASE64_STANDARD.encode(value),
        Encoding::Hex => hex::encode(value),
    }
}
