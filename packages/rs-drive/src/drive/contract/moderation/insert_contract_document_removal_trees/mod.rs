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
    /// The operations creating the trees that hold the records of the documents a contract's
    /// moderators deleted: the tree of all of them (`[64, id, 2, 16]`) when `with_root` is set,
    /// and under it one tree per document type of `document_type_names`.
    ///
    /// A contract insertion calls it with the root and every document type that sets
    /// `canBeDeletedByModerators`, when there is one. A contract update calls it for the
    /// document types it adds that set the keyword, with the root when they are the
    /// contract's first: an existing document type never changes the keyword, so whether the
    /// root exists is read off the stored contract, and a type's tree is created exactly
    /// once, with the type. A contract without such a document type has no root, so its other
    /// tree keeps the shape it would have had. No tree is made lazily by the first removal.
    #[allow(clippy::too_many_arguments)]
    pub fn insert_contract_document_removal_trees_operations(
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
            .insert_contract_document_removal_trees
        {
            0 => self.insert_contract_document_removal_trees_operations_v0(
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
                method: "insert_contract_document_removal_trees_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
