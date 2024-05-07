use dpp::block::block_info::BlockInfo;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::Index;
use dpp::fee::Credits;
use dpp::platform_value::Identifier;
use std::collections::BTreeMap;
use std::sync::Arc;

use dpp::ProtocolError;
use dpp::state_transition::documents_batch_transition::document_create_transition::v0::DocumentCreateTransitionV0;
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::{DocumentBaseTransitionAction, DocumentBaseTransitionActionAccessorsV0};
use crate::state_transition_action::document::documents_batch::document_transition::document_create_transition_action::DocumentCreateTransitionActionV0;

impl DocumentCreateTransitionActionV0 {
    /// try from document create transition with contract lookup
    pub fn try_from_document_create_transition_with_contract_lookup(
        value: DocumentCreateTransitionV0,
        block_info: &BlockInfo,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
    ) -> Result<Self, ProtocolError> {
        let DocumentCreateTransitionV0 {
            base,
            data,
            prefunded_voting_balances,
            ..
        } = value;
        let base = DocumentBaseTransitionAction::from_base_transition_with_contract_lookup(
            base,
            get_data_contract,
        )?;

        let document_type = base.document_type()?;

        let document_type_indexes = document_type.indexes();

        let prefunded_voting_balances_by_index = prefunded_voting_balances
            .into_iter()
            .map(|(index_name, credits)| {
                let index = document_type_indexes.get(&index_name).ok_or(
                    ProtocolError::UnknownContestedIndexResolution(format!(
                        "index {} not found on document type {}",
                        index_name,
                        document_type.name()
                    )),
                )?;
                Ok((index.clone(), credits))
            })
            .collect::<Result<BTreeMap<Index, Credits>, ProtocolError>>()?;

        Ok(DocumentCreateTransitionActionV0 {
            base,
            block_info: *block_info,
            data,
            prefunded_voting_balances: prefunded_voting_balances_by_index,
        })
    }

    /// try from borrowed document create transition with contract lookup
    pub fn try_from_borrowed_document_create_transition_with_contract_lookup(
        value: &DocumentCreateTransitionV0,
        block_info: &BlockInfo,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
    ) -> Result<Self, ProtocolError> {
        let DocumentCreateTransitionV0 {
            base,
            data,
            prefunded_voting_balances,
            ..
        } = value;
        let base =
            DocumentBaseTransitionAction::from_borrowed_base_transition_with_contract_lookup(
                base,
                get_data_contract,
            )?;

        let document_type = base.document_type()?;

        let document_type_indexes = document_type.indexes();

        let prefunded_voting_balances_by_index = prefunded_voting_balances
            .iter()
            .map(|(index_name, credits)| {
                let index = document_type_indexes.get(index_name).ok_or(
                    ProtocolError::UnknownContestedIndexResolution(format!(
                        "index {} not found on document type {}",
                        index_name,
                        document_type.name()
                    )),
                )?;
                Ok((index.clone(), *credits))
            })
            .collect::<Result<BTreeMap<Index, Credits>, ProtocolError>>()?;

        Ok(DocumentCreateTransitionActionV0 {
            base,
            block_info: *block_info,
            //todo: get rid of clone
            data: data.clone(),
            prefunded_voting_balances: prefunded_voting_balances_by_index,
        })
    }
}
