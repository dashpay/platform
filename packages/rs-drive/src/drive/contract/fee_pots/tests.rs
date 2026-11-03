//! Drive-level tests of the contract fee pots.

use crate::drive::contract::fee_pots::types::{ContractFeePotState, ContractFeePots};
use crate::drive::contract::paths::contract_fee_pots_key;
use crate::drive::credit_pools::epochs::operations_factory::EpochOperations;
use crate::drive::prefunded_specialized_balances::prefunded_specialized_balances_path;
use crate::drive::Drive;
use crate::util::batch::drive_op_batch::ContractFeePotOperationType;
use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
use crate::util::batch::{DriveOperation, GroveDbOpBatch};
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::Epoch;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::action_fees::{ContractFeePot, ContractFeePotLastClaim};
use dpp::data_contract::DataContract;
use dpp::fee::fee_result::FeeResult;
use dpp::identifier::Identifier;
use dpp::tests::fixtures::get_data_contract_fixture;
use dpp::version::PlatformVersion;

const BOTH: [ContractFeePot; 2] = [ContractFeePot::Owner, ContractFeePot::Moderators];

/// A claim in `epoch_index`, at a block time and by a claimant that differ from epoch to epoch.
fn claim_in(epoch_index: u16) -> ContractFeePotLastClaim {
    ContractFeePotLastClaim {
        epoch_index,
        time_ms: 1_700_000_000_000 + u64::from(epoch_index),
        claimant_id: Identifier::from([epoch_index.to_be_bytes()[1].wrapping_add(1); 32]),
    }
}

/// A drive holding one contract, whose other tree the last claims live in.
fn drive_with_contract() -> (Drive, DataContract) {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract =
        get_data_contract_fixture(None, 0, platform_version.protocol_version).data_contract_owned();
    drive
        .insert_contract(
            &contract,
            BlockInfo::default(),
            true,
            None,
            platform_version,
        )
        .expect("expected to insert the contract");
    (drive, contract)
}

fn apply(drive: &Drive, operations: Vec<ContractFeePotOperationType>, apply: bool) -> FeeResult {
    drive
        .apply_drive_operations(
            operations
                .into_iter()
                .map(DriveOperation::ContractFeePotOperation)
                .collect(),
            apply,
            &BlockInfo::default(),
            None,
            PlatformVersion::latest(),
            None,
        )
        .expect("expected to apply the fee pot operations")
}

fn add(drive: &Drive, contract_id: Identifier, pot: ContractFeePot, amount: u64) {
    apply(
        drive,
        vec![ContractFeePotOperationType::AddToPot {
            contract_id,
            pot,
            amount,
        }],
        true,
    );
}

fn fetch(drive: &Drive, contract_id: Identifier, pot: ContractFeePot) -> ContractFeePotState {
    drive
        .fetch_contract_fee_pot(contract_id, pot, None, PlatformVersion::latest())
        .expect("expected to fetch the pot")
}

/// Proves and verifies `pots` and checks the result against a fetch of each.
fn assert_proved(drive: &Drive, contract_id: Identifier, pots: &[ContractFeePot]) {
    let platform_version = PlatformVersion::latest();
    let proof = drive
        .prove_contract_fee_pots(contract_id, pots, None, platform_version)
        .expect("expected to prove the pots");
    let (root_hash, proved) =
        Drive::verify_contract_fee_pots(&proof, contract_id, pots, false, platform_version)
            .expect("expected to verify the pots");
    assert_eq!(
        root_hash,
        drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("expected a root hash")
    );
    for pot in pots {
        assert_eq!(proved.pot(*pot), &fetch(drive, contract_id, *pot));
    }
}

#[test]
fn should_hold_nothing_and_no_claim_before_the_first_fee() {
    let (drive, contract) = drive_with_contract();
    for pot in BOTH {
        assert_eq!(
            fetch(&drive, contract.id(), pot),
            ContractFeePotState::default()
        );
    }
    assert_proved(&drive, contract.id(), &BOTH);
}

#[test]
fn should_accumulate_each_pot_of_each_contract_on_its_own() {
    let (drive, contract) = drive_with_contract();
    let other_contract_id = Identifier::from([7; 32]);

    add(&drive, contract.id(), ContractFeePot::Owner, 10);
    add(&drive, contract.id(), ContractFeePot::Owner, 5);
    add(&drive, contract.id(), ContractFeePot::Moderators, 100);
    add(&drive, other_contract_id, ContractFeePot::Moderators, 3);

    assert_eq!(
        fetch(&drive, contract.id(), ContractFeePot::Owner).credits,
        15
    );
    assert_eq!(
        fetch(&drive, contract.id(), ContractFeePot::Moderators).credits,
        100
    );
    assert_eq!(
        fetch(&drive, other_contract_id, ContractFeePot::Owner).credits,
        0
    );
    assert_eq!(
        fetch(&drive, other_contract_id, ContractFeePot::Moderators).credits,
        3
    );
}

