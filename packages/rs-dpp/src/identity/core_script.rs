use bincode::de::{BorrowDecoder, Decoder};
use bincode::enc::Encoder;
use bincode::error::{DecodeError, EncodeError};
use bincode::{BorrowDecode, Decode, Encode};
use dashcore::blockdata::opcodes;
use std::fmt;
use std::ops::Deref;

use dashcore::{Script as DashcoreScript, Script};
use platform_value::string_encoding::{self, Encoding};
use rand::rngs::StdRng;
use rand::Rng;

use serde::de::Visitor;
use serde::{Deserialize, Serialize};

use crate::ProtocolError;
use bincode::de::read::Reader;

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct CoreScript(DashcoreScript);

impl CoreScript {
    pub fn new(script: DashcoreScript) -> Self {
        CoreScript(script)
    }

    pub fn to_string(&self, encoding: Encoding) -> String {
        string_encoding::encode(&self.0.to_bytes(), encoding)
    }

    pub fn from_string(encoded_value: &str, encoding: Encoding) -> Result<Self, ProtocolError> {
        let vec = string_encoding::decode(encoded_value, encoding)?;

        Ok(Self(vec.into()))
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self(bytes.into())
    }

    pub fn random_p2pkh(rng: &mut StdRng) -> Self {
        let mut bytes = vec![
            opcodes::all::OP_DUP.into_u8(),
            opcodes::all::OP_HASH160.into_u8(),
            opcodes::all::OP_PUSHBYTES_20.into_u8(),
        ];
        bytes.append(&mut rng.gen::<[u8; 20]>().to_vec());
        bytes.push(opcodes::all::OP_EQUALVERIFY.into_u8());
        bytes.push(opcodes::all::OP_CHECKSIG.into_u8());
        Self::from_bytes(bytes)
    }

    pub fn random_p2sh(rng: &mut StdRng) -> Self {
        let mut bytes = vec![
            opcodes::all::OP_HASH160.into_u8(),
            opcodes::all::OP_PUSHBYTES_20.into_u8(),
        ];
        bytes.append(&mut rng.gen::<[u8; 20]>().to_vec());
        bytes.push(opcodes::all::OP_EQUAL.into_u8());
        Self::from_bytes(bytes)
    }
}

impl From<Vec<u8>> for CoreScript {
    fn from(value: Vec<u8>) -> Self {
        CoreScript::from_bytes(value)
    }
}

impl Deref for CoreScript {
    type Target = DashcoreScript;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// Implement the bincode::Encode trait for CoreScript
impl Encode for CoreScript {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        self.0.as_bytes().encode(encoder)
    }
}

// Implement the bincode::Decode trait for CoreScript
impl Decode for CoreScript {
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        let bytes = Vec::<u8>::decode(decoder)?;
        // Create a CoreScript instance using the decoded DashCoreScript
        Ok(CoreScript(Script(bytes.into_boxed_slice())))
    }
}

impl<'de> BorrowDecode<'de> for CoreScript {
    fn borrow_decode<D: BorrowDecoder<'de>>(decoder: &mut D) -> Result<Self, DecodeError> {
        // Read the serialized bytes from the decoder into a Vec<u8>
        let mut bytes = Vec::new();
        loop {
            let buf_len = 1024; // Adjust the buffer size as needed
            let mut temp = Vec::with_capacity(buf_len);
            match decoder.reader().read(&mut temp) {
                Ok(()) => {
                    let read_bytes = temp.iter().position(|&x| x == 0).unwrap_or(temp.len());
                    bytes.extend_from_slice(&temp[..read_bytes]);
                    if read_bytes < buf_len {
                        break;
                    }
                }
                Err(DecodeError::Io { inner, additional })
                    if inner.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    if additional > 0 {
                        return Err(DecodeError::Io { inner, additional });
                    } else {
                        break;
                    }
                }
                Err(e) => return Err(e),
            }
        }

        // Convert Vec<u8> to Box<[u8]> and create a DashCoreScript instance
        let dash_core_script = DashcoreScript(bytes.into_boxed_slice());

        // Create a CoreScript instance using the decoded DashCoreScript
        Ok(CoreScript(dash_core_script))
    }
}

impl Serialize for CoreScript {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_str(&self.to_string(Encoding::Base64))
        } else {
            serializer.serialize_bytes(self.as_bytes())
        }
    }
}

impl<'de> Deserialize<'de> for CoreScript {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            let data: String = Deserialize::deserialize(deserializer)?;

            Self::from_string(&data, Encoding::Base64)
                .map_err(|e| serde::de::Error::custom(e.to_string()))
        } else {
            struct BytesVisitor;

            impl<'de> Visitor<'de> for BytesVisitor {
                type Value = CoreScript;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("a byte array")
                }

                fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    Ok(CoreScript::from_bytes(v.to_vec()))
                }
            }

            deserializer.deserialize_bytes(BytesVisitor)
        }
    }
}

impl std::fmt::Display for CoreScript {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string(Encoding::Base64))
    }
}
