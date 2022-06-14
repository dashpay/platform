pub const DEFAULT_GROVE_BATCHING_ENABLED: bool = false;

pub struct DriveConfig {
    pub batching_enabled: bool,
}

impl DriveConfig {
    pub fn default() -> Self {
        DriveConfig {
            batching_enabled: DEFAULT_GROVE_BATCHING_ENABLED,
        }
    }
}