#[test]
fn should_deduct_from_a_pot_and_refuse_more_than_it_holds() {
    let (drive, contract) = drive_with_contract();
    add(&drive, contract.id(), ContractFeePot::Moderators, 100);

    apply(
        &drive,
        vec![ContractFeePotOperationType::DeductFromPot {
            contract_id: contract.id(),
            pot: ContractFeePot::Moderators,
            amount: 99,
        }],
        true,
    );
    assert_eq!(
        fetch(&drive, contract.id(), ContractFeePot::Moderators).credits,
        1
    );

    let result = drive.apply_drive_operations(
        vec![DriveOperation::ContractFeePotOperation(
            ContractFeePotOperationType::DeductFromPot {
                contract_id: contract.id(),
                pot: ContractFeePot::Moderators,
                amount: 2,
            },
        )],
        true,
        &BlockInfo::default(),
        None,
        PlatformVersion::latest(),
        None,
    );
    assert!(result.is_err(), "a pot can not go below zero");
    assert_eq!(
        fetch(&drive, contract.id(), ContractFeePot::Moderators).credits,
        1
    );
}

#[test]
fn should_record_the_last_claim_of_each_pot_on_its_own() {
    let (drive, contract) = drive_with_contract();
    let set = |pot, epoch_index| {
        apply(
            &drive,
            vec![ContractFeePotOperationType::SetLastClaim {
                contract_id: contract.id(),
                pot,
                last_claim: claim_in(epoch_index),
            }],
            true,
        );
    };

    set(ContractFeePot::Moderators, 7);
    assert_eq!(
        fetch(&drive, contract.id(), ContractFeePot::Moderators).last_claim,
        Some(claim_in(7))
    );
    assert_eq!(
        fetch(&drive, contract.id(), ContractFeePot::Owner).last_claim,
        None
    );

    // A later claim replaces the whole record, its time and its claimant with its epoch;
    // epoch 0 is an epoch like any other.
    set(ContractFeePot::Moderators, 300);
    set(ContractFeePot::Owner, 0);
    assert_eq!(
        fetch(&drive, contract.id(), ContractFeePot::Moderators).last_claim,
        Some(claim_in(300))
    );
    let owner_pot = fetch(&drive, contract.id(), ContractFeePot::Owner);
    assert_eq!(owner_pot.last_claim, Some(claim_in(0)));
    assert_eq!(owner_pot.last_claim_epoch(), Some(0));
}

#[test]
fn should_prove_the_pots_asked_for_and_nothing_about_the_other() {
    let (drive, contract) = drive_with_contract();
    add(&drive, contract.id(), ContractFeePot::Owner, 10);
    add(&drive, contract.id(), ContractFeePot::Moderators, 100);
    apply(
        &drive,
        vec![ContractFeePotOperationType::SetLastClaim {
            contract_id: contract.id(),
            pot: ContractFeePot::Moderators,
            last_claim: claim_in(4),
        }],
        true,
    );

    assert_proved(&drive, contract.id(), &BOTH);
    assert_proved(&drive, contract.id(), &[ContractFeePot::Owner]);
    assert_proved(&drive, contract.id(), &[ContractFeePot::Moderators]);

    // A proof of one pot leaves the other at its default rather than reading it.
    let platform_version = PlatformVersion::latest();
    let proof = drive
        .prove_contract_fee_pots(
            contract.id(),
            &[ContractFeePot::Owner],
            None,
            platform_version,
        )
        .expect("expected to prove the owner pot");
    let (_, proved) = Drive::verify_contract_fee_pots(
        &proof,
        contract.id(),
        &[ContractFeePot::Owner],
        false,
        platform_version,
    )
    .expect("expected to verify the owner pot");
    assert_eq!(
        proved,
        ContractFeePots {
            owner: ContractFeePotState {
                credits: 10,
                last_claim: None,
            },
            moderators: ContractFeePotState::default(),
        }
    );

    // A proof of one pot does not verify as a proof of the other.
    assert!(Drive::verify_contract_fee_pots(
        &proof,
        contract.id(),
        &[ContractFeePot::Moderators],
        false,
        platform_version,
    )
    .is_err());
}

#[test]
fn should_count_the_pots_in_the_total_credits_of_the_platform() {
    let (drive, contract) = drive_with_contract();
    let platform_version = PlatformVersion::latest();
    let balanced = |drive: &Drive| {
        drive
            .calculate_total_credits_balance(None, &platform_version.drive)
            .expect("expected to sum the credits")
            .ok()
            .expect("expected the sum to be judged")
    };
    assert!(balanced(&drive));

    // Credits that appear in a pot out of nowhere unbalance the platform: the pots are inside
    // the sum every block is checked against.
    add(&drive, contract.id(), ContractFeePot::Owner, 40);
    add(&drive, contract.id(), ContractFeePot::Moderators, 60);
    assert!(!balanced(&drive));

    // Once the platform accounts for them, it balances again.
    drive
        .add_to_system_credits(100, None, platform_version)
        .expect("expected to add to the system credits");
    assert!(balanced(&drive));
}

