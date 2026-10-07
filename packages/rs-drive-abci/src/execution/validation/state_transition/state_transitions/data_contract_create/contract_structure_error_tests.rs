//! Registering a contract whose document type breaks a structural rule of the parser or of the
//! document meta-schema, through `check_tx` and block processing.

use crate::execution::validation::state_transition::state_transitions::data_contract_common::contract_structure_test_harness::{
    assert_paid_contract_structure_error, assert_paid_contract_structure_error_in_block,
    assert_unpaid_internal_error, check_and_process,
    contested_unbounded_sum_schema, expiring_unbounded_sum_schema, resign_with_contract, resign_with_schemas,
    summed_u64_schema, terminal_without_index_only_schema, Outcome,
    CONTESTED_UNBOUNDED_SUM_MESSAGE, EXPIRING_UNBOUNDED_SUM_MESSAGE, SUMMED_U64_MESSAGE,
    TERMINAL_WITHOUT_INDEX_ONLY_MESSAGE,
};
use crate::execution::validation::state_transition::state_transitions::tests::setup_identity;
use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
use crate::rpc::core::MockCoreRPCLike;
use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
use assert_matches::assert_matches;
use dpp::consensus::basic::BasicError;
use dpp::consensus::ConsensusError;
use dpp::dash_to_credits;
use dpp::data_contract::errors::DataContractError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::data_contract::serialized_version::DataContractInSerializationFormat;
use dpp::data_contract::DataContract;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_nonce::IDENTITY_NONCE_VALUE_FILTER;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{Identity, IdentityPublicKey};
use dpp::platform_value::{platform_value, Value};
use dpp::serialization::PlatformSerializable;
use dpp::state_transition::batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
use dpp::state_transition::batch_transition::BatchTransition;
use dpp::state_transition::data_contract_create_transition::methods::DataContractCreateTransitionMethodsV0;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::tests::fixtures::get_data_contract_fixture;
use platform_version::version::{PlatformVersion, ProtocolVersion};
use simple_signer::signer::SimpleSigner;
use std::collections::BTreeMap;
use std::time::Duration;

/// A registration through `check_tx` and one block, with the platform it ran on and the owner of
/// the contract, so a test can go on to use the contract.
struct Registration {
    platform: TempPlatform<MockCoreRPCLike>,
    owner: Identity,
    signer: SimpleSigner,
    key: IdentityPublicKey,
    outcome: Outcome,
}

/// Registers a contract whose only document type is `item`, with `schema`.
async fn register_contract_with_schema(
    schema: Value,
    protocol_version: ProtocolVersion,
) -> Registration {
    register_contract_with_schema_and_balance(schema, protocol_version, dash_to_credits!(1.0)).await
}

/// [`register_contract_with_schema`] by an owner holding `balance`.
async fn register_contract_with_schema_and_balance(
    schema: Value,
    protocol_version: ProtocolVersion,
    balance: u64,
) -> Registration {
    register_contract_with(
        |contract| {
            let schemas = contract.document_schemas_mut();
            schemas.clear();
            schemas.insert("item".to_string(), schema);
        },
        protocol_version,
        balance,
    )
    .await
}

/// Registers the fixture contract after `edit_contract` edits its serialized form, by an owner
/// holding `balance`.
async fn register_contract_with(
    edit_contract: impl FnOnce(&mut DataContractInSerializationFormat),
    protocol_version: ProtocolVersion,
    balance: u64,
) -> Registration {
    let platform_version =
        PlatformVersion::get(protocol_version).expect("expected the protocol version");
    let mut platform = TestPlatformBuilder::new()
        .with_initial_protocol_version(protocol_version)
        .build_with_mock_rpc()
        .set_genesis_state();

    let (identity, signer, key) = setup_identity(&mut platform, 5077, balance);

    let data_contract =
        get_data_contract_fixture(Some(identity.id()), 1, platform_version.protocol_version)
            .data_contract_owned();

    let mut state_transition = DataContractCreateTransition::new_from_data_contract(
        data_contract,
        1,
        &identity.clone().into_partial_identity_info(),
        key.id(),
        &signer,
        platform_version,
        None,
    )
    .await
    .expect("expected to create the contract create transition");
    let transition_bytes =
        resign_with_contract(&mut state_transition, edit_contract, &key, &signer).await;

    let owner_id = identity.id();
    let outcome = check_and_process(
        &platform,
        owner_id,
        transition_bytes,
        |platform| {
            platform
                .drive
                .fetch_identity_nonce(owner_id.to_buffer(), true, None, platform_version)
                .expect("expected to fetch the identity nonce")
        },
        platform_version,
    );
    Registration {
        platform,
        owner: identity,
        signer,
        key,
        outcome,
    }
}

