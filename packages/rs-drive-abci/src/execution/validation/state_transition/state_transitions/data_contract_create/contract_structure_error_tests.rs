//! A contract the document type parser refuses with an error that is not a consensus error at
//! protocol version 13, registered through `check_tx` and block processing: one breaking a
//! doctype-level aggregate rule, one with a value the parser cannot read, and, for the keywords
//! protocol version 14 introduces, one whose string is too long for the parser to size.
//!
//! From protocol version 14 each is a consensus error: `check_tx` refuses the transition with it
//! where `check_tx` parses that far, and a block charges the owner and bumps its identity nonce.
//! At protocol version 13 the parser reports it as an error of its own: `check_tx` fails, and a
//! block records an internal error, which leaves the owner untouched and keeps the transition out
//! of any block.

use crate::execution::check_tx::CheckTxLevel;
use crate::execution::validation::state_transition::state_transitions::tests::setup_identity;
use crate::platform_types::platform::PlatformRef;
use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
use crate::test::helpers::setup::TestPlatformBuilder;
use assert_matches::assert_matches;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::basic::BasicError;
use dpp::consensus::ConsensusError;
use dpp::dash_to_credits;
use dpp::data_contract::errors::DataContractError;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::SecurityLevel;
use dpp::platform_value::{platform_value, Value};
use dpp::prelude::Identifier;
use dpp::serialization::PlatformSerializable;
use dpp::state_transition::data_contract_create_transition::accessors::DataContractCreateTransitionAccessorsV0;
use dpp::state_transition::data_contract_create_transition::methods::DataContractCreateTransitionMethodsV0;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::state_transition::StateTransition;
use dpp::tests::fixtures::get_data_contract_fixture;
use dpp::ProtocolError;
use platform_version::version::{PlatformVersion, ProtocolVersion};

/// The fragment of the parser's message for a summed property that parses as `u64`.
const SUMMED_U64_MESSAGE: &str = "must be an integer type whose values fit in i64";

/// A document type summing `amount`, which has a `minimum` and no `maximum`, so it parses as an
/// unsigned 64-bit integer, which a sum tree cannot hold.
fn summed_u64_schema() -> Value {
    platform_value!({
        "type": "object",
        "documentsSummable": "amount",
        "properties": {
            "amount": {
                "type": "integer",
                "minimum": 1,
                "position": 0,
            },
        },
        "required": ["amount"],
        "additionalProperties": false,
    })
}

struct Outcome {
    /// The consensus errors `check_tx` refused the transition with, or its own error.
    check_tx: Result<Vec<ConsensusError>, String>,
    block: StateTransitionExecutionResult,
    nonce_before: Option<u64>,
    nonce_after: Option<u64>,
    balance_before: Option<u64>,
    balance_after: Option<u64>,
}

async fn register_contract_with_schema(
    protocol_version: ProtocolVersion,
    schema: Value,
) -> Outcome {
    let platform_version =
        PlatformVersion::get(protocol_version).expect("expected the protocol version");
    let mut platform = TestPlatformBuilder::new()
        .with_initial_protocol_version(protocol_version)
        .build_with_mock_rpc()
        .set_genesis_state();

    let (identity, signer, key) = setup_identity(&mut platform, 5077, dash_to_credits!(1.0));

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

    // The rule is enforced whenever the contract is parsed, so a contract breaking it cannot be
    // built; the schema is swapped into the serialized contract and the transition signed again.
    match &mut state_transition {
        StateTransition::DataContractCreate(create) => {
            let mut serialized_contract = create.data_contract().clone();
            let schemas = serialized_contract.document_schemas_mut();
            schemas.clear();
            schemas.insert("item".to_string(), schema);
            create.set_data_contract(serialized_contract);
        }
        _ => panic!("expected a data contract create transition"),
    }
    state_transition
        .sign_external(
            &key,
            &signer,
            None::<fn(Identifier, String) -> Result<SecurityLevel, ProtocolError>>,
        )
        .await
        .expect("expected to sign the transition again");
    let transition_bytes = state_transition
        .serialize_to_bytes()
        .expect("expected to serialize the transition");

    let platform_state = platform.state.load();
    let platform_ref = PlatformRef {
        drive: &platform.drive,
        state: &platform_state,
        config: &platform.config,
        core_rpc: &platform.core_rpc,
    };
    let check_tx = platform
        .check_tx(
            &transition_bytes,
            CheckTxLevel::FirstTimeCheck,
            &platform_ref,
            platform_version,
        )
        .map(|result| result.errors)
        .map_err(|error| error.to_string());

    let fetch_nonce_and_balance = || {
        (
            platform
                .drive
                .fetch_identity_nonce(identity.id().to_buffer(), true, None, platform_version)
                .expect("expected to fetch the identity nonce"),
            platform
                .drive
                .fetch_identity_balance(identity.id().to_buffer(), None, platform_version)
                .expect("expected to fetch the identity balance"),
        )
    };
    let (nonce_before, balance_before) = fetch_nonce_and_balance();

    let transaction = platform.drive.grove.start_transaction();
    let processing_result = platform
        .platform
        .process_raw_state_transitions(
            &[transition_bytes],
            &platform_state,
            &BlockInfo::default(),
            &transaction,
            platform_version,
            false,
            None,
        )
        .expect("block processing must return a result for the transition");
    platform
        .drive
        .grove
        .commit_transaction(transaction)
        .unwrap()
        .expect("expected to commit the transaction");

    let (nonce_after, balance_after) = fetch_nonce_and_balance();

    Outcome {
        check_tx,
        block: processing_result
            .execution_results()
            .first()
            .expect("expected one execution result")
            .clone(),
        nonce_before,
        nonce_after,
        balance_before,
        balance_after,
    }
}

