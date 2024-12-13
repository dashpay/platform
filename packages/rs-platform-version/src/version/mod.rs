mod protocol_version;
// use crate::version::v7::PROTOCOL_VERSION_7;
pub use protocol_version::*;
use crate::version::v6::PROTOCOL_VERSION_6;

mod consensus_versions;
pub mod dpp_versions;
pub mod drive_abci_versions;
pub mod drive_versions;
pub mod fee;
#[cfg(feature = "mock-versions")]
pub mod mocks;
pub mod patches;
pub mod system_data_contract_versions;
mod system_limits;
pub mod v1;
pub mod v2;
pub mod v3;
pub mod v4;
pub mod v5;
pub mod v6;
pub mod v7;

pub type ProtocolVersion = u32;

pub const LATEST_VERSION: ProtocolVersion = PROTOCOL_VERSION_6;
pub const INITIAL_PROTOCOL_VERSION: ProtocolVersion = 1;
