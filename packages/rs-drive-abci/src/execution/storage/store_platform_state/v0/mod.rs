use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::recent::PlatformStateRecent;
use crate::platform_types::platform_state::PlatformState;
use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0Getters;
use dpp::serialization::PlatformSerializable;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use drive::query::TransactionArg;

impl<C> Platform<C> {
    pub(super) fn store_platform_state_v0(
        &self,
        state: &PlatformState,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        #[cfg(feature = "testing-config")]
        let should_store = self.config.testing_configs.store_platform_state;
        #[cfg(not(feature = "testing-config"))]
        let should_store = true;

        if should_store {
            // The masternode lists, validator sets and quorum sets are most of
            // the record — over a megabyte on mainnet — and only change when
            // Core's do, which is a minority of blocks. While replaying history
            // the full record is rewritten only when one of them moved, and the
            // small record below carries the per-block fields in between. Both
            // are written in the block's transaction, so a reader never sees
            // them disagree.
            //
            // Once the node is at the tip the full record is written every block
            // again, so a node that is up to date always has a complete record on
            // disk and an older drive-abci — which knows nothing about the small
            // record — can still read it. Skipping is confined to a node that is
            // catching up, where the remedy for any format trouble is the resync
            // it is already doing.
            if state.heavy_fields_dirty
                || !state
                    .last_committed_block_info
                    .as_ref()
                    .is_some_and(|info| {
                        crate::utils::is_historical_block(info.basic_info().time_ms)
                    })
            {
                let bytes = state.serialize_to_bytes()?;
                self.drive
                    .store_platform_state_bytes(&bytes, transaction, platform_version)
                    .map_err(Error::Drive)?;
            }

            let recent: PlatformStateRecent = state.into();
            let recent_bytes = bincode::encode_to_vec(
                recent,
                bincode::config::standard()
                    .with_big_endian()
                    .with_no_limit(),
            )
            .map_err(|e| {
                Error::Protocol(ProtocolError::PlatformSerializationError(format!(
                    "unable to serialize recent platform state: {e}"
                )))
            })?;
            self.drive
                .store_platform_state_recent_bytes(&recent_bytes, transaction)
                .map_err(Error::Drive)?;
        }

        // We need to persist new protocol version as well be able to read block state
        self.drive
            .store_current_protocol_version(platform_version.protocol_version, transaction)?;

        Ok(())
    }
}
