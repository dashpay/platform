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
    /// Destroys `amount` of the token held in its shielded pool: the spent nullifiers are
    /// recorded, the change notes appended, and the pool balance and total supply both shrink by
    /// `amount`. No identity balance is touched.
    #[allow(clippy::too_many_arguments)]
    pub fn token_burn_from_pool_operations(
        &self,
        token_id: [u8; 32],
        amount: TokenAmount,
        nullifiers: &[[u8; 32]],
        notes: &[ShieldedActionNote],
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version.drive.methods.token.update.burn_from_pool {
            0 => self.token_burn_from_pool_operations_v0(
                token_id,
                amount,
                nullifiers,
                notes,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "token_burn_from_pool_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
