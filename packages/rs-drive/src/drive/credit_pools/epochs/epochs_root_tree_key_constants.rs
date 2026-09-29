/// Storage fee pool key
pub const KEY_STORAGE_FEE_POOL: &[u8; 1] = b"s";

/// Storage fee pool key
pub const KEY_STORAGE_FEE_POOL_U8: u8 = b's';
/// Unpaid epoch index key
pub const KEY_UNPAID_EPOCH_INDEX: &[u8; 1] = b"u";
/// Unpaid epoch index key
pub const KEY_UNPAID_EPOCH_INDEX_U8: u8 = b'u';
/// Pending refunds that will be deducted from epoch storage fee pools
pub const KEY_PENDING_EPOCH_REFUNDS: &[u8; 1] = b"p";
/// Pending refunds that will be deducted from epoch storage fee pools
pub const KEY_PENDING_EPOCH_REFUNDS_U8: u8 = b'p';
/// Storage fees for storage that lives a known number of epochs, by the epoch they were
/// collected in and that number (protocol version 14, document time to live), waiting to be
/// spread over those epochs
pub const KEY_LIFETIME_STORAGE_FEE_POOLS: &[u8; 1] = b"l";
