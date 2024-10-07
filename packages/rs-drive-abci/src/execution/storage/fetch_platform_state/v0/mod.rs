use crate::error::Error;
use crate::platform_types::platform::Platform;
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
                    tracing::trace!(
                        bytes = hex::encode(&bytes),
                        "Unable deserialize platform state for version {}",
                        platform_version.protocol_version
                    );
                }

                result
            })
            .transpose()
    }
}
