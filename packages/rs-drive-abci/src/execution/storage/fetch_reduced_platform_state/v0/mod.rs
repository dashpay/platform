use crate::error::Error;
use crate::platform_types::platform::Platform;
use dpp::reduced_platform_state::ReducedPlatformState;
use dpp::serialization::PlatformDeserializableFromVersionedStructure;
use dpp::version::PlatformVersion;
use drive::query::TransactionArg;

impl<C> Platform<C> {
    pub(super) fn fetch_reduced_platform_state_v0(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ReducedPlatformState>, Error> {
        self.drive
            .fetch_reduced_platform_state_bytes(transaction, platform_version)
            .map_err(Error::Drive)?
            .map(|bytes| {
                ReducedPlatformState::versioned_deserialize(&bytes, platform_version)
                    .map_err(Error::Protocol)
            })
            .transpose()
    }
}
