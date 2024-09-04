mod protocol_version;
use crate::version::v2::PROTOCOL_VERSION_2;
pub use protocol_version::*;

pub mod contracts;
pub mod dpp_versions;
pub mod drive_abci_versions;
pub mod drive_versions;
pub mod fee;
mod limits;
#[cfg(feature = "mock-versions")]
pub mod mocks;
pub mod patches;
pub mod v1;
pub mod v2;

pub type ProtocolVersion = u32;

pub const LATEST_VERSION: ProtocolVersion = PROTOCOL_VERSION_2;
pub const INITIAL_PROTOCOL_VERSION: ProtocolVersion = 1;
