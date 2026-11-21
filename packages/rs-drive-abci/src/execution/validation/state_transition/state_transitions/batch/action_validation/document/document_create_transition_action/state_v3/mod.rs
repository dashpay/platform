use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::document::duplicate_unique_index_error::DuplicateUniqueIndexError;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::ConsensusError;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::identifier::Identifier;
use dpp::platform_value::Value;
use dpp::document::Document;
use dpp::validation::{ConsensusValidationResult, SimpleConsensusValidationResult};
use dpp::version::PlatformVersion;
use dpp::voting::vote_info_storage::contested_document_vote_poll_stored_info::{
    ContestedDocumentVotePollStatus, ContestedDocumentVotePollStoredInfoV0Getters,
};
use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use drive::drive::document::shared_unique_entry::unique_index_entry_is_shared;
use drive::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::resolve::ContestedDocumentResourceVotePollResolver;
use drive::query::TransactionArg;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::{
    DocumentCreateTransitionAction, DocumentCreateTransitionActionAccessorsV0,
    DocumentFromCreateTransitionAction,
};
use drive::util::object_size_info::DocumentInfo;

use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::batch::action_validation::document::document_create_transition_action::state_v2::DocumentCreateTransitionActionStateValidationV2;
use crate::execution::validation::state_transition::batch::action_validation::document::document_reference_validation::ConsumedLookupDocument;
use crate::platform_types::platform::PlatformStateRef;

pub(in crate::execution::validation::state_transition::state_transitions::batch::action_validation) trait DocumentCreateTransitionActionStateValidationV3
{
    #[allow(clippy::too_many_arguments)]
    fn validate_state_v3(
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

impl DocumentCreateTransitionActionStateValidationV3 for DocumentCreateTransitionAction {
    fn validate_state_v3(
        &mut self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        consumed_documents: &mut Vec<ConsumedLookupDocument>,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let validation_result = self.validate_state_v2(
            platform,
            owner_id,
            block_info,
            consumed_documents,
            execution_context,
            transaction,
            platform_version,
        )?;
        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        // A non-contested create must not take the index values of a live contest. A contest
        // decided by its index values takes every document with them, so this holds by
        // itself; a contest whose `fieldMatches` read something else, which registration
        // refuses from protocol version 15 but a contract registered earlier may keep, lets a
        // document with the same values bypass it. Such a document would take the unique index
        // entry the contest awards at its end, leaving the award nothing to take.
        // The poll is read at its path, which encodes the values as index keys, so a create of
        // `"\0"` finds the live contest of `""` (both are the key `[0]`) and is refused too.
        // A batch holds one document transition (`max_transitions_in_documents_batch`, pinned
        // to 1 at every protocol version by a test), so no earlier create of the same batch can
        // open the contest before this one is checked.
        if self.prefunded_voting_balance().is_some() {
            return Ok(SimpleConsensusValidationResult::new());
        }
        let document_type = self.base().document_type()?;
        let Some(contested_index) = document_type.find_contested_index() else {
            return Ok(SimpleConsensusValidationResult::new());
        };
        if contested_index.contest_is_decided_by_index_values(document_type.flattened_properties())
        {
            return Ok(SimpleConsensusValidationResult::new());
        }

        let index_values = contested_index
            .extract_contested_values(
                self.data(),
                document_type.flattened_properties(),
                platform_version,
            )
            .map_err(Error::Protocol)?;
        // The values are read from the create's data, as a contested create reads the values of
        // the poll it opens. A missing one is either a user property the document lacks, whose
        // unique index entry Drive keeps per document, or a system property (`$ownerId`), which
        // the data never holds: a contested create can not address a poll through it, so no
        // contest over this index can be live. Either way nothing is read
        if index_values.iter().any(Value::is_null) {
            return Ok(SimpleConsensusValidationResult::new());
        }
        // Nor for a document whose unique index entry Drive keeps per document because a value
        // has an empty index key, such as an empty byte array
        let document =
            Document::try_from_create_transition_action(self, owner_id, platform_version)?;
        if !unique_index_entry_is_shared(
            &DocumentInfo::DocumentRefInfo((&document, None)),
            contested_index,
            document_type,
            Some(owner_id.to_buffer()),
            platform_version,
        )? {
            return Ok(SimpleConsensusValidationResult::new());
        }
        let vote_poll = ContestedDocumentResourceVotePoll {
            contract_id: self.base().data_contract_id(),
            document_type_name: self.base().document_type_name().clone(),
            index_name: contested_index.name.clone(),
            index_values,
        }
        .resolve_owned_with_provided_arc_contract_fetch_info(
            self.base().data_contract_fetch_info(),
        )?;

        let (fee_result, stored_info) = platform
            .drive
            .fetch_contested_document_vote_poll_stored_info(
                &vote_poll,
                Some(&block_info.epoch),
                transaction,
                platform_version,
            )?;
        if let Some(fee_result) = fee_result {
            execution_context
                .add_operation(ValidationOperation::PrecalculatedOperation(fee_result));
        }

        if stored_info.is_some_and(|stored_info| {
            matches!(
                stored_info.vote_poll_status(),
                ContestedDocumentVotePollStatus::Started(_)
            )
        }) {
            return Ok(ConsensusValidationResult::new_with_error(
                ConsensusError::StateError(StateError::DuplicateUniqueIndexError(
                    DuplicateUniqueIndexError::new(
                        self.base().id(),
                        contested_index
                            .properties
                            .iter()
                            .map(|property| property.name.clone())
                            .collect(),
                    ),
                )),
            ));
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
    use std::collections::BTreeMap;
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
            .validate_state_v3(
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
