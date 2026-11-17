use dpp::block::block_info::BlockInfo;
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV1Getters};
use dpp::data_contract::document_type::methods::DocumentTypeBasicMethods;
use dpp::fee::fee_result::FeeResult;
use dpp::platform_value::Identifier;
use grovedb::TransactionArg;
use std::sync::Arc;
use dpp::prelude::{ConsensusValidationResult, UserFeeIncrease};
use dpp::ProtocolError;
use dpp::decision_proposal::is_decision_proposal;
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
use crate::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::{DecisionProposalCreate, DocumentCreateTransitionActionV0};
use crate::state_transition_action::batch::batched_transition::document_transition::state_generated_properties::generate_state_read_properties;
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
                |action_fees| action_fees.document_creation_action_fee(),
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

        // Added in place at protocol version 14, inert before it: `fill_generated_properties`
        // is `None` there and leaves the data as sent. From 14 on, every `generatedFrom`
        // property the transition leaves out is generated from its params here, before the
        // contest resolution below and every later check read the data.
        let mut data = data.clone();
        document_type.fill_generated_properties(&mut data, platform_version)?;

        // Added in place at protocol version 15, inert before it, where no type declares a
        // `generatedFrom` function that reads state: every such property is written from
        // state here, or checked when sent, before the contest resolution and every later
        // check read the data. A refusal bumps the nonce, and the write pays for the reads
        // either way.
        let (refusal, mut fee_result) = generate_state_read_properties(
            drive,
            document_type,
            base.data_contract_fetch_info_ref(),
            &mut data,
            None,
            block_info,
            transaction,
            platform_version,
        )?;
        if let Some(error) = refusal {
            let bump_action =
                BumpIdentityDataContractNonceAction::from_borrowed_document_base_transition_action(
                    &base,
                    owner_id,
                    user_fee_increase,
                );
            return Ok((
                ConsensusValidationResult::new_with_data_and_errors(
                    BatchedTransitionAction::BumpIdentityDataContractNonce(bump_action),
                    vec![error],
                ),
                fee_result,
            ));
        }

        let document_type_indexes = document_type.indexes();

        // A decision proposal (protocol version 15; its contract does not exist before it)
        // states the decision proposal fund, which names its own type rather than a contested
        // index: it is kept as stated for validation to judge, and opens no contest.
        let decision_proposal =
            is_decision_proposal(&base.data_contract_id(), base.document_type_name()).then(|| {
                Box::new(DecisionProposalCreate {
                    stated_fund: prefunded_voting_balance.clone(),
                })
            });

        let mut prefunded_voting_balances_by_vote_poll = prefunded_voting_balance
            .as_ref()
            .filter(|_| decision_proposal.is_none())
            .map(|(index_name, credits)| {
                let index = document_type_indexes.get(index_name).ok_or(
                    ProtocolError::UnknownContestedIndexResolution(format!(
                        "index {} not found on document type {}",
                        index_name.clone(),
                        document_type.name()
                    )),
                )?;
                // Every contender of a contest names it with the same poll and prefunds the
                // same balance: from protocol version 15 every index value is written the one
                // way its property's type keys it, at protocol version 14 identifier values
                // alone are written one way, and before 14 the values are taken as given
                let index_values = index.extract_contested_values(
                    &data,
                    document_type.flattened_properties(),
                    platform_version,
                )?;

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

        let (current_store_contest_info, should_store_contest_info) =
            if let Some((contested_document_resource_vote_poll, _)) =
                &mut prefunded_voting_balances_by_vote_poll
            {
                let (fetch_fee_result, maybe_current_store_contest_info) = drive
                    .fetch_contested_document_vote_poll_stored_info(
                        contested_document_resource_vote_poll,
                        Some(&block_info.epoch),
                        transaction,
                        platform_version,
                    )?;

                fee_result.checked_add_assign(fetch_fee_result.ok_or(Error::Drive(
                    DriveError::CorruptedCodeExecution("expected fee result"),
                ))?)?;

                // A contest under way keeps the poll it was opened under: its prefunded balance
                // and its end date are kept under that poll. A contender joining it under a
                // later protocol version, which writes index values another way, names that
                // poll by writing its values as the version the contest was opened at did. The
                // contest is found at the path of its index keys, which every version writes
                // alike. Only values this version does not write as they were sent can be
                // written another way by an earlier one, so nothing is read for the others.
                // In place: inert before protocol version 15, where
                // `fetch_contest_opening_protocol_version` is `None` and reads nothing.
                if let Some(current_store_contest_info) = &maybe_current_store_contest_info {
                    let index = document_type_indexes
                        .get(&contested_document_resource_vote_poll.index_name)
                        .ok_or_else(|| {
                            ProtocolError::UnknownContestedIndexResolution(format!(
                                "index {} not found on document type {}",
                                contested_document_resource_vote_poll.index_name,
                                document_type.name()
                            ))
                        })?;
                    if contested_document_resource_vote_poll.index_values
                        != index.extract_values(&data)
                    {
                        let (read_fee_result, opening_protocol_version) = drive
                            .fetch_contest_opening_protocol_version(
                                current_store_contest_info,
                                &block_info.epoch,
                                platform_version,
                            )?;
                        fee_result.checked_add_assign(read_fee_result)?;
                        if let Some(opening_protocol_version) = opening_protocol_version
                            .filter(|version| *version != platform_version.protocol_version)
                        {
                            let opening_platform_version =
                                PlatformVersion::get(opening_protocol_version)
                                    .map_err(ProtocolError::from)?;
                            contested_document_resource_vote_poll.index_values = index
                                .extract_contested_values(
                                    &data,
                                    document_type.flattened_properties(),
                                    opening_platform_version,
                                )?;
                        }
                    }
                }
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
                    data,
                    prefunded_voting_balance: prefunded_voting_balances_by_vote_poll,
                    current_store_contest_info,
                    should_store_contest_info: should_store_contest_info.map(Box::new),
                    property_constraint_reads: Default::default(),
                    moderated: false,
                    consumed_documents: Vec::new(),
                    derived_index_values: None,
                    decision_proposal,
                }
                .into(),
            ))
            .into(),
            fee_result,
        ))
    }
}
