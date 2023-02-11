use drive::drive::config::DriveConfig;

#[derive(Clone, Debug)]
/// Platform configuration struct
pub struct PlatformConfig {
    /// The underlying drive configuration
    pub drive_config: DriveConfig,

    /// Should we verify sum trees? Useful to set as no for tests
    pub verify_sum_trees: bool,

    /// The default quorum size
    pub quorum_size: u16,

    /// How often should quorums change?
    pub quorum_switch_block_count: u32,
}

impl Default for PlatformConfig {
    fn default() -> Self {
        PlatformConfig {
            drive_config: Default::default(),
            verify_sum_trees: true,
            quorum_size: 100,
            quorum_switch_block_count: 25,
        }
    }
}
