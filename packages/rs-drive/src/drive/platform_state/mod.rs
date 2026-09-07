mod fetch_platform_state_bytes;
mod fetch_platform_state_recent_bytes;
mod store_platform_state_bytes;
mod store_platform_state_recent_bytes;

const PLATFORM_STATE_KEY: &[u8; 11] = b"saved_state";

/// The small companion to [`PLATFORM_STATE_KEY`]: the fields of the platform
/// state that change on every block. The full record is rewritten only when the
/// masternode lists, validator sets or quorum sets change, so this one carries
/// the block info in between. Both are written in the block's transaction, so a
/// reader always sees a pair that committed together.
const PLATFORM_STATE_RECENT_KEY: &[u8; 18] = b"saved_state_recent";
