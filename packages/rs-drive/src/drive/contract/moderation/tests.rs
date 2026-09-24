use crate::drive::contract::moderation::types::{
    ContractModerationEntriesQuery, ContractModerationEntry,
};
use crate::drive::contract::paths::{
    contract_other_path, CONTRACT_BANLIST_KEY, CONTRACT_OTHER_KEY, CONTRACT_SUSPENSIONS_KEY,
    CONTRACT_VERSION_KEY, CONTRACT_WARNINGS_KEY,
};
use crate::drive::{Drive, RootTree};
use crate::util::grove_operations::DirectQueryType;
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::Epoch;
use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
use dpp::data_contract::config::moderation::{
    ContractBan, ContractModerationConfig, ContractModerationList, ContractModerationListStatuses,
    ContractModerationReason, ContractModerationStatus, ContractModerators, ContractSuspension,
    ContractWarning,
};
use dpp::data_contract::DataContract;
use dpp::identifier::Identifier;
use dpp::tests::fixtures::get_data_contract_fixture;
use dpp::version::PlatformVersion;
use grovedb::Element;

fn reason(text: &str) -> ContractModerationReason {
    ContractModerationReason::from_text(text)
}

fn banned_for(text: &str) -> ContractModerationStatus {
    ContractModerationStatus {
        ban: Some(ContractBan {
            reason: reason(text),
        }),
        suspension: None,
        warnings: vec![],
    }
}

fn suspended_until(until: u64, text: &str) -> ContractModerationStatus {
    ContractModerationStatus {
        ban: None,
        suspension: Some(ContractSuspension {
            until,
            reason: reason(text),
        }),
        warnings: vec![],
    }
}

fn identity(seed: u8) -> Identifier {
    Identifier::from([seed; 32])
}

fn moderated_contract(banlist: bool, suspensions: bool) -> DataContract {
    moderated_contract_keeping(banlist, suspensions, false)
}

fn moderated_contract_keeping(banlist: bool, suspensions: bool, warnings: bool) -> DataContract {
    let platform_version = PlatformVersion::latest();
    let mut contract =
        get_data_contract_fixture(None, 0, platform_version.protocol_version).data_contract_owned();
    let moderation = (banlist || suspensions || warnings).then_some(ContractModerationConfig {
        banlist,
        suspensions,
        warnings,
        moderators: ContractModerators::ContractOwner,
    });
    contract.set_config(contract.config().clone().with_moderation(moderation));
    contract
}

fn warning(warned_at: u64, text: &str) -> ContractWarning {
    ContractWarning {
        warned_at,
        reason: reason(text),
    }
}

fn warned_with(warnings: Vec<ContractWarning>) -> ContractModerationStatus {
    ContractModerationStatus {
        ban: None,
        suspension: None,
        warnings,
    }
}

fn insert(drive: &Drive, contract: &DataContract, platform_version: &PlatformVersion) {
    drive
        .insert_contract(contract, BlockInfo::default(), true, None, platform_version)
        .expect("expected to insert the contract");
}

fn has_list_tree(drive: &Drive, contract_id: Identifier, key: u8) -> bool {
    drive
        .grove_has_raw(
            (&contract_other_path(contract_id.as_slice())).into(),
            &[key],
            DirectQueryType::StatefulDirectQuery,
            None,
            &mut vec![],
            &PlatformVersion::latest().drive,
        )
        .expect("expected to query the contract tree")
}

fn root_hash(drive: &Drive, platform_version: &PlatformVersion) -> [u8; 32] {
    drive
        .grove
        .root_hash(None, &platform_version.drive.grove_version)
        .unwrap()
        .expect("expected a root hash")
}

/// Fetches, proves and verifies one identity's status and checks all three agree.
fn assert_status(
    drive: &Drive,
    contract_id: Identifier,
    identity_id: Identifier,
    lists: &[ContractModerationList],
    expected: ContractModerationStatus,
) {
    let platform_version = PlatformVersion::latest();
    let fetched = drive
        .fetch_contract_moderation_status(contract_id, identity_id, lists, None, platform_version)
        .expect("expected to fetch the status");
    assert_eq!(fetched, expected, "fetched status of {}", identity_id);

    let proof = drive
        .prove_contract_moderation_status(contract_id, identity_id, lists, None, platform_version)
        .expect("expected a status proof");
    let (proved_root, proved) = Drive::verify_contract_moderation_status(
        &proof,
        contract_id,
        identity_id,
        lists,
        false,
        platform_version,
    )
    .expect("expected to verify the status proof");
    assert_eq!(proved_root, root_hash(drive, platform_version));
    assert_eq!(
        proved,
        ContractModerationListStatuses::from_status(lists, &expected),
        "proved status of {}",
        identity_id
    );
}

