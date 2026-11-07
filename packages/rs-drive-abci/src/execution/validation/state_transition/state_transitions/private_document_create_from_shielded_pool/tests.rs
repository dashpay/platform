use crate::execution::validation::state_transition::state_transitions::shielded_common::compute_platform_sighash;
use crate::execution::validation::state_transition::state_transitions::test_helpers::{
    create_dummy_serialized_action, get_proving_key, insert_anchor_into_state,
    insert_dummy_encrypted_notes, process_transition, serialize_authorized_bundle_i64,
    set_pool_total_balance, setup_platform, setup_platform_at_protocol_version,
};
use crate::execution::validation::state_transition::state_transitions::tests::process_state_transitions;
use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
use crate::rpc::core::MockCoreRPCLike;
use crate::test::helpers::setup::TempPlatform;
use assert_matches::assert_matches;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::basic::BasicError;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::ConsensusError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
use dpp::data_contract::DataContract;
use dpp::platform_value::string_encoding::Encoding;
use dpp::platform_value::{BinaryData, Identifier};
use dpp::serialization::PlatformSerializable;
use dpp::shielded::{
    compute_private_document_create_from_shielded_pool_fee,
    private_document_create_from_shielded_pool_extra_sighash_data_v0, SerializedAction,
};
use dpp::state_transition::private_document_create_from_shielded_pool_transition::v0::PrivateDocumentCreateFromShieldedPoolTransitionV0;
use dpp::state_transition::private_document_create_from_shielded_pool_transition::PrivateDocumentCreateFromShieldedPoolTransition;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::state_transition::StateTransition;
use drive::drive::Drive;
use grovedb_commitment_tree::{
    Builder, BundleType, ClientMemoryCommitmentTree, DashMemo, ExtractedNoteCommitment,
    FullViewingKey, Note, NoteValue, Position, RandomSeed, Retention, Rho, Scope,
    SpendAuthorizingKey, SpendingKey,
};
use platform_version::version::PlatformVersion;
use rand::rngs::StdRng;
use rand::SeedableRng;
use serde_json::json;

const NOTE_VALUE: u64 = 5_000_000_000;
const CONTRACT_ID: [u8; 32] = [7; 32];
const ENTRY_SIZE: u32 = 24;
/// A bundle with one spend and one change output carries two actions.
const BUNDLE_ACTIONS: usize = 2;

fn entries(count: u8, size: u32) -> Vec<BinaryData> {
    (0..count)
        .map(|i| BinaryData::new(vec![i + 1; size as usize]))
        .collect()
}

fn fee(entries: &[BinaryData]) -> u64 {
    compute_private_document_create_from_shielded_pool_fee(
        BUNDLE_ACTIONS,
        entries.len(),
        entries.first().map(|e| e.len() as u32).unwrap_or(0),
        PlatformVersion::latest(),
    )
    .expect("fee")
}

fn transition(
    data_contract_id: Identifier,
    document_type_name: &str,
    entries: Vec<BinaryData>,
    actions: Vec<SerializedAction>,
    value_balance: u64,
    anchor: [u8; 32],
    proof: Vec<u8>,
    binding_signature: [u8; 64],
) -> StateTransition {
    PrivateDocumentCreateFromShieldedPoolTransition::V0(
        PrivateDocumentCreateFromShieldedPoolTransitionV0 {
            data_contract_id,
            document_type_name: document_type_name.to_string(),
            entries,
            actions,
            value_balance,
            anchor,
            proof,
            binding_signature,
        },
    )
    .into()
}

/// A transition whose bundle is never proven: for the checks that run before the proof.
fn dummy_transition(entries: Vec<BinaryData>, value_balance: u64) -> StateTransition {
    transition(
        Identifier::from(CONTRACT_ID),
        "secret",
        entries,
        vec![create_dummy_serialized_action()],
        value_balance,
        [42u8; 32],
        vec![7u8; 100],
        [0u8; 64],
    )
}

