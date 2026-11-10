//! Delta contract updates (version 1 of the update transition, from protocol version 15):
//! block execution, state validation, check_tx's transformation, the size a contract can grow
//! to, and execution proofs.

use super::*;
use crate::execution::check_tx::CheckTxLevel;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::processor::basic_structure::StateTransitionBasicStructureValidationV0;
use crate::execution::validation::state_transition::processor::identity_nonces::StateTransitionIdentityNonceValidationV0;
use crate::execution::validation::state_transition::processor::state::StateTransitionStateValidation;
use crate::execution::validation::state_transition::transformer::StateTransitionActionTransformer;
use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform::PlatformStateRef;
use dpp::consensus::basic::data_contract::DataContractUpdateEntryKind;
use dpp::consensus::basic::UnsupportedVersionError;
use dpp::consensus::signature::SignatureError;
use dpp::dashcore::Network;
use dpp::data_contract::accessors::v1::DataContractV1Setters;
use dpp::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
use dpp::data_contract::config::v0::{DataContractConfigSettersV0, DataContractConfigV0};
use dpp::data_contract::schema::DataContractSchemaMethodsV0;
use dpp::identity::identity_nonce::IDENTITY_NONCE_VALUE_FILTER;
use dpp::platform_value::{platform_value, Value};
use dpp::prelude::ConsensusValidationResult;
use dpp::serialization::PlatformSerializableWithPlatformVersion;
use dpp::state_transition::data_contract_update_transition::accessors::DataContractUpdateTransitionAccessorsV0;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransitionV1;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::state_transition::{StateTransition, StateTransitionOwned};
use drive::drive::Drive;
use drive::error::proof::ProofError;
use drive::error::Error as DriveError;
use drive::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceActionAccessorsV0;
use drive::state_transition_action::StateTransitionAction;
use platform_version::DefaultForPlatformVersion;
use std::sync::Arc;

/// A platform at the latest protocol version, with genesis state: a contract's keywords are
/// indexed through the keyword search contract, which only genesis state holds.
fn platform() -> TempPlatform<MockCoreRPCLike> {
    TestPlatformBuilder::new()
        .build_with_mock_rpc()
        .set_genesis_state()
}

fn new_document_schema() -> Value {
    platform_value!({
        "type": "object",
        "properties": {
            "name": {
                "type": "string",
                "position": 0
            }
        },
        "additionalProperties": false
    })
}

/// A document type whose schema takes about `properties` times 300 bytes.
fn wide_document_schema(properties: u32) -> Value {
    let text = |text: &str| Value::Text(text.to_string());
    let properties = (0..properties)
        .map(|position| {
            (
                Value::Text(format!("property{position:03}")),
                Value::Map(vec![
                    (text("type"), text("string")),
                    (text("maxLength"), Value::U32(63)),
                    (text("position"), Value::U32(position)),
                    (text("description"), Value::Text("d".repeat(256))),
                ]),
            )
        })
        .collect();
    Value::Map(vec![
        (text("type"), text("object")),
        (text("properties"), Value::Map(properties)),
        (text("additionalProperties"), Value::Bool(false)),
    ])
}

fn fixture_contract() -> DataContract {
    get_data_contract_fixture(None, 0, PlatformVersion::latest().protocol_version)
        .data_contract_owned()
}

/// `contract` at its next version, with a new document type, two keywords and a description.
fn extended(contract: &DataContract) -> DataContract {
    let mut updated = contract.clone();
    updated.increment_version();
    updated
        .set_document_schema(
            "newType",
            new_document_schema(),
            true,
            &mut vec![],
            PlatformVersion::latest(),
        )
        .expect("expected to add a document type");
    updated.set_keywords(vec!["alpha".to_string(), "beta".to_string()]);
    updated.set_description(Some("a described contract".to_string()));
    updated
}

/// The unsigned delta that turns `old_contract` into `new_contract`, under nonce 1.
fn delta(old_contract: &DataContract, new_contract: &DataContract) -> DataContractUpdateTransition {
    let transition = DataContractUpdateTransition::from_contract_update(
        old_contract,
        new_contract,
        1,
        PlatformVersion::latest(),
    )
    .expect("expected the update to be expressible as a delta");
    assert_matches!(transition, DataContractUpdateTransition::V1(_));
    transition
}