/// Fetches, proves and verifies one entries page and checks all three agree.
fn assert_entries(
    drive: &Drive,
    contract_id: Identifier,
    query: &ContractModerationEntriesQuery,
    expected: &[ContractModerationEntry],
) {
    let platform_version = PlatformVersion::latest();
    let fetched = drive
        .fetch_contract_moderation_entries(contract_id, query, None, platform_version)
        .expect("expected to fetch the entries");
    assert_eq!(fetched.as_slice(), expected, "fetched {:?}", query);

    let proof = drive
        .prove_contract_moderation_entries(contract_id, query, None, platform_version)
        .expect("expected an entries proof");
    let (proved_root, proved) =
        Drive::verify_contract_moderation_entries(&proof, contract_id, query, platform_version)
            .expect("expected to verify the entries proof");
    assert_eq!(proved_root, root_hash(drive, platform_version));
    assert_eq!(proved.as_slice(), expected, "proved {:?}", query);
}

const BOTH: [ContractModerationList; 2] = [
    ContractModerationList::Banlist,
    ContractModerationList::Suspensions,
];

#[test]
fn should_create_the_list_trees_only_when_the_config_declares_them() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));

    let unmoderated = moderated_contract(false, false);
    insert(&drive, &unmoderated, platform_version);
    assert!(!has_list_tree(
        &drive,
        unmoderated.id(),
        CONTRACT_BANLIST_KEY
    ));
    assert!(!has_list_tree(
        &drive,
        unmoderated.id(),
        CONTRACT_SUSPENSIONS_KEY
    ));

    let mut banlist_only = moderated_contract(true, false);
    banlist_only.set_id(identity(0x11));
    insert(&drive, &banlist_only, platform_version);
    assert!(has_list_tree(
        &drive,
        banlist_only.id(),
        CONTRACT_BANLIST_KEY
    ));
    assert!(!has_list_tree(
        &drive,
        banlist_only.id(),
        CONTRACT_SUSPENSIONS_KEY
    ));

    let mut both = moderated_contract(true, true);
    both.set_id(identity(0x12));
    insert(&drive, &both, platform_version);
    assert!(has_list_tree(&drive, both.id(), CONTRACT_BANLIST_KEY));
    assert!(has_list_tree(&drive, both.id(), CONTRACT_SUSPENSIONS_KEY));
}

#[test]
fn should_keep_the_list_trees_and_their_entries_across_a_contract_update() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));

    let contract = moderated_contract(true, false);
    insert(&drive, &contract, platform_version);
    let target = identity(0x41);
    drive
        .add_contract_ban(
            contract.id(),
            target,
            &reason("spam"),
            contract.owner_id(),
            &BlockInfo::default(),
            true,
            None,
            platform_version,
        )
        .expect("expected to ban");

    // Which lists a contract keeps is fixed at creation, so an update only ever changes the
    // moderators. It leaves the trees and what they hold alone, and creates none.
    let mut updated = contract.clone();
    updated.set_version(contract.version() + 1);
    updated.set_config(
        updated
            .config()
            .clone()
            .with_moderation(Some(ContractModerationConfig {
                banlist: true,
                suspensions: false,
                moderators: ContractModerators::AppointedModerators(
                    [identity(0x42)].into_iter().collect(),
                ),
                warnings: false,
            })),
    );
    drive
        .update_contract(
            &updated,
            BlockInfo::default(),
            true,
            None,
            platform_version,
            None,
        )
        .expect("expected to update the contract");

    assert!(has_list_tree(&drive, contract.id(), CONTRACT_BANLIST_KEY));
    assert!(!has_list_tree(
        &drive,
        contract.id(),
        CONTRACT_SUSPENSIONS_KEY
    ));
    assert_status(
        &drive,
        contract.id(),
        target,
        &[ContractModerationList::Banlist],
        banned_for("spam"),
    );
}