/// A transition with a real bundle (one note of `NOTE_VALUE` in, its change out) paying the
/// fee for `entries`, whose sighash binds `data_contract_id`, `document_type_name` and the
/// entries. Returns it with its anchor.
fn proven_transition(
    data_contract_id: Identifier,
    document_type_name: &str,
    entries: Vec<BinaryData>,
) -> (StateTransition, [u8; 32]) {
    let fee = fee(&entries);
    let mut rng = StdRng::seed_from_u64(0);
    let pk = get_proving_key();

    let sk = SpendingKey::from_bytes([0u8; 32]).unwrap();
    let fvk = FullViewingKey::from(&sk);
    let recipient = fvk.address_at(0u32, Scope::External);
    let ask = SpendAuthorizingKey::from(&sk);

    let mut rho_bytes = [0u8; 32];
    rho_bytes[0] = 1;
    let rho = Rho::from_bytes(&rho_bytes).unwrap();
    let rseed = RandomSeed::from_bytes([42u8; 32], &rho).unwrap();
    let note = Note::from_parts(recipient, NoteValue::from_raw(NOTE_VALUE), rho, rseed).unwrap();

    let cmx = ExtractedNoteCommitment::from(note.commitment());
    let mut tree = ClientMemoryCommitmentTree::new(100);
    tree.append(cmx.to_bytes(), Retention::Marked).unwrap();
    tree.checkpoint(0u32).unwrap();
    let anchor = tree.anchor().unwrap();
    let merkle_path = tree.witness(Position::from(0u64), 0).unwrap().unwrap();

    let mut builder = Builder::<DashMemo>::new(BundleType::DEFAULT, anchor);
    builder.add_spend(fvk.clone(), note, merkle_path).unwrap();
    builder
        .add_output(
            None,
            recipient,
            NoteValue::from_raw(NOTE_VALUE - fee),
            [0u8; 36],
        )
        .unwrap();
    let (unauthorized, _) = builder.build::<i64>(&mut rng).unwrap().unwrap();

    let extra_sighash_data = private_document_create_from_shielded_pool_extra_sighash_data_v0(
        &data_contract_id.to_buffer(),
        document_type_name,
        &entries,
    );
    let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
    let sighash = compute_platform_sighash(&bundle_commitment, &extra_sighash_data);
    let proven = unauthorized.create_proof(pk, &mut rng).unwrap();
    let bundle = proven.apply_signatures(rng, sighash, &[ask]).unwrap();

    let (actions, value_balance, anchor, proof, binding_signature) =
        serialize_authorized_bundle_i64(&bundle);
    assert_eq!(actions.len(), BUNDLE_ACTIONS);
    assert_eq!(value_balance as u64, fee, "the bundle pays exactly the fee");
    (
        transition(
            data_contract_id,
            document_type_name,
            entries,
            actions,
            value_balance as u64,
            anchor,
            proof,
            binding_signature,
        ),
        anchor,
    )
}

/// Registers the contract holding the private `secret` type of `ENTRY_SIZE`-byte entries and
/// the public `note` type.
fn register_contract(platform: &TempPlatform<MockCoreRPCLike>) -> DataContract {
    let platform_version = PlatformVersion::latest();
    let contract = DataContract::from_value(
        dpp::platform_value::to_value(json!({
            "$formatVersion": "1",
            "id": Identifier::from(CONTRACT_ID).to_string(Encoding::Base58),
            "ownerId": Identifier::from([8; 32]).to_string(Encoding::Base58),
            "version": 1,
            "documentSchemas": {
                "secret": {
                    "type": "object",
                    "private": { "entrySize": ENTRY_SIZE },
                    "documentsMutable": false,
                    "canBeDeleted": false,
                    "properties": {
                        "accountReference": {
                            "type": "integer", "minimum": 0, "maximum": 4294967295u64,
                            "position": 0
                        }
                    },
                    "additionalProperties": false
                },
                "note": {
                    "type": "object",
                    "properties": {
                        "text": { "type": "string", "maxLength": 63, "position": 0 }
                    },
                    "additionalProperties": false
                }
            }
        }))
        .expect("the contract converts"),
        true,
        platform_version,
    )
    .expect("the contract registers");
    platform
        .drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            None,
            None,
            platform_version,
        )
        .expect("expected to register the contract");
    contract
}

/// Seeds the pool the spend needs: enough notes for the minimum-notes floor, the bundle's
/// anchor and the pool's total balance, booked into system credits.
fn seed_pool(platform: &TempPlatform<MockCoreRPCLike>, anchor: &[u8; 32]) {
    insert_dummy_encrypted_notes(platform, 250);
    insert_anchor_into_state(platform, anchor);
    set_pool_total_balance(platform, NOTE_VALUE);
}

fn single_unpaid_error(
    platform: &TempPlatform<MockCoreRPCLike>,
    st: StateTransition,
) -> ConsensusError {
    let result = process_transition(platform, st, PlatformVersion::latest());
    match result.execution_results().as_slice() {
        [StateTransitionExecutionResult::UnpaidConsensusError(error)] => error.clone(),
        results => panic!("expected one unpaid refusal, got {results:?}"),
    }
}

