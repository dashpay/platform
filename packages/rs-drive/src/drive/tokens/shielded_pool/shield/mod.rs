mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::state_transition_action::shielded::ShieldedActionNote;
use dpp::balances::credits::TokenAmount;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Moves `amount` of the token from `from_identity_id`'s balance into the token's shielded pool, appending the bundle's output notes.
    #[allow(clippy::too_many_arguments)]
    pub fn token_shield_operations(
        &self,
        token_id: [u8; 32],
        from_identity_id: [u8; 32],
        amount: TokenAmount,
        notes: &[ShieldedActionNote],
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version.drive.methods.token.update.shield {
            0 => self.token_shield_operations_v0(
                token_id,
                from_identity_id,
                amount,
                notes,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "token_shield_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
