mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Creates the two sum trees that hold every contract's fee pots, `[40, 64]` for the owner
    /// pots and `[40, 192]` for the moderators pots, when they are not there yet.
    ///
    /// Both the genesis state structure (version 4) and the upgrade to protocol version 14
    /// call this, so a chain born at version 14 and a chain upgraded to it hold the same
    /// subtrees.
    ///
    /// # Parameters
    ///
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(())` once both trees exist.
    /// * `Err(Error)` when the version is unknown or a write fails.
    pub fn insert_contract_fee_pot_trees(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .contract
            .fee_pots
            .insert_contract_fee_pot_trees
        {
            0 => self.insert_contract_fee_pot_trees_v0(transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "insert_contract_fee_pot_trees".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
