//! The moderation action counts of an elected contract's seated team (`[64, id, 2, 48]`).

use crate::drive::contract::paths::{
    contract_other_path, CONTRACT_BANLIST_KEY, CONTRACT_MODERATION_ACTION_COUNTS_KEY,
    CONTRACT_OTHER_KEY, CONTRACT_VERSION_KEY,
};
use crate::drive::{Drive, RootTree};
use crate::util::batch::{ContractModerationOperationType, DriveOperation};
use crate::util::grove_operations::DirectQueryType;
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::Epoch;
use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
use dpp::data_contract::config::moderation::{
    ContractModerationConfig, ContractModerators, ElectedModerators, InterimModerators,
    ModerationAbility, DEFAULT_ELECTION_WINDOW_SECONDS,
};
use dpp::data_contract::DataContract;
use dpp::identifier::Identifier;
use dpp::tests::fixtures::get_data_contract_fixture;
use dpp::version::PlatformVersion;
use grovedb::Element;
use std::collections::{BTreeMap, BTreeSet};

fn member(seed: u8) -> Identifier {
    Identifier::from([seed; 32])
}

/// A contract keeping the lists given, whose moderators are elected when `elected`, the owner
/// otherwise
fn contract_keeping(
    banlist: bool,
    suspensions: bool,
    warnings: bool,
    elected: bool,
) -> DataContract {
    let platform_version = PlatformVersion::latest();
    let mut contract =
        get_data_contract_fixture(None, 0, platform_version.protocol_version).data_contract_owned();
    let moderators = if elected {
        ContractModerators::Elected(Box::new(ElectedModerators {
            join_window: DEFAULT_ELECTION_WINDOW_SECONDS,
            vote_window: DEFAULT_ELECTION_WINDOW_SECONDS,
            challenge_cool_down: Some(1_209_600),
            election_delay: None,
            max_added_moderators: 0,
            moderated_document_types: BTreeMap::from([(
                "niceDocument".to_string(),
                BTreeSet::from([ModerationAbility::Ban]),
            )]),
            interim: InterimModerators::NotYetUsable,
            owner_protected: false,
        }))
    } else {
        ContractModerators::ContractOwner
    };
    contract.set_config(contract.config().clone().with_moderation(Some(
        ContractModerationConfig {
            banlist,
            suspensions,
            warnings,
            moderators,
        },
    )));
    contract
}

fn elected_contract(drive: &Drive, platform_version: &PlatformVersion) -> DataContract {
    let contract = contract_keeping(true, false, false, true);
    drive
        .insert_contract(
            &contract,
            BlockInfo::default(),
            true,
            None,
            platform_version,
        )
        .expect("expected to insert the contract");
    contract
}

fn has_counts_tree(drive: &Drive, contract_id: Identifier) -> bool {
    drive
        .grove_has_raw(
            (&contract_other_path(contract_id.as_slice())).into(),
            &[CONTRACT_MODERATION_ACTION_COUNTS_KEY],
            DirectQueryType::StatefulDirectQuery,
            None,
            &mut vec![],
            &PlatformVersion::latest().drive,
        )
        .expect("expected to query the other tree")
}

fn set_count(
    identity_id: Identifier,
    count: u32,
    contract_id: Identifier,
) -> DriveOperation<'static> {
    DriveOperation::ContractModerationOperation(ContractModerationOperationType::SetActionCount {
        contract_id,
        identity_id,
        count,
    })
}

fn apply(
    drive: &Drive,
    operations: Vec<DriveOperation>,
    apply: bool,
) -> dpp::fee::fee_result::FeeResult {
    drive
        .apply_drive_operations(
            operations,
            apply,
            &BlockInfo::default(),
            None,
            PlatformVersion::latest(),
            None,
        )
        .expect("expected to apply the operations")
}

#[test]
fn should_create_the_action_counts_tree_with_an_elected_contract_only() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let elected = elected_contract(&drive, platform_version);
    assert!(has_counts_tree(&drive, elected.id()));

    let owned = contract_keeping(true, false, false, false);
    let mut owned_contract = owned;
    owned_contract.set_id(member(0x77));
    drive
        .insert_contract(
            &owned_contract,
            BlockInfo::default(),
            true,
            None,
            platform_version,
        )
        .expect("expected to insert the contract");
    assert!(!has_counts_tree(&drive, owned_contract.id()));

    // A contract without the tree (an elected one stored before the counts existed reads the
    // same) has no counts rather than a failing read.
    let epoch = Epoch::new(0).expect("epoch");
    let (_, count) = drive
        .fetch_contract_moderation_action_count_with_fee(
            owned_contract.id(),
            member(1),
            &epoch,
            None,
            platform_version,
        )
        .expect("expected the read to find no tree");
    assert_eq!(count, None);
    assert!(drive
        .fetch_contract_moderation_action_counts(owned_contract.id(), 31, None, platform_version)
        .expect("expected the read to find no tree")
        .is_empty());
}

