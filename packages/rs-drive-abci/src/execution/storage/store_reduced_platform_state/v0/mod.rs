use crate::error::Error;
use crate::platform_types::platform::Platform;
use dpp::reduced_platform_state::ReducedPlatformStateForSaving;
use dpp::serialization::PlatformSerializable;
use dpp::version::PlatformVersion;
use drive::query::TransactionArg;

impl<C> Platform<C> {
    pub(super) fn store_reduced_platform_state_v0(
        &self,
        state: &ReducedPlatformStateForSaving,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // TODO: refactor to platform serialization

        let state_bytes = state.serialize_to_bytes()?;

        tracing::trace!(
            reduced_state=?state,
            len = state_bytes.len(),
            "state_sync: storing reduced platform state"
        );

        self.drive
            .store_reduced_platform_state_bytes(&state_bytes, transaction, platform_version)
            .map_err(Error::Drive)?;
        Ok(())
    }
}
