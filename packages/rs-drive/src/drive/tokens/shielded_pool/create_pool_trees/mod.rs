mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Gathers the operations that create a token's shielded pool subtree: the pool SumTree
    /// under the token shielded pools root and its five children (notes, nullifiers, anchors,
    /// anchors-by-height, total balance).
    ///
    /// With `allow_already_exists` the call is idempotent (a contract update re-registering the
    /// token); without it, an existing pool is a `CorruptedDriveState` error.
    pub fn create_token_shielded_pool_trees_operations(
        &self,
        token_id: [u8; 32],
        allow_already_exists: bool,
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
            .create_token_shielded_pool_trees
        {
            0 => self.create_token_shielded_pool_trees_operations_v0(
                token_id,
                allow_already_exists,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "create_token_shielded_pool_trees_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