#[test]
fn should_write_read_and_reset_the_action_counts() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract_id = elected_contract(&drive, platform_version).id();

    assert!(drive
        .fetch_contract_moderation_action_counts(contract_id, 31, None, platform_version)
        .expect("expected to read the counts")
        .is_empty());

    // The first count of a member is an insert, estimated as one.
    let estimated = apply(&drive, vec![set_count(member(1), 1, contract_id)], false);
    let applied = apply(&drive, vec![set_count(member(1), 1, contract_id)], true);
    assert_eq!(estimated.storage_fee, applied.storage_fee);
    assert!(estimated.processing_fee >= applied.processing_fee);
    // A later one replaces it with the same size: no storage.
    let replaced = apply(&drive, vec![set_count(member(1), 2, contract_id)], true);
    assert_eq!(replaced.storage_fee, 0);
    apply(&drive, vec![set_count(member(2), 5, contract_id)], true);

    let epoch = Epoch::new(0).expect("epoch");
    let (fee, counts) = drive
        .fetch_contract_moderation_action_counts_with_fee(
            contract_id,
            31,
            &epoch,
            None,
            platform_version,
        )
        .expect("expected to read the counts");
    assert!(fee.processing_fee > 0);
    assert_eq!(counts, BTreeMap::from([(member(1), 2), (member(2), 5)]));
    let (_, count) = drive
        .fetch_contract_moderation_action_count_with_fee(
            contract_id,
            member(2),
            &epoch,
            None,
            platform_version,
        )
        .expect("expected to read a count");
    assert_eq!(count, Some(5));
    let (_, count) = drive
        .fetch_contract_moderation_action_count_with_fee(
            contract_id,
            member(3),
            &epoch,
            None,
            platform_version,
        )
        .expect("expected to read a count");
    assert_eq!(count, Some(0), "a member that did not act has no count");

    // The limit bounds the read.
    assert_eq!(
        drive
            .fetch_contract_moderation_action_counts(contract_id, 1, None, platform_version)
            .expect("expected to read the counts")
            .len(),
        1
    );

    // A settle deletes them, refunding nobody: they carry no storage flags.
    let reset = DriveOperation::ContractModerationOperation(
        ContractModerationOperationType::RemoveActionCounts {
            contract_id,
            identity_ids: vec![member(1), member(2)],
        },
    );
    let estimated = apply(&drive, vec![reset.clone()], false);
    let applied = apply(&drive, vec![reset], true);
    assert!(estimated.processing_fee >= applied.processing_fee);
    assert!(applied.fee_refunds.0.is_empty());
    assert!(drive
        .fetch_contract_moderation_action_counts(contract_id, 31, None, platform_version)
        .expect("expected to read the counts")
        .is_empty());
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

/// The counts sort below the version item, so that created with two or three lists they leave
/// the banlist on top; with the banlist alone the version item is.
#[test]
fn should_keep_the_banlist_on_top_of_an_elected_contracts_other_tree_with_two_or_more_lists() {
    let platform_version = PlatformVersion::latest();
    // (banlist, suspensions, warnings) -> the key on top of the contract's other tree
    for (banlist, suspensions, warnings, top_of_other) in [
        (true, false, false, CONTRACT_VERSION_KEY),
        (true, true, false, CONTRACT_BANLIST_KEY),
        (true, false, true, CONTRACT_BANLIST_KEY),
        // Without the counts, the one combination where the banlist sat a level down.
        (true, true, true, CONTRACT_BANLIST_KEY),
    ] {
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let contract = contract_keeping(banlist, suspensions, warnings, true);
        drive
            .insert_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to insert the contract");
        let contracts_root: &[u8] = Into::<&[u8; 1]>::into(RootTree::DataContractDocuments);
        assert_eq!(
            merk_root_key(
                &drive,
                &[contracts_root, contract.id().as_slice()],
                &[CONTRACT_OTHER_KEY]
            ),
            Some(vec![top_of_other]),
            "banlist {banlist}, suspensions {suspensions}, warnings {warnings}"
        );
    }
}
