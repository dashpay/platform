//! Runs a data contract create or update carrying a document type that breaks a structural rule
//! of the document type parser through `check_tx` and block processing.
//!
//! From protocol version 14 such a rule is a consensus error: `check_tx` refuses the transition
//! with it, and a block charges the owner and bumps its nonce. At protocol version 13 some of
//! these rules are reported as an error of the parser's own: `check_tx` fails, and a block records
//! an internal error, which leaves the owner untouched and keeps the transition out of any block.

use crate::execution::check_tx::CheckTxLevel;
use crate::platform_types::platform::PlatformRef;
use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
use crate::rpc::core::MockCoreRPCLike;
use crate::test::helpers::setup::TempPlatform;
use assert_matches::assert_matches;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::basic::BasicError;
use dpp::consensus::ConsensusError;
use dpp::data_contract::errors::DataContractError;
use dpp::identity::identity_nonce::IDENTITY_NONCE_VALUE_FILTER;
use dpp::identity::{IdentityPublicKey, SecurityLevel};
use dpp::platform_value::{platform_value, Value};
use dpp::prelude::Identifier;
use dpp::serialization::PlatformSerializable;
use dpp::state_transition::data_contract_create_transition::accessors::DataContractCreateTransitionAccessorsV0;
use dpp::state_transition::data_contract_update_transition::accessors::DataContractUpdateTransitionAccessorsV0;
use dpp::state_transition::StateTransition;
use dpp::ProtocolError;
use platform_version::version::PlatformVersion;
use simple_signer::signer::SimpleSigner;
use std::collections::BTreeMap;

/// The fragment of the parser's message for a summed property that parses as `u64`.
pub(in crate::execution) const SUMMED_U64_MESSAGE: &str =
    "must be an integer type whose values fit in i64";

