use crate::drive::Drive;
use crate::error::{proof::ProofError, Error};
use crate::fees::op::LowLevelDriveOperation;
use crate::state_transition_action::shielded::shielded_withdrawal::v0::ShieldedWithdrawalTransitionActionV0;
use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use dpp::address_funds::PlatformAddress;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::DataContract;
use dpp::data_contracts::withdrawals_contract::v1::document_types::withdrawal;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::{Document, DocumentV0Getters};
use dpp::identity::core_script::CoreScript;
use dpp::shielded::{compute_shielded_withdrawal_fee, SerializedAction};
use dpp::state_transition::proof_result::{
    StateTransitionProofGuarantee, StateTransitionProofResult,
};
use dpp::state_transition::shielded_transfer_transition::v0::ShieldedTransferTransitionV0;
use dpp::state_transition::shielded_transfer_transition::ShieldedTransferTransition;
use dpp::state_transition::shielded_withdrawal_transition::v0::ShieldedWithdrawalTransitionV0;
use dpp::state_transition::shielded_withdrawal_transition::ShieldedWithdrawalTransition;
use dpp::state_transition::unshield_transition::v0::UnshieldTransitionV0;
use dpp::state_transition::unshield_transition::UnshieldTransition;
use dpp::state_transition::StateTransition;
use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
use dpp::version::PlatformVersion;
use dpp::withdrawal::Pooling;
use std::collections::BTreeMap;
use std::sync::Arc;

const NULLIFIERS: [[u8; 32]; 2] = [[0x11; 32], [0x22; 32]];
const OUTPUT_ADDRESS: PlatformAddress = PlatformAddress::P2pkh([0x33; 20]);
const GROSS_AMOUNT: u64 = 10_000_000_000;

fn versions() -> [&'static PlatformVersion; 3] {
    [
        PlatformVersion::get(12).expect("protocol version 12 exists"),
        PlatformVersion::get(13).expect("protocol version 13 exists"),
        PlatformVersion::latest(),
    ]
}

// These fixtures seed committed state, not an executed Orchard transaction. The
// client verifier authenticates that state without verifying an Orchard bundle.
fn actions(output_commitment: u8) -> Vec<SerializedAction> {
    NULLIFIERS
        .iter()
        .map(|nullifier| SerializedAction {
            nullifier: *nullifier,
            rk: [0; 32],
            cmx: [output_commitment; 32],
            encrypted_note: vec![0; 216],
            cv_net: [0; 32],
            spend_auth_sig: [0; 64],
        })
        .collect()
}

fn transfer(output_commitment: u8) -> StateTransition {
    StateTransition::ShieldedTransfer(ShieldedTransferTransition::V0(
        ShieldedTransferTransitionV0 {
            actions: actions(output_commitment),
            value_balance: 0,
            anchor: [0; 32],
            proof: vec![],
            binding_signature: [0; 64],
        },
    ))
}

fn unshield(amount: u64) -> StateTransition {
    StateTransition::Unshield(UnshieldTransition::V0(UnshieldTransitionV0 {
        output_address: OUTPUT_ADDRESS,
        actions: actions(0x44),
        unshielding_amount: amount,
        anchor: [0; 32],
        proof: vec![],
        binding_signature: [0; 64],
    }))
}

fn withdrawal_transition(amount: u64, output_commitment: u8) -> ShieldedWithdrawalTransitionV0 {
    ShieldedWithdrawalTransitionV0 {
        actions: actions(output_commitment),
        unshielding_amount: amount,
        anchor: [0; 32],
        proof: vec![],
        binding_signature: [0; 64],
        core_fee_per_byte: 1,
        pooling: Pooling::Never,
        output_script: CoreScript::from_bytes(
            [vec![0x76, 0xa9, 0x14], vec![0x55; 20], vec![0x88, 0xac]].concat(),
        ),
    }
}

fn apply_operations(drive: &Drive, operations: Vec<LowLevelDriveOperation>, pv: &PlatformVersion) {
    drive
        .grove_apply_batch(
            LowLevelDriveOperation::grovedb_operations_batch_consume(operations),
            false,
            None,
            &pv.drive,
        )
        .expect("should commit fixture state");
}

fn seed_nullifiers(drive: &Drive, nullifiers: &[[u8; 32]], pv: &PlatformVersion) {
    apply_operations(
        drive,
        drive
            .insert_nullifiers(nullifiers, pv)
            .expect("should prepare spent markers"),
        pv,
    );
}

