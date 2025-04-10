use crate::drive::document::query::QueryDocumentsOutcomeV0Methods;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::query::{DriveDocumentQuery, WhereClause, WhereOperator};
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::document::DocumentV0Getters;
use dpp::fee::fee_result::FeeResult;
use dpp::identifier::Identifier;
use dpp::platform_value::Value;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::HashMap;

impl Drive {
    /// Adds operations for updating keywords and returns the fee result.
    pub(super) fn update_contract_keywords_v1(
        &self,
        contract_id: Identifier,
        owner_id: Identifier,
        keywords: &[String],
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.update_contract_keywords_add_to_operations_v1(
            contract_id,
            owner_id,
            keywords,
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

    /// Adds keyword update operations to drive operations
    pub(super) fn update_contract_keywords_add_to_operations_v1(
        &self,
        contract_id: Identifier,
        owner_id: Identifier,
        keywords: &[String],
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

        let batch_operations = self.update_contract_keywords_operations(
            contract_id,
            owner_id,
            keywords,
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

    /// The operations needed to update the keywords search contract documents
    /// First we delete the existing keywords then we add the new ones
    pub(super) fn update_contract_keywords_operations_v1(
        &self,
        contract_id: Identifier,
        owner_id: Identifier,
        keywords: &[String],
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut operations: Vec<LowLevelDriveOperation> = vec![];

        // First delete the existing keywords
        let contract = self.cache.system_data_contracts.load_search().clone();
        let document_type = contract.document_type_for_name("contractKeywords")?;

        let mut query = DriveDocumentQuery::all_items_query(&contract, document_type, None);
        query.internal_clauses.equal_clauses.insert(
            "contractId".to_string(),
            WhereClause {
                field: "contractId".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Identifier(contract_id.to_buffer()),
            },
        );

        let query_outcome = self.query_documents(
            query,
            Some(&block_info.epoch),
            false,
            transaction,
            Some(platform_version.protocol_version),
        )?;

        let existing_documents = query_outcome.documents();
        for document in existing_documents {
            operations.extend(self.delete_document_for_contract_operations(
                document.id(),
                &contract,
                document_type,
                None,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            )?);
        }

        // Then add the new ones
        operations.extend(self.add_new_contract_keywords_operations_v1(
            contract_id,
            owner_id,
            keywords,
            block_info,
            estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )?);

        Ok(operations)
    }
}
