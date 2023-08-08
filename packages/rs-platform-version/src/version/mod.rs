mod protocol_version;
pub use protocol_version::*;
pub mod dpp_versions;
pub mod drive_abci_versions;
pub mod drive_versions;
mod v1;
#[cfg(feature = "mock-versions")]
pub mod mocks;

pub const LATEST_VERSION: u32 = 1;
