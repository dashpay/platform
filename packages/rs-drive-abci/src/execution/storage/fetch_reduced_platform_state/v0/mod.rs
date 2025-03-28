use crate::error::Error;
use crate::platform_types::platform::Platform;
use dpp::reduced_platform_state::ReducedPlatformStateForSaving;
use dpp::serialization::PlatformDeserializableFromVersionedStructure;
use dpp::version::PlatformVersion;
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
                    ReducedPlatformStateForSaving::versioned_deserialize(&bytes, platform_version)
                        .inspect(|d| {
                            tracing::trace!(
                                len = bytes.len(),
                                reduced_platform_state = ?d,
                                "state_sync: reduced platform state deserialized successfully for version {}",
                                platform_version.protocol_version
                            );
                        })
                        .inspect_err(|e|
                    tracing::error!(
                        bytes = hex::encode(&bytes),
                        "Unable deserialize reduced platform state for version {}: {:?}",
                        platform_version.protocol_version,e
                    )).map_err(Error::Protocol)
                       
            })
            .transpose()
    }
}
