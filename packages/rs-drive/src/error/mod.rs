use contract::ContractError;
use drive::DriveError;
use fee::FeeError;
use identity::IdentityError;
use query::QueryError;
use structure::StructureError;
pub mod contract;
pub mod drive;
pub mod fee;
pub mod identity;
pub mod query;
pub mod structure;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Query(#[from] QueryError),
    #[error(transparent)]
    Drive(#[from] DriveError),
    #[error(transparent)]
    GroveDB(#[from] grovedb::Error),
    #[error(transparent)]
    Contract(#[from] ContractError),
    #[error(transparent)]
    Identity(#[from] IdentityError),
    #[error(transparent)]
    Structure(#[from] StructureError),
    #[error(transparent)]
    Fee(#[from] FeeError),
}