/// A document type summing `amount`, which has a `minimum` and no `maximum`, so it parses as an
/// unsigned 64-bit integer, which a sum tree cannot hold. The rule is shared with protocol
/// versions 12 and 13.
pub(in crate::execution) fn summed_u64_schema() -> Value {
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

/// The fragment of the parser's message for a `terminal` outside an indexOnly type.
pub(in crate::execution) const TERMINAL_WITHOUT_INDEX_ONLY_MESSAGE: &str =
    "which is only allowed on indexOnly document types";

/// A document type that is not indexOnly but gives an index a `terminal`, one of the indexOnly
/// rules only protocol version 14 has.
pub(in crate::execution) fn terminal_without_index_only_schema() -> Value {
    platform_value!({
        "type": "object",
        "properties": {
            "label": {
                "type": "string",
                "maxLength": 20,
                "position": 0,
            },
        },
        "required": ["label"],
        "indices": [
            {"name": "byLabel", "properties": [{"label": "asc"}], "terminal": "$ownerId"},
        ],
        "additionalProperties": false,
    })
}

/// What `check_tx` and one block made of a transition.
pub(in crate::execution) struct Outcome {
    /// The consensus errors `check_tx` refused the transition with, or its own error.
    pub check_tx: Result<Vec<ConsensusError>, String>,
    pub block: StateTransitionExecutionResult,
    pub nonce_before: Option<u64>,
    pub nonce_after: Option<u64>,
    pub balance_before: Option<u64>,
    pub balance_after: Option<u64>,
}

/// Edits the document schemas of the contract a create or update transition carries and signs
/// the transition again. A rule the parser enforces cannot be broken by a contract built in
/// memory, so the schema goes into the serialized contract instead.
pub(in crate::execution) async fn resign_with_schemas(
    state_transition: &mut StateTransition,
    edit_schemas: impl FnOnce(&mut BTreeMap<String, Value>),
    key: &IdentityPublicKey,
    signer: &SimpleSigner,
) -> Vec<u8> {
    match state_transition {
        StateTransition::DataContractCreate(create) => {
            let mut serialized_contract = create.data_contract().clone();
            edit_schemas(serialized_contract.document_schemas_mut());
            create.set_data_contract(serialized_contract);
        }
        StateTransition::DataContractUpdate(update) => {
            let mut serialized_contract = update.data_contract().clone();
            edit_schemas(serialized_contract.document_schemas_mut());
            update.set_data_contract(serialized_contract);
        }
        _ => panic!("expected a data contract create or update transition"),
    }
    state_transition
        .sign_external(
            key,
            signer,
            None::<fn(Identifier, String) -> Result<SecurityLevel, ProtocolError>>,
        )
        .await
        .expect("expected to sign the transition again");
    state_transition
        .serialize_to_bytes()
        .expect("expected to serialize the transition")
}

/// Runs the transition through `check_tx`, then through one block, reading the nonce with
/// `fetch_nonce` and the owner's balance before and after the block.
pub(in crate::execution) fn check_and_process(
    platform: &TempPlatform<MockCoreRPCLike>,
    owner_id: Identifier,
    transition_bytes: Vec<u8>,
    fetch_nonce: impl Fn(&TempPlatform<MockCoreRPCLike>) -> Option<u64>,
    platform_version: &PlatformVersion,
) -> Outcome {
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

    let fetch_balance = || {
        platform
            .drive
            .fetch_identity_balance(owner_id.to_buffer(), None, platform_version)
            .expect("expected to fetch the owner's balance")
    };
    let nonce_before = fetch_nonce(platform);
    let balance_before = fetch_balance();

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

    Outcome {
        check_tx,
        block: processing_result
            .execution_results()
            .first()
            .expect("expected one execution result")
            .clone(),
        nonce_before,
        nonce_after: fetch_nonce(platform),
        balance_before,
        balance_after: fetch_balance(),
    }
}

/// A paid rejection: `check_tx` refuses the transition with the parser's
/// `InvalidContractStructure`, and the block charges the owner for it and bumps the nonce to
/// `bumped_nonce`, the nonce the transition was signed with.
pub(in crate::execution) fn assert_paid_contract_structure_error(
    outcome: &Outcome,
    needle: &str,
    bumped_nonce: u64,
) {
    assert_matches!(
        outcome.check_tx.as_deref(),
        Ok([ConsensusError::BasicError(BasicError::ContractError(
            DataContractError::InvalidContractStructure(message)
        ))]) if message.contains(needle),
        "check_tx: {:?}",
        outcome.check_tx
    );
    assert_matches!(
        &outcome.block,
        StateTransitionExecutionResult::PaidConsensusError { error: ConsensusError::BasicError(
            BasicError::ContractError(DataContractError::InvalidContractStructure(message))
        ), .. } if message.contains(needle),
        "block: {:?}",
        outcome.block
    );
    // The stored nonce keeps the nonces skipped below it in its high bits.
    assert_eq!(
        outcome
            .nonce_after
            .map(|nonce| nonce & IDENTITY_NONCE_VALUE_FILTER),
        Some(bumped_nonce),
        "the rejection bumps the nonce"
    );
    assert!(
        outcome.balance_after < outcome.balance_before,
        "the rejection is charged: {:?} -> {:?}",
        outcome.balance_before,
        outcome.balance_after
    );
}

/// An unpaid refusal: `check_tx` fails with the parser's own error, and the block records an
/// internal error and leaves the owner untouched.
pub(in crate::execution) fn assert_unpaid_internal_error(outcome: &Outcome, needle: &str) {
    assert_matches!(
        &outcome.check_tx,
        Err(message) if message.contains(needle),
        "check_tx: {:?}",
        outcome.check_tx
    );
    assert_matches!(
        &outcome.block,
        StateTransitionExecutionResult::InternalError(message) if message.contains(needle),
        "block: {:?}",
        outcome.block
    );
    assert_eq!(outcome.nonce_after, outcome.nonce_before);
    assert_eq!(outcome.balance_after, outcome.balance_before);
}
