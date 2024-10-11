pub mod protocol_version;
use crate::version::v4::PROTOCOL_VERSION_4;
pub use protocol_version::*;

mod consensus_versions;
pub mod contracts;
pub mod dpp_versions;
pub mod drive_abci_versions;
pub mod drive_versions;
pub mod fee;
pub mod limits;
#[cfg(feature = "mock-versions")]
pub mod mocks;
pub mod patches;
pub mod v1;
pub mod v2;
pub mod v3;
pub mod v4;

pub type ProtocolVersion = u32;

pub const LATEST_VERSION: ProtocolVersion = PROTOCOL_VERSION_4;
pub const INITIAL_PROTOCOL_VERSION: ProtocolVersion = 1;