#[test]
fn should_ban_and_unban_and_prove_the_status_and_the_entries() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = moderated_contract(true, true);
    insert(&drive, &contract, platform_version);
    let contract_id = contract.id();
    let moderator = contract.owner_id();
    let target = identity(0x21);

    assert_status(
        &drive,
        contract_id,
        target,
        &BOTH,
        ContractModerationStatus::default(),
    );

    let fee = drive
        .add_contract_ban(
            contract_id,
            target,
            &reason("spam"),
            moderator,
            &BlockInfo::default(),
            true,
            None,
            platform_version,
        )
        .expect("expected to ban");
    assert!(fee.storage_fee > 0, "a ban stores an entry");

    assert_status(&drive, contract_id, target, &BOTH, banned_for("spam"));
    assert_entries(
        &drive,
        contract_id,
        &ContractModerationEntriesQuery {
            list: ContractModerationList::Banlist,
            start_after: None,
            limit: 10,
        },
        &[ContractModerationEntry {
            identity_id: target,
            until: None,
            reason: reason("spam"),
            warnings: vec![],
        }],
    );

    let fee = drive
        .remove_contract_ban(
            contract_id,
            target,
            &BlockInfo::default(),
            true,
            None,
            platform_version,
        )
        .expect("expected to unban");
    assert!(
        fee.fee_refunds
            .calculate_refunds_amount_for_identity(moderator)
            .is_some(),
        "the moderator gets the storage back"
    );

    assert_status(
        &drive,
        contract_id,
        target,
        &BOTH,
        ContractModerationStatus::default(),
    );
    assert_entries(
        &drive,
        contract_id,
        &ContractModerationEntriesQuery {
            list: ContractModerationList::Banlist,
            start_after: None,
            limit: 10,
        },
        &[],
    );
}

#[test]
fn should_suspend_replace_and_unsuspend() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = moderated_contract(false, true);
    insert(&drive, &contract, platform_version);
    let contract_id = contract.id();
    let moderator = contract.owner_id();
    let target = identity(0x31);
    let lists = [ContractModerationList::Suspensions];

    drive
        .add_contract_suspension(
            contract_id,
            target,
            10,
            &reason("flooding"),
            false,
            moderator,
            &BlockInfo::default(),
            true,
            None,
            platform_version,
        )
        .expect("expected to suspend");
    assert_status(
        &drive,
        contract_id,
        target,
        &lists,
        suspended_until(10, "flooding"),
    );

    drive
        .add_contract_suspension(
            contract_id,
            target,
            20,
            &reason("flooding again, after a warning"),
            true,
            moderator,
            &BlockInfo::default(),
            true,
            None,
            platform_version,
        )
        .expect("expected to replace the suspension");
    // The replacement brings its own reason, of another length.
    assert_status(
        &drive,
        contract_id,
        target,
        &lists,
        suspended_until(20, "flooding again, after a warning"),
    );
    assert_entries(
        &drive,
        contract_id,
        &ContractModerationEntriesQuery {
            list: ContractModerationList::Suspensions,
            start_after: None,
            limit: 10,
        },
        &[ContractModerationEntry {
            identity_id: target,
            until: Some(20),
            reason: reason("flooding again, after a warning"),
            warnings: vec![],
        }],
    );

    drive
        .remove_contract_suspension(
            contract_id,
            target,
            &BlockInfo::default(),
            true,
            None,
            platform_version,
        )
        .expect("expected to unsuspend");
    assert_status(
        &drive,
        contract_id,
        target,
        &lists,
        ContractModerationStatus::default(),
    );
}

#[test]
fn should_estimate_before_applying_every_writer() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = moderated_contract(true, true);
    insert(&drive, &contract, platform_version);
    let contract_id = contract.id();
    let moderator = contract.owner_id();
    let target = identity(0x41);
    let block_info = BlockInfo::default();

    let estimated = drive
        .add_contract_ban(
            contract_id,
            target,
            &reason("spam"),
            moderator,
            &block_info,
            false,
            None,
            platform_version,
        )
        .expect("expected to estimate a ban");
    let applied = drive
        .add_contract_ban(
            contract_id,
            target,
            &reason("spam"),
            moderator,
            &block_info,
            true,
            None,
            platform_version,
        )
        .expect("expected to ban");
    assert_eq!(estimated.storage_fee, applied.storage_fee, "ban storage");
    assert!(estimated.processing_fee > 0);

    let estimated = drive
        .add_contract_suspension(
            contract_id,
            target,
            7,
            &reason("flooding"),
            false,
            moderator,
            &block_info,
            false,
            None,
            platform_version,
        )
        .expect("expected to estimate a suspension");
    let applied = drive
        .add_contract_suspension(
            contract_id,
            target,
            7,
            &reason("flooding"),
            false,
            moderator,
            &block_info,
            true,
            None,
            platform_version,
        )
        .expect("expected to suspend");
    assert_eq!(
        estimated.storage_fee, applied.storage_fee,
        "suspension storage"
    );

    let estimated = drive
        .remove_contract_suspension(
            contract_id,
            target,
            &block_info,
            false,
            None,
            platform_version,
        )
        .expect("expected to estimate an unsuspend");
    assert!(estimated.processing_fee > 0);
    drive
        .remove_contract_suspension(
            contract_id,
            target,
            &block_info,
            true,
            None,
            platform_version,
        )
        .expect("expected to unsuspend");
    let estimated = drive
        .remove_contract_ban(
            contract_id,
            target,
            &block_info,
            false,
            None,
            platform_version,
        )
        .expect("expected to estimate an unban");
    assert!(estimated.processing_fee > 0);
    drive
        .remove_contract_ban(
            contract_id,
            target,
            &block_info,
            true,
            None,
            platform_version,
        )
        .expect("expected to unban");
    assert_status(
        &drive,
        contract_id,
        target,
        &BOTH,
        ContractModerationStatus::default(),
    );
}

