use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use dpp::serialization::PlatformDeserializableFromVersionedStructure;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::query::TransactionArg;

impl<C> Platform<C> {
    pub(super) fn fetch_platform_state_v0(
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<PlatformState>, Error> {
        drive
            .fetch_platform_state_bytes(transaction, platform_version)
            .map_err(Error::Drive)?
            .map(|bytes| {
                let result = PlatformState::versioned_deserialize(&bytes, platform_version)
                    .map_err(Error::Protocol);

                if result.is_err() {
                    tracing::error!(
                        bytes = hex::encode(&bytes),
                        "Unable deserialize platform state for version {}",
                        platform_version.protocol_version
                    );
                }

                let mut platform_state = result?;

                if platform_state.last_committed_block_height() >= 32326 {
                    // Apply a corrective patch to handle the commit failure at block 32326
                    //
                    // Due to a missed commit of block 32326, an Evonode deletion was not saved to disk.
                    // This discrepancy caused an inconsistency between the in-memory cache and the disk state,
                    // leading to potential issues in consensus.
                    //
                    // Calling `patch_error_block_32326_commit_failure` here replays the deletion event
                    // during state loading, ensuring that the in-memory state reflects the actual state
                    // on the network, allowing the chain to proceed correctly.
                    platform_state.patch_error_block_32326_commit_failure::<C>();
                }

                Ok(platform_state)
            })
            .transpose()
    }
}