#[test]
fn should_estimate_a_pot_write_without_writing() {
    let (drive, contract) = drive_with_contract();
    let estimated = apply(
        &drive,
        vec![
            ContractFeePotOperationType::AddToPot {
                contract_id: contract.id(),
                pot: ContractFeePot::Moderators,
                amount: 100,
            },
            ContractFeePotOperationType::SetLastClaim {
                contract_id: contract.id(),
                pot: ContractFeePot::Moderators,
                last_claim: claim_in(1),
            },
        ],
        false,
    );
    assert!(estimated.processing_fee > 0);
    assert_eq!(
        fetch(&drive, contract.id(), ContractFeePot::Moderators),
        ContractFeePotState::default()
    );

    // The estimate covers what the write then costs.
    let actual = apply(
        &drive,
        vec![
            ContractFeePotOperationType::AddToPot {
                contract_id: contract.id(),
                pot: ContractFeePot::Moderators,
                amount: 100,
            },
            ContractFeePotOperationType::SetLastClaim {
                contract_id: contract.id(),
                pot: ContractFeePot::Moderators,
                last_claim: claim_in(1),
            },
        ],
        true,
    );
    assert!(
        estimated.total_base_fee() >= actual.total_base_fee(),
        "estimated {} < actual {}",
        estimated.total_base_fee(),
        actual.total_base_fee()
    );
}

#[test]
fn should_read_the_epoch_fee_multiplier_and_fall_back_in_the_first_block_of_an_epoch() {
    let (drive, _) = drive_with_contract();
    let platform_version = PlatformVersion::latest();
    let epoch = Epoch::new(5).expect("expected an epoch");
    let schedule_multiplier = platform_version
        .fee_version
        .uses_version_fee_multiplier_permille
        .expect("expected the fee schedule to set a multiplier");

    // The epoch's tree is there but the epoch was not initialized yet: state transitions of
    // the first block of an epoch execute before the block end initializes it.
    let (fee, multiplier) = drive
        .fetch_action_fee_multiplier_with_fee(&epoch, None, platform_version)
        .expect("expected to read the multiplier");
    assert_eq!(multiplier, schedule_multiplier);
    assert!(fee.processing_fee > 0, "the read is billed");

    let mut batch = GroveDbOpBatch::new();
    batch.push(epoch.update_fee_multiplier_operation(1_500));
    drive
        .grove_apply_batch(batch, false, None, &platform_version.drive)
        .expect("expected to set the epoch's multiplier");

    let (_, multiplier) = drive
        .fetch_action_fee_multiplier_with_fee(&epoch, None, platform_version)
        .expect("expected to read the multiplier");
    assert_eq!(multiplier, 1_500);
}

#[test]
fn should_create_the_pots_tree_of_a_chain_that_reached_version_14_without_it() {
    // A chain that upgraded to protocol version 14, or was born at it, on a build from before
    // the fee pots has neither tree: the upgrade step that creates them already ran.
    let (drive, contract) = drive_with_contract();
    let platform_version = PlatformVersion::latest();
    let prefunded_path = prefunded_specialized_balances_path();
    for pot in BOTH {
        drive
            .grove
            .delete(
                &prefunded_path,
                contract_fee_pots_key(pot),
                None,
                None,
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .expect("expected to delete the empty pots tree");
    }

    // Reading a pot of a chain without the trees finds nothing, as on any other chain.
    assert_eq!(
        fetch(&drive, contract.id(), ContractFeePot::Moderators),
        ContractFeePotState::default()
    );

    // The first fee of each pot creates its tree, and later ones find it there.
    add(&drive, contract.id(), ContractFeePot::Moderators, 100);
    add(&drive, contract.id(), ContractFeePot::Moderators, 11);
    add(&drive, contract.id(), ContractFeePot::Owner, 5);
    assert_eq!(
        fetch(&drive, contract.id(), ContractFeePot::Moderators).credits,
        111
    );
    assert_eq!(
        fetch(&drive, contract.id(), ContractFeePot::Owner).credits,
        5
    );
    assert_proved(&drive, contract.id(), &BOTH);

    // The credits are inside the sum the platform checks, as on a chain born with the trees.
    drive
        .add_to_system_credits(116, None, platform_version)
        .expect("expected to add to the system credits");
    assert!(drive
        .calculate_total_credits_balance(None, &platform_version.drive)
        .expect("expected to sum the credits")
        .ok()
        .expect("expected the sum to be judged"));
}
