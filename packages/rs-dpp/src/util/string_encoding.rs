use bs58;
use base64;

pub enum Encoding {
    Base58,
    Base64
}

pub enum StringDecodeError {
    Base64(base64::DecodeError),
    Base58(bs58::decode::Error)
}

impl From<base64::DecodeError> for StringDecodeError {
    fn from(err: base64::DecodeError) -> Self {
        StringDecodeError::Base64(err)
    }
}

impl From<bs58::decode::Error> for StringDecodeError {
    fn from(err: bs58::decode::Error) -> Self {
        StringDecodeError::Base58(err)
    }
}

pub fn decode(encoded_value: &str, encoding: Encoding) -> Result<Vec<u8>, StringDecodeError> {
    match encoding {
        Encoding::Base58 => Ok(bs58::decode(encoded_value).into_vec()?),
        Encoding::Base64 => Ok(base64::decode(encoded_value)?)
    }
}

pub fn encode(value: &[u8], encoding: Encoding) -> String {
    match encoding {
        Encoding::Base58 => bs58::encode(value).into_string(),
        Encoding::Base64 => base64::encode(value)
    }
}