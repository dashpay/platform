use std::collections::BTreeMap;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::document::document_contest_document_with_same_id_already_present_error::DocumentContestDocumentWithSameIdAlreadyPresentError;
use dpp::consensus::state::document::document_contest_maximum_contenders_reached_error::DocumentContestMaximumContendersReachedError;
use dpp::consensus::state::document::document_contest_not_paid_for_error::DocumentContestNotPaidForError;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::ConsensusError;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::data_contract::document_type::property_constraints::DocumentSystemValues;
use dpp::identifier::Identifier;
use dpp::validation::{ConsensusValidationResult, SimpleConsensusValidationResult};
use dpp::version::PlatformVersion;
use dpp::voting::vote_polls::contested_document_resource_vote_poll::required_vote_resolution_fund_to_join;
use drive::query::TransactionArg;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::{
    DocumentCreateTransitionAction, DocumentCreateTransitionActionAccessorsV0,
};

use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::batch::action_validation::document::document_create_transition_action::state_v1::DocumentCreateTransitionActionStateValidationV1;
use crate::execution::validation::state_transition::batch::action_validation::document::document_reference_validation::{ConsumedLookupDocument, DocumentReferenceValidation};
use crate::execution::validation::state_transition::batch::state::v0::fetch_documents::has_contested_document_with_document_id;
use crate::platform_types::platform::PlatformStateRef;

pub(in crate::execution::validation::state_transition::state_transitions::batch::action_validation) trait DocumentCreateTransitionActionStateValidationV2
{
    #[allow(clippy::too_many_arguments)]
    fn validate_state_v2(
        &mut self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        consumed_documents: &mut Vec<ConsumedLookupDocument>,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl DocumentCreateTransitionActionStateValidationV2 for DocumentCreateTransitionAction {
    fn validate_state_v2(
        &mut self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        consumed_documents: &mut Vec<ConsumedLookupDocument>,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let validation_result = self.validate_state_v1(
            platform,
            owner_id,
            block_info,
            execution_context,
            transaction,
            platform_version,
        )?;
        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        // A contest accepts at most `max_contenders_per_contest` contenders, so the end of the
        // poll can tally and clean up every one in a block, and the fund a contender pays to
        // join doubles once it holds `contested_document_contenders_before_fund_doubling`
        // contenders and again for every `contested_document_contenders_per_fund_doubling` more,
        // so filling it costs far more than the fund times the contenders. v1 has let the
        // document join the contest when it exists; a new contest has no contenders to count.
        if let Some((contested_document_resource_vote_poll, most_it_pays)) =
            self.prefunded_voting_balance()
        {
            let contenders = if self.current_store_contest_info().is_some() {
                let max_contenders = platform_version.system_limits.max_contenders_per_contest;
                let (fee_result, contenders) = platform
                    .drive
                    .fetch_contested_document_vote_poll_contender_count(
                        contested_document_resource_vote_poll,
                        max_contenders,
                        &block_info.epoch,
                        transaction,
                        platform_version,
                    )?;

                execution_context
                    .add_operation(ValidationOperation::PrecalculatedOperation(fee_result));

                if contenders >= max_contenders {
                    return Ok(ConsensusValidationResult::new_with_error(
                        ConsensusError::StateError(
                            StateError::DocumentContestMaximumContendersReachedError(
                                DocumentContestMaximumContendersReachedError::new(
                                    contested_document_resource_vote_poll.into(),
                                    max_contenders,
                                ),
                            ),
                        ),
                    ));
                }
                contenders
            } else {
                0
            };

            // The contender states the most it pays and is charged the fund to join, what it
            // stated beyond that staying with it
            let fund_to_join = required_vote_resolution_fund_to_join(
                &contested_document_resource_vote_poll.contract.id(),
                &contested_document_resource_vote_poll.document_type_name,
                contenders,
                platform_version,
            );
            if *most_it_pays < fund_to_join {
                return Ok(ConsensusValidationResult::new_with_error(
                    ConsensusError::StateError(StateError::DocumentContestNotPaidForError(
                        DocumentContestNotPaidForError::new(
                            self.base().id(),
                            fund_to_join,
                            *most_it_pays,
                        ),
                    )),
                ));
            }
            self.set_prefunded_voting_fund(fund_to_join);
        }

        // The creator of a document being created is its writer; the commitments the create
        // reveals and consumes are collected for the batch to delete with it, and the values
        // of the type's derived index properties, from the documents the references fetched,
        // for Drive to key the document by without reading those documents again. The
        // document is recorded in this block, for every time and height a `where` entry
        // reads, and the values the type's rules read of the documents its references point
        // at are collected from those documents too
        let system = DocumentSystemValues::created_in_block(owner_id, &self.block_info());
        let mut derived_index_values = BTreeMap::new();
        let mut referenced_documents = BTreeMap::new();
        let reference_result = self.base().validate_document_references(
            self.data(),
            owner_id,
            Some(owner_id),
            &system,
            None,
            None,
            platform,
            block_info,
            consumed_documents,
            Some(&mut derived_index_values),
            &mut referenced_documents,
            transaction,
            execution_context,
            platform_version,
        )?;
        if !reference_result.is_valid() {
            return Ok(reference_result);
        }
        self.set_derived_index_values(derived_index_values);

        // The rules reading a document a reference points at (protocol version 14), judged
        // once the references hold, with the values their checks read: the structure
        // validation judged every other rule, and could not judge these before the reads.
        // A type without such a rule judges nothing more, and copies nothing
        let result = self
            .base()
            .document_type()?
            .validate_referencing_property_constraints(
                self.data(),
                referenced_documents,
                DocumentSystemValues {
                    aggregates: Some(self.property_constraint_aggregates().clone()),
                    consensus: true,
                    ..system
                },
                platform_version,
            )
            .map_err(Error::Protocol)?;
        if !result.is_valid() {
            return Ok(result);
        }

        // A non-contested create must not take the id of a live contested
        // document. Both derive the id from the same owner, document type and
        // entropy, and v1 probes contested storage only for contested creates,
        // so the non-contested document could occupy primary storage while the
        // contest is live and awarding the winner would then fail with
        // `CorruptedDocumentAlreadyExists`, which fails block execution.
        if self.prefunded_voting_balance().is_none() {
            let contract_fetch_info = self.base().data_contract_fetch_info_ref();
            let document_type = self.base().document_type()?;

            if document_type.find_contested_index().is_some() {
                let (contested_document_already_exists, fee_result) =
                    has_contested_document_with_document_id(
                        platform.drive,
                        &contract_fetch_info.contract,
                        document_type,
                        self.base().id(),
                        Some(&block_info.epoch),
                        transaction,
                        platform_version,
                    )?;

                execution_context
                    .add_operation(ValidationOperation::PrecalculatedOperation(fee_result));

                if contested_document_already_exists {
                    return Ok(ConsensusValidationResult::new_with_error(
                        ConsensusError::StateError(
                            StateError::DocumentContestDocumentWithSameIdAlreadyPresentError(
                                DocumentContestDocumentWithSameIdAlreadyPresentError::new(
                                    self.base().id(),
                                ),
                            ),
                        ),
                    ));
                }
            }
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform_types::platform::PlatformStateRef;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
    use dpp::data_contract::DataContract;
    use dpp::fee::fee_result::FeeResult;
    use dpp::platform_value::{platform_value, Value};
    use dpp::state_transition::batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
    use dpp::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
    use dpp::state_transition::batch_transition::document_create_transition::v0::DocumentCreateTransitionV0;
    use dpp::version::DefaultForPlatformVersion;
    use dpp::document::Document;
    use drive::drive::contract::DataContractFetchInfo;
    use drive::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::DocumentFromCreateTransitionAction;
    use drive::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::DocumentCreateTransitionActionV0;
    use drive::state_transition_action::batch::batched_transition::document_transition::DocumentTransitionAction;
    use drive::state_transition_action::batch::batched_transition::BatchedTransitionAction;
    use std::sync::Arc;

    const OWNER: [u8; 32] = [42; 32];
    const CONTRACT_ID: [u8; 32] = [9; 32];

    /// The latest platform version with `sys.random.uniform` admitted, which no protocol
    /// version does until block execution draws the values.
    fn drawing_platform_version() -> PlatformVersion {
        let mut platform_version = PlatformVersion::latest().clone();
        platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .schema
            .random_generated_from = Some(0);
        platform_version
    }

    /// A card whose `rarity` is drawn from 1 to 100, indexed by it when `indexed`.
    fn card_schema(indexed: bool) -> Value {
        let mut schema = platform_value!({
            "type": "object",
            "documentsMutable": false,
            "properties": {
                "name": { "type": "string", "maxLength": 32, "position": 0 },
                "rarity": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 100,
                    "generatedFrom": { "function": "sys.random.uniform" },
                    "position": 1
                }
            },
            "required": ["name", "rarity"],
            "additionalProperties": false
        });
        if indexed {
            schema
                .insert(
                    "indices".to_string(),
                    platform_value!([{ "name": "byRarity", "properties": [{ "rarity": "asc" }] }]),
                )
                .expect("the schema is a map");
        }
        schema
    }

    /// The precalculated fees the state validation of a create of a `document_type_name`
    /// document charged, and what `Drive::prepay_random_document_index_entries` prices for
    /// its document.
    fn state_validation_charges(document_type_name: &str) -> (Vec<FeeResult>, FeeResult) {
        let platform_version = drawing_platform_version();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let contract = DataContract::from_value(
            platform_value!({
                "$formatVersion": "1",
                "id": Identifier::from(CONTRACT_ID),
                "ownerId": Identifier::from(OWNER),
                "version": 1u32,
                "documentSchemas": {
                    "card": card_schema(true),
                    "looseCard": card_schema(false)
                }
            }),
            true,
            &platform_version,
        )
        .expect("the contract parses");
        platform
            .drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                None,
                &platform_version,
            )
            .expect("the contract is stored");
        let fetch_info: Arc<DataContractFetchInfo> = platform
            .drive
            .get_contract_with_fetch_info(CONTRACT_ID, false, None, &platform_version)
            .expect("the contract reads back")
            .expect("the contract is stored");

        let transition = DocumentCreateTransitionV0 {
            base: DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
                id: Identifier::from([1; 32]),
                identity_contract_nonce: 1,
                document_type_name: document_type_name.to_string(),
                data_contract_id: Identifier::from(CONTRACT_ID),
            }),
            entropy: [2; 32],
            data: BTreeMap::from([("name".to_string(), Value::Text("Ace".to_string()))]),
            prefunded_voting_balance: None,
        };
        let (result, _) =
            DocumentCreateTransitionActionV0::try_from_borrowed_document_create_transition_with_contract_lookup(
                &platform.drive,
                Identifier::from(OWNER),
                None,
                &transition,
                &BlockInfo::default(),
                0,
                |_| Ok(fetch_info.clone()),
                &platform_version,
            )
            .expect("the action builds");
        let BatchedTransitionAction::DocumentAction(DocumentTransitionAction::CreateAction(
            mut action,
        )) = result.into_data().expect("an action")
        else {
            panic!("a create action");
        };

        let state = platform.state.load();
        let platform_ref = PlatformStateRef {
            drive: &platform.drive,
            state: &state,
            config: &platform.config,
        };
        let mut execution_context =
            StateTransitionExecutionContext::default_for_platform_version(&platform_version)
                .expect("an execution context");
        let validation = action
            .validate_state_v2(
                &platform_ref,
                Identifier::from(OWNER),
                &BlockInfo::default(),
                &mut vec![],
                &mut execution_context,
                None,
                &platform_version,
            )
            .expect("the state validation runs");
        assert!(validation.is_valid(), "{:?}", validation.errors);

        let charged = execution_context
            .operations_slice()
            .iter()
            .filter_map(|operation| match operation {
                ValidationOperation::PrecalculatedOperation(fee_result) => Some(fee_result.clone()),
                _ => None,
            })
            .collect();
        let document = Document::try_from_create_transition_action(
            &action,
            Identifier::from(OWNER),
            &platform_version,
        )
        .expect("the document");
        let prepayment = platform
            .drive
            .prepay_random_document_index_entries(
                &document,
                Identifier::from(OWNER),
                &fetch_info.contract,
                fetch_info
                    .contract
                    .document_type_for_name(document_type_name)
                    .expect("the type"),
                &BlockInfo::default(),
                None,
                &platform_version,
            )
            .expect("the prepayment is priced");
        (charged, prepayment)
    }

    #[test]
    fn should_leave_the_draw_prepayment_to_the_insert() {
        // State validation charges a create indexing a random property what it charges one
        // that indexes none: the prepayment rides on the document's insert, so a create
        // refused at a later stage, whose document is never inserted, prepays nothing
        let (indexed, prepayment) = state_validation_charges("card");
        let (unindexed, nothing) = state_validation_charges("looseCard");
        assert_ne!(prepayment, FeeResult::default());
        assert_eq!(nothing, FeeResult::default());
        assert_eq!(indexed, unindexed);
    }
}
