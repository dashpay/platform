use crate::abci::AbciError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use bincode::config;
use dpp::reduced_platform_state::v0::ReducedPlatformStateForSavingV0;
use dpp::reduced_platform_state::ReducedPlatformStateForSaving;
use dpp::serialization::{
    PlatformDeserializableFromVersionedStructure, ReducedPlatformDeserializable,
};
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use drive::drive::Drive;
use drive::query::TransactionArg;

impl<C> Platform<C> {
    pub(super) fn fetch_reduced_platform_state_v0(
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ReducedPlatformStateForSaving>, Error> {
        drive
            .fetch_reduced_platform_state_bytes(transaction, platform_version)
            .map_err(Error::Drive)?
            .map(|bytes| {
                let result =
                    ReducedPlatformStateForSaving::versioned_deserialize(&bytes, platform_version)
                        .map_err(Error::Protocol)
                        .inspect(|reduced_platform_state| {
                            tracing::trace!(
                                bytes = hex::encode(&bytes),
                                reduced_platform_state = ?reduced_platform_state,
                                "state_sync: reduced platform state deserialized successfully for version {}",
                                platform_version.protocol_version
                            );
                        });

                if result.is_err() {
                    tracing::error!(
                        bytes = hex::encode(&bytes),
                        "Unable deserialize reduced platform state for version {}",
                        platform_version.protocol_version
                    );
                }

                result
            })
            .transpose()
    }
}
