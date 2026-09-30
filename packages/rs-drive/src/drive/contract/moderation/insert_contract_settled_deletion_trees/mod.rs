mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::storage_flags::StorageFlags;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// The operations creating the trees that hold the approvals a seated moderation team gives
    /// the deletion of settled documents: the tree of all of them (`[64, id, 2, 24]`) when
    /// `with_root` is set, and under it one tree per document type of `document_type_names`.
    ///
    /// Called as [`Drive::insert_contract_document_removal_trees_operations`] is: by a contract
    /// insertion with the root and every document type that sets
    /// `moderatorAbilities.deleteSettled`, and by a contract update for the document types it
    /// adds that set it, with the root when they are the contract's first. The keyword is fixed
    /// with its type, so a type's tree is created exactly once, with the type.
    ///
    /// # Parameters
    ///
    /// * `contract_id`: The contract the trees belong to.
    /// * `with_root`: Whether to also create the tree of all the approvals.
    /// * `document_type_names`: The document types to create an approvals tree for.
    /// * `storage_flags`: The storage flags of the new trees.
    /// * `estimated_costs_only_with_layer_info`: The estimation map, when only estimating costs.
    /// * `transaction`: The GroveDB transaction.
    /// * `batch_operations`: The operations accumulator the tree inserts are appended to.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(())` once the tree inserts are appended to `batch_operations`.
    /// * `Err(Error)` when the method version is unknown or building an insert fails.
    #[allow(clippy::too_many_arguments)]
    pub fn insert_contract_settled_deletion_trees_operations(
        &self,
        contract_id: [u8; 32],
        with_root: bool,
        document_type_names: &[&str],
        storage_flags: Option<&StorageFlags>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .contract
            .moderation
            .insert_contract_settled_deletion_trees
        {
            0 => self.insert_contract_settled_deletion_trees_operations_v0(
                contract_id,
                with_root,
                document_type_names,
                storage_flags,
                estimated_costs_only_with_layer_info,
                transaction,
                batch_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "insert_contract_settled_deletion_trees_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
