mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::config::moderation::ContractSettledDeletion;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// The operations writing the approvals a seated moderation team gave the deletion of a
    /// settled document: the record under the document's type, keyed by the document's id. A
    /// fresh record, or with `replaces_existing` the replacement of the one the document has:
    /// every approval rewrites it, and the first approval after the record closed (the document
    /// deleted, replaced since, or the approvals lapsed) starts a fresh one in its place.
    /// `moderator_id`, the approval's signer, pays for the record, or for the bytes a
    /// replacement adds.
    ///
    /// # Parameters
    ///
    /// * `contract_id`: The contract the document is of.
    /// * `document_type_name`: The document's type.
    /// * `document_id`: The document's id, which keys the record.
    /// * `settled_deletion`: The record: the approvals, their reason and times, and the deletion.
    /// * `replaces_existing`: Whether the document already has a record this one replaces.
    /// * `moderator_id`: The moderator that pays for the record, named in its storage flags.
    /// * `block_info`: The block being executed; its epoch goes in the storage flags.
    /// * `estimated_costs_only_with_layer_info`: The estimation map, when only estimating costs.
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<LowLevelDriveOperation>)` with the insert (or, outside estimation, the
    ///   replace) of the record.
    /// * `Err(Error)` when the method version is unknown or building an operation fails.
    #[allow(clippy::too_many_arguments)]
    pub fn add_contract_settled_deletion_operations(
        &self,
        contract_id: Identifier,
        document_type_name: &str,
        document_id: Identifier,
        settled_deletion: &ContractSettledDeletion,
        replaces_existing: bool,
        moderator_id: Identifier,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version
            .drive
            .methods
            .contract
            .moderation
            .add_contract_settled_deletion
        {
            0 => self.add_contract_settled_deletion_operations_v0(
                contract_id,
                document_type_name,
                document_id,
                settled_deletion,
                replaces_existing,
                moderator_id,
                block_info,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_contract_settled_deletion_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
