//! Structure 1 fetch: the record together with its masternode and validator set entries.

use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::platform_state_for_saving::PlatformStateForSaving;
use crate::platform_types::platform_state::PlatformState;
use dpp::bincode::config;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use drive::drive::platform_state::PlatformStateEntryKind;
use drive::drive::Drive;
use drive::query::TransactionArg;

impl<C> Platform<C> {
    pub(super) fn fetch_platform_state_v1(
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

        let record: PlatformStateForSaving = bincode::decode_from_slice(
            &bytes,
            config::standard().with_big_endian().with_no_limit(),
        )
        .map(|(record, _)| record)
        .map_err(|e| {
            tracing::error!(
                bytes = hex::encode(&bytes),
                "Unable deserialize platform state for version {}",
                platform_version.protocol_version
            );
            Error::Protocol(ProtocolError::PlatformDeserializationError(format!(
                "unable to deserialize PlatformStateForSaving: {e}"
            )))
        })?;

        let PlatformStateForSaving::V2(record) = record else {
            // An earlier structure carries the whole state itself, possibly with
            // a newer small record beside it: the structure 0 fetch knows both.
            return Self::fetch_platform_state_v0(drive, transaction, platform_version);
        };

        let masternode_entries = drive
            .fetch_platform_state_entries_bytes(
                PlatformStateEntryKind::Masternodes,
                transaction,
                platform_version,
            )
            .map_err(Error::Drive)?;
        let validator_set_entries = drive
            .fetch_platform_state_entries_bytes(
                PlatformStateEntryKind::ValidatorSets,
                transaction,
                platform_version,
            )
            .map_err(Error::Drive)?;

        record
            .into_platform_state(masternode_entries, validator_set_entries)
            .map(Some)
    }
}