fn delta_fields(transition: &DataContractUpdateTransition) -> DataContractUpdateTransitionV1 {
    let DataContractUpdateTransition::V1(delta) = transition else {
        panic!("expected a delta contract update");
    };
    delta.clone()
}

fn platform_ref<'a>(
    platform: &'a TempPlatform<MockCoreRPCLike>,
    state: &'a crate::platform_types::platform_state::PlatformState,
) -> PlatformRef<'a, MockCoreRPCLike> {
    PlatformRef {
        drive: &platform.drive,
        state,
        config: &platform.config,
        core_rpc: &platform.core_rpc,
    }
}

/// State validation as a block runs it.
fn validate(
    platform: &TempPlatform<MockCoreRPCLike>,
    transition: &DataContractUpdateTransition,
) -> ConsensusValidationResult<StateTransitionAction> {
    let state = platform.state.load();
    let mut execution_context = StateTransitionExecutionContext::default_for_platform_version(
        state
            .current_platform_version()
            .expect("expected a platform version"),
    )
    .expect("expected an execution context");
    transition
        .validate_state(
            None,
            &platform_ref(platform, &state),
            ValidationMode::Validator,
            &BlockInfo::default(),
            &mut execution_context,
            None,
        )
        .expect("expected state validation to run")
}

/// The transformation into an action as check_tx runs it.
fn transform_in_check_tx(
    platform: &TempPlatform<MockCoreRPCLike>,
    transition: &DataContractUpdateTransition,
) -> ConsensusValidationResult<StateTransitionAction> {
    let state = platform.state.load();
    let mut execution_context = StateTransitionExecutionContext::default_for_platform_version(
        state
            .current_platform_version()
            .expect("expected a platform version"),
    )
    .expect("expected an execution context");
    transition
        .transform_into_action(
            &platform_ref(platform, &state),
            &BlockInfo::default(),
            &None,
            ValidationMode::CheckTx,
            &mut execution_context,
            None,
        )
        .expect("expected the transformation to run")
}

fn assert_bumps_nonce(
    result: &ConsensusValidationResult<StateTransitionAction>,
    identity_id: Identifier,
    contract_id: Identifier,
) {
    assert_matches!(
        &result.data,
        Some(StateTransitionAction::BumpIdentityDataContractNonceAction(action))
            if action.identity_id() == identity_id
                && action.data_contract_id() == contract_id
                && action.identity_contract_nonce() == 1
    );
}

fn stored_contract(
    platform: &TempPlatform<MockCoreRPCLike>,
    contract_id: Identifier,
) -> DataContract {
    platform
        .drive
        .get_contract_with_fetch_info(
            contract_id.to_buffer(),
            false,
            None,
            PlatformVersion::latest(),
        )
        .expect("expected to read the contract")
        .expect("expected the contract to be stored")
        .contract
        .clone()
}

/// The size of `contract` in the format state transitions carry, which the protocol caps.
fn serialized_size(contract: &DataContract) -> usize {
    contract
        .serialize_to_bytes_with_platform_version(PlatformVersion::latest())
        .expect("expected to serialize the contract")
        .len()
}

/// An identity, its signer, and a contract it owns, stored.
struct OwnedContract {
    platform: TempPlatform<MockCoreRPCLike>,
    identity: Identity,
    signer: SimpleSigner,
    key: IdentityPublicKey,
    contract: DataContract,
}