// ==========================================
// Success, fee, conservation, proof
// ==========================================

#[test]
fn should_append_the_entries_behind_the_block_header_and_pay_exactly_the_fee() {
    let platform_version = PlatformVersion::latest();
    let platform = setup_platform();
    let contract = register_contract(&platform);
    let entries = entries(3, ENTRY_SIZE);
    let (st, anchor) = proven_transition(contract.id(), "secret", entries.clone());
    seed_pool(&platform, &anchor);

    let credits_before = platform
        .drive
        .calculate_total_credits_balance(None, &platform_version.drive)
        .expect("total credits before");
    assert!(
        credits_before.ok().expect("no overflow"),
        "fixture must start balanced"
    );

    {
        let guard_bytes = st.serialize_to_bytes().expect("serialize for guard");
        crate::test::helpers::state_mutation_guard::assert_check_tx_valid_at_all_levels(
            &platform,
            &guard_bytes,
            "private document create from shielded pool",
        );
    }

    let block_info = BlockInfo {
        height: 7,
        time_ms: 1_700_000_000_000,
        ..Default::default()
    };
    let platform_state = platform.state.load();
    let (fee_results, _) = process_state_transitions(&platform, &[st], block_info, &platform_state);
    let fee = fee(&entries);
    assert_eq!(
        fee_results[0].total_base_fee(),
        fee,
        "the pool-paid fee must be booked exactly"
    );

    let credits_after = platform
        .drive
        .calculate_total_credits_balance(None, &platform_version.drive)
        .expect("total credits after");
    assert!(
        credits_after.ok().expect("no overflow"),
        "credits must stay balanced: {credits_after}"
    );
    assert_eq!(
        credits_after.total_credits_in_platform, credits_before.total_credits_in_platform,
        "paying the fee from the pool must not mint or burn system credits"
    );
    assert_eq!(
        credits_before.total_in_shielded_balances - credits_after.total_in_shielded_balances,
        fee as i64,
        "the pool must lose exactly the fee"
    );

    let secret = contract
        .document_type_for_name("secret")
        .expect("the secret type");
    let page = platform
        .drive
        .fetch_private_document_entries(CONTRACT_ID, secret, 0, None, platform_version)
        .expect("expected to read the store");
    assert_eq!(page.total_count, 3);
    for (i, (stored, entry)) in page.entries.iter().zip(&entries).enumerate() {
        assert_eq!(stored.position, i as u64);
        assert_eq!(stored.block_height, 7);
        assert_eq!(stored.block_time_ms, 1_700_000_000_000);
        assert_eq!(stored.data, entry.to_vec());
    }
}

#[test]
fn should_prove_the_spent_nullifiers_as_the_affected_state() {
    let platform_version = PlatformVersion::latest();
    let platform = setup_platform();
    let contract = register_contract(&platform);
    let (st, anchor) = proven_transition(contract.id(), "secret", entries(1, ENTRY_SIZE));
    seed_pool(&platform, &anchor);
    let StateTransition::PrivateDocumentCreateFromShieldedPool(
        PrivateDocumentCreateFromShieldedPoolTransition::V0(v0),
    ) = &st
    else {
        panic!("expected a private document create");
    };
    let expected_nullifiers: Vec<Vec<u8>> =
        v0.actions.iter().map(|a| a.nullifier.to_vec()).collect();

    let platform_state = platform.state.load();
    process_state_transitions(
        &platform,
        &[st.clone()],
        BlockInfo::default(),
        &platform_state,
    );

    let proof_bytes = platform
        .drive
        .prove_state_transition(&st, None, platform_version)
        .expect("prove")
        .into_data()
        .expect("proof data");
    let (root_hash, outcome) = Drive::verify_state_transition_was_executed_with_proof(
        &st,
        &BlockInfo::default(),
        &proof_bytes,
        &|_| Ok(None),
        platform_version,
    )
    .expect("verify");
    assert_ne!(root_hash, [0u8; 32]);
    // The spent nullifiers name the notes paying the fee, not the entries, so a competing create
    // spending the same notes for other entries leaves the same evidence.
    assert!(
        !outcome.is_execution_proved(),
        "a private document create proof must be classified as affected state, got {outcome:?}"
    );
    let result = outcome.into_result();
    let StateTransitionProofResult::VerifiedShieldedNullifiers(statuses) = result else {
        panic!("expected VerifiedShieldedNullifiers, got {result:?}");
    };
    assert_eq!(
        statuses
            .iter()
            .map(|(nf, _)| nf.clone())
            .collect::<Vec<_>>(),
        expected_nullifiers
    );
    assert!(statuses.iter().all(|(_, spent)| *spent));
}

