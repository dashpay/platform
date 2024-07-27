pub mod protocol_version;
use crate::version::v1::PROTOCOL_VERSION_1;
pub use protocol_version::{AbciStructureVersion, FeatureVersion, FeatureVersionBounds, OptionalFeatureVersion, PlatformArchitectureVersion, PlatformVersion, PLATFORM_VERSIONS, LATEST_PLATFORM_VERSION};

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

pub type ProtocolVersion = u32;

pub const LATEST_VERSION: ProtocolVersion = PROTOCOL_VERSION_1;
pub const INITIAL_PROTOCOL_VERSION: ProtocolVersion = 1;
