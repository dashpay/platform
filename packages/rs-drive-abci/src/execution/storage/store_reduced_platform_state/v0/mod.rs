use crate::error::serialization::SerializationError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use dpp::reduced_platform_state::ReducedPlatformStateForSaving;
use dpp::serialization::{PlatformSerializable, ReducedPlatformSerializable};
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
        let state_bytes =
            bincode::encode_to_vec(state, bincode::config::standard()).map_err(|e| {
                Error::Serialization(SerializationError::CorruptedSerialization(e.to_string()))
            })?;

        self.drive
            .store_reduced_platform_state_bytes(&state_bytes, transaction, platform_version)
            .map_err(Error::Drive)?;
        Ok(())
    }
}
