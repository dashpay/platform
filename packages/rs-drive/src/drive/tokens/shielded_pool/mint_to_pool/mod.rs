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
    /// Mints `amount` of the token straight into its shielded pool: the total supply and the pool
    /// balance both grow by `amount` and the bundle's output notes are appended. No identity
    /// balance is touched.
    #[allow(clippy::too_many_arguments)]
    pub fn token_mint_to_pool_operations(
        &self,
        token_id: [u8; 32],
        amount: TokenAmount,
        allow_first_mint: bool,
        notes: &[ShieldedActionNote],
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version.drive.methods.token.update.mint_to_pool {
            0 => self.token_mint_to_pool_operations_v0(
                token_id,
                amount,
                allow_first_mint,
                notes,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "token_mint_to_pool_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