impl OwnedContract {
    fn stored(contract: DataContract) -> Self {
        let mut platform = platform();
        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(50.0));
        let mut contract = contract;
        contract.set_owner_id(identity.id());
        apply_contract(&platform, &contract, BlockInfo::default());
        Self {
            platform,
            identity,
            signer,
            key,
            contract,
        }
    }

    /// The signed update that turns the stored contract into `new_contract`, in the form
    /// the latest protocol version builds by default, the delta.
    async fn signed_update(
        &self,
        old_contract: &DataContract,
        new_contract: &DataContract,
        nonce: u64,
    ) -> StateTransition {
        let transition = DataContractUpdateTransition::new_from_contract_update(
            old_contract,
            new_contract,
            &self.identity.clone().into_partial_identity_info(),
            self.key.id(),
            nonce,
            0,
            &self.signer,
            PlatformVersion::latest(),
            None,
        )
        .await
        .expect("expected a contract update transition");
        assert_matches!(
            transition,
            StateTransition::DataContractUpdate(DataContractUpdateTransition::V1(_))
        );
        transition
    }

    /// Executes `transition` in a block of its own and commits it.
    fn execute(&self, transition: &StateTransition) -> StateTransitionExecutionResult {
        self.execute_in(transition, &BlockInfo::default())
    }

    fn execute_in(
        &self,
        transition: &StateTransition,
        block_info: &BlockInfo,
    ) -> StateTransitionExecutionResult {
        let platform_version = PlatformVersion::latest();
        let platform_state = self.platform.state.load();
        let bytes = transition
            .serialize_to_bytes()
            .expect("expected to serialize the transition");
        let transaction = self.platform.drive.grove.start_transaction();
        let processing_result = self
            .platform
            .platform
            .process_raw_state_transitions(
                &[bytes],
                &platform_state,
                block_info,
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process the state transition");
        self.platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit the block");
        let mut execution_results = processing_result.into_execution_results();
        assert_eq!(execution_results.len(), 1, "{execution_results:?}");
        execution_results.remove(0)
    }

    fn nonce(&self) -> Option<u64> {
        self.platform
            .drive
            .fetch_identity_contract_nonce(
                self.identity.id().to_buffer(),
                self.contract.id().to_buffer(),
                true,
                None,
                PlatformVersion::latest(),
            )
            .expect("expected to read the identity contract nonce")
            .map(|nonce| nonce & IDENTITY_NONCE_VALUE_FILTER)
    }
}

mod block_execution {
    use super::*;

    #[tokio::test]
    async fn should_execute_a_delta_and_store_the_merged_contract() {
        let owned = OwnedContract::stored(fixture_contract());
        let updated = extended(&owned.contract);
        let transition = owned.signed_update(&owned.contract, &updated, 1).await;

        assert_matches!(
            owned.execute(&transition),
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );

        let stored = stored_contract(&owned.platform, owned.contract.id());
        assert_eq!(stored.version(), owned.contract.version() + 1);
        assert_eq!(stored.owner_id(), owned.identity.id());
        assert_eq!(
            stored.document_types().len(),
            owned.contract.document_types().len() + 1
        );
        assert!(stored.has_document_type_for_name("newType"));
        assert_eq!(stored.keywords(), updated.keywords());
        assert_eq!(stored.description(), updated.description());
        assert_eq!(stored.created_at(), owned.contract.created_at());
        assert_eq!(owned.nonce(), Some(1));
    }

    /// An update that embeds its contract cannot grow a contract past what one state
    /// transition carries. A delta can, a transition at a time, up to the size a contract may
    /// take, where it is refused with a consensus error: Drive refuses to store a larger
    /// contract, which would stop block execution.
    #[tokio::test]
    async fn should_grow_a_contract_past_one_transition_and_refuse_it_over_the_size_cap() {
        let platform_version = PlatformVersion::latest();
        let max_state_transition_size =
            platform_version.system_limits.max_state_transition_size as usize;
        let max_serialized_size = platform_version.dpp.contract_versions.max_serialized_size;
        let owned = OwnedContract::stored(fixture_contract());
        assert!(serialized_size(&owned.contract) < max_state_transition_size);

        let mut current = owned.contract.clone();
        let mut grown_past_one_transition = false;
        let mut refused = None;
        for step in 1..=12_u64 {
            let mut next = current.clone();
            next.increment_version();
            next.set_document_schema(
                &format!("wide{step}"),
                wide_document_schema(28),
                true,
                &mut vec![],
                platform_version,
            )
            .expect("expected to add a document type");
            let transition = owned.signed_update(&current, &next, step).await;
            assert!(
                transition
                    .serialize_to_bytes()
                    .expect("expected to serialize the delta")
                    .len()
                    < max_state_transition_size
            );

            match owned.execute(&transition) {
                StateTransitionExecutionResult::SuccessfulExecution { .. } => {
                    assert_eq!(
                        stored_contract(&owned.platform, owned.contract.id()).version(),
                        next.version()
                    );
                    current = next;
                    grown_past_one_transition |=
                        serialized_size(&current) > max_state_transition_size;
                }
                result => {
                    refused = Some((result, next, step));
                    break;
                }
            }
        }

        assert!(
            grown_past_one_transition,
            "expected deltas to grow the contract past what one state transition carries"
        );
        let (result, too_large, step) =
            refused.expect("expected a delta to be refused at the size cap");
        assert!(serialized_size(&current) <= max_serialized_size as usize);
        assert!(serialized_size(&too_large) > max_serialized_size as usize);
        assert_matches!(
            result,
            StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::BasicError(BasicError::DataContractMaxSizeExceededError(
                    ref error
                )),
                ..
            } if error.max_serialized_size() == max_serialized_size
                && error.serialized_size() > u64::from(max_serialized_size)
        );
        // The refusal is paid for with the nonce, and the contract stays as it was
        assert_eq!(owned.nonce(), Some(step));
        assert_eq!(
            stored_contract(&owned.platform, owned.contract.id()).version(),
            current.version()
        );
    }
}

