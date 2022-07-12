use DriveEncoding::DriveProtobuf;

pub const DEFAULT_GROVE_BATCHING_ENABLED: bool = true;
pub const DEFAULT_GROVE_HAS_RAW_ENABLED: bool = true;

pub enum DriveEncoding {
    DriveCbor,
    DriveProtobuf,
}

pub struct DriveConfig {
    pub batching_enabled: bool,
    pub has_raw_enabled: bool,
    pub encoding: DriveEncoding,
}

impl Default for DriveConfig {
    fn default() -> Self {
        DriveConfig {
            batching_enabled: DEFAULT_GROVE_BATCHING_ENABLED,
            has_raw_enabled: DEFAULT_GROVE_HAS_RAW_ENABLED,
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
