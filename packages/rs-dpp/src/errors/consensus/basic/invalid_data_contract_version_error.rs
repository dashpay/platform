use crate::consensus::basic::BasicError;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Data Contract version must be {expected_version}, go {version}")]
pub struct InvalidDataContractVersionError {
    expected_version: u32,
    version: u32,
}

impl InvalidDataContractVersionError {
    pub fn new(expected_version: u32, version: u32) -> Self {
        Self {
            expected_version,
            version,
        }
    }

    pub fn expected_version(&self) -> u32 {
        self.expected_version
    }

    pub fn version(&self) -> u32 {
        self.version
    }
}

impl From<InvalidDataContractVersionError> for BasicError {
    fn from(err: InvalidDataContractVersionError) -> Self {
        Self::InvalidDataContractVersionError(err)
    }
}
