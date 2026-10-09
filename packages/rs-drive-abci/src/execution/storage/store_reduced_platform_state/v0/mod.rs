use crate::error::Error;
use crate::platform_types::platform::Platform;
use dpp::reduced_platform_state::ReducedPlatformState;
use dpp::serialization::PlatformSerializable;
use dpp::version::PlatformVersion;
use drive::query::TransactionArg;

impl<C> Platform<C> {
    pub(super) fn store_reduced_platform_state_v0(
        &self,
        state: &ReducedPlatformState,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        self.drive
            .store_reduced_platform_state_bytes(
                &state.serialize_to_bytes()?,
                transaction,
                platform_version,
            )
            .map_err(Error::Drive)
    }
}
