use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::object_size_info::DocumentInfo::DocumentOwnedInfo;
use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contracts::keyword_search_contract;
use dpp::document::{Document, DocumentV0};
use dpp::fee::fee_result::FeeResult;
use dpp::identifier::Identifier;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::{BTreeMap, HashMap};

impl Drive {
    /// Creates the documents in the Keyword Search contract for the contract keywords and
    /// returns the fee result
    pub(super) fn add_new_contract_keywords_v0(
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
        self.add_new_contract_keywords_add_to_operations_v0(
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

    /// Creates and applies the LowLeveLDriveOperations needed to create
    /// the documents in the Keyword Search contract for the contract keywords
    pub(super) fn add_new_contract_keywords_add_to_operations_v0(
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

    /// Creates and returns the LowLeveLDriveOperations needed to create
    /// the documents in the Keyword Search contract for the contract keywords
    pub(crate) fn add_new_contract_keywords_operations_v0(
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
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];

        let contract = self.cache.system_data_contracts.load_keyword_search();
        let document_type = contract.document_type_for_name("contractKeywords")?;

        for keyword in keywords.iter() {
            let document = self.build_contract_keyword_document_owned_v0(
                contract_id,
                owner_id,
                keyword, // since keywords are unique in the contract, we can use it as entropy
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
                &mut Some(&mut drive_operations),
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            )?;

            drive_operations.extend(ops);
        }

        Ok(drive_operations)
    }

    /// Creates and returns a `contractKeyword` document for the Keyword Search contract
    pub(super) fn build_contract_keyword_document_owned_v0(
        &self,
        contract_id: Identifier,
        owner_id: Identifier,
        keyword: &String,
    ) -> Result<Document, Error> {
        let mut entropy = Vec::with_capacity(contract_id.len() + keyword.len());
        entropy.extend_from_slice(contract_id.as_slice());
        entropy.extend_from_slice(keyword.as_bytes());

        let document_id = Document::generate_document_id_v0(
            &keyword_search_contract::ID_BYTES.into(),
            &owner_id,
            "contractKeywords",
            entropy.as_slice(),
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
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            creator_id: None,
        }
        .into();

        Ok(document)
    }
}