mod state_validation {
    use super::*;

    #[test]
    fn should_merge_a_delta_and_pay_the_registration_cost_of_what_it_registers() {
        let platform_version = PlatformVersion::latest();
        let platform = platform();
        let contract = fixture_contract();
        apply_contract(&platform, &contract, BlockInfo::default());
        let updated = extended(&contract);
        let transition = delta(&contract, &updated);

        let result = validate(&platform, &transition);

        assert!(result.is_valid(), "unexpected errors: {:?}", result.errors);
        let StateTransitionAction::DataContractUpdateAction(action) =
            result.into_data().expect("expected an action")
        else {
            panic!("expected a data contract update action");
        };
        let merged = action.data_contract_ref();
        assert_eq!(merged.id(), contract.id());
        assert_eq!(merged.owner_id(), contract.owner_id());
        assert_eq!(merged.version(), contract.version() + 1);
        assert!(merged.has_document_type_for_name("newType"));
        assert_eq!(
            merged.document_types().len(),
            contract.document_types().len() + 1
        );
        assert_eq!(merged.keywords(), updated.keywords());
        assert_eq!(merged.description(), updated.description());
        assert_eq!(merged.created_at(), contract.created_at());
        assert_eq!(action.identity_contract_nonce(), 1);

        let fees = &platform_version.fee_version.data_contract_registration;
        let registration_cost = action
            .registration_cost(platform_version)
            .expect("expected a registration cost");
        assert_eq!(
            registration_cost,
            fees.document_type_registration_fee + 2 * fees.search_keyword_fee
        );
        assert!(
            registration_cost
                < merged
                    .registration_cost(platform_version)
                    .expect("expected a registration cost")
        );
    }

