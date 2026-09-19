mod v0;

use crate::drive::tokens::paths::{
    token_distributions_root_path, TOKEN_ONCE_PER_IDENTITY_DISTRIBUTIONS_KEY,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Adds the once-per-identity distribution claims subtree of a token, under the root
    /// once-per-identity distributions tree, using the appropriate versioned method.
    ///
    /// Called when a contract registers a token whose distribution rules carry a
    /// once-per-identity distribution. The subtree starts empty; every claim inserts one item
    /// keyed by the claimant's identity id.
    pub fn add_once_per_identity_distribution(
        &self,
        token_id: [u8; 32],
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .token
            .distribution
            .add_once_per_identity_distribution
        {
            0 => self.add_once_per_identity_distribution_v0(
                token_id,
                estimated_costs_only_with_layer_info,
                batch_operations,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_once_per_identity_distribution".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Inserts the root once-per-identity distributions tree under the token distributions tree
    /// if it does not exist yet.
    ///
    /// Both the genesis state structure (version 4) and the upgrade to protocol version 14 call
    /// this, so a chain born at version 14 and a chain upgraded to it hold the same subtree.
    pub fn insert_once_per_identity_distributions_root_tree(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let path = token_distributions_root_path();
        self.grove_insert_if_not_exists(
            (&path).into(),
            &[TOKEN_ONCE_PER_IDENTITY_DISTRIBUTIONS_KEY],
            Element::empty_tree(),
            transaction,
            None,
            &platform_version.drive,
        )?;
        Ok(())
    }
}
