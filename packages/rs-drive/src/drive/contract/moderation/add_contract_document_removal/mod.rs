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
    /// The operations recording that a moderator deleted a document: the record under the
    /// document's type, keyed by the document's id. `replaces_existing` says a record of that
    /// id is already there (its author created the id again and it is removed again), as read
    /// when the transition was validated.
    #[allow(clippy::too_many_arguments)]
    pub fn add_contract_document_removal_operations(
        &self,
        contract_id: Identifier,
        document_type_name: &str,
        document_id: Identifier,
        removal: &ContractDocumentRemoval,
        replaces_existing: bool,
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
                replaces_existing,
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
