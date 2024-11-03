use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use dpp::dashcore::ProTxHash;

impl PlatformState {
    /// Patches the state to handle a commit failure at block 32326 where an Evonode deletion event was skipped.
    ///
    /// ### Background
    /// At block 32326, a commit to storage failed due to a busy error from RocksDB. This caused the deletion
    /// of an Evonode to not be saved to disk, even though the chain lock update included this deletion in
    /// the in-memory cache. As a result, the in-memory state and disk state became inconsistent, with the
    /// cache containing an Evonode that was no longer valid according to the chain lock.
    ///
    /// This inconsistency led the chain to continue with an incorrect state representation, causing issues
    /// in achieving consensus when nodes would reload.
    ///
    /// ### Purpose
    /// To resolve the inconsistency, this function replays the deletion event of the Evonode when the
    /// state is loaded from disk, ensuring the cache and disk states are aligned. This enables the chain
    /// to proceed without stalling.
    ///
    /// ### Implementation
    /// - Identifies the Evonode removed at block 32326 by its ProTxHash.
    /// - Removes the Evonode from the current validator sets, ensuring it is no longer part of
    ///   any active validator set.
    /// - Updates both the full and HPMN (high-performance masternode) lists by removing the Evonode
    ///   entry, fully clearing it from the in-memory state.
    ///
    /// ### Usage
    /// This function should be called during state loading to apply the corrective deletion event,
    /// aligning the in-memory cache with the on-disk state for correct chain progression.
    pub fn patch_error_block_32326_commit_failure<C>(&mut self) {
        // At block 32326 we only had 1 event which was the removal of an Evonode
        let removed_evonode_hash =
            ProTxHash::from_hex("fc2e4a2037610a2f90a0f26d3e8e0b9d84e35cb318dba8e3b9a8056922e09a93")
                .expect("expected valid evonode hash");
        Platform::<C>::remove_masternode_in_validator_sets(
            &removed_evonode_hash,
            self.validator_sets_mut(),
        );

        self.full_masternode_list_mut()
            .remove(&removed_evonode_hash);
        self.hpmn_masternode_list_mut()
            .remove(&removed_evonode_hash);
    }
}