/// A document type whose property `p` is a `$ref` to `d0`, where each `d<i>` below `levels` is an
/// object with two members that both `$ref` `d<i+1>`: an acyclic `$defs` graph the non-validating
/// parse would expand into 2^`levels` properties.
fn register_ref_dag(levels: usize) -> impl FnOnce(&mut DataContractInSerializationFormat) {
    move |contract| {
        let mut defs = BTreeMap::new();
        for i in 0..levels {
            let next = format!("#/$defs/d{}", i + 1);
            defs.insert(
                format!("d{i}"),
                platform_value!({
                    "type": "object",
                    "properties": {
                        "b": { "$ref": next.clone(), "position": 0 },
                        "c": { "$ref": next, "position": 1 },
                    },
                    "additionalProperties": false,
                }),
            );
        }
        // A leaf that parses, so the non-validating parse would not stop at the first one.
        defs.insert(format!("d{levels}"), platform_value!({ "type": "integer" }));
        match contract {
            DataContractInSerializationFormat::V0(v0) => v0.schema_defs = Some(defs),
            DataContractInSerializationFormat::V1(v1) => v1.schema_defs = Some(defs),
        }
        let schemas = contract.document_schemas_mut();
        schemas.clear();
        schemas.insert(
            "item".to_string(),
            platform_value!({
                "type": "object",
                "properties": { "p": { "$ref": "#/$defs/d0", "position": 0 } },
                "additionalProperties": false,
            }),
        );
    }
}

/// `check_tx` runs the schema depth check before its non-validating parse, so it refuses an
/// acyclic `$defs` graph that parse would expand exponentially, quickly and with the error the
/// block charges for.
#[tokio::test]
async fn should_refuse_an_exponential_ref_dag_in_check_tx_as_the_block_does() {
    let outcome = register_contract_with(
        register_ref_dag(40),
        PlatformVersion::latest().protocol_version,
        dash_to_credits!(1.0),
    )
    .await
    .outcome;

    assert!(
        outcome.check_tx_elapsed < Duration::from_secs(1),
        "check_tx took {:?}",
        outcome.check_tx_elapsed
    );
    let is_cycle_error = |error: &ConsensusError| {
        matches!(
            error,
            ConsensusError::BasicError(BasicError::InvalidJsonSchemaRefError(e))
                if e.to_string().contains("contains cycles")
        )
    };
    assert_matches!(
        outcome.check_tx.as_deref(),
        Ok([error]) if is_cycle_error(error),
        "check_tx: {:?}",
        outcome.check_tx
    );
    assert_matches!(
        &outcome.block,
        StateTransitionExecutionResult::PaidConsensusError { error, .. } if is_cycle_error(error),
        "block: {:?}",
        outcome.block
    );
    assert_eq!(
        outcome
            .nonce_after
            .map(|nonce| nonce & IDENTITY_NONCE_VALUE_FILTER),
        Some(1)
    );
}

/// The depth check `check_tx` runs first lets an ordinary contract through.
#[tokio::test]
async fn should_accept_an_ordinary_contract_in_check_tx() {
    let outcome = register_contract_with(
        |_| {},
        PlatformVersion::latest().protocol_version,
        dash_to_credits!(1.0),
    )
    .await
    .outcome;

    assert_matches!(
        outcome.check_tx.as_deref(),
        Ok([]),
        "check_tx: {:?}",
        outcome.check_tx
    );
    assert_matches!(
        &outcome.block,
        StateTransitionExecutionResult::SuccessfulExecution { .. },
        "block: {:?}",
        outcome.block
    );
}

#[tokio::test]
async fn should_refuse_a_summed_u64_property_with_a_paid_consensus_error() {
    let outcome = register_contract_with_schema(
        summed_u64_schema(),
        PlatformVersion::latest().protocol_version,
    )
    .await
    .outcome;

    assert_eq!(outcome.nonce_before, Some(0));
    assert_paid_contract_structure_error(&outcome, SUMMED_U64_MESSAGE, 1);
}

