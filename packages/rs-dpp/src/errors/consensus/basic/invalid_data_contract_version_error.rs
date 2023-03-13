use crate::{consensus::basic::BasicError, prelude::DataContractVersion};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Data Contract version must be {expected_version}, go {version}")]
pub struct InvalidDataContractVersionError {
    expected_version: DataContractVersion,
    version: DataContractVersion,
}

impl InvalidDataContractVersionError {
    pub fn new(expected_version: DataContractVersion, version: DataContractVersion) -> Self {
        Self {
            expected_version,
            version,
        }
    }

    pub fn expected_version(&self) -> DataContractVersion {
        self.expected_version
    }

    pub fn version(&self) -> DataContractVersion {
        self.version
    }
}

impl From<InvalidDataContractVersionError> for BasicError {
    fn from(err: InvalidDataContractVersionError) -> Self {
        Self::InvalidDataContractVersionError(err)
    }
}
