//! A Batch naming a private document type (protocol version 15). A private type's documents
//! are encrypted entries only the shielded transition appends, so the Batch is refused as soon
//! as the type is resolved, before any document is read or written.

use super::*;
use crate::execution::check_tx::CheckTxLevel::FirstTimeCheck;
use crate::platform_types::platform::PlatformRef;
use dpp::consensus::basic::BasicError;
use dpp::data_contract::config::moderation::{
    ContractModerationConfig, ContractModerationList, ContractModerationReason, ContractModerators,
};
use dpp::data_contract::config::DataContractConfig;
use dpp::data_contract::conversion::json::DataContractJsonConversionMethodsV0;
use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
use dpp::data_contract::DataContract;
use dpp::document::{Document, DocumentV0};
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::platform_value::string_encoding::Encoding;
use dpp::state_transition::contract_user_moderation_transition::methods::ContractUserModerationTransitionMethodsV0;
use dpp::state_transition::contract_user_moderation_transition::{
    ContractUserModerationAction, ContractUserModerationTransition,
};
use dpp::tests::json_document::json_document_to_json_value;
use drive::drive::RootTree;
use drive::grovedb::Element;
use drive::grovedb_path::SubtreePath;
use simple_signer::signer::SimpleSigner;
use std::collections::BTreeMap;

/// A contract whose only type, `secret`, is private, with one plaintext property, `note`.
const PRIVATE_CONTRACT_PATH: &str =
    "tests/supporting_files/contract/reference-validation/reference-validation-contract-private-foreign.json";

/// The create, signed by `identity` with contract nonce 1, of a `secret` document holding a
/// note. Built by hand: the type names no system field, so the document carries only its note.
async fn secret_create(
    contract: &DataContract,
    identity: &Identity,
    key: &IdentityPublicKey,
    signer: &SimpleSigner,
    platform_version: &PlatformVersion,
) -> Vec<u8> {
    let secret_type = contract
        .document_type_for_name("secret")
        .expect("expected the secret type");
    let entropy = [7u8; 32];
    let mut document = Document::V0(DocumentV0 {
        owner_id: identity.id(),
        properties: BTreeMap::from([("note".to_string(), Value::Text("hello".to_string()))]),
        ..Default::default()
    });
    document
        .set_id_for_creation(secret_type, &entropy, 1, platform_version)
        .expect("expected to set the document id");
    BatchTransition::new_document_creation_transition_from_document(
        document,
        secret_type,
        entropy,
        key,
        1,
        0,
        None,
        signer,
        platform_version,
        None,
    )
    .await
    .expect("expected the create transition")
    .serialize_to_bytes()
    .expect("expected to serialize the create transition")
}

/// What check_tx says about `raw`, then the result of a block processing it, committed.
fn check_tx_and_process(
    platform: &TempPlatform<MockCoreRPCLike>,
    raw: Vec<u8>,
    platform_version: &PlatformVersion,
) -> (Vec<ConsensusError>, StateTransitionExecutionResult) {
    let platform_state = platform.state.load();
    let platform_ref = PlatformRef {
        drive: &platform.drive,
        state: &platform_state,
        config: &platform.config,
        core_rpc: &platform.core_rpc,
    };
    let check_tx_errors = platform
        .check_tx(&raw, FirstTimeCheck, &platform_ref, platform_version)
        .expect("expected check_tx to return a result")
        .errors;

    let transaction = platform.drive.grove.start_transaction();
    let processing_result = platform
        .platform
        .process_raw_state_transitions(
            &[raw],
            &platform_state,
            &BlockInfo::default(),
            &transaction,
            platform_version,
            false,
            None,
        )
        .expect("expected to process the transition");
    platform
        .drive
        .grove
        .commit_transaction(transaction)
        .unwrap()
        .expect("expected to commit the transaction");

    let mut execution_results = processing_result.into_execution_results();
    assert_eq!(execution_results.len(), 1, "{execution_results:?}");
    (check_tx_errors, execution_results.remove(0))
}

