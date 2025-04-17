use crate::drive::document::query::QueryDocumentsOutcomeV0Methods;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::query::{DriveDocumentQuery, WhereClause, WhereOperator};
use crate::util::object_size_info::{DocumentAndContractInfo, DocumentInfo, OwnedDocumentInfo};
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::document::DocumentV0Setters;
use dpp::fee::fee_result::FeeResult;
use dpp::identifier::Identifier;
use dpp::platform_value::Value;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::HashMap;

impl Drive {
    /// Updates the documents in the Keyword Search contract for the contract
    /// update description and returns the fee result
    #[allow(clippy::too_many_arguments)]
    pub(super) fn update_contract_description_v0(
        &self,
        contract_id: Identifier,
        owner_id: Identifier,
        description: &String,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.update_contract_description_add_to_operations_v0(
            contract_id,
            owner_id,
            description,
            block_info,
            apply,
            transaction,
            &mut drive_operations,
            platform_version,
        )?;
        let fees = Drive::calculate_fee(
            None,
            Some(drive_operations),
            &block_info.epoch,
            self.config.epochs_per_era,
            platform_version,
            None,
        )?;
        Ok(fees)
    }

    /// Creates and applies the LowLeveLDriveOperations needed to update
    /// the documents in the Keyword Search contract for the contract description
    #[allow(clippy::too_many_arguments)]
    pub(super) fn update_contract_description_add_to_operations_v0(
        &self,
        contract_id: Identifier,
        owner_id: Identifier,
        description: &String,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };

        let batch_operations = self.update_contract_description_operations(
            contract_id,
            owner_id,
            description,
            block_info,
            &mut estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )?;

        self.apply_batch_low_level_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            batch_operations,
            drive_operations,
            &platform_version.drive,
        )
    }

    /// Creates and returns the LowLeveLDriveOperations needed to update
    /// the documents in the Keyword Search contract for the contract description
    #[allow(clippy::too_many_arguments)]
    pub(super) fn update_contract_description_operations_v0(
        &self,
        contract_id: Identifier,
        owner_id: Identifier,
        description: &String,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut operations: Vec<LowLevelDriveOperation> = vec![];

        let contract = self.cache.system_data_contracts.load_keyword_search();
        let document_type = contract.document_type_for_name("shortDescription")?;

        let mut query = DriveDocumentQuery::all_items_query(&contract, document_type, None);
        query.internal_clauses.equal_clauses.insert(
            "contractId".to_string(),
            WhereClause {
                field: "contractId".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Identifier(contract_id.to_buffer()),
            },
        );

        // todo: deal with cost of this operation
        let query_outcome = self.query_documents(
            query,
            Some(&block_info.epoch),
            false,
            transaction,
            Some(platform_version.protocol_version),
        )?;

        let mut existing_documents = query_outcome.documents_owned();

        if existing_documents.len() > 1 {
            return Err(Error::Drive(DriveError::CorruptedContractIndexes(
                "There should be only one `shortDescription` document per contract in the Keyword Search contract".to_string(),
            )));
        }

        if existing_documents.is_empty() {
            // Add the new one
            operations.extend(self.add_new_contract_description_operations(
                contract_id,
                owner_id,
                description,
                true,
                block_info,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            )?);
        } else {
            // Replace the existing one
            let mut new_document = existing_documents.remove(0);
            new_document.set("description", Value::Text(description.clone()));
            new_document.set_updated_at(Some(block_info.time_ms));
            new_document.bump_revision();

            let info = DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentInfo::DocumentOwnedInfo((new_document, None)),
                    owner_id: Some(owner_id.to_buffer()),
                },
                contract: &contract.clone(),
                document_type,
            };

            operations.extend(self.update_document_for_contract_operations(
                info,
                block_info,
                &mut None,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            )?);
        }

        Ok(operations)
    }
}
