mod protocol_version;
<<<<<<< HEAD
use crate::version::v9::PROTOCOL_VERSION_9;
=======
use crate::version::v8::PROTOCOL_VERSION_8;
>>>>>>> v1.8-dev
pub use protocol_version::*;

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
pub mod v8;
pub mod v9;

pub type ProtocolVersion = u32;

pub const LATEST_VERSION: ProtocolVersion = PROTOCOL_VERSION_9;
pub const INITIAL_PROTOCOL_VERSION: ProtocolVersion = 1;