fn balance(
    platform: &TempPlatform<MockCoreRPCLike>,
    identity_id: Identifier,
    platform_version: &PlatformVersion,
) -> Credits {
    platform
        .drive
        .fetch_identity_balance(identity_id.to_buffer(), None, platform_version)
        .expect("expected to fetch the balance")
        .expect("expected the identity to have a balance")
}

/// The `secret` documents stored under `contract`.
fn stored_secrets(
    platform: &TempPlatform<MockCoreRPCLike>,
    contract: &DataContract,
    platform_version: &PlatformVersion,
) -> Vec<Document> {
    let query = DriveDocumentQuery::from_sql_expr(
        "select * from secret",
        contract,
        Some(&platform.config.drive),
        platform_version,
    )
    .expect("expected the query");
    platform
        .drive
        .query_documents(
            query,
            None,
            false,
            None,
            Some(platform_version.protocol_version),
        )
        .expect("expected to query the type")
        .documents()
        .to_vec()
}

/// How many entries the store of `secret` holds: its key `[0]` holds the type's private document
/// store, which no document query reads.
fn secret_store_entries(
    platform: &TempPlatform<MockCoreRPCLike>,
    contract: &DataContract,
    platform_version: &PlatformVersion,
) -> u64 {
    let contract_id = contract.id();
    let type_path: [&[u8]; 4] = [
        &[RootTree::DataContractDocuments as u8],
        contract_id.as_bytes(),
        &[1],
        b"secret",
    ];
    let element = platform
        .drive
        .grove
        .get_raw(
            SubtreePath::from(&type_path[..]),
            &[0],
            None,
            &platform_version.drive.grove_version,
        )
        .unwrap()
        .expect("expected the store of the private type");
    let Element::PrivateDocumentStore(entries, ..) = element else {
        panic!("expected a private document store at the type's key [0], got {element:?}");
    };
    entries
}

/// Asserts that check_tx and the block both refused a Batch on `secret` of `contract` with
/// `DocumentTypePrivateError`, unpaid: the signer's balance is `balance_before`, it has no
/// nonce for the contract, and nothing is written to the type.
fn assert_refused_as_private(
    platform: &TempPlatform<MockCoreRPCLike>,
    contract: &DataContract,
    identity_id: Identifier,
    balance_before: Credits,
    (check_tx_errors, execution_result): (Vec<ConsensusError>, StateTransitionExecutionResult),
    platform_version: &PlatformVersion,
) {
    assert_matches!(
        check_tx_errors.as_slice(),
        [ConsensusError::BasicError(BasicError::DocumentTypePrivateError(e))]
            if e.document_type_name() == "secret" && e.data_contract_id() == contract.id(),
        "check_tx: {check_tx_errors:?}"
    );

    // Refused unpaid, as a type the contract does not have is (`InvalidDocumentTypeError`): the
    // refusal carries no action to charge, and check_tx refuses the batch first
    assert_matches!(
        &execution_result,
        StateTransitionExecutionResult::UnpaidConsensusError(
            ConsensusError::BasicError(BasicError::DocumentTypePrivateError(e))
        ) if e.document_type_name() == "secret" && e.data_contract_id() == contract.id(),
        "block: {execution_result:?}"
    );
    assert_eq!(
        balance(platform, identity_id, platform_version),
        balance_before
    );
    assert_eq!(
        platform
            .drive
            .fetch_identity_contract_nonce(
                identity_id.to_buffer(),
                contract.id().to_buffer(),
                true,
                None,
                platform_version,
            )
            .expect("expected to fetch the contract nonce"),
        None,
        "the refusal records no contract nonce"
    );

    // Nothing is written to the type: its store holds no entry
    assert_eq!(
        secret_store_entries(platform, contract, platform_version),
        0,
        "nothing is appended to the store"
    );
}

