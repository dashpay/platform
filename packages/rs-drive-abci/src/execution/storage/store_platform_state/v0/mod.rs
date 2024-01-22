use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use dpp::serialization::PlatformSerializable;
use drive::query::TransactionArg;
use platform_version::version::PlatformVersion;

impl<C> Platform<C> {
    pub(super) fn store_platform_state_v0(
        &self,
        state: &PlatformState,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        self.drive
            .store_platform_state_bytes(
                state.versioned_serialize()?,
                transaction,
                platform_version,
            )
            .map_err(Error::Drive)
    }
}
