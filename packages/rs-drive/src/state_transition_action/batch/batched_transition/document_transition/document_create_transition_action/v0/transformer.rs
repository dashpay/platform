use dpp::block::block_info::BlockInfo;
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV1Getters};
use dpp::fee::fee_result::FeeResult;
use dpp::platform_value::Identifier;
use grovedb::TransactionArg;
use std::sync::Arc;
use dpp::prelude::{ConsensusValidationResult, UserFeeIncrease};
use dpp::ProtocolError;
use dpp::state_transition::batch_transition::document_create_transition::v0::DocumentCreateTransitionV0;
use dpp::voting::vote_info_storage::contested_document_vote_poll_stored_info::ContestedDocumentVotePollStoredInfo;
use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use platform_version::version::PlatformVersion;
use crate::drive::contract::DataContractFetchInfo;
use crate::drive::Drive;
use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::resolve::ContestedDocumentResourceVotePollResolver;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::batch::batched_transition::BatchedTransitionAction;
use crate::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::{DocumentBaseTransitionAction, DocumentBaseTransitionActionAccessorsV0};
use crate::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::DocumentCreateTransitionActionV0;
use crate::state_transition_action::batch::batched_transition::document_transition::DocumentTransitionAction;
use crate::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceAction;

impl DocumentCreateTransitionActionV0 {
    /// try from borrowed document create transition with contract lookup
    #[allow(clippy::too_many_arguments)]
    pub fn try_from_borrowed_document_create_transition_with_contract_lookup(
        drive: &Drive,
        owner_id: Identifier,
        transaction: TransactionArg,
        value: &DocumentCreateTransitionV0,
        block_info: &BlockInfo,
        user_fee_increase: UserFeeIncrease,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
        platform_version: &PlatformVersion,
    ) -> Result<
        (
            ConsensusValidationResult<BatchedTransitionAction>,
            FeeResult,
        ),
        Error,
    > {
        let DocumentCreateTransitionV0 {
            base,
            data,
            prefunded_voting_balance,
            ..
        } = value;
        let base_action_validation_result =
            DocumentBaseTransitionAction::try_from_borrowed_base_transition_with_contract_lookup(
                base,
                get_data_contract,
                |document_type| document_type.document_creation_token_cost(),
                "create",
            )?;

        let base = match base_action_validation_result.is_valid() {
            true => base_action_validation_result.into_data()?,
            false => {
                let bump_action =
                    BumpIdentityDataContractNonceAction::from_borrowed_document_base_transition(
                        base,
                        owner_id,
                        user_fee_increase,
                    );
                let batched_action =
                    BatchedTransitionAction::BumpIdentityDataContractNonce(bump_action);

                return Ok((
                    ConsensusValidationResult::new_with_data_and_errors(
                        batched_action,
                        base_action_validation_result.errors,
                    ),
                    FeeResult::default(),
                ));
            }
        };

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
                let index_values = index.extract_values(data);

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
            BatchedTransitionAction::DocumentAction(DocumentTransitionAction::CreateAction(
                DocumentCreateTransitionActionV0 {
                    base,
                    block_info: *block_info,
                    data: data.clone(),
                    prefunded_voting_balance: prefunded_voting_balances_by_vote_poll,
                    current_store_contest_info,
                    should_store_contest_info,
                }
                .into(),
            ))
            .into(),
            fee_result,
        ))
    }
}