fn seed_withdrawal(drive: &Drive, pv: &PlatformVersion) -> (Arc<DataContract>, Document) {
    let contract = Arc::new(
        load_system_data_contract(SystemDataContract::Withdrawals, pv)
            .expect("should load withdrawals contract"),
    );
    drive
        .apply_contract(&contract, BlockInfo::default(), true, None, None, pv)
        .expect("should store withdrawals contract");
    let transition = withdrawal_transition(GROSS_AMOUNT, 0x44);
    let fee = compute_shielded_withdrawal_fee(transition.actions.len(), pv)
        .expect("should compute Platform withdrawal fee");
    let document = ShieldedWithdrawalTransitionActionV0::try_from_transition(
        &transition,
        GROSS_AMOUNT * 2,
        1_000,
        fee,
        pv,
    )
    .into_data()
    .expect("should build net-amount withdrawal document")
    .prepared_withdrawal_document;
    let document_type = contract
        .document_type_for_name(withdrawal::NAME)
        .expect("should find withdrawal document type");
    // Storage serialization normalizes integer widths and byte arrays. Compare
    // the proof's document to that stored representation, not the builder's types.
    let document = Document::from_bytes(
        &document
            .serialize(document_type, &contract, pv)
            .expect("should serialize withdrawal document"),
        document_type,
        pv,
    )
    .expect("should normalize withdrawal document for storage");
    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((&document, None)),
                    owner_id: None,
                },
                contract: &contract,
                document_type,
            },
            false,
            BlockInfo::default(),
            true,
            None,
            pv,
            None,
        )
        .expect("should store withdrawal document");
    (contract, document)
}

fn prove(drive: &Drive, transition: &StateTransition, pv: &PlatformVersion) -> Vec<u8> {
    drive
        .prove_state_transition(transition, None, pv)
        .expect("should create a genuine state proof")
        .into_data()
        .expect("should return proof bytes")
}

fn spent_statuses() -> Vec<(Vec<u8>, bool)> {
    NULLIFIERS.iter().map(|nf| (nf.to_vec(), true)).collect()
}

#[test]
fn should_not_prove_transfer_execution_when_outputs_differ_for_the_same_spent_notes() {
    let mut guarantees = Vec::new();
    for pv in versions() {
        let drive = setup_drive_with_initial_state_structure(Some(pv));
        seed_nullifiers(&drive, &NULLIFIERS, pv);
        let requested = transfer(0x44);
        let different_outputs = transfer(0x66);
        let proof = prove(&drive, &requested, pv);
        let root = drive
            .grove
            .root_hash(None, &pv.drive.grove_version)
            .unwrap()
            .unwrap();
        for transition in [requested, different_outputs] {
            let (verified_root, outcome) = Drive::verify_state_transition_was_executed_with_proof(
                &transition,
                &BlockInfo::default(),
                &proof,
                &|_| Ok(None),
                pv,
            )
            .expect("the same spent-note proof verifies both output requests");
            assert_eq!(verified_root, root);
            assert_eq!(
                outcome.result(),
                &StateTransitionProofResult::VerifiedShieldedNullifiers(spent_statuses())
            );
            guarantees.push(outcome.guarantee());
        }
    }
    assert_eq!(
        guarantees,
        vec![StateTransitionProofGuarantee::AffectedState; 6]
    );
}

#[test]
fn should_not_prove_unshield_execution_from_spent_notes_and_an_unrelated_balance_snapshot() {
    let mut guarantees = Vec::new();
    for pv in versions() {
        let drive = setup_drive_with_initial_state_structure(Some(pv));
        seed_nullifiers(&drive, &NULLIFIERS, pv);
        let operations = drive
            .set_balance_to_address_operations(OUTPUT_ADDRESS, 7, 123_456, &mut None, pv)
            .expect("should prepare an unrelated address balance");
        apply_operations(&drive, operations, pv);
        let requested = unshield(GROSS_AMOUNT);
        let different_amount = unshield(GROSS_AMOUNT + 1_000_000);
        let proof = prove(&drive, &requested, pv);
        let root = drive
            .grove
            .root_hash(None, &pv.drive.grove_version)
            .unwrap()
            .unwrap();
        for transition in [requested, different_amount] {
            let (verified_root, outcome) = Drive::verify_state_transition_was_executed_with_proof(
                &transition,
                &BlockInfo::default(),
                &proof,
                &|_| Ok(None),
                pv,
            )
            .expect("the same state proof verifies both unshield amounts");
            assert_eq!(verified_root, root);
            assert_eq!(
                outcome.result(),
                &StateTransitionProofResult::VerifiedShieldedNullifiersWithAddressInfos(
                    spent_statuses(),
                    BTreeMap::from([(OUTPUT_ADDRESS, Some((7, 123_456)))]),
                )
            );
            guarantees.push(outcome.guarantee());
        }
    }
    assert_eq!(
        guarantees,
        vec![StateTransitionProofGuarantee::AffectedState; 6]
    );
}