#[tokio::test]
async fn should_refuse_a_summed_u64_property_with_a_paid_consensus_error() {
    let outcome = register_contract_with_schema(
        PlatformVersion::latest().protocol_version,
        summed_u64_schema(),
    )
    .await;

    assert_matches!(
        outcome.check_tx.as_deref(),
        Ok([ConsensusError::BasicError(BasicError::ContractError(
            DataContractError::InvalidContractStructure(message)
        ))]) if message.contains(SUMMED_U64_MESSAGE)
    );
    assert_matches!(
        &outcome.block,
        StateTransitionExecutionResult::PaidConsensusError { error: ConsensusError::BasicError(
            BasicError::ContractError(DataContractError::InvalidContractStructure(message))
        ), .. } if message.contains(SUMMED_U64_MESSAGE)
    );
    assert_eq!(outcome.nonce_before, Some(0));
    assert_eq!(
        outcome.nonce_after,
        Some(1),
        "the rejection bumps the identity nonce"
    );
    assert!(
        outcome.balance_after < outcome.balance_before,
        "the rejection is charged: {:?} -> {:?}",
        outcome.balance_before,
        outcome.balance_after
    );
}

#[tokio::test]
async fn should_keep_refusing_a_summed_u64_property_unpaid_at_protocol_version_13() {
    let outcome = register_contract_with_schema(13, summed_u64_schema()).await;

    assert_matches!(
        &outcome.check_tx,
        Err(message) if message.contains(SUMMED_U64_MESSAGE)
    );
    assert_matches!(
        &outcome.block,
        StateTransitionExecutionResult::InternalError(message) if message.contains(SUMMED_U64_MESSAGE)
    );
    assert_eq!(outcome.nonce_after, outcome.nonce_before);
    assert_eq!(outcome.balance_after, outcome.balance_before);
}

/// A document type whose second property sits at a position past `u32::MAX`. Positions are only
/// checked to be continuous under full validation, so `check_tx`, which parses without it, admits
/// the contract, and a block refuses it.
fn position_past_u32_schema() -> Value {
    platform_value!({
        "type": "object",
        "properties": {
            "amount": {"type": "integer", "minimum": 0, "maximum": 1000, "position": 0},
            "label": {
                "type": "string",
                "maxLength": 20,
                "position": u64::from(u32::MAX) + 1,
            },
        },
        "additionalProperties": false,
    })
}

#[tokio::test]
async fn should_refuse_a_position_past_u32_with_a_paid_consensus_error() {
    let outcome = register_contract_with_schema(
        PlatformVersion::latest().protocol_version,
        position_past_u32_schema(),
    )
    .await;

    assert_matches!(outcome.check_tx.as_deref(), Ok([]));
    assert_matches!(
        &outcome.block,
        StateTransitionExecutionResult::PaidConsensusError {
            error: ConsensusError::BasicError(BasicError::ValueError(_)),
            ..
        }
    );
    assert_eq!(outcome.nonce_before, Some(0));
    assert_eq!(
        outcome.nonce_after,
        Some(1),
        "the rejection bumps the identity nonce"
    );
    assert!(
        outcome.balance_after < outcome.balance_before,
        "the rejection is charged: {:?} -> {:?}",
        outcome.balance_before,
        outcome.balance_after
    );
}

#[tokio::test]
async fn should_keep_refusing_a_position_past_u32_unpaid_at_protocol_version_13() {
    let outcome = register_contract_with_schema(13, position_past_u32_schema()).await;

    assert_matches!(outcome.check_tx.as_deref(), Ok([]));
    assert_matches!(
        &outcome.block,
        StateTransitionExecutionResult::InternalError(message)
            if message.contains("integer out of bounds")
    );
    assert_eq!(outcome.nonce_after, outcome.nonce_before);
    assert_eq!(outcome.balance_after, outcome.balance_before);
}

/// A document type whose create cost names its token position as text. `check_tx` parses
/// without the meta-schema, so the parser reads the malformed value; a block runs the
/// meta-schema, which refuses it first.
fn text_token_position_schema() -> Value {
    platform_value!({
        "type": "object",
        "tokenCost": {"create": {"tokenPosition": "first", "amount": 1}},
        "properties": {
            "label": {"type": "string", "maxLength": 20, "position": 0},
        },
        "additionalProperties": false,
    })
}

