use DriveEncoding::DriveProtobuf;

pub const DEFAULT_GROVE_BATCHING_ENABLED: bool = true;
pub const DEFAULT_GROVE_BATCHING_CONSISTENCY_VERIFICATION_ENABLED: bool = false;
pub const DEFAULT_GROVE_HAS_RAW_ENABLED: bool = true;

pub enum DriveEncoding {
    DriveCbor,
    DriveProtobuf,
}

pub struct DriveConfig {
    pub batching_enabled: bool,
    pub batching_consistency_verification: bool,
    pub has_raw_enabled: bool,
    pub default_genesis_time: Option<u64>,
    pub encoding: DriveEncoding,
}

impl Default for DriveConfig {
    fn default() -> Self {
        DriveConfig {
            batching_enabled: DEFAULT_GROVE_BATCHING_ENABLED,
            batching_consistency_verification:
                DEFAULT_GROVE_BATCHING_CONSISTENCY_VERIFICATION_ENABLED,
            has_raw_enabled: DEFAULT_GROVE_HAS_RAW_ENABLED,
            default_genesis_time: None,
            encoding: DriveProtobuf,
        }
    }
}

impl DriveConfig {
    pub fn default_with_batches() -> Self {
        DriveConfig {
            batching_enabled: true,
            ..Default::default()
        }
    }

    pub fn default_without_batches() -> Self {
        DriveConfig {
            batching_enabled: false,
            ..Default::default()
        }
    }
}
