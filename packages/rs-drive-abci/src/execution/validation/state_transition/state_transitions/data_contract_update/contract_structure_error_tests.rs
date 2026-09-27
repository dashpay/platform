//! An update adding a document type the parser refuses with an error that is not a consensus
//! error at protocol version 13, through `check_tx` and block processing: one breaking a
//! doctype-level aggregate rule, one with a value the parser cannot read, and, for the keywords
//! protocol version 14 introduces, one whose string is too long for the parser to size.
//!
//! From protocol version 14 each is a consensus error: `check_tx` refuses the update with it
//! where `check_tx` parses that far, and a block charges the owner and bumps its contract nonce.
//! At protocol version 13 the parser reports it as an error of its own: `check_tx` fails, and a
//! block records an internal error, which leaves the owner untouched and keeps the update out of
//! any block.

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
use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
use dpp::data_contract::errors::DataContractError;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::SecurityLevel;
use dpp::platform_value::{platform_value, Value};
use dpp::prelude::Identifier;
use dpp::serialization::PlatformSerializable;
use dpp::state_transition::data_contract_update_transition::accessors::DataContractUpdateTransitionAccessorsV0;
use dpp::state_transition::data_contract_update_transition::methods::DataContractUpdateTransitionMethodsV0;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::state_transition::StateTransition;
use dpp::tests::fixtures::get_data_contract_fixture;
use dpp::ProtocolError;
use drive::util::storage_flags::StorageFlags;
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
    /// The consensus errors `check_tx` refused the update with, or its own error.
    check_tx: Result<Vec<ConsensusError>, String>,
    block: StateTransitionExecutionResult,
    nonce_before: Option<u64>,
    nonce_after: Option<u64>,
    balance_before: Option<u64>,
    balance_after: Option<u64>,
}

async fn update_contract_adding_schema(
    protocol_version: ProtocolVersion,
    schema: Value,
) -> Outcome {
    let platform_version =
        PlatformVersion::get(protocol_version).expect("expected the protocol version");
    let mut platform = TestPlatformBuilder::new()
        .with_initial_protocol_version(protocol_version)
        .build_with_mock_rpc()
        .set_genesis_state();

    let (identity, signer, key) = setup_identity(&mut platform, 5078, dash_to_credits!(1.0));

    let mut data_contract =
        get_data_contract_fixture(None, 0, platform_version.protocol_version).data_contract_owned();
    data_contract.set_owner_id(identity.id());
    platform
        .drive
        .apply_contract(
            &data_contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            platform_version,
        )
        .expect("expected to apply the contract");

    let mut updated_data_contract = data_contract.clone();
    updated_data_contract.set_version(2);

    let mut state_transition = DataContractUpdateTransition::new_from_data_contract(
        updated_data_contract,
        &identity.clone().into_partial_identity_info(),
        key.id(),
        2,
        0,
        &signer,
        platform_version,
        None,
    )
    .await
    .expect("expected to create the contract update transition");

    // The rule is enforced whenever the contract is parsed, so a contract breaking it cannot be
    // built; the new document type is added to the serialized contract and the transition signed
    // again.
    match &mut state_transition {
        StateTransition::DataContractUpdate(update) => {
            let mut serialized_contract = update.data_contract().clone();
            serialized_contract
                .document_schemas_mut()
                .insert("item".to_string(), schema);
            update.set_data_contract(serialized_contract);
        }
        _ => panic!("expected a data contract update transition"),
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
                .fetch_identity_contract_nonce(
                    identity.id().to_buffer(),
                    data_contract.id().to_buffer(),
                    true,
                    None,
                    platform_version,
                )
                .expect("expected to fetch the contract nonce"),
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
async fn should_refuse_an_update_adding_a_summed_u64_property_with_a_paid_consensus_error() {
    let outcome = update_contract_adding_schema(
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
    assert_ne!(
        outcome.nonce_after, outcome.nonce_before,
        "the rejection bumps the contract nonce"
    );
    assert!(
        outcome.balance_after < outcome.balance_before,
        "the rejection is charged: {:?} -> {:?}",
        outcome.balance_before,
        outcome.balance_after
    );
}

#[tokio::test]
async fn should_keep_refusing_an_update_adding_a_summed_u64_property_unpaid_at_protocol_version_13()
{
    let outcome = update_contract_adding_schema(13, summed_u64_schema()).await;

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
/// the update, and a block refuses it.
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
async fn should_refuse_an_update_adding_a_position_past_u32_with_a_paid_consensus_error() {
    let outcome = update_contract_adding_schema(
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
    assert_ne!(
        outcome.nonce_after, outcome.nonce_before,
        "the rejection bumps the contract nonce"
    );
    assert!(
        outcome.balance_after < outcome.balance_before,
        "the rejection is charged: {:?} -> {:?}",
        outcome.balance_before,
        outcome.balance_after
    );
}

#[tokio::test]
async fn should_keep_refusing_an_update_adding_a_position_past_u32_unpaid_at_protocol_version_13() {
    let outcome = update_contract_adding_schema(13, position_past_u32_schema()).await;

    assert_matches!(outcome.check_tx.as_deref(), Ok([]));
    assert_matches!(
        &outcome.block,
        StateTransitionExecutionResult::InternalError(message)
            if message.contains("integer out of bounds")
    );
    assert_eq!(outcome.nonce_after, outcome.nonce_before);
    assert_eq!(outcome.balance_after, outcome.balance_before);
}

/// The fragment of the parser's message for an entry payload property it cannot size.
const ENTRY_PAYLOAD_TOO_LONG_MESSAGE: &str = "may encode to more than 65535 bytes";

#[tokio::test]
async fn should_refuse_an_update_adding_an_entry_payload_string_too_long_to_size_with_a_paid_consensus_error(
) {
    // An indexOnly document type whose entry payload is a string of 16384 characters with no
    // `maxBytes`: at four bytes a character, more than a `u16` counts.
    let schema = platform_value!({
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
    });
    let outcome =
        update_contract_adding_schema(PlatformVersion::latest().protocol_version, schema).await;

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
    assert_ne!(
        outcome.nonce_after, outcome.nonce_before,
        "the rejection bumps the contract nonce"
    );
    assert!(
        outcome.balance_after < outcome.balance_before,
        "the rejection is charged: {:?} -> {:?}",
        outcome.balance_before,
        outcome.balance_after
    );
}