#[tokio::test]
async fn should_refuse_a_terminal_outside_an_index_only_type_with_a_paid_consensus_error() {
    let outcome = register_contract_with_schema(
        terminal_without_index_only_schema(),
        PlatformVersion::latest().protocol_version,
    )
    .await
    .outcome;

    assert_eq!(outcome.nonce_before, Some(0));
    assert_paid_contract_structure_error(&outcome, TERMINAL_WITHOUT_INDEX_ONLY_MESSAGE, 1);
}

#[tokio::test]
async fn should_keep_refusing_a_summed_u64_property_unpaid_at_protocol_version_13() {
    let outcome = register_contract_with_schema(summed_u64_schema(), 13)
        .await
        .outcome;

    assert_unpaid_internal_error(&outcome, SUMMED_U64_MESSAGE);
}

/// The bound is a registration rule, checked under full validation only, so `check_tx`, which
/// parses a contract without it, admits the transition, and the block refuses it, charging
/// the owner and bumping its nonce.
#[tokio::test]
async fn should_refuse_a_contested_type_summing_an_unbounded_property_with_a_paid_consensus_error()
{
    let outcome = register_contract_with_schema_and_balance(
        contested_unbounded_sum_schema(),
        PlatformVersion::latest().protocol_version,
        // A contested index costs a registration fee the default balance does not cover
        dash_to_credits!(2.0),
    )
    .await
    .outcome;

    assert_eq!(outcome.nonce_before, Some(0));
    assert_matches!(
        outcome.check_tx.as_deref(),
        Ok([]),
        "check_tx: {:?}",
        outcome.check_tx
    );
    assert_matches!(
        &outcome.block,
        StateTransitionExecutionResult::PaidConsensusError {
            error: ConsensusError::BasicError(BasicError::ContractError(
                DataContractError::InvalidContractStructure(message)
            )),
            ..
        } if message.contains(CONTESTED_UNBOUNDED_SUM_MESSAGE),
        "block: {:?}",
        outcome.block
    );
    // The stored nonce keeps the nonces skipped below it in its high bits.
    assert_eq!(
        outcome
            .nonce_after
            .map(|nonce| nonce & IDENTITY_NONCE_VALUE_FILTER),
        Some(1),
        "the rejection bumps the nonce"
    );
    assert!(
        outcome.balance_after < outcome.balance_before,
        "the rejection is charged: {:?} -> {:?}",
        outcome.balance_before,
        outcome.balance_after
    );
}

#[tokio::test]
async fn should_keep_registering_a_contested_type_summing_an_unbounded_property_at_protocol_version_13(
) {
    let outcome = register_contract_with_schema_and_balance(
        contested_unbounded_sum_schema(),
        13,
        // A contested index costs a registration fee the default balance does not cover
        dash_to_credits!(2.0),
    )
    .await
    .outcome;

    assert_matches!(
        outcome.check_tx.as_deref(),
        Ok([]),
        "check_tx: {:?}",
        outcome.check_tx
    );
    assert_matches!(
        &outcome.block,
        StateTransitionExecutionResult::SuccessfulExecution { .. },
        "block: {:?}",
        outcome.block
    );
}

/// Like the contested bound, a registration rule: `check_tx` admits the transition, and the
/// block refuses it, charging the owner and bumping its nonce. `ttl` arrives with the protocol
/// version that brings the rule, so no earlier version registers the type.
#[tokio::test]
async fn should_refuse_an_expiring_type_summing_an_unbounded_property_with_a_paid_consensus_error()
{
    let outcome = register_contract_with_schema(
        expiring_unbounded_sum_schema(),
        PlatformVersion::latest().protocol_version,
    )
    .await
    .outcome;

    assert_eq!(outcome.nonce_before, Some(0));
    assert_paid_contract_structure_error_in_block(&outcome, EXPIRING_UNBOUNDED_SUM_MESSAGE, 1);
}

/// A document type summing `payment.amount` in an index: the dotted path of `amount`, an integer
/// in the required `payment` object. Drive reads a document's sum contribution from its top level,
/// where no property has that name.
fn dotted_summable_schema() -> Value {
    platform_value!({
        "type": "object",
        "properties": {
            "payment": {
                "type": "object",
                "properties": {
                    "amount": {
                        "type": "integer",
                        "minimum": 0,
                        "maximum": 1000,
                        "position": 0,
                    },
                },
                "required": ["amount"],
                "additionalProperties": false,
                "position": 0,
            },
            "label": {
                "type": "string",
                "maxLength": 20,
                "position": 1,
            },
        },
        "required": ["payment", "label"],
        "indices": [
            {"name": "byLabel", "properties": [{"label": "asc"}], "summable": "payment.amount"},
        ],
        "additionalProperties": false,
    })
}