    #[test]
    fn should_refuse_a_delta_of_a_contract_that_does_not_exist_as_a_paid_nonce_bump() {
        let platform = platform();
        let contract = fixture_contract();
        let transition = delta(&contract, &extended(&contract));

        let result = validate(&platform, &transition);

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(BasicError::DataContractNotPresentError(error))]
                if error.data_contract_id() == contract.id()
        );
        assert_bumps_nonce(&result, contract.owner_id(), contract.id());
    }

    #[test]
    fn should_refuse_a_delta_of_an_identity_that_does_not_own_the_contract() {
        let platform = platform();
        let contract = fixture_contract();
        apply_contract(&platform, &contract, BlockInfo::default());
        let mut fields = delta_fields(&delta(&contract, &extended(&contract)));
        let intruder = Identifier::random();
        fields.owner_id = intruder;

        let result = validate(&platform, &fields.into());

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::StateError(StateError::DataContractUpdatePermissionError(error))]
                if *error.data_contract_id() == contract.id() && *error.identity_id() == intruder
        );
        // The intruder's nonce is bumped, never the owner's
        assert_bumps_nonce(&result, intruder, contract.id());
    }

    #[test]
    fn should_refuse_a_delta_that_does_not_fit_the_stored_contract() {
        let platform = platform();
        let contract = fixture_contract();
        apply_contract(&platform, &contract, BlockInfo::default());
        let mut updated = contract.clone();
        updated.increment_version();
        let fields = delta_fields(&delta(&contract, &updated));

        let mut adds_a_type_the_contract_has = fields.clone();
        adds_a_type_the_contract_has
            .new_document_schemas
            .insert("niceDocument".to_string(), new_document_schema());
        let result = validate(&platform, &adds_a_type_the_contract_has.into());
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::StateError(StateError::DataContractUpdateEntryAlreadyExistsError(error))]
                if error.entry_kind() == DataContractUpdateEntryKind::DocumentType
                    && error.name() == "niceDocument"
        );
        assert_bumps_nonce(&result, contract.owner_id(), contract.id());

        let mut replaces_a_type_the_contract_lacks = fields;
        replaces_a_type_the_contract_lacks
            .updated_document_schemas
            .insert("missingType".to_string(), new_document_schema());
        let result = validate(&platform, &replaces_a_type_the_contract_lacks.into());
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::StateError(StateError::DataContractUpdateEntryNotFoundError(error))]
                if error.entry_kind() == DataContractUpdateEntryKind::DocumentType
                    && error.name() == "missingType"
        );
        assert_bumps_nonce(&result, contract.owner_id(), contract.id());
    }

    /// The merged contract is validated as the contract of an update that embeds it: by the
    /// update rules against the stored contract.
    #[test]
    fn should_hold_a_delta_to_the_update_rules_of_an_update_that_embeds_its_contract() {
        let platform = platform();
        let contract = fixture_contract();
        apply_contract(&platform, &contract, BlockInfo::default());
        let mut updated = contract.clone();
        updated.set_version(contract.version() + 2);

        let result = validate(&platform, &delta(&contract, &updated));

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(BasicError::InvalidDataContractVersionError(error))]
                if error.expected_version() == contract.version() + 1
                    && error.version() == contract.version() + 2
        );
        assert_bumps_nonce(&result, contract.owner_id(), contract.id());
    }

    /// The merged contract is held to the structure rules too, which basic structure checks
    /// on an update that embeds its contract and cannot check on a delta: here, that token
    /// positions are contiguous.
    #[test]
    fn should_hold_the_merged_contract_to_the_structure_rules() {
        let platform = platform();
        let contract = fixture_contract();
        apply_contract(&platform, &contract, BlockInfo::default());
        let mut updated = contract.clone();
        updated.increment_version();
        let mut fields = delta_fields(&delta(&contract, &updated));
        fields.new_tokens.insert(
            5,
            TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive()),
        );

        let result = validate(&platform, &fields.into());

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::NonContiguousContractTokenPositionsError(_)
            )]
        );
        assert_bumps_nonce(&result, contract.owner_id(), contract.id());
    }

    /// A delta without a config keeps the stored one, and the merged contract is refused for
    /// it as an update that embeds a contract with that config is.
    #[test]
    fn should_refuse_a_delta_that_keeps_a_config_no_longer_admitted() {
        let platform = platform();
        let mut contract = fixture_contract();
        contract.set_config(DataContractConfig::V0(DataContractConfigV0::default()));
        apply_contract(&platform, &contract, BlockInfo::default());
        let transition = delta(&contract, &extended(&contract));
        assert!(delta_fields(&transition).config.is_none());

        let result = validate(&platform, &transition);

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::StateError(StateError::DataContractConfigUpdateError(error))]
                if error.additional_message().contains("minimum version")
        );
        assert_bumps_nonce(&result, contract.owner_id(), contract.id());
    }
}

mod basic_structure {
    use super::*;

    fn validate_basic_structure(
        transition: &DataContractUpdateTransition,
    ) -> dpp::validation::SimpleConsensusValidationResult {
        transition
            .validate_basic_structure(Network::Testnet, PlatformVersion::latest())
            .expect("expected basic structure validation to run")
    }

    #[test]
    fn should_admit_a_delta_whose_sections_do_not_overlap() {
        let contract = fixture_contract();
        let result = validate_basic_structure(&delta(&contract, &extended(&contract)));
        assert!(result.is_valid(), "unexpected errors: {:?}", result.errors);
    }

    #[test]
    fn should_refuse_a_delta_naming_an_entry_in_two_sections_that_cannot_both_apply() {
        let contract = fixture_contract();
        let fields = delta_fields(&delta(&contract, &extended(&contract)));

        let mut document_type = fields.clone();
        document_type
            .updated_document_schemas
            .insert("newType".to_string(), new_document_schema());
        let mut definition = fields.clone();
        definition
            .new_schema_defs
            .insert("label".to_string(), platform_value!({"type": "string"}));
        definition
            .updated_schema_defs
            .insert("label".to_string(), platform_value!({"type": "string"}));
        let mut keyword = fields;
        keyword.remove_keywords.push("alpha".to_string());

        for (fields, kind, name) in [
            (
                document_type,
                DataContractUpdateEntryKind::DocumentType,
                "newType",
            ),
            (definition, DataContractUpdateEntryKind::SchemaDef, "label"),
            (keyword, DataContractUpdateEntryKind::Keyword, "alpha"),
        ] {
            let result = validate_basic_structure(&fields.into());
            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::DataContractUpdateOverlappingEntriesError(error)
                )] if error.entry_kind() == kind && error.name() == name
            );
        }
    }
}

