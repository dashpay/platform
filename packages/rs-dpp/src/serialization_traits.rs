use crate::ProtocolError;

pub trait Signable {
    fn signable_bytes(&self) -> Result<Vec<u8>, ProtocolError>;
}

pub trait PlatformDeserializable {
    fn deserialize(data: &[u8]) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}
