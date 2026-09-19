use crate::drive::contract::moderation::types::{
    ContractModerationEntriesQuery, ContractModerationEntry,
};
use crate::drive::contract::paths::{
    contract_root_path, CONTRACT_BANLIST_KEY, CONTRACT_SUSPENSIONS_KEY,
};
use crate::drive::Drive;
use crate::util::grove_operations::DirectQueryType;
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::Epoch;
use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
use dpp::data_contract::config::moderation::{
    ContractModerationConfig, ContractModerationList, ContractModerationStatus, ContractModerators,
};
use dpp::data_contract::DataContract;
use dpp::identifier::Identifier;
use dpp::tests::fixtures::get_data_contract_fixture;
use dpp::version::PlatformVersion;

fn identity(seed: u8) -> Identifier {
    Identifier::from([seed; 32])
}

fn moderated_contract(banlist: bool, suspensions: bool) -> DataContract {
    let platform_version = PlatformVersion::latest();
    let mut contract =
        get_data_contract_fixture(None, 0, platform_version.protocol_version).data_contract_owned();
    let moderation = (banlist || suspensions).then_some(ContractModerationConfig {
        banlist,
        suspensions,
        moderators: ContractModerators::ContractOwner,
    });
    contract.set_config(contract.config().clone().with_moderation(moderation));
    contract
}

fn insert(drive: &Drive, contract: &DataContract, platform_version: &PlatformVersion) {
    drive
        .insert_contract(contract, BlockInfo::default(), true, None, platform_version)
        .expect("expected to insert the contract");
}

fn has_list_tree(drive: &Drive, contract_id: Identifier, key: u8) -> bool {
    drive
        .grove_has_raw(
            (&contract_root_path(contract_id.as_slice())).into(),
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
        platform_version,
    )
    .expect("expected to verify the status proof");
    assert_eq!(proved_root, root_hash(drive, platform_version));
    assert_eq!(proved, expected, "proved status of {}", identity_id);
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
                moderators: ContractModerators::OwnerAndIdentities(
                    [identity(0x42)].into_iter().collect(),
                ),
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
        ContractModerationStatus {
            banned: true,
            suspended_until: None,
        },
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
            moderator,
            &BlockInfo::default(),
            true,
            None,
            platform_version,
        )
        .expect("expected to ban");
    assert!(fee.storage_fee > 0, "a ban stores an entry");

    assert_status(
        &drive,
        contract_id,
        target,
        &BOTH,
        ContractModerationStatus {
            banned: true,
            suspended_until: None,
        },
    );
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
        ContractModerationStatus {
            banned: false,
            suspended_until: Some(10),
        },
    );

    drive
        .add_contract_suspension(
            contract_id,
            target,
            20,
            true,
            moderator,
            &BlockInfo::default(),
            true,
            None,
            platform_version,
        )
        .expect("expected to replace the suspension");
    assert_status(
        &drive,
        contract_id,
        target,
        &lists,
        ContractModerationStatus {
            banned: false,
            suspended_until: Some(20),
        },
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
            false,
            first_moderator,
            &BlockInfo::default_with_epoch(Epoch::new(0).expect("epoch 0")),
            true,
            None,
            platform_version,
        )
        .expect("expected to suspend");
    assert!(fee.storage_fee > 0, "a suspension stores an entry");

    // Another moderator, a later epoch. The entry keeps its size, so the replacement stores
    // nothing new, and the storage stays the first moderator's.
    let later = BlockInfo::default_with_epoch(Epoch::new(3).expect("epoch 3"));
    let fee = drive
        .add_contract_suspension(
            contract_id,
            target,
            20,
            true,
            second_moderator,
            &later,
            true,
            None,
            platform_version,
        )
        .expect("expected to replace the suspension");
    assert_eq!(
        fee.storage_fee, 0,
        "a fixed-size replacement stores nothing"
    );

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