#[tokio::test]
async fn should_refuse_a_batch_create_on_a_private_document_type() {
    let platform_version = PlatformVersion::latest();
    let mut platform = TestPlatformBuilder::new()
        .build_with_mock_rpc()
        .set_initial_state_structure();
    let (identity, signer, key) = setup_identity(&mut platform, 967, dash_to_credits!(0.5));

    let contract = json_document_to_contract(PRIVATE_CONTRACT_PATH, true, platform_version)
        .expect("expected the private contract to pass full validation");
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
    assert!(contract
        .document_type_for_name("secret")
        .expect("expected the secret type")
        .is_private());

    let balance_before = balance(&platform, identity.id(), platform_version);
    let raw = secret_create(&contract, &identity, &key, &signer, platform_version).await;
    let outcome = check_tx_and_process(&platform, raw, platform_version);
    assert_refused_as_private(
        &platform,
        &contract,
        identity.id(),
        balance_before,
        outcome,
        platform_version,
    );
}

/// A signer banned on a moderated contract is refused a Batch on its private type as any
/// signer is: unpaid, with `DocumentTypePrivateError`, before the moderation gate could charge
/// it with `ContractUserBannedError`. Its create on the contract's public type shows the ban
/// is in force.
#[tokio::test]
async fn should_refuse_a_banned_signers_batch_on_a_private_type_as_private_rather_than_banned() {
    let platform_version = PlatformVersion::latest();
    let mut platform = TestPlatformBuilder::new()
        .build_with_mock_rpc()
        .set_initial_state_structure();
    let (owner, owner_signer, owner_key) =
        setup_identity(&mut platform, 968, dash_to_credits!(0.5));
    let (identity, signer, key) = setup_identity(&mut platform, 967, dash_to_credits!(0.5));

    // The private contract, owned by `owner`, which moderates its banlist, with a public type
    // `memo` beside `secret`
    let mut contract_json = json_document_to_json_value(PRIVATE_CONTRACT_PATH)
        .expect("expected to read the private contract");
    contract_json["ownerId"] = serde_json::json!(owner.id().to_string(Encoding::Base58));
    contract_json["documentSchemas"]["memo"] = serde_json::json!({
        "type": "object",
        "properties": {
            "text": { "type": "string", "maxLength": 63, "position": 0 }
        },
        "required": ["text"],
        "additionalProperties": false
    });
    contract_json["config"] = serde_json::to_value(
        DataContractConfig::default_for_version(platform_version)
            .expect("expected the default config")
            .with_moderation(Some(ContractModerationConfig {
                banlist: true,
                suspensions: false,
                warnings: false,
                moderators: ContractModerators::ContractOwner,
            })),
    )
    .expect("expected the config to serialize");
    let contract = DataContract::from_json(contract_json, true, platform_version)
        .expect("expected the moderated private contract to pass full validation");
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
    assert!(contract
        .document_type_for_name("secret")
        .expect("expected the secret type")
        .is_private());

    // The owner bans the identity
    let ban = ContractUserModerationTransition::try_from_identity_with_signer(
        &owner,
        &owner_key.id(),
        contract.id(),
        ContractUserModerationAction::Ban {
            identity_id: identity.id(),
            reason: ContractModerationReason::from_text("spam"),
        },
        1,
        0,
        &owner_signer,
        platform_version,
        None,
    )
    .await
    .expect("expected the ban transition")
    .serialize_to_bytes()
    .expect("expected to serialize the ban transition");
    let (_, ban_result) = check_tx_and_process(&platform, ban, platform_version);
    assert_matches!(
        ban_result,
        StateTransitionExecutionResult::SuccessfulExecution { .. },
        "ban: {ban_result:?}"
    );
    assert!(
        platform
            .drive
            .fetch_contract_moderation_status(
                contract.id(),
                identity.id(),
                &[ContractModerationList::Banlist],
                None,
                platform_version,
            )
            .expect("expected to fetch the moderation status")
            .ban
            .is_some(),
        "the identity is banned"
    );

    let balance_before = balance(&platform, identity.id(), platform_version);
    let raw = secret_create(&contract, &identity, &key, &signer, platform_version).await;
    let outcome = check_tx_and_process(&platform, raw, platform_version);
    assert_refused_as_private(
        &platform,
        &contract,
        identity.id(),
        balance_before,
        outcome,
        platform_version,
    );

    // The same signer's create on `memo`, with the contract nonce the refusal left unused,
    // meets the moderation gate, which charges it
    let memo_type = contract
        .document_type_for_name("memo")
        .expect("expected the memo type");
    let mut rng = StdRng::seed_from_u64(967);
    let entropy = Bytes32::random_with_rng(&mut rng);
    let mut memo = memo_type
        .random_document_with_identifier_and_entropy(
            &mut rng,
            identity.id(),
            entropy,
            DocumentFieldFillType::FillIfNotRequired,
            DocumentFieldFillSize::MinDocumentFillSize,
            platform_version,
        )
        .expect("expected a random memo");
    memo.set_id_for_creation(memo_type, &entropy.0, 1, platform_version)
        .expect("expected to set the document id");
    let memo_create = BatchTransition::new_document_creation_transition_from_document(
        memo,
        memo_type,
        entropy.0,
        &key,
        1,
        0,
        None,
        &signer,
        platform_version,
        None,
    )
    .await
    .expect("expected the memo create transition")
    .serialize_to_bytes()
    .expect("expected to serialize the memo create transition");
    let (check_tx_errors, memo_result) =
        check_tx_and_process(&platform, memo_create, platform_version);
    assert_matches!(
        check_tx_errors.as_slice(),
        [ConsensusError::StateError(
            StateError::ContractUserBannedError(_)
        )],
        "check_tx: {check_tx_errors:?}"
    );
    assert_matches!(
        memo_result,
        StateTransitionExecutionResult::PaidConsensusError {
            error: ConsensusError::StateError(StateError::ContractUserBannedError(_)),
            ..
        },
        "block: {memo_result:?}"
    );
}

