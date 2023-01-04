use drive::drive::config::DriveConfig;

#[derive(Clone,Debug)]
/// Platform configuration struct
pub struct PlatformConfig {
    /// The underlying drive configuration
    pub drive_config: DriveConfig,

    /// Should we verify sum trees? Useful to set as no for tests
    pub verify_sum_trees: bool,
}

impl Default for PlatformConfig {
    fn default() -> Self {
        PlatformConfig {
            drive_config: Default::default(),
            verify_sum_trees: true,
        }
    }
}