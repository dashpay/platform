//! Signed contracts carrying a combination the lifecycle refuses.
//!
//! The parser cannot produce a `DataContract` holding one, so these build a
//! valid contract, rewrite the schema the transition carries on the wire, and
//! re-sign — which is all a hostile client has to do. What the tests pin is the
//! error category: only a consensus error becomes a paid rejection with a nonce
//! bump, while a bare data-contract error escapes as an internal execution
//! error that costs the submitter nothing.

use super::*;
use crate::rpc::core::MockCoreRPCLike;
use crate::test::helpers::setup::TempPlatform;
use dpp::data_contract::accessors::v0::DataContractV0Setters;
use dpp::data_contract::DataContractFactory;
use dpp::identifier::Identifier;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::SecurityLevel;
use dpp::platform_value::platform_value;
use dpp::prelude::IdentityNonce;
use dpp::state_transition::data_contract_create_transition::methods::DataContractCreateTransitionMethodsV0;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::state_transition::data_contract_update_transition::methods::DataContractUpdateTransitionMethodsV0;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use drive::util::storage_flags::StorageFlags;

/// A keep-history type the parser does admit, which every test below starts
/// from before rewriting the schema on the wire.
fn admissible_schema() -> Value {
    platform_value!({
        "type": "object",
        "documentsKeepHistory": true,
        "documentsMutable": true,
        "canBeDeleted": true,
        "properties": {
            "message": {"type": "string", "maxLength": 64, "position": 0},
        },
        "required": ["message"],
        "additionalProperties": false,
    })
}

/// Erasure asked for by a type with no history to erase.
fn erasure_without_history() -> Value {
    platform_value!({
        "type": "object",
        "documentsKeepHistory": false,
        "documentsMutable": true,
        "canBeDeleted": true,
        "canBeErased": true,
        "properties": {
            "message": {"type": "string", "maxLength": 64, "position": 0},
        },
        "required": ["message"],
        "additionalProperties": false,
    })
}

/// A contested resource whose type also retains history.
fn contested_keep_history() -> Value {
    platform_value!({
        "type": "object",
        "documentsKeepHistory": true,
        "documentsMutable": false,
        "canBeDeleted": true,
        "indices": [
            {
                "name": "byMessage",
                "properties": [{"message": "asc"}],
                "unique": true,
                "contested": {
                    "fieldMatches": [{"field": "message", "regexPattern": "^[a-z]{3,10}$"}],
                    "resolution": 0,
                },
            },
        ],
        "properties": {
            "message": {"type": "string", "maxLength": 64, "position": 0},
        },
        "required": ["message"],
        "additionalProperties": false,
    })
}

async fn refused_create(
    schema: Value,
) -> (TempPlatform<MockCoreRPCLike>, Identifier, IdentityNonce) {
    let platform_version = PlatformVersion::get(14).expect("protocol 14 exists");
    let mut platform = TestPlatformBuilder::new()
        .with_initial_protocol_version(14)
        .build_with_mock_rpc()
        .set_genesis_state();
    let platform_state = platform.state.load();
    let (identity, signer, key) = setup_identity(&mut platform, 4001, dash_to_credits!(10.0));

    let contract = DataContractFactory::new(14)
        .expect("expected a contract factory")
        .create_with_value_config(
            identity.id(),
            1,
            platform_value!({ "note": admissible_schema() }),
            None,
            None,
        )
        .expect("the admissible type must parse")
        .data_contract_owned();

    let mut transition = DataContractCreateTransition::new_from_data_contract(
        contract,
        1,
        &identity.clone().into_partial_identity_info(),
        key.id(),
        &signer,
        platform_version,
        None,
    )
    .await
    .expect("expected a contract create transition");

    // Rewrite the schema the wire carries; the parser refuses to build one.
    let StateTransition::DataContractCreate(DataContractCreateTransition::V0(create)) =
        &mut transition
    else {
        panic!("expected a v0 contract create");
    };
    create
        .data_contract
        .document_schemas_mut()
        .insert("note".to_string(), schema);
    transition
        .sign_external(
            &key,
            &signer,
            None::<fn(Identifier, String) -> Result<SecurityLevel, ProtocolError>>,
        )
        .await
        .expect("expected to re-sign the rewritten contract");

    let transaction = platform.drive.grove.start_transaction();
    let processing_result = platform
        .platform
        .process_raw_state_transitions(
            &[transition.serialize_to_bytes().expect("serialized")],
            &platform_state,
            &BlockInfo::default(),
            &transaction,
            platform_version,
            false,
            None,
        )
        .expect("expected transition processing");
    platform
        .drive
        .grove
        .commit_transaction(transaction)
        .unwrap()
        .unwrap();

    assert_eq!(
        processing_result.invalid_paid_count(),
        1,
        "a refused contract must cost its submitter, not vanish into an internal error"
    );
    assert_matches!(
        processing_result.execution_results().as_slice(),
        [StateTransitionExecutionResult::PaidConsensusError { .. }]
    );

    let nonce = platform
        .drive
        .fetch_identity_nonce(identity.id().to_buffer(), true, None, platform_version)
        .expect("expected to read the identity nonce")
        .expect("the nonce must be persisted");

    (platform, identity.id(), nonce)
}

#[tokio::test]
async fn should_charge_for_a_signed_contract_asking_for_erasure_without_history() {
    let (_platform, _identity, nonce) = refused_create(erasure_without_history()).await;
    assert_eq!(
        nonce, 1,
        "the refusal must persist the nonce bump, so the same transition cannot be replayed"
    );
}

#[tokio::test]
async fn should_charge_for_a_signed_contested_keep_history_contract() {
    let (_platform, _identity, nonce) = refused_create(contested_keep_history()).await;
    assert_eq!(nonce, 1, "the refusal must persist the nonce bump");
}

/// The same refusal on the update path, which has its own error-category split.
#[tokio::test]
async fn should_charge_for_a_signed_contract_update_asking_for_erasure_without_history() {
    let platform_version = PlatformVersion::get(14).expect("protocol 14 exists");
    let mut platform = TestPlatformBuilder::new()
        .with_initial_protocol_version(14)
        .build_with_mock_rpc()
        .set_genesis_state();
    let platform_state = platform.state.load();
    let (identity, signer, key) = setup_identity(&mut platform, 4002, dash_to_credits!(1.0));

    let mut contract = DataContractFactory::new(14)
        .expect("expected a contract factory")
        .create_with_value_config(
            identity.id(),
            1,
            platform_value!({ "note": admissible_schema() }),
            None,
            None,
        )
        .expect("the admissible type must parse")
        .data_contract_owned();
    platform
        .drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            platform_version,
        )
        .expect("expected to apply the contract");

    let contract_id = contract.id();
    contract.set_version(2);
    let mut transition = DataContractUpdateTransition::new_from_data_contract(
        contract,
        &identity.clone().into_partial_identity_info(),
        key.id(),
        1,
        0,
        &signer,
        platform_version,
        None,
    )
    .await
    .expect("expected a contract update transition");

    let StateTransition::DataContractUpdate(DataContractUpdateTransition::V0(update)) =
        &mut transition
    else {
        panic!("expected a v0 contract update");
    };
    update
        .data_contract
        .document_schemas_mut()
        .insert("note".to_string(), erasure_without_history());
    transition
        .sign_external(
            &key,
            &signer,
            None::<fn(Identifier, String) -> Result<SecurityLevel, ProtocolError>>,
        )
        .await
        .expect("expected to re-sign the rewritten update");

    let transaction = platform.drive.grove.start_transaction();
    let processing_result = platform
        .platform
        .process_raw_state_transitions(
            &[transition.serialize_to_bytes().expect("serialized")],
            &platform_state,
            &BlockInfo::default(),
            &transaction,
            platform_version,
            false,
            None,
        )
        .expect("expected transition processing");
    platform
        .drive
        .grove
        .commit_transaction(transaction)
        .unwrap()
        .unwrap();

    assert_eq!(processing_result.invalid_paid_count(), 1);
    assert_matches!(
        processing_result.execution_results().as_slice(),
        [StateTransitionExecutionResult::PaidConsensusError { .. }]
    );

    let nonce = platform
        .drive
        .fetch_identity_contract_nonce(
            identity.id().to_buffer(),
            contract_id.to_buffer(),
            true,
            None,
            platform_version,
        )
        .expect("expected to read the identity contract nonce");
    assert_eq!(
        nonce,
        Some(1),
        "the refusal must persist the contract nonce bump"
    );
}
