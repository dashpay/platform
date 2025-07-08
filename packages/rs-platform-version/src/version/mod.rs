mod protocol_version;

use crate::version::v10::PROTOCOL_VERSION_10;
pub use protocol_version::*;
use std::ops::RangeInclusive;

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
pub mod v10;
pub mod v2;
pub mod v3;
pub mod v4;
pub mod v5;
pub mod v6;
pub mod v7;
pub mod v8;
pub mod v9;

pub type ProtocolVersion = u32;

pub const ALL_VERSIONS: RangeInclusive<ProtocolVersion> = 1..=LATEST_VERSION;

pub const LATEST_VERSION: ProtocolVersion = PROTOCOL_VERSION_10;
pub const INITIAL_PROTOCOL_VERSION: ProtocolVersion = 1;
