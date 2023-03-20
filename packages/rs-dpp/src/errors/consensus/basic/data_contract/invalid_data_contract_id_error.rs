use crate::consensus::basic::BasicError;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Data Contract Id must be {}, got {}", bs58::encode(expected_id).into_string(), bs58::encode(invalid_id).into_string())]
pub struct InvalidDataContractIdError {
    expected_id: Vec<u8>,
    invalid_id: Vec<u8>,
}

impl InvalidDataContractIdError {
    pub fn new(expected_id: Vec<u8>, invalid_id: Vec<u8>) -> Self {
        Self {
            expected_id,
            invalid_id,
        }
    }

    pub fn expected_id(&self) -> Vec<u8> {
        self.expected_id.clone()
    }
    pub fn invalid_id(&self) -> Vec<u8> {
        self.invalid_id.clone()
    }
}

impl From<InvalidDataContractIdError> for BasicError {
    fn from(err: InvalidDataContractIdError) -> Self {
        Self::InvalidDataContractIdError(err)
    }
}
