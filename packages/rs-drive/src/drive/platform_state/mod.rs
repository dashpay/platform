mod fetch_platform_state_bytes;
mod fetch_reduced_platform_state_bytes;
mod store_platform_state_bytes;
mod store_reduced_platform_state_bytes;

const PLATFORM_STATE_KEY: &[u8; 11] = b"saved_state";
const REDUCED_PLATFORM_STATE_KEY: &[u8; 19] = b"reduced_saved_state";