#[test]
fn should_page_entries_with_a_cursor_and_bound_the_limit() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = moderated_contract(true, false);
    insert(&drive, &contract, platform_version);
    let contract_id = contract.id();
    let moderator = contract.owner_id();
    let targets = [identity(0x51), identity(0x52), identity(0x53)];
    for target in targets {
        drive
            .add_contract_ban(
                contract_id,
                target,
                &reason("spam"),
                moderator,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to ban");
    }
    let entry = |identity_id| ContractModerationEntry {
        identity_id,
        until: None,
        reason: reason("spam"),
        warnings: vec![],
    };

    let first_page = ContractModerationEntriesQuery {
        list: ContractModerationList::Banlist,
        start_after: None,
        limit: 2,
    };
    assert_entries(
        &drive,
        contract_id,
        &first_page,
        &[entry(targets[0]), entry(targets[1])],
    );
    let second_page = ContractModerationEntriesQuery {
        start_after: Some(targets[1]),
        ..first_page.clone()
    };
    assert_entries(&drive, contract_id, &second_page, &[entry(targets[2])]);

    // The suspension list is not kept: an empty page, no error.
    let fetched = drive
        .fetch_contract_moderation_entries(
            contract_id,
            &ContractModerationEntriesQuery {
                list: ContractModerationList::Suspensions,
                start_after: None,
                limit: 2,
            },
            None,
            platform_version,
        )
        .expect("expected an empty page for a list the contract does not keep");
    assert!(fetched.is_empty());

    let zero = ContractModerationEntriesQuery {
        limit: 0,
        ..first_page
    };
    assert!(drive
        .fetch_contract_moderation_entries(contract_id, &zero, None, platform_version)
        .is_err());
    assert!(drive
        .prove_contract_moderation_entries(contract_id, &zero, None, platform_version)
        .is_err());
}

#[test]
fn should_refund_the_first_moderator_when_another_replaces_the_suspension() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = moderated_contract(false, true);
    insert(&drive, &contract, platform_version);
    let contract_id = contract.id();
    let first_moderator = identity(0x51);
    let second_moderator = identity(0x52);
    let target = identity(0x53);

    let fee = drive
        .add_contract_suspension(
            contract_id,
            target,
            10,
            &reason("flooding"),
            false,
            first_moderator,
            &BlockInfo::default_with_epoch(Epoch::new(0).expect("epoch 0")),
            true,
            None,
            platform_version,
        )
        .expect("expected to suspend");
    assert!(fee.storage_fee > 0, "a suspension stores an entry");

    // Another moderator, a later epoch, a reason of the same length. The entry keeps its size,
    // so the replacement stores nothing new, and the storage stays the first moderator's.
    let later = BlockInfo::default_with_epoch(Epoch::new(3).expect("epoch 3"));
    let fee = drive
        .add_contract_suspension(
            contract_id,
            target,
            20,
            &reason("flooding"),
            true,
            second_moderator,
            &later,
            true,
            None,
            platform_version,
        )
        .expect("expected to replace the suspension");
    assert_eq!(fee.storage_fee, 0, "a same-size replacement stores nothing");

    let fee = drive
        .remove_contract_suspension(contract_id, target, &later, true, None, platform_version)
        .expect("expected to unsuspend");
    assert!(
        fee.fee_refunds
            .calculate_refunds_amount_for_identity(first_moderator)
            .is_some(),
        "the moderator that paid for the entry gets the storage back"
    );
    assert!(
        fee.fee_refunds
            .calculate_refunds_amount_for_identity(second_moderator)
            .is_none(),
        "the replacing moderator paid for no storage"
    );
}

