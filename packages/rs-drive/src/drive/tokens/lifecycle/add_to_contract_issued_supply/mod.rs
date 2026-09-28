mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::balances::credits::TokenAmount;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

/// The direction a supply write moves a token's issuer rollup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssuedSupplyChange {
    /// Supply was created: mint, mint to many, claim, direct purchase.
    Increase(TokenAmount),
    /// Supply was removed: burn, destroy frozen funds.
    Decrease(TokenAmount),
}

impl Drive {
    /// The operations that move a token's issuer rollup by the amount a supply write applies.
    ///
    /// Reads the token's contract info to find the issuer, then the issuer's lifecycle record,
    /// and replaces the record with the rollup moved. A destroyed issuer is refused as
    /// corrupted state: every path that changes supply is closed by validation before it
    /// reaches Drive, so reaching a wiped record here means a validator missed the check.
    ///
    /// A batch lowers every operation before applying any, so two supply writes for tokens
    /// of one issuer in the same batch would both read the stored record and emit two
    /// replacements of the same key. When the caller hands in the batch accumulated so far,
    /// a replacement of the issuer's record already pending in it is taken as the base and
    /// rewritten in place, so one replacement per issuer leaves the batch whatever the number
    /// of its tokens written.
    ///
    /// # Parameters
    ///
    /// * `token_id` - The token whose supply changed.
    /// * `change` - The amount and direction of the change, as actually applied to the
    ///   supply leaf.
    /// * `previous_batch_operations` - The operations accumulated by the batch so far, where
    ///   a pending replacement of the issuer's record is folded into; `None` outside a batch.
    /// * `estimated_costs_only_with_layer_info` - `Some` to price the write without state.
    /// * `transaction` - The current transaction.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * The batch operations, or `Err(DriveError::VersionNotActive)` on a platform version
    ///   without the ledger.
    #[allow(clippy::too_many_arguments)]
    pub fn add_to_contract_issued_supply_operations(
        &self,
        token_id: [u8; 32],
        change: IssuedSupplyChange,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
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
            .lifecycle
            .add_to_contract_issued_supply
        {
            Some(0) => self.add_to_contract_issued_supply_operations_v0(
                token_id,
                change,
                previous_batch_operations,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "add_to_contract_issued_supply_operations".to_string(),
                known_versions: vec![0],
            })),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_to_contract_issued_supply_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