#[test]
fn should_not_prove_withdrawal_execution_when_amount_or_change_outputs_differ() {
    let mut guarantees = Vec::new();
    for pv in versions() {
        let drive = setup_drive_with_initial_state_structure(Some(pv));
        seed_nullifiers(&drive, &NULLIFIERS, pv);
        let (contract, document) = seed_withdrawal(&drive, pv);
        let transitions = [
            withdrawal_transition(GROSS_AMOUNT, 0x44),
            withdrawal_transition(GROSS_AMOUNT + 1_000_000, 0x44),
            withdrawal_transition(GROSS_AMOUNT, 0x66),
        ]
        .map(|st| StateTransition::ShieldedWithdrawal(ShieldedWithdrawalTransition::V0(st)));
        let proof = prove(&drive, &transitions[0], pv);
        let root = drive
            .grove
            .root_hash(None, &pv.drive.grove_version)
            .unwrap()
            .unwrap();
        for transition in transitions {
            let (verified_root, outcome) = Drive::verify_state_transition_was_executed_with_proof(
                &transition,
                &BlockInfo::default(),
                &proof,
                &|id| Ok((id == &contract.id()).then(|| Arc::clone(&contract))),
                pv,
            )
            .expect("the same withdrawal document verifies different amount and change requests");
            assert_eq!(verified_root, root);
            assert_eq!(
                outcome.result(),
                &StateTransitionProofResult::VerifiedShieldedNullifiersWithWithdrawalDocument(
                    spent_statuses(),
                    BTreeMap::from([(document.id(), Some(document.clone()))]),
                )
            );
            guarantees.push(outcome.guarantee());
        }
    }
    assert_eq!(
        guarantees,
        vec![StateTransitionProofGuarantee::AffectedState; 9]
    );
}

#[test]
fn should_refuse_credit_spend_proofs_when_one_requested_nullifier_is_unspent() {
    for pv in versions() {
        let drive = setup_drive_with_initial_state_structure(Some(pv));
        seed_nullifiers(&drive, &NULLIFIERS[..1], pv);
        let (contract, _) = seed_withdrawal(&drive, pv);
        for transition in [
            transfer(0x44),
            unshield(GROSS_AMOUNT),
            StateTransition::ShieldedWithdrawal(ShieldedWithdrawalTransition::V0(
                withdrawal_transition(GROSS_AMOUNT, 0x44),
            )),
        ] {
            let proof = prove(&drive, &transition, pv);
            let result = Drive::verify_state_transition_was_executed_with_proof(
                &transition,
                &BlockInfo::default(),
                &proof,
                &|id| Ok((id == &contract.id()).then(|| Arc::clone(&contract))),
                pv,
            );
            assert!(
                matches!(result, Err(Error::Proof(ProofError::IncorrectProof(_)))),
                "an authenticated absence must refuse the spend: {result:?}"
            );
        }
    }
}

#[test]
fn should_refuse_malformed_credit_spend_proofs_even_when_the_state_matches() {
    for pv in versions() {
        let drive = setup_drive_with_initial_state_structure(Some(pv));
        seed_nullifiers(&drive, &NULLIFIERS, pv);
        let (contract, _) = seed_withdrawal(&drive, pv);
        for transition in [
            transfer(0x44),
            unshield(GROSS_AMOUNT),
            StateTransition::ShieldedWithdrawal(ShieldedWithdrawalTransition::V0(
                withdrawal_transition(GROSS_AMOUNT, 0x44),
            )),
        ] {
            let mut proof = prove(&drive, &transition, pv);
            let provider = |id: &_| Ok((id == &contract.id()).then(|| Arc::clone(&contract)));
            Drive::verify_state_transition_was_executed_with_proof(
                &transition,
                &BlockInfo::default(),
                &proof,
                &provider,
                pv,
            )
            .expect("the intact proof should authenticate the matching state");
            proof.truncate(proof.len() / 2);
            let result = Drive::verify_state_transition_was_executed_with_proof(
                &transition,
                &BlockInfo::default(),
                &proof,
                &provider,
                pv,
            );
            assert!(
                matches!(result, Err(Error::Proof(_)) | Err(Error::GroveDB(_))),
                "matching state must not make a truncated proof acceptable: {result:?}"
            );
        }
    }
}