#[test]
fn should_bill_the_replacing_moderator_for_a_longer_reason() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = moderated_contract(false, true);
    insert(&drive, &contract, platform_version);
    let contract_id = contract.id();
    let first_moderator = identity(0x51);
    let second_moderator = identity(0x52);
    let target = identity(0x53);

    let first_fee = drive
        .add_contract_suspension(
            contract_id,
            target,
            10,
            &reason("flooding"),
            false,
            first_moderator,
            &BlockInfo::default_with_epoch(Epoch::new(0).expect("epoch 0")),
            true,
            None,
            platform_version,
        )
        .expect("expected to suspend");

    // Another moderator, a later epoch, a longer reason: the added bytes are the second
    // moderator's to pay, and the flags merge as they do when a document changes hands, so
    // the replacement does not fail on the two owners.
    let later = BlockInfo::default_with_epoch(Epoch::new(3).expect("epoch 3"));
    let longer = reason(&"flooding ".repeat(40));
    let second_fee = drive
        .add_contract_suspension(
            contract_id,
            target,
            20,
            &longer,
            true,
            second_moderator,
            &later,
            true,
            None,
            platform_version,
        )
        .expect("expected to replace the suspension with a longer reason");
    assert!(second_fee.storage_fee > 0, "the added bytes are stored");
    assert!(
        second_fee.storage_fee > first_fee.storage_fee,
        "the longer reason costs more than the whole first entry"
    );
    assert_status(
        &drive,
        contract_id,
        target,
        &[ContractModerationList::Suspensions],
        ContractModerationStatus {
            ban: None,
            suspension: Some(ContractSuspension {
                until: 20,
                reason: longer,
            }),
            warnings: vec![],
        },
    );

    // The entry, and the refund of its removal, passed to the moderator that replaced it.
    let fee = drive
        .remove_contract_suspension(contract_id, target, &later, true, None, platform_version)
        .expect("expected to unsuspend");
    assert!(
        fee.fee_refunds
            .calculate_refunds_amount_for_identity(second_moderator)
            .is_some(),
        "the replacing moderator owns the entry"
    );
    assert!(
        fee.fee_refunds
            .calculate_refunds_amount_for_identity(first_moderator)
            .is_none(),
        "the first moderator's bytes passed on with the entry"
    );
}

#[test]
fn should_keep_a_shorter_replacement_with_the_first_moderator() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = moderated_contract(false, true);
    insert(&drive, &contract, platform_version);
    let contract_id = contract.id();
    let first_moderator = identity(0x51);
    let second_moderator = identity(0x52);
    let target = identity(0x53);

    drive
        .add_contract_suspension(
            contract_id,
            target,
            10,
            &reason(&"flooding ".repeat(40)),
            false,
            first_moderator,
            &BlockInfo::default_with_epoch(Epoch::new(0).expect("epoch 0")),
            true,
            None,
            platform_version,
        )
        .expect("expected to suspend");

    // Another moderator, a later epoch, a shorter reason: nothing is added, the removed bytes
    // go back to the moderator that paid for them, and the entry stays that moderator's.
    let later = BlockInfo::default_with_epoch(Epoch::new(3).expect("epoch 3"));
    let fee = drive
        .add_contract_suspension(
            contract_id,
            target,
            20,
            &reason("flooding"),
            true,
            second_moderator,
            &later,
            true,
            None,
            platform_version,
        )
        .expect("expected to replace the suspension with a shorter reason");
    assert_eq!(fee.storage_fee, 0, "a shorter replacement stores nothing");
    assert!(
        fee.fee_refunds
            .calculate_refunds_amount_for_identity(first_moderator)
            .is_some(),
        "the removed bytes are refunded to the moderator that paid for them"
    );
    assert!(fee
        .fee_refunds
        .calculate_refunds_amount_for_identity(second_moderator)
        .is_none());
    assert_status(
        &drive,
        contract_id,
        target,
        &[ContractModerationList::Suspensions],
        suspended_until(20, "flooding"),
    );

    let fee = drive
        .remove_contract_suspension(contract_id, target, &later, true, None, platform_version)
        .expect("expected to unsuspend");
    assert!(
        fee.fee_refunds
            .calculate_refunds_amount_for_identity(first_moderator)
            .is_some(),
        "the first moderator still owns the entry"
    );
    assert!(fee
        .fee_refunds
        .calculate_refunds_amount_for_identity(second_moderator)
        .is_none());
}

