mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::config::moderation::ContractDocumentRemoval;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// The operations writing the record of a moderator's deletion of a document: the record
    /// under the document's type, keyed by the document's id. A fresh record, or with
    /// `replaced_record_size` the replacement of the one the document has: a restore marks the
    /// record restored, and the deletion of a restored document writes a fresh record in its
    /// place. `moderator_id` pays for the record, or for the bytes a replacement adds.
    ///
    /// # Parameters
    ///
    /// * `contract_id`: The contract the document belonged to.
    /// * `document_type_name`: The document's type.
    /// * `document_id`: The deleted document's id, which keys the record.
    /// * `removal`: The record: owner, moderator, time, document hash, restoration and reason.
    /// * `replaced_record_size`: The stored size of the record the document already has, which
    ///   this one replaces; `None` for a fresh record.
    /// * `estimated_kept_fields_size`: What the type's records are estimated to keep
    ///   (`types::estimated_document_removal_kept_fields_size`), which sizes the records a
    ///   write walks past when estimating.
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
    pub fn add_contract_document_removal_operations(
        &self,
        contract_id: Identifier,
        document_type_name: &str,
        document_id: Identifier,
        removal: &ContractDocumentRemoval,
        replaced_record_size: Option<u32>,
        estimated_kept_fields_size: u32,
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
            .add_contract_document_removal
        {
            0 => self.add_contract_document_removal_operations_v0(
                contract_id,
                document_type_name,
                document_id,
                removal,
                replaced_record_size,
                estimated_kept_fields_size,
                moderator_id,
                block_info,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_contract_document_removal_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
