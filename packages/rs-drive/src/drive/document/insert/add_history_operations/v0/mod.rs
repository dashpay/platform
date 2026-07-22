use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::object_size_info::DocumentInfo::DocumentOwnedInfo;
use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use dpp::block::block_info::BlockInfo;
use dpp::document::document_event::DocumentEvent;
use dpp::identifier::Identifier;
use dpp::prelude::IdentityNonce;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::HashMap;

impl Drive {
    /// Adds a document event to the document history system contract
    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_document_history_operations_v0(
        &self,
        source_data_contract_id: Identifier,
        source_document_type_name: &str,
        source_document_id: Identifier,
        owner_id: Identifier,
        owner_nonce: IdentityNonce,
        event: DocumentEvent,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let contract = self.cache.system_data_contracts.load_document_history();

        let document_type = event.associated_document_type(&contract)?;

        let document = event.build_historical_document_owned(
            source_data_contract_id,
            source_document_type_name,
            source_document_id,
            owner_id,
            owner_nonce,
            block_info,
        );

        let operations = self.add_document_for_contract_operations(
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

        Ok(operations)
    }
}