#[test]
fn should_not_estimate_a_replacement_below_what_it_costs() {
    let platform_version = PlatformVersion::latest();
    let max_length = platform_version
        .system_limits
        .max_contract_moderation_reason_length as usize;

    // From the shortest entry to the longest, and the other way: GroveDB's average-case replace
    // assumes an item keeps its size, which would price no storage for the longer reason.
    for (from, to) in [(0, max_length), (max_length, 0), (8, 8)] {
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let contract = moderated_contract(false, true);
        insert(&drive, &contract, platform_version);
        let contract_id = contract.id();
        let moderator = contract.owner_id();
        let target = identity(0x54);
        let suspend = |until: u64, length: usize, replaces_existing: bool, apply: bool| {
            drive
                .add_contract_suspension(
                    contract_id,
                    target,
                    until,
                    &reason(&"x".repeat(length)),
                    replaces_existing,
                    moderator,
                    &BlockInfo::default(),
                    apply,
                    None,
                    platform_version,
                )
                .expect("expected to suspend")
        };

        suspend(10, from, false, true);
        let estimated = suspend(20, to, true, false);
        let applied = suspend(20, to, true, true);
        assert!(
            estimated.storage_fee >= applied.storage_fee,
            "{from} -> {to} bytes: estimated storage {} below applied {}",
            estimated.storage_fee,
            applied.storage_fee
        );
        assert!(
            estimated.total_base_fee() >= applied.total_base_fee(),
            "{from} -> {to} bytes: estimated {} below applied {}",
            estimated.total_base_fee(),
            applied.total_base_fee()
        );
    }
}

#[test]
fn should_charge_a_ban_by_the_length_of_its_reason() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = moderated_contract(true, false);
    insert(&drive, &contract, platform_version);
    let contract_id = contract.id();
    let moderator = contract.owner_id();
    let max_length = platform_version
        .system_limits
        .max_contract_moderation_reason_length as usize;

    let ban = |seed: u8, reason: &ContractModerationReason, apply: bool| {
        drive
            .add_contract_ban(
                contract_id,
                identity(seed),
                reason,
                moderator,
                &BlockInfo::default(),
                apply,
                None,
                platform_version,
            )
            .expect("expected to ban")
    };

    let empty = ban(0x61, &ContractModerationReason::default(), true);
    let longest = ContractModerationReason {
        code: Some(u16::MAX),
        text: "x".repeat(max_length),
        documents: vec![],
        reason_document_id: None,
    };
    let estimated = ban(0x62, &longest, false);
    let full = ban(0x62, &longest, true);

    // Two bytes of code and the text, at the storage price of a byte, and the few bytes the
    // length prefixes of a longer value grow by.
    let per_byte = platform_version
        .fee_version
        .storage
        .storage_disk_usage_credit_per_byte;
    let added_bytes = (full.storage_fee - empty.storage_fee) / per_byte;
    let reason_bytes = max_length as u64 + 2;
    assert!(
        (reason_bytes..=reason_bytes + 8).contains(&added_bytes),
        "{} bytes added for a reason of {}",
        added_bytes,
        reason_bytes
    );
    assert_eq!(estimated.storage_fee, full.storage_fee, "estimated storage");

    // The longest entry reads back whole.
    assert_status(
        &drive,
        contract_id,
        identity(0x62),
        &[ContractModerationList::Banlist],
        ContractModerationStatus {
            ban: Some(ContractBan { reason: longest }),
            suspension: None,
            warnings: vec![],
        },
    );
}

#[test]
fn should_say_nothing_about_a_list_the_status_proof_does_not_cover() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = moderated_contract(true, true);
    insert(&drive, &contract, platform_version);
    let target = identity(0x61);
    drive
        .add_contract_ban(
            contract.id(),
            target,
            &reason("spam"),
            contract.owner_id(),
            &BlockInfo::default(),
            true,
            None,
            platform_version,
        )
        .expect("expected to ban");

    // A proof over the suspension list alone verifies, and its result leaves the banlist
    // unknown rather than reporting the banned identity as not banned.
    let lists = [ContractModerationList::Suspensions];
    let proof = drive
        .prove_contract_moderation_status(contract.id(), target, &lists, None, platform_version)
        .expect("expected a status proof");
    let (_, proved) = Drive::verify_contract_moderation_status(
        &proof,
        contract.id(),
        target,
        &lists,
        false,
        platform_version,
    )
    .expect("expected to verify the status proof");
    assert_eq!(proved.banned(), None);
    assert_eq!(proved.suspended_until(), Some(None));
    assert!(!proved.is_barred_on_queried_lists_at(0));
    assert_eq!(proved.0.len(), 1);
}

