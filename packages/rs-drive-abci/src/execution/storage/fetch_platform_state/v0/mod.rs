use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::recent::PlatformStateRecent;
use crate::platform_types::platform_state::PlatformState;
use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0Getters;
use dpp::serialization::PlatformDeserializableFromVersionedStructure;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use drive::drive::Drive;
use drive::query::TransactionArg;

impl<C> Platform<C> {
    pub(super) fn fetch_platform_state_v0(
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<PlatformState>, Error> {
        let Some(bytes) = drive
            .fetch_platform_state_bytes(transaction, platform_version)
            .map_err(Error::Drive)?
        else {
            return Ok(None);
        };

        let mut state = PlatformState::versioned_deserialize(&bytes, platform_version)
            .inspect_err(|_| {
                tracing::error!(
                    bytes = hex::encode(&bytes),
                    "Unable deserialize platform state for version {}",
                    platform_version.protocol_version
                );
            })
            .map_err(Error::Protocol)?;

        // The full record is only rewritten when a heavy field changes, so a
        // newer small record holds the block info and quorum hashes for the
        // blocks since. An older one (or none, on a database written before this
        // existed) is ignored: the full record already has those fields.
        if let Some(recent_bytes) = drive
            .fetch_platform_state_recent_bytes(transaction)
            .map_err(Error::Drive)?
        {
            let (recent, _): (PlatformStateRecent, _) = bincode::decode_from_slice(
                &recent_bytes,
                bincode::config::standard()
                    .with_big_endian()
                    .with_no_limit(),
            )
            .map_err(|e| {
                Error::Protocol(ProtocolError::PlatformDeserializationError(format!(
                    "unable to deserialize recent platform state: {e}"
                )))
            })?;

            if recent.height()
                >= state
                    .last_committed_block_info
                    .as_ref()
                    .map(|i| i.basic_info().height)
            {
                recent.apply_to(&mut state);
            }
        }

        Ok(Some(state))
    }
}