/// The document meta-schema refuses the name. `check_tx` parses a contract without full
/// validation, so the meta-schema does not run there, and the block refuses the transition with
/// the meta-schema's error, charging the owner and bumping its nonce.
#[tokio::test]
async fn should_refuse_a_dotted_summable_name_with_a_paid_consensus_error() {
    let outcome = register_contract_with_schema(
        dotted_summable_schema(),
        PlatformVersion::latest().protocol_version,
    )
    .await
    .outcome;

    assert_matches!(
        outcome.check_tx.as_deref(),
        Ok([]),
        "check_tx: {:?}",
        outcome.check_tx
    );
    assert_matches!(
        &outcome.block,
        StateTransitionExecutionResult::PaidConsensusError {
            error: ConsensusError::BasicError(BasicError::JsonSchemaError(error)),
            ..
        } if error.keyword() == "pattern" && error.instance_path() == "/indices/0/summable",
        "block: {:?}",
        outcome.block
    );
    assert_eq!(outcome.nonce_before, Some(0));
    // The stored nonce keeps the nonces skipped below it in its high bits.
    assert_eq!(
        outcome
            .nonce_after
            .map(|nonce| nonce & IDENTITY_NONCE_VALUE_FILTER),
        Some(1),
        "the rejection bumps the nonce"
    );
    assert!(
        outcome.balance_after < outcome.balance_before,
        "the rejection is charged: {:?} -> {:?}",
        outcome.balance_before,
        outcome.balance_after
    );
}

/// Meta-schema v2 bounds only the length of the name, so the contract still registers at
/// protocol version 13, and each document create of the type still fails in Drive, as an internal
/// error that leaves the owner untouched.
#[tokio::test]
async fn should_keep_registering_a_dotted_summable_name_at_protocol_version_13() {
    let platform_version = PlatformVersion::get(13).expect("expected protocol version 13");
    let registration = register_contract_with_schema(dotted_summable_schema(), 13).await;
    let outcome = &registration.outcome;

    assert_matches!(
        outcome.check_tx.as_deref(),
        Ok([]),
        "check_tx: {:?}",
        outcome.check_tx
    );
    assert_matches!(
        &outcome.block,
        StateTransitionExecutionResult::SuccessfulExecution { .. },
        "block: {:?}",
        outcome.block
    );

    let owner_id = registration.owner.id();
    let contract_id = DataContract::generate_data_contract_id_v0(owner_id, 1);
    let contract = registration
        .platform
        .drive
        .fetch_contract(contract_id.to_buffer(), None, None, None, platform_version)
        .value
        .expect("expected to fetch the contract")
        .expect("expected the contract to be registered");
    let item = contract
        .contract
        .document_type_for_name("item")
        .expect("expected the item document type");

    // The registration took the owner's first nonce on the contract
    let identity_contract_nonce = 2;
    let entropy = [9; 32];
    let mut document = item
        .create_document_from_data(
            platform_value!({"payment": {"amount": 5}, "label": "first"}),
            owner_id,
            0,
            0,
            entropy,
            platform_version,
        )
        .expect("expected to create the document");
    document
        .set_id_for_creation(item, &entropy, identity_contract_nonce, platform_version)
        .expect("expected to set the document id");
    let create_transition = BatchTransition::new_document_creation_transition_from_document(
        document,
        item,
        entropy,
        &registration.key,
        identity_contract_nonce,
        0,
        None,
        &registration.signer,
        platform_version,
        None,
    )
    .await
    .expect("expected to create the document create transition");

    let create_outcome = check_and_process(
        &registration.platform,
        owner_id,
        create_transition
            .serialize_to_bytes()
            .expect("expected to serialize the document create transition"),
        |platform| {
            platform
                .drive
                .fetch_identity_contract_nonce(
                    owner_id.to_buffer(),
                    contract_id.to_buffer(),
                    true,
                    None,
                    platform_version,
                )
                .expect("expected to fetch the contract nonce")
        },
        platform_version,
    );
    assert_unpaid_internal_error(&create_outcome, "summable property absent");
}
