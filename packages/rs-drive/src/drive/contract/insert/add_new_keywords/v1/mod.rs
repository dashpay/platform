use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::object_size_info::DocumentInfo::DocumentOwnedInfo;
use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::document::{Document, DocumentV0};
use dpp::fee::fee_result::FeeResult;
use dpp::identifier::Identifier;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::{BTreeMap, HashMap};

impl Drive {
    /// Adds a keyword by inserting a new keyword subtree structure to the `Identities` subtree.
    pub(super) fn add_new_contract_keywords_v1(
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
        self.add_new_contract_keywords_add_to_operations_v1(
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

    /// Adds keyword creation operations to drive operations
    pub(super) fn add_new_contract_keywords_add_to_operations_v1(
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

        let batch_operations = self.add_new_contract_keywords_operations(
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

    /// The operations needed to create a keyword
    pub(crate) fn add_new_contract_keywords_operations_v1(
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

        let contract = self.cache.system_data_contracts.load_search();
        let document_type = contract.document_type_for_name("contractKeywords")?;

        for (i, keyword) in keywords.iter().enumerate() {
            let document = self.build_contract_keyword_document_owned_v1(
                contract_id,
                owner_id,
                keyword,
                i as u64,
                block_info,
            )?;

            let ops = self.add_document_for_contract_operations(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentOwnedInfo((document, None)),
                        owner_id: Some(owner_id.to_buffer()),
                    },
                    contract: &contract,
                    document_type,
                },
                true,
                block_info,
                &mut None,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            )?;

            operations.extend(ops);
        }

        Ok(operations)
    }

    pub(super) fn build_contract_keyword_document_owned_v1(
        &self,
        contract_id: Identifier,
        owner_id: Identifier,
        keyword: &String,
        keyword_index: u64,
        block_info: &BlockInfo,
    ) -> Result<Document, Error> {
        let document_id = Document::generate_document_id_v0(
            &contract_id,
            &owner_id,
            "contractKeywords",
            keyword_index.to_be_bytes().as_slice(),
        );

        let properties = BTreeMap::from([
            ("keyword".to_string(), keyword.into()),
            ("contractId".to_string(), contract_id.into()),
        ]);

        let document: Document = DocumentV0 {
            id: document_id,
            owner_id,
            properties,
            revision: None,
            created_at: Some(block_info.time_ms),
            updated_at: None,
            transferred_at: None,
            created_at_block_height: Some(block_info.height),
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
        }
        .into();

        Ok(document)
    }
}