mod check_tx {
    use super::*;

    #[test]
    fn should_merge_a_delta_onto_the_stored_contract() {
        let platform_version = PlatformVersion::latest();
        let platform = platform();
        let contract = fixture_contract();
        apply_contract(&platform, &contract, BlockInfo::default());
        let transition = delta(&contract, &extended(&contract));

        let result = transform_in_check_tx(&platform, &transition);

        assert!(result.is_valid(), "unexpected errors: {:?}", result.errors);
        let Some(StateTransitionAction::DataContractUpdateAction(action)) = result.data else {
            panic!("expected a data contract update action");
        };
        assert!(action
            .data_contract_ref()
            .has_document_type_for_name("newType"));
        assert_eq!(
            action
                .registration_cost(platform_version)
                .expect("expected a registration cost"),
            delta_fields(&transition)
                .registration_cost(platform_version)
                .expect("expected a registration cost")
        );
    }

    /// A delta reads state in check_tx. One naming a contract that is not there is refused
    /// with a consensus error, never an execution error.
    #[test]
    fn should_refuse_a_delta_of_a_contract_that_does_not_exist() {
        let platform = platform();
        let contract = fixture_contract();
        let transition = delta(&contract, &extended(&contract));

        let result = transform_in_check_tx(&platform, &transition);

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::DataContractNotPresentError(_)
            )]
        );
        assert_bumps_nonce(&result, contract.owner_id(), contract.id());
    }

    /// check_tx validates a transition's basic structure before it looks for the identity
    /// that signed it, so that validation runs for anyone, on whatever a transition can
    /// carry. A delta from an identity that does not exist, naming thousands of keywords to
    /// add and as many to remove, is refused for its signer: comparing the two lists costs
    /// what their lengths add up to, not their product.
    #[test]
    fn should_refuse_a_delta_with_long_keyword_lists_from_an_unknown_identity_for_its_signer() {
        let platform_version = PlatformVersion::latest();
        let platform = platform();
        let contract = fixture_contract();
        let mut next_version = contract.clone();
        next_version.increment_version();
        let mut fields = delta_fields(&delta(&contract, &next_version));
        fields.add_keywords = vec!["a".to_string(); 3_000];
        fields.remove_keywords = vec!["b".to_string(); 3_000];
        let transition: StateTransition = DataContractUpdateTransition::V1(fields).into();
        let bytes = transition
            .serialize_to_bytes()
            .expect("expected to serialize the transition");
        assert!(
            bytes.len() <= platform_version.system_limits.max_state_transition_size as usize,
            "the transition must be one a node decodes"
        );

        let state = platform.state.load();
        let result = platform
            .check_tx(
                &bytes,
                CheckTxLevel::FirstTimeCheck,
                &platform_ref(&platform, &state),
                platform_version,
            )
            .expect("expected check_tx to run");

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::SignatureError(
                SignatureError::IdentityNotFoundError(_)
            )]
        );
    }
}

/// A delta is not active before protocol version 15, where decoding refuses it. Were one to
/// reach a validator there anyway, every dispatcher refuses it with a consensus error before a
/// shipped generation runs, so it can never stop block execution.
mod before_protocol_version_15 {
    use super::*;

    fn platform_at_14() -> TempPlatform<MockCoreRPCLike> {
        TestPlatformBuilder::new()
            .with_initial_protocol_version(14)
            .build_with_mock_rpc()
            .set_initial_state_structure()
    }

    fn assert_unsupported(errors: &[ConsensusError]) {
        assert_matches!(
            errors,
            [ConsensusError::BasicError(BasicError::UnsupportedVersionError(error))]
                if *error == UnsupportedVersionError::new(1, 0, 0)
        );
    }

