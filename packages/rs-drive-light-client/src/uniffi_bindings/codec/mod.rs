use bytes::Buf;

#[cfg(feature = "json")]
pub mod json;

#[cfg(feature = "json")]
pub static DEFAULT_CODEC: json::JsonCodec = json::JsonCodec {};

pub trait Codec {
    fn encode<T>(&self, data: &T) -> Result<Vec<u8>, crate::Error>
    where
        T: serde::Serialize;
    fn decode<T>(&self, buf: &mut dyn Buf) -> Result<T, crate::Error>
    where
        T: serde::de::DeserializeOwned;
}
