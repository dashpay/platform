use thiserror::Error;

#[derive(Debug, Error, Clone, Eq, PartialEq)]
#[error("Payload reached a {max_size_kbytes}KB limit")]
pub struct MaxEncodedBytesReachedError {
    payload: Vec<u8>,
    max_size_kbytes: usize,
}

impl MaxEncodedBytesReachedError {
    pub fn new(payload: Vec<u8>, max_size_kbytes: usize) -> Self {
        Self {
            payload,
            max_size_kbytes,
        }
    }

    pub fn payload(&self) -> Vec<u8> {
        self.payload.clone()
    }

    pub fn max_size_kbytes(&self) -> usize {
        self.max_size_kbytes
    }
}