    /// The prover and the verifier generations protocol version 14 selects prove and verify
    /// updates that embed their contract. They refuse a delta, which embeds none.
    #[test]
    fn should_refuse_to_prove_or_verify_a_delta() {
        let platform_version_14 =
            PlatformVersion::get(14).expect("expected protocol version 14 to exist");
        let platform = platform_at_14();
        let contract = get_data_contract_fixture(None, 0, 14).data_contract_owned();
        platform
            .drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                None,
                platform_version_14,
            )
            .expect("expected to store the contract");
        let transition: StateTransition = delta(&contract, &extended(&contract)).into();

        let proof = platform
            .drive
            .prove_state_transition(&transition, None, platform_version_14)
            .expect("expected the prover to run");
        assert_matches!(proof.errors.as_slice(), [ProofError::InvalidTransition(_)]);

        let known_contract = Arc::new(contract);
        assert_matches!(
            Drive::verify_state_transition_was_executed_with_proof(
                &transition,
                &BlockInfo::default(),
                &[],
                &|_| Ok(Some(known_contract.clone())),
                platform_version_14,
            ),
            Err(DriveError::Proof(ProofError::InvalidTransition(_)))
        );
    }

    #[test]
    fn should_refuse_a_delta_in_every_validation_with_a_consensus_error() {
        let platform_version_14 =
            PlatformVersion::get(14).expect("expected protocol version 14 to exist");
        let platform = platform_at_14();
        let contract = get_data_contract_fixture(None, 0, 14).data_contract_owned();
        platform
            .drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                None,
                platform_version_14,
            )
            .expect("expected to store the contract");
        let transition = delta(&contract, &extended(&contract));
        let state = platform.state.load();

        let basic_structure = transition
            .validate_basic_structure(Network::Testnet, platform_version_14)
            .expect("expected a consensus error, not an execution error");
        assert_unsupported(&basic_structure.errors);

        let mut execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version_14)
                .expect("expected an execution context");
        let nonce = transition
            .validate_identity_nonces(
                &PlatformStateRef {
                    drive: &platform.drive,
                    state: &state,
                    config: &platform.config,
                },
                &BlockInfo::default(),
                None,
                &mut execution_context,
                platform_version_14,
            )
            .expect("expected a consensus error, not an execution error");
        assert_unsupported(&nonce.errors);

        let state_result = validate(&platform, &transition);
        assert_unsupported(&state_result.errors);
        assert!(state_result.data.is_none());

        let transformed = transform_in_check_tx(&platform, &transition);
        assert_unsupported(&transformed.errors);
        assert!(transformed.data.is_none());
    }

    /// An update that embeds its contract is validated at protocol version 14 as it was.
    #[test]
    fn should_validate_an_update_that_embeds_its_contract_as_before() {
        let platform_version_14 =
            PlatformVersion::get(14).expect("expected protocol version 14 to exist");
        let platform = platform_at_14();
        let contract = get_data_contract_fixture(None, 0, 14).data_contract_owned();
        platform
            .drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                None,
                platform_version_14,
            )
            .expect("expected to store the contract");
        let mut updated = contract.clone();
        updated.increment_version();
        let transition = DataContractUpdateTransition::from_contract_update(
            &contract,
            &updated,
            1,
            platform_version_14,
        )
        .expect("expected an update transition");
        assert_matches!(transition, DataContractUpdateTransition::V0(_));
        assert!(transition.data_contract().is_some());

        let result = validate(&platform, &transition);

        assert!(result.is_valid(), "unexpected errors: {:?}", result.errors);
        assert_matches!(
            result.data,
            Some(StateTransitionAction::DataContractUpdateAction(_))
        );
    }
}

mod execution_proofs {
    use super::*;

