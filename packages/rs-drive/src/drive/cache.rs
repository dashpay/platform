use dpp::identity::TimestampMillis;
use std::sync::RwLock;

mod data_contract;
mod protocol_version;
mod system_contracts;

pub use data_contract::DataContractCache;
pub use protocol_version::ProtocolVersionsCache;
pub use system_contracts::SystemDataContracts;

/// Drive cache struct
pub struct DriveCache {
    /// Cached contracts
    pub data_contracts: DataContractCache,
    // TODO: We probably don't need this since we have it genesis cache in the platform
    /// Genesis time in ms
    pub genesis_time_ms: RwLock<Option<TimestampMillis>>,
    // TODO: Make protocol versions cache thread-safe
    /// Lazy loaded counter of votes to upgrade protocol version
    pub protocol_versions_counter: RwLock<ProtocolVersionsCache>,
    /// Versioned system data contracts
    pub system_data_contracts: SystemDataContracts,
}
