use dpp::block::block_info::BlockInfo;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::fee::fee_result::FeeResult;
use dpp::platform_value::Identifier;
use grovedb::TransactionArg;
use std::sync::Arc;

use dpp::ProtocolError;
use dpp::state_transition::documents_batch_transition::document_create_transition::v0::DocumentCreateTransitionV0;
use dpp::voting::vote_info_storage::contested_document_vote_poll_stored_info::ContestedDocumentVotePollStoredInfo;
use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use platform_version::version::PlatformVersion;
use crate::drive::contract::DataContractFetchInfo;
use crate::drive::Drive;
use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::resolve::ContestedDocumentResourceVotePollResolver;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::{DocumentBaseTransitionAction, DocumentBaseTransitionActionAccessorsV0};
use crate::state_transition_action::document::documents_batch::document_transition::document_create_transition_action::DocumentCreateTransitionActionV0;

impl DocumentCreateTransitionActionV0 {
    /// try from document create transition with contract lookup
    pub fn try_from_document_create_transition_with_contract_lookup(
        drive: &Drive,
        transaction: TransactionArg,
        value: DocumentCreateTransitionV0,
        block_info: &BlockInfo,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
        platform_version: &PlatformVersion,
    ) -> Result<(Self, FeeResult), Error> {
        let DocumentCreateTransitionV0 {
            base,
            data,
            prefunded_voting_balance,
            ..
        } = value;
        let base = DocumentBaseTransitionAction::from_base_transition_with_contract_lookup(
            base,
            get_data_contract,
        )?;

        let document_type = base.document_type()?;

        let document_type_indexes = document_type.indexes();

        let prefunded_voting_balances_by_vote_poll = prefunded_voting_balance
            .map(|(index_name, credits)| {
                let index = document_type_indexes.get(&index_name).ok_or(
                    ProtocolError::UnknownContestedIndexResolution(format!(
                        "index {} not found on document type {}",
                        index_name.clone(),
                        document_type.name()
                    )),
                )?;
                let index_values = index.extract_values(&data);

                let vote_poll = ContestedDocumentResourceVotePoll {
                    contract_id: base.data_contract_id(),
                    document_type_name: base.document_type_name().clone(),
                    index_name,
                    index_values,
                };

                let resolved_vote_poll = vote_poll
                    .resolve_owned_with_provided_arc_contract_fetch_info(
                        base.data_contract_fetch_info(),
                    )?;

                Ok::<_, Error>((resolved_vote_poll, credits))
            })
            .transpose()?;

        let mut fee_result = FeeResult::default();

        let (current_store_contest_info, should_store_contest_info) =
            if let Some((contested_document_resource_vote_poll, _)) =
                &prefunded_voting_balances_by_vote_poll
            {
                let (fetch_fee_result, maybe_current_store_contest_info) = drive
                    .fetch_contested_document_vote_poll_stored_info(
                        contested_document_resource_vote_poll,
                        Some(&block_info.epoch),
                        transaction,
                        platform_version,
                    )?;

                fee_result = fetch_fee_result.ok_or(Error::Drive(
                    DriveError::CorruptedCodeExecution("expected fee result"),
                ))?;
                let should_store_contest_info = if maybe_current_store_contest_info.is_none() {
                    // We are starting a new contest
                    Some(ContestedDocumentVotePollStoredInfo::new(
                        *block_info,
                        platform_version,
                    )?)
                } else {
                    None
                };
                (maybe_current_store_contest_info, should_store_contest_info)
            } else {
                (None, None)
            };

        Ok((
            DocumentCreateTransitionActionV0 {
                base,
                block_info: *block_info,
                data,
                prefunded_voting_balance: prefunded_voting_balances_by_vote_poll,
                current_store_contest_info,
                should_store_contest_info,
            },
            fee_result,
        ))
    }

    /// try from borrowed document create transition with contract lookup
    pub fn try_from_borrowed_document_create_transition_with_contract_lookup(
        drive: &Drive,
        transaction: TransactionArg,
        value: &DocumentCreateTransitionV0,
        block_info: &BlockInfo,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
        platform_version: &PlatformVersion,
    ) -> Result<(Self, FeeResult), Error> {
        let DocumentCreateTransitionV0 {
            base,
            data,
            prefunded_voting_balance,
            ..
        } = value;
        let base =
            DocumentBaseTransitionAction::from_borrowed_base_transition_with_contract_lookup(
                base,
                get_data_contract,
            )?;

        let document_type = base.document_type()?;

        let document_type_indexes = document_type.indexes();

        let prefunded_voting_balances_by_vote_poll = prefunded_voting_balance
            .as_ref()
            .map(|(index_name, credits)| {
                let index = document_type_indexes.get(index_name).ok_or(
                    ProtocolError::UnknownContestedIndexResolution(format!(
                        "index {} not found on document type {}",
                        index_name.clone(),
                        document_type.name()
                    )),
                )?;
                let index_values = index.extract_values(&data);

                let vote_poll = ContestedDocumentResourceVotePoll {
                    contract_id: base.data_contract_id(),
                    document_type_name: base.document_type_name().clone(),
                    index_name: index_name.clone(),
                    index_values,
                };

                let resolved_vote_poll = vote_poll
                    .resolve_owned_with_provided_arc_contract_fetch_info(
                        base.data_contract_fetch_info(),
                    )?;

                Ok::<_, Error>((resolved_vote_poll, *credits))
            })
            .transpose()?;

        let mut fee_result = FeeResult::default();

        let (current_store_contest_info, should_store_contest_info) =
            if let Some((contested_document_resource_vote_poll, _)) =
                &prefunded_voting_balances_by_vote_poll
            {
                let (fetch_fee_result, maybe_current_store_contest_info) = drive
                    .fetch_contested_document_vote_poll_stored_info(
                        contested_document_resource_vote_poll,
                        Some(&block_info.epoch),
                        transaction,
                        platform_version,
                    )?;

                fee_result = fetch_fee_result.ok_or(Error::Drive(
                    DriveError::CorruptedCodeExecution("expected fee result"),
                ))?;
                let should_store_contest_info = if maybe_current_store_contest_info.is_none() {
                    // We are starting a new contest
                    Some(ContestedDocumentVotePollStoredInfo::new(
                        *block_info,
                        platform_version,
                    )?)
                } else {
                    None
                };
                (maybe_current_store_contest_info, should_store_contest_info)
            } else {
                (None, None)
            };

        Ok((
            DocumentCreateTransitionActionV0 {
                base,
                block_info: *block_info,
                data: data.clone(),
                prefunded_voting_balance: prefunded_voting_balances_by_vote_poll,
                current_store_contest_info,
                should_store_contest_info,
            },
            fee_result,
        ))
    }
}