#[test]
fn should_create_the_warning_list_tree_only_when_the_config_declares_it() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));

    let without = moderated_contract(true, true);
    insert(&drive, &without, platform_version);
    assert!(!has_list_tree(&drive, without.id(), CONTRACT_WARNINGS_KEY));

    let mut with = moderated_contract_keeping(false, false, true);
    with.set_id(identity(0x13));
    insert(&drive, &with, platform_version);
    assert!(has_list_tree(&drive, with.id(), CONTRACT_WARNINGS_KEY));
    assert!(!has_list_tree(&drive, with.id(), CONTRACT_BANLIST_KEY));
    assert!(!has_list_tree(&drive, with.id(), CONTRACT_SUSPENSIONS_KEY));
}

#[test]
fn should_warn_accumulate_clear_and_prove_the_status_and_the_entries() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = moderated_contract_keeping(true, false, true);
    insert(&drive, &contract, platform_version);
    let contract_id = contract.id();
    let first_moderator = contract.owner_id();
    let second_moderator = identity(0x62);
    let target = identity(0x61);
    let lists = [ContractModerationList::Warnings];
    let first = warning(1_000, "first strike");
    let second = warning(2_000, "second strike, a longer one");

    let first_fee = drive
        .add_contract_warning(
            contract_id,
            target,
            std::slice::from_ref(&first),
            false,
            first_moderator,
            &BlockInfo::default_with_epoch(Epoch::new(0).expect("epoch 0")),
            true,
            None,
            platform_version,
        )
        .expect("expected to warn");
    assert!(first_fee.storage_fee > 0, "a warning stores an entry");
    assert_status(
        &drive,
        contract_id,
        target,
        &lists,
        warned_with(vec![first.clone()]),
    );
    // The banlist says nothing about it.
    assert_status(
        &drive,
        contract_id,
        target,
        &BOTH[..1],
        ContractModerationStatus::default(),
    );

    // A second warning, by another moderator in a later epoch: the entry is rewritten one
    // warning longer, so it passes to the moderator that warned last, who pays the added
    // bytes.
    let later = BlockInfo::default_with_epoch(Epoch::new(3).expect("epoch 3"));
    let second_fee = drive
        .add_contract_warning(
            contract_id,
            target,
            &[first.clone(), second.clone()],
            true,
            second_moderator,
            &later,
            true,
            None,
            platform_version,
        )
        .expect("expected to warn again");
    assert!(second_fee.storage_fee > 0, "the added warning is stored");
    assert_status(
        &drive,
        contract_id,
        target,
        &lists,
        warned_with(vec![first.clone(), second.clone()]),
    );
    assert_entries(
        &drive,
        contract_id,
        &ContractModerationEntriesQuery {
            list: ContractModerationList::Warnings,
            start_after: None,
            limit: 10,
        },
        &[ContractModerationEntry {
            identity_id: target,
            until: None,
            // The entry's reason is the latest warning's; every warning comes along.
            reason: second.reason.clone(),
            warnings: vec![first, second],
        }],
    );

    let fee = drive
        .remove_contract_warnings(contract_id, target, &later, true, None, platform_version)
        .expect("expected to clear the warnings");
    assert!(
        fee.fee_refunds
            .calculate_refunds_amount_for_identity(second_moderator)
            .is_some(),
        "the moderator that warned last owns the entry"
    );
    assert!(
        fee.fee_refunds
            .calculate_refunds_amount_for_identity(first_moderator)
            .is_none(),
        "the first moderator's bytes passed on with the entry"
    );
    assert_status(
        &drive,
        contract_id,
        target,
        &lists,
        ContractModerationStatus::default(),
    );
    assert_entries(
        &drive,
        contract_id,
        &ContractModerationEntriesQuery {
            list: ContractModerationList::Warnings,
            start_after: None,
            limit: 10,
        },
        &[],
    );
}