// ==========================================
// Refusals, all unpaid
// ==========================================

#[test]
fn should_refuse_the_transition_before_protocol_version_15() {
    let platform = setup_platform_at_protocol_version(14);
    let entries = entries(1, ENTRY_SIZE);
    let value_balance = fee(&entries);
    let result = process_transition(
        &platform,
        dummy_transition(entries, value_balance),
        PlatformVersion::get(14).expect("protocol version 14"),
    );
    assert_matches!(
        result.execution_results().as_slice(),
        [StateTransitionExecutionResult::UnpaidConsensusError(ConsensusError::BasicError(
            BasicError::StateTransitionNotActiveError(error)
        ))] if error.state_transition_type().starts_with("PrivateDocumentCreateFromShieldedPool")
    );
}

#[test]
fn should_refuse_a_value_balance_below_or_above_the_fee() {
    let platform = setup_platform();
    let entries = entries(2, ENTRY_SIZE);
    // The fee check reads the action count: the dummy bundle has one action.
    let exact = compute_private_document_create_from_shielded_pool_fee(
        1,
        2,
        ENTRY_SIZE,
        PlatformVersion::latest(),
    )
    .expect("fee");
    assert_matches!(
        single_unpaid_error(&platform, dummy_transition(entries.clone(), exact - 1)),
        ConsensusError::StateError(StateError::InsufficientShieldedFeeError(_))
    );
    assert_matches!(
        single_unpaid_error(&platform, dummy_transition(entries, exact + 1)),
        ConsensusError::BasicError(BasicError::ShieldedInvalidValueBalanceError(_))
    );
}

#[test]
fn should_refuse_two_entries_that_are_the_same_before_the_proof() {
    let platform = setup_platform();
    let entries = vec![
        BinaryData::new(vec![1u8; ENTRY_SIZE as usize]),
        BinaryData::new(vec![1u8; ENTRY_SIZE as usize]),
    ];
    let value_balance = fee(&entries);
    assert_matches!(
        single_unpaid_error(&platform, dummy_transition(entries, value_balance)),
        ConsensusError::BasicError(BasicError::DuplicatePrivateDocumentEntryError(_))
    );
}

#[test]
fn should_refuse_entries_changed_after_the_bundle_was_signed() {
    let platform = setup_platform();
    let contract = register_contract(&platform);
    let (mut st, anchor) = proven_transition(contract.id(), "secret", entries(2, ENTRY_SIZE));
    seed_pool(&platform, &anchor);
    let StateTransition::PrivateDocumentCreateFromShieldedPool(
        PrivateDocumentCreateFromShieldedPoolTransition::V0(v0),
    ) = &mut st
    else {
        panic!("expected a private document create");
    };
    let mut changed = v0.entries[1].to_vec();
    changed[0] ^= 1;
    v0.entries[1] = BinaryData::new(changed);
    assert_matches!(
        single_unpaid_error(&platform, st),
        ConsensusError::StateError(StateError::InvalidShieldedProofError(_))
    );
}

#[test]
fn should_refuse_a_target_that_is_no_private_type_taking_the_entries() {
    let platform = setup_platform();
    let contract = register_contract(&platform);

    let (st, anchor) = proven_transition(
        Identifier::from([9u8; 32]),
        "secret",
        entries(1, ENTRY_SIZE),
    );
    seed_pool(&platform, &anchor);
    assert_matches!(
        single_unpaid_error(&platform, st),
        ConsensusError::StateError(StateError::DataContractNotFoundError(_))
    );

    let (st, _) = proven_transition(contract.id(), "missing", entries(1, ENTRY_SIZE));
    assert_matches!(
        single_unpaid_error(&platform, st),
        ConsensusError::BasicError(BasicError::InvalidDocumentTypeError(_))
    );

    let (st, _) = proven_transition(contract.id(), "note", entries(1, ENTRY_SIZE));
    assert_matches!(
        single_unpaid_error(&platform, st),
        ConsensusError::StateError(StateError::DocumentTypeNotPrivateError(e))
            if e.document_type_name() == "note"
    );

    let (st, _) = proven_transition(contract.id(), "secret", entries(1, ENTRY_SIZE + 1));
    assert_matches!(
        single_unpaid_error(&platform, st),
        ConsensusError::StateError(StateError::PrivateDocumentEntrySizeMismatchError(e))
            if e.entry_size() == ENTRY_SIZE && e.size() == ENTRY_SIZE + 1
    );
}
