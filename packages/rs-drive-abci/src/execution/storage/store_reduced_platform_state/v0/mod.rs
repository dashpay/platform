use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use dpp::serialization::{PlatformSerializable, ReducedPlatformSerializable};
use dpp::version::PlatformVersion;
use drive::query::TransactionArg;

impl<C> Platform<C> {
    pub(super) fn store_reduced_platform_state_v0(
        &self,
        state: &PlatformState,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        self.drive
            .store_reduced_platform_state_bytes(
                &state.reduced_serialize_to_bytes()?,
                transaction,
                platform_version,
            )
            .map_err(Error::Drive)?;
        Ok(())
    }
}
