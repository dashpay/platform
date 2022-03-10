use serde::{Deserialize, Serialize};
use std::convert::TryInto;

use crate::errors::ProtocolError;
use crate::util::string_encoding;
use crate::util::string_encoding::Encoding;

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub struct Identifier {
    buffer: [u8; 32],
}

fn encoding_string_to_encoding(encoding_string: Option<&str>) -> Encoding {
    match encoding_string {
        Some(str) => {
            //? should it be case-sensitive??
            if str == "base58" {
                Encoding::Base58
            } else {
                Encoding::Base64
            }
        }
        None => Encoding::Base58,
    }
}

impl Identifier {
    fn new(buffer: [u8; 32]) -> Identifier {
        Identifier { buffer }
    }

    pub fn from_string(
        encoded_value: &str,
        encoding: Encoding,
    ) -> Result<Identifier, ProtocolError> {
        let vec = string_encoding::decode(encoded_value, encoding)?;

        Identifier::from_bytes(&vec)
    }

    pub fn from_string_with_encoding_string(
        encoded_value: &str,
        encoding_string: Option<&str>,
    ) -> Result<Identifier, ProtocolError> {
        let encoding = encoding_string_to_encoding(encoding_string);

        Identifier::from_string(encoded_value, encoding)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Identifier, ProtocolError> {
        if bytes.len() != 32 {
            return Err(ProtocolError::IdentifierError(String::from(
                "Identifier must be 32 bytes long",
            )));
        }

        // Since we checked that vector size is 32, we can use unwrap
        Ok(Identifier::new(bytes.try_into().unwrap()))
    }

    pub fn to_buffer(&self) -> [u8; 32] {
        self.buffer
    }

    pub fn to_string(&self, encoding: Encoding) -> String {
        string_encoding::encode(&self.buffer, encoding)
    }

    pub fn to_string_with_encoding_string(&self, encoding_string: Option<&str>) -> String {
        let encoding = encoding_string_to_encoding(encoding_string);

        self.to_string(encoding)
    }
}
