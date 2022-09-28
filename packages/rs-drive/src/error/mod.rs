use document::DocumentError;
use dpp::data_contract::extra::ContractError;
use drive::DriveError;
use fee::FeeError;
use identity::IdentityError;
use query::QueryError;
use structure::StructureError;

/// Document module
pub mod document;
/// Drive module
pub mod drive;
/// Fee module
pub mod fee;
/// Identity module
pub mod identity;
/// Query module
pub mod query;
/// Structure module
pub mod structure;

/// Errors
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Query error
    #[error("query: {0}")]
    Query(#[from] QueryError),
    /// Drive error
    #[error("drive: {0}")]
    Drive(#[from] DriveError),
    /// GroveDB error
    #[error("grovedb: {0}")]
    GroveDB(#[from] grovedb::Error),
    /// Contract error
    #[error("contract: {0}")]
    Contract(#[from] ContractError),
    /// Identity error
    #[error("identity: {0}")]
    Identity(#[from] IdentityError),
    /// Structure error
    #[error("structure: {0}")]
    Structure(#[from] StructureError),
    /// Fee error
    #[error("fee: {0}")]
    Fee(#[from] FeeError),
    /// Document error
    #[error("document: {0}")]
    Document(#[from] DocumentError),
}
