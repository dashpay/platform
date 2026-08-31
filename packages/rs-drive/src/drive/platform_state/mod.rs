use crate::drive::Drive;
mod fetch_platform_state_bytes;
mod store_platform_state_bytes;

const PLATFORM_STATE_KEY: &[u8; 11] = b"saved_state";

/// The small companion to [`PLATFORM_STATE_KEY`]: the fields of the platform
/// state that change on every block. The full record is rewritten only when the
/// masternode lists, validator sets or quorum sets change, so this one carries
/// the block info in between. Both are written in the block's transaction, so a
/// reader always sees a pair that committed together.
const PLATFORM_STATE_RECENT_KEY: &[u8; 18] = b"saved_state_recent";

impl Drive {
    /// Stores the per-block part of the platform state in auxiliary storage.
    pub fn store_platform_state_recent_bytes(
        &self,
        state_bytes: &[u8],
        transaction: grovedb::TransactionArg,
    ) -> Result<(), crate::error::Error> {
        self.grove
            .put_aux(PLATFORM_STATE_RECENT_KEY, state_bytes, None, transaction)
            .unwrap()
            .map_err(crate::error::Error::from)
    }

    /// Fetches the per-block part of the platform state, if one was ever written.
    pub fn fetch_platform_state_recent_bytes(
        &self,
        transaction: grovedb::TransactionArg,
    ) -> Result<Option<Vec<u8>>, crate::error::Error> {
        self.grove
            .get_aux(PLATFORM_STATE_RECENT_KEY, transaction)
            .unwrap()
            .map_err(crate::error::Error::from)
    }
}
