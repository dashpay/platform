use crate::Error;
use bytes::Buf;
use serde::{de::DeserializeOwned, Serialize};

use super::Codec;

pub struct JsonCodec {}

impl Codec for JsonCodec {
    fn decode<T>(&self, buf: &mut dyn Buf) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        serde_json::from_reader(buf.reader()).map_err(|e| Error::DataEncodingError {
            error: e.to_string(),
        })
    }
    fn encode<T>(&self, data: &T) -> Result<Vec<u8>, Error>
    where
        T: Serialize,
    {
        serde_json::to_vec(data).map_err(|e| Error::DataEncodingError {
            error: e.to_string(),
        })
    }
}

// impl Serializer for GetIdentityRequest {
//     fn deserialize() -> Result<Self, Error> {
//         todo!()
//     }
//     fn serialize(&self) -> Result<Vec<u8>, Error> {
//         todo!()
//     }
// }

// impl Serializer for GetIdentityResponse {
//     fn deserialize() -> Result<Self, Error> {
//         todo!()
//     }
//     fn serialize(&self) -> Result<Vec<u8>, Error> {
//         todo!()
//     }
// }

// impl Serializer for Identity {
//     fn deserialize() -> Result<Self, Error> {
//         todo!()
//     }
//     fn serialize(&self) -> Result<Vec<u8>, Error> {
//         todo!()
//     }
// }
