/// Processing fee pool key
pub const KEY_POOL_PROCESSING_FEES: &[u8; 1] = b"p";
/// Storage fee pool key
pub const KEY_POOL_STORAGE_FEES: &[u8; 1] = b"s";
/// Start time key
pub const KEY_START_TIME: &[u8; 1] = b"t";
/// Start block height key
pub const KEY_START_BLOCK_HEIGHT: &[u8; 1] = b"c";
/// Proposers key
pub const KEY_PROPOSERS: &[u8; 1] = b"m";
/// Fee multiplier key
pub const KEY_FEE_MULTIPLIER: &[u8; 1] = b"x";
/// Epoch storage offset
pub(crate) const EPOCH_STORAGE_OFFSET: u16 = 256;