#[tokio::test]
async fn should_refuse_a_text_token_position_in_check_tx_with_a_consensus_error() {
    let outcome = register_contract_with_schema(
        PlatformVersion::latest().protocol_version,
        text_token_position_schema(),
    )
    .await;

    assert_matches!(
        outcome.check_tx.as_deref(),
        Ok([ConsensusError::BasicError(BasicError::ValueError(_))])
    );
    assert_matches!(
        &outcome.block,
        StateTransitionExecutionResult::PaidConsensusError {
            error: ConsensusError::BasicError(BasicError::JsonSchemaError(_)),
            ..
        }
    );
    assert_eq!(
        outcome.nonce_after,
        Some(1),
        "the rejection bumps the identity nonce"
    );
}

#[tokio::test]
async fn should_keep_failing_check_tx_on_a_text_token_position_at_protocol_version_13() {
    let outcome = register_contract_with_schema(13, text_token_position_schema()).await;

    assert_matches!(
        &outcome.check_tx,
        Err(message) if message.contains("value error")
    );
    assert_matches!(
        &outcome.block,
        StateTransitionExecutionResult::PaidConsensusError {
            error: ConsensusError::BasicError(BasicError::JsonSchemaError(_)),
            ..
        }
    );
}

/// An indexOnly document type whose entry payload is a string of 16384 characters with no
/// `maxBytes`: at four bytes a character, more than a `u16` counts.
fn entry_payload_string_too_long_to_size_schema() -> Value {
    platform_value!({
        "type": "object",
        "indexOnly": true,
        "documentsMutable": false,
        "canBeDeleted": true,
        "indices": [
            {"name": "byRequest", "terminal": ["requestHash", "$ownerId"]},
        ],
        "entryPayload": ["reply"],
        "properties": {
            "requestHash": {
                "type": "array",
                "byteArray": true,
                "minItems": 20,
                "maxItems": 20,
                "position": 0,
            },
            "reply": {"type": "string", "maxLength": 16384, "position": 1},
        },
        "required": ["requestHash", "reply"],
        "additionalProperties": false,
    })
}

/// The fragment of the parser's message for an entry payload property it cannot size.
const ENTRY_PAYLOAD_TOO_LONG_MESSAGE: &str = "may encode to more than 65535 bytes";

#[tokio::test]
async fn should_refuse_an_entry_payload_string_too_long_to_size_with_a_paid_consensus_error() {
    let outcome = register_contract_with_schema(
        PlatformVersion::latest().protocol_version,
        entry_payload_string_too_long_to_size_schema(),
    )
    .await;

    assert_matches!(
        outcome.check_tx.as_deref(),
        Ok([ConsensusError::BasicError(BasicError::ContractError(
            DataContractError::InvalidContractStructure(message)
        ))]) if message.contains(ENTRY_PAYLOAD_TOO_LONG_MESSAGE)
    );
    assert_matches!(
        &outcome.block,
        StateTransitionExecutionResult::PaidConsensusError { error: ConsensusError::BasicError(
            BasicError::ContractError(DataContractError::InvalidContractStructure(message))
        ), .. } if message.contains(ENTRY_PAYLOAD_TOO_LONG_MESSAGE)
    );
    assert_eq!(
        outcome.nonce_after,
        Some(1),
        "the rejection bumps the identity nonce"
    );
    assert!(
        outcome.balance_after < outcome.balance_before,
        "the rejection is charged: {:?} -> {:?}",
        outcome.balance_before,
        outcome.balance_after
    );
}

/// A document type ranking by count a string of 16384 characters with no `maxBytes`. The ranked
/// key length is checked under full validation only, so `check_tx` admits the contract, and a
/// block refuses it.
fn ranked_string_too_long_to_size_schema() -> Value {
    platform_value!({
        "type": "object",
        "properties": {
            "restaurantId": {"type": "string", "maxLength": 16384, "position": 0},
            "grade": {"type": "integer", "minimum": 0, "maximum": 100, "position": 1},
        },
        "required": ["restaurantId", "grade"],
        "additionalProperties": false,
        "indices": [
            {
                "name": "byRestaurant",
                "properties": [{"restaurantId": "asc"}],
                "countable": "countable",
                "rangeCountable": true,
                "rankedCountable": true,
            },
        ],
    })
}

#[tokio::test]
async fn should_refuse_a_ranked_string_too_long_to_size_with_a_paid_consensus_error() {
    let outcome = register_contract_with_schema(
        PlatformVersion::latest().protocol_version,
        ranked_string_too_long_to_size_schema(),
    )
    .await;

    assert_matches!(outcome.check_tx.as_deref(), Ok([]));
    assert_matches!(
        &outcome.block,
        StateTransitionExecutionResult::PaidConsensusError {
            error: ConsensusError::BasicError(
                BasicError::InvalidIndexedPropertyConstraintError(error)
            ),
            ..
        } if error.constraint_name() == "maxLength"
    );
    assert_eq!(
        outcome.nonce_after,
        Some(1),
        "the rejection bumps the identity nonce"
    );
    assert!(
        outcome.balance_after < outcome.balance_before,
        "the rejection is charged: {:?} -> {:?}",
        outcome.balance_before,
        outcome.balance_after
    );
}
