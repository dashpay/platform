mod protocol_version;
pub use protocol_version::*;
pub mod dpp_versions;
pub mod drive_abci_versions;
pub mod drive_versions;
mod v0;

pub const LATEST_VERSION: u32 = 1;
