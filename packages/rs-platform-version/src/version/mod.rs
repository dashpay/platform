mod protocol_version;
pub use protocol_version::*;
use crate::version::v2::PROTOCOL_VERSION_2;

pub mod contracts;
pub mod dpp_versions;
pub mod drive_abci_versions;
pub mod drive_versions;
pub mod fee;
#[cfg(feature = "mock-versions")]
pub mod mocks;
pub mod v1;
pub mod v2;

pub const LATEST_VERSION: u32 = PROTOCOL_VERSION_2;