    /// Stores `contract`, executes the delta that extends it, and returns the contract as it
    /// was before the update, as the update asks for it, the executed transition and the proof
    /// of its execution.
    async fn executed_delta(
        contract: DataContract,
    ) -> (DataContract, DataContract, StateTransition, Vec<u8>) {
        let owned = OwnedContract::stored(contract);
        let updated = extended(&owned.contract);
        let transition = owned.signed_update(&owned.contract, &updated, 1).await;
        // In a later block than the one that stored the contract: a contract that keeps
        // history holds one revision per block time
        assert_matches!(
            owned.execute_in(&transition, &BlockInfo::default_with_time(1_000)),
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        let proof = owned
            .platform
            .drive
            .prove_state_transition(&transition, None, PlatformVersion::latest())
            .expect("expected to prove the state transition");
        assert!(proof.is_valid(), "unexpected errors: {:?}", proof.errors);
        (
            owned.contract.clone(),
            updated,
            transition,
            proof.data.expect("expected proof data"),
        )
    }

    fn verify(
        transition: &StateTransition,
        proof: &[u8],
        known_contract: Option<DataContract>,
    ) -> Result<StateTransitionProofResult, DriveError> {
        let known_contract = known_contract.map(Arc::new);
        Drive::verify_state_transition_was_executed_with_proof(
            transition,
            &BlockInfo::default(),
            proof,
            &|_| Ok(known_contract.clone()),
            PlatformVersion::latest(),
        )
        .map(|(_root_hash, outcome)| outcome.into_result())
    }

    fn delta_of(transition: &StateTransition) -> DataContractUpdateTransitionV1 {
        let StateTransition::DataContractUpdate(update) = transition else {
            panic!("expected a contract update");
        };
        delta_fields(update)
    }

    #[tokio::test]
    async fn should_verify_the_whole_contract_a_delta_produces() {
        let (original, updated, transition, proof) = executed_delta(fixture_contract()).await;

        let result = verify(&transition, &proof, Some(original))
            .expect("expected the proof of the delta's execution to verify");

        let StateTransitionProofResult::VerifiedDataContract(contract) = result else {
            panic!("expected a verified data contract");
        };
        assert_eq!(contract.version(), updated.version());
        assert_eq!(contract.keywords(), updated.keywords());
        assert_eq!(contract.description(), updated.description());
        assert!(contract.has_document_type_for_name("newType"));
    }

    /// A delta embeds no contract, so whether its contract keeps history, which decides where
    /// the contract is proved, is read from the stored contract.
    #[tokio::test]
    async fn should_prove_and_verify_a_delta_of_a_contract_that_keeps_history() {
        let mut contract = fixture_contract();
        contract.config_mut().set_keeps_history(true);
        let (original, updated, transition, proof) = executed_delta(contract).await;

        let result = verify(&transition, &proof, Some(original))
            .expect("expected the proof of the delta's execution to verify");

        assert_matches!(
            result,
            StateTransitionProofResult::VerifiedDataContract(contract)
                if contract.version() == updated.version()
        );
    }

    #[tokio::test]
    async fn should_need_the_contract_the_delta_was_built_on() {
        let (_original, updated, transition, proof) = executed_delta(fixture_contract()).await;

        // unknown to the verifier's contract lookup
        assert_matches!(
            verify(&transition, &proof, None),
            Err(DriveError::Proof(ProofError::MissingContextRequirement(_)))
        );
        // known as it is after the update, which is not the contract the delta merges onto
        assert_matches!(
            verify(&transition, &proof, Some(updated)),
            Err(DriveError::Proof(ProofError::MissingContextRequirement(_)))
        );
    }

    /// The whole proved contract must be the one the delta merges into: a delta that names
    /// less than the proved state holds did not produce that state.
    #[tokio::test]
    async fn should_refuse_a_proved_contract_the_delta_does_not_produce() {
        let (original, _updated, transition, proof) = executed_delta(fixture_contract()).await;
        let mut other = delta_of(&transition);
        other.add_keywords.clear();

        assert_matches!(
            verify(&other.into(), &proof, Some(original)),
            Err(DriveError::Proof(ProofError::IncorrectProof(_)))
        );
    }

    #[tokio::test]
    async fn should_refuse_a_delta_a_node_would_refuse() {
        let (original, _updated, transition, proof) = executed_delta(fixture_contract()).await;

        // an entry in two sections that cannot both apply
        let mut overlapping = delta_of(&transition);
        overlapping
            .updated_document_schemas
            .insert("newType".to_string(), new_document_schema());
        assert_matches!(
            verify(&overlapping.into(), &proof, Some(original.clone())),
            Err(DriveError::Proof(ProofError::InvalidTransition(message)))
                if message.contains("two conflicting sections")
        );

        // an entry to replace that the contract does not have
        let mut misfit = delta_of(&transition);
        misfit
            .updated_document_schemas
            .insert("missingType".to_string(), new_document_schema());
        assert_matches!(
            verify(&misfit.into(), &proof, Some(original)),
            Err(DriveError::Proof(ProofError::InvalidTransition(message)))
                if message.contains("missingType")
        );
    }
}
