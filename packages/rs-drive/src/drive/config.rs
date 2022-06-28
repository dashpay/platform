use DriveEncoding::DriveProtobuf;

pub const DEFAULT_GROVE_BATCHING_ENABLED: bool = false;

pub enum DriveEncoding {
    DriveCbor,
    DriveProtobuf,
}

pub struct DriveConfig {
    pub batching_enabled: bool,
    pub encoding: DriveEncoding,
}

impl DriveConfig {
    pub fn default() -> Self {
        DriveConfig {
            batching_enabled: DEFAULT_GROVE_BATCHING_ENABLED,
            encoding: DriveProtobuf,
        }
    }
}
