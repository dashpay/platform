mod fetch_last_block_info_bytes;
mod fetch_platform_state_bytes;
mod fetch_reduced_platform_state_bytes;
mod store_last_block_info_bytes;
mod store_platform_state_bytes;
mod store_reduced_platform_state_bytes;

const PLATFORM_STATE_KEY: &[u8; 11] = b"saved_state";
const REDUCED_PLATFORM_STATE_KEY: &[u8; 19] = b"reduced_saved_state";
const LAST_BLOCK_INFO_KEY: &[u8; 15] = b"last_block_info";
