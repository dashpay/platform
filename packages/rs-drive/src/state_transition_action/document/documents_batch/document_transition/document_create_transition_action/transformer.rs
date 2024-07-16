use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::platform_value::Identifier;
use grovedb::TransactionArg;
use std::sync::Arc;

use dpp::ProtocolError;
use dpp::state_transition::documents_batch_transition::document_create_transition::DocumentCreateTransition;
use platform_version::version::PlatformVersion;
use crate::drive::contract::DataContractFetchInfo;
use crate::drive::Drive;
use crate::error::Error;
use crate::state_transition_action::document::documents_batch::document_transition::document_create_transition_action::{DocumentCreateTransitionAction, DocumentCreateTransitionActionV0};

impl DocumentCreateTransitionAction {
    /// from_document_create_transition_with_contract_lookup
    pub fn from_document_create_transition_with_contract_lookup(
        drive: &Drive,
        transaction: TransactionArg,
        value: DocumentCreateTransition,
        block_info: &BlockInfo,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
        platform_version: &PlatformVersion,
    ) -> Result<(Self, FeeResult), Error> {
        match value {
            DocumentCreateTransition::V0(v0) => {
                let (v0, fee) = DocumentCreateTransitionActionV0::try_from_document_create_transition_with_contract_lookup(drive, transaction, v0, block_info, get_data_contract, platform_version)?;
                Ok((v0.into(), fee))
            }
        }
    }

    /// from_document_borrowed_create_transition_with_contract_lookup
    pub fn from_document_borrowed_create_transition_with_contract_lookup(
        drive: &Drive,
        transaction: TransactionArg,
        value: &DocumentCreateTransition,
        block_info: &BlockInfo,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
        platform_version: &PlatformVersion,
    ) -> Result<(Self, FeeResult), Error> {
        match value {
            DocumentCreateTransition::V0(v0) => {
                let (v0, fee) = DocumentCreateTransitionActionV0::try_from_borrowed_document_create_transition_with_contract_lookup(drive, transaction, v0, block_info, get_data_contract, platform_version)?;
                Ok((v0.into(), fee))
            }
        }
    }
}
