use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::object_size_info::DocumentInfo::DocumentOwnedInfo;
use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
use dpp::prelude::IdentityNonce;
use dpp::tokens::token_event::TokenEvent;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::HashMap;

impl Drive {
    /// Adds token transaction history
    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_token_transaction_history_operations_v0(
        &self,
        token_id: Identifier,
        owner_id: Identifier,
        owner_nonce: IdentityNonce,
        event: TokenEvent,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let contract = self.cache.system_data_contracts.load_token_history();

        let document_type = event.associated_document_type(&contract)?;

        let document = event.build_historical_document_owned(
            token_id,
            owner_id,
            owner_nonce,
            block_info,
            platform_version,
        )?;

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
