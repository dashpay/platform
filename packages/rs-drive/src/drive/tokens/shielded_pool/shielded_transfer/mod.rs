mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::state_transition_action::shielded::ShieldedActionNote;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Applies a transfer inside the token's shielded pool: the spent nullifiers are recorded and the bundle's output notes appended; the pool balance is unchanged.
    #[allow(clippy::too_many_arguments)]
    pub fn token_shielded_transfer_operations(
        &self,
        token_id: [u8; 32],
        nullifiers: &[[u8; 32]],
        notes: &[ShieldedActionNote],
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version
            .drive
            .methods
            .token
            .update
            .shielded_transfer
        {
            0 => self.token_shielded_transfer_operations_v0(
                token_id,
                nullifiers,
                notes,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "token_shielded_transfer_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
