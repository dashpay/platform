use crate::error::Error;
use crate::platform_types::platform::Platform;
use dpp::reduced_platform_state::ReducedPlatformState;
use dpp::serialization::PlatformDeserializableTrusted;
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
            // Trusted decode: these bytes are always read back from this node's grovedb.
            // After a state sync restore they arrived from a peer, but they are read only
            // once the restored tree's root hash matched the quorum-signed app hash.
            .map(|bytes| {
                ReducedPlatformState::deserialize_from_bytes_trusted(&bytes)
                    .map_err(Error::Protocol)
            })
            .transpose()
    }
}