#[test]
fn should_not_estimate_a_warning_below_what_it_costs() {
    let platform_version = PlatformVersion::latest();
    let max_length = platform_version
        .system_limits
        .max_contract_moderation_reason_length as usize;
    let max_warnings = platform_version
        .system_limits
        .max_contract_warnings_per_identity as usize;
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = moderated_contract_keeping(false, false, true);
    insert(&drive, &contract, platform_version);
    let contract_id = contract.id();
    let moderator = contract.owner_id();
    let target = identity(0x54);
    let warn = |warnings: &[ContractWarning], replaces_existing: bool, apply: bool| {
        drive
            .add_contract_warning(
                contract_id,
                target,
                warnings,
                replaces_existing,
                moderator,
                &BlockInfo::default(),
                apply,
                None,
                platform_version,
            )
            .expect("expected to warn")
    };

    // The first warning is an insert, estimated as one.
    let first = vec![warning(1, "spam")];
    let estimated = warn(&first, false, false);
    let applied = warn(&first, false, true);
    assert_eq!(estimated.storage_fee, applied.storage_fee, "first warning");

    // Every later one replaces the entry with a longer one. GroveDB's average-case replace
    // assumes an item keeps its size, so a replacement is estimated as an insert of the whole
    // entry, an upper bound: up to the fullest entry the protocol admits.
    let mut warnings = first;
    for index in 1..max_warnings {
        warnings.push(warning(index as u64 + 1, &"x".repeat(max_length)));
        let estimated = warn(&warnings, true, false);
        let applied = warn(&warnings, true, true);
        assert!(
            estimated.storage_fee >= applied.storage_fee,
            "warning {}: estimated storage {} below applied {}",
            index + 1,
            estimated.storage_fee,
            applied.storage_fee
        );
        assert!(
            estimated.total_base_fee() >= applied.total_base_fee(),
            "warning {}: estimated {} below applied {}",
            index + 1,
            estimated.total_base_fee(),
            applied.total_base_fee()
        );
    }
    // The fullest entry reads back whole.
    assert_status(
        &drive,
        contract_id,
        target,
        &[ContractModerationList::Warnings],
        warned_with(warnings),
    );
    let estimated = drive
        .remove_contract_warnings(
            contract_id,
            target,
            &BlockInfo::default(),
            false,
            None,
            platform_version,
        )
        .expect("expected to estimate a clearing");
    assert!(estimated.processing_fee > 0);
}

/// The root key of the Merk at `path`/`key`, read from the tree element that points at it.
fn merk_root_key(drive: &Drive, path: &[&[u8]], key: &[u8]) -> Option<Vec<u8>> {
    let platform_version = PlatformVersion::latest();
    let element = drive
        .grove
        .get_raw(
            path.into(),
            key,
            None,
            &platform_version.drive.grove_version,
        )
        .unwrap()
        .expect("expected the tree element");
    match element {
        Element::Tree(root_key, _) => root_key,
        other => panic!("expected a tree, got {other:?}"),
    }
}

#[test]
fn should_keep_the_documents_on_top_of_the_contract_subtree_and_the_banlist_on_top_of_the_other_tree(
) {
    let platform_version = PlatformVersion::latest();
    // (banlist, suspensions, warnings) -> the key on top of the contract's other tree
    for (banlist, suspensions, warnings, top_of_other) in [
        (false, false, false, CONTRACT_VERSION_KEY),
        (true, false, false, CONTRACT_BANLIST_KEY),
        (false, true, false, CONTRACT_SUSPENSIONS_KEY),
        (true, true, false, CONTRACT_BANLIST_KEY),
        (false, false, true, CONTRACT_WARNINGS_KEY),
        (true, false, true, CONTRACT_BANLIST_KEY),
        (false, true, true, CONTRACT_SUSPENSIONS_KEY),
        // Four keys created at once root at the upper middle: the one combination where the
        // banlist sits a level down. With the removal records tree (key 16) beside them it is
        // on top again.
        (true, true, true, CONTRACT_SUSPENSIONS_KEY),
    ] {
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let contract = moderated_contract_keeping(banlist, suspensions, warnings);
        insert(&drive, &contract, platform_version);
        let contract_id = contract.id();
        let contracts_root: &[u8] = Into::<&[u8; 1]>::into(RootTree::DataContractDocuments);

        // Three keys under the contract (itself, its documents, its other tree), so the
        // documents, which every document proof and write goes through, stay on top whatever
        // the contract keeps in its other tree.
        assert_eq!(
            merk_root_key(&drive, &[contracts_root], contract_id.as_slice()),
            Some(vec![1]),
            "banlist {banlist}, suspensions {suspensions}, warnings {warnings}"
        );
        assert_eq!(
            merk_root_key(
                &drive,
                &[contracts_root, contract_id.as_slice()],
                &[CONTRACT_OTHER_KEY]
            ),
            Some(vec![top_of_other]),
            "banlist {banlist}, suspensions {suspensions}, warnings {warnings}"
        );
    }
}
