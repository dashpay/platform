use self::drive::DriveError;
use crate::error::cache::CacheError;
use crate::error::contract::DataContractError;
use crate::error::proof::ProofError;
use document::DocumentError;
use dpp::data_contract::errors::DataContractError as ProtocolDataContractError;
use dpp::platform_value::Error as ValueError;
use dpp::ProtocolError;
use fee::FeeError;
use grovedb_epoch_based_storage_flags::error::StorageFlagsError;
use identity::IdentityError;
use query::QuerySyntaxError;

/// Cache errors
pub mod cache;
///DataContract errors
pub mod contract;
/// Document module
pub mod document;
/// Drive module
pub mod drive;
/// Fee module
pub mod fee;
/// Identity module
pub mod identity;
/// Proof module
pub mod proof;
/// Query module
pub mod query;

/// Errors
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Query error
    #[error("query: {0}")]
    Query(#[from] QuerySyntaxError),
    /// Storage Flags error
    #[error("storage flags: {0}")]
    StorageFlags(#[from] StorageFlagsError),
    /// Drive error
    #[error("drive: {0}")]
    Drive(#[from] DriveError),
    /// Drive error
    #[error("proof: {0}")]
    Proof(#[from] ProofError),
    /// GroveDB error
    #[error("grovedb: {0}")]
    GroveDB(Box<grovedb::Error>),
    /// Protocol error
    #[error("protocol: {0}")]
    Protocol(Box<ProtocolError>),
    /// Identity error
    #[error("identity: {0}")]
    Identity(#[from] IdentityError),
    /// Fee error
    #[error("fee: {0}")]
    Fee(#[from] FeeError),
    /// Document error
    #[error("document: {0}")]
    Document(#[from] DocumentError),
    /// Value error
    #[error("value: {0}")]
    Value(#[from] ValueError),
    ///DataContract error
    #[error("contract: {0}")]
    DataContract(#[from] DataContractError),
    ///Cache error
    #[error("contract: {0}")]
    Cache(#[from] CacheError),
    /// Protocol error
    #[error("protocol: {0} ({1})")]
    ProtocolWithInfoString(Box<ProtocolError>, String),
}

impl From<ProtocolDataContractError> for Error {
    fn from(value: ProtocolDataContractError) -> Self {
        Self::Protocol(Box::new(ProtocolError::DataContractError(value)))
    }
}

impl From<ProtocolError> for Error {
    fn from(value: ProtocolError) -> Self {
        Self::Protocol(Box::new(value))
    }
}

impl From<grovedb::Error> for Error {
    fn from(value: grovedb::Error) -> Self {
        Self::GroveDB(Box::new(value))
    }
}
