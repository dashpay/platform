use drive::drive::block_info::BlockInfo;

/// Platform state
pub struct PlatformState {
    /// Information about the last block
    pub last_block_info: Option<BlockInfo>,
}
