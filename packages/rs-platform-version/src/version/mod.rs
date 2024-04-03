mod protocol_version;
pub use protocol_version::*;
pub mod contracts;
pub mod dpp_versions;
pub mod drive_abci_versions;
pub mod drive_versions;
pub mod fee;
#[cfg(feature = "mock-versions")]
pub mod mocks;
mod v1;

pub const LATEST_VERSION: u32 = 1;