/// Protocol version 14 has no private types: its parse switch, `parse_private_document`, is
/// unset, so a contract stored with the key parses with `secret` public there, and the Batch
/// transformer, edited in place, creates its documents as before the keyword existed.
#[tokio::test]
async fn should_let_a_batch_create_on_a_stored_type_carrying_the_private_key_at_protocol_version_14(
) {
    let platform_version = PlatformVersion::get(14).expect("expected protocol version 14");
    let mut platform = TestPlatformBuilder::new()
        .with_initial_protocol_version(14)
        .build_with_mock_rpc()
        .set_initial_state_structure();
    let (identity, signer, key) = setup_identity(&mut platform, 967, dash_to_credits!(0.5));

    // The stored path: parsed without validation, as a contract read back from state is
    let mut contract_json = json_document_to_json_value(PRIVATE_CONTRACT_PATH)
        .expect("expected to read the private contract");
    contract_json["config"] = serde_json::to_value(
        DataContractConfig::default_for_version(platform_version)
            .expect("expected the default config"),
    )
    .expect("expected the config to serialize");
    let contract = DataContract::from_json(contract_json, false, platform_version)
        .expect("expected the contract to parse without validation");
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

    // Read back from storage, not from the cache the write filled
    platform.drive.cache.data_contracts.clear();
    let stored = platform
        .drive
        .fetch_contract(
            contract.id().to_buffer(),
            None,
            None,
            None,
            platform_version,
        )
        .unwrap()
        .expect("expected to fetch the contract")
        .expect("expected the contract to exist");
    let stored_secret = stored
        .contract
        .document_type_for_name("secret")
        .expect("expected the secret type");
    assert!(
        stored_secret
            .schema()
            .get_optional_value("private")
            .expect("expected the schema to be a map")
            .is_some(),
        "the stored schema keeps the key"
    );
    assert!(!stored_secret.is_private());

    let raw = secret_create(&stored.contract, &identity, &key, &signer, platform_version).await;
    let (check_tx_errors, execution_result) =
        check_tx_and_process(&platform, raw, platform_version);
    assert!(check_tx_errors.is_empty(), "check_tx: {check_tx_errors:?}");
    assert_matches!(
        execution_result,
        StateTransitionExecutionResult::SuccessfulExecution { .. },
        "block: {execution_result:?}"
    );

    let stored_documents = stored_secrets(&platform, &stored.contract, platform_version);
    assert_matches!(
        stored_documents.as_slice(),
        [document] if document.get("note") == Some(&Value::Text("hello".to_string())),
        "stored: {stored_documents:?}"
    );
}
