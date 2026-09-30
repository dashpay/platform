//! The approvals a seated moderation team gives the deletion of settled documents: their trees,
//! their writer, and the reads and proofs over them.

use crate::drive::contract::moderation::types::{
    ContractDocumentRemovalsSelection, ContractSettledDeletionEntry, ContractSettledDeletionsQuery,
};
use crate::drive::contract::paths::{
    contract_other_path, contract_settled_deletions_path, CONTRACT_SETTLED_DELETIONS_KEY,
};
use crate::drive::Drive;
use crate::util::batch::DriveOperation::ContractModerationOperation;
use crate::util::batch::{ContractModerationOperationType, DriveOperation};
use crate::util::grove_operations::DirectQueryType;
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::Epoch;
use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
use dpp::data_contract::config::moderation::{
    ContractModerationConfig, ContractModerationReason, ContractModerators,
    ContractSettledDeletion, ElectedModerators, InterimModerators, ModerationAbility,
    DEFAULT_ELECTION_WINDOW_SECONDS,
};
use dpp::data_contract::schema::DataContractSchemaMethodsV0;
use dpp::data_contract::DataContract;
use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use dpp::fee::fee_result::FeeResult;
use dpp::identifier::Identifier;
use dpp::platform_value::{platform_value, Value};
use dpp::tests::fixtures::get_data_contract_fixture;
use dpp::version::fee::FeeVersion;
use dpp::version::PlatformVersion;
use once_cell::sync::Lazy;
use std::collections::{BTreeMap, BTreeSet};

static FEE_VERSIONS: Lazy<CachedEpochIndexFeeVersions> =
    Lazy::new(|| BTreeMap::from([(0, FeeVersion::first())]));

const POST: &str = "post";

fn identity(seed: u8) -> Identifier {
    Identifier::from([seed; 32])
}

/// A post its moderators delete within a day, and once settled by `rule` when it is given.
fn post_schema(rule: Option<Value>) -> Value {
    let mut abilities = platform_value!({ "delete": true, "deleteWithin": 86400 });
    if let Some(rule) = rule {
        abilities
            .insert("deleteSettled".to_string(), rule)
            .expect("expected to set the rule");
    }
    platform_value!({
        "type": "object",
        "properties": {
            "text": { "type": "string", "maxLength": 50, "position": 0 },
        },
        "required": ["text", "$updatedAt"],
        "additionalProperties": false,
        "moderatorAbilities": abilities,
    })
}

/// The fixture contract with an elected moderation declaration moderating `types` with
/// deletions, each of `types` a post whose settled documents the team deletes by `rule`.
fn elected_contract_with(types: &[&str], rule: Option<Value>) -> DataContract {
    let platform_version = PlatformVersion::latest();
    let mut contract =
        get_data_contract_fixture(None, 0, platform_version.protocol_version).data_contract_owned();
    let moderation = ContractModerationConfig {
        banlist: false,
        suspensions: false,
        moderators: ContractModerators::Elected(Box::new(ElectedModerators {
            join_window: DEFAULT_ELECTION_WINDOW_SECONDS,
            vote_window: DEFAULT_ELECTION_WINDOW_SECONDS,
            challenge_cool_down: None,
            election_delay: None,
            max_added_moderators: 0,
            moderated_document_types: types
                .iter()
                .map(|name| {
                    (
                        name.to_string(),
                        BTreeSet::from([ModerationAbility::DeleteDocuments]),
                    )
                })
                .collect(),
            interim: InterimModerators::ContractOwner,
            owner_protected: false,
        })),
        warnings: false,
    };
    contract.set_config(contract.config().clone().with_moderation(Some(moderation)));
    for name in types {
        add_post_type(&mut contract, name, rule.clone());
    }
    contract
}

fn add_post_type(contract: &mut DataContract, name: &str, rule: Option<Value>) {
    contract
        .set_document_schema(
            name,
            post_schema(rule),
            true,
            &mut vec![],
            PlatformVersion::latest(),
        )
        .expect("expected to add the post type");
}

fn insert(drive: &Drive, contract: &DataContract) {
    drive
        .insert_contract(
            contract,
            BlockInfo::default(),
            true,
            None,
            PlatformVersion::latest(),
        )
        .expect("expected to insert the contract");
}

fn has_settled_deletions_root(drive: &Drive, contract_id: Identifier) -> bool {
    drive
        .grove_has_raw(
            (&contract_other_path(contract_id.as_slice())).into(),
            &[CONTRACT_SETTLED_DELETIONS_KEY],
            DirectQueryType::StatefulDirectQuery,
            None,
            &mut vec![],
            &PlatformVersion::latest().drive,
        )
        .expect("expected to query the contract's other tree")
}

fn has_settled_deletions_tree_of(
    drive: &Drive,
    contract_id: Identifier,
    document_type_name: &str,
) -> bool {
    drive
        .grove_has_raw(
            (&contract_settled_deletions_path(contract_id.as_slice())).into(),
            document_type_name.as_bytes(),
            DirectQueryType::StatefulDirectQuery,
            None,
            &mut vec![],
            &PlatformVersion::latest().drive,
        )
        .expect("expected to query the contract's approvals tree")
}

fn approvals(text: &str, approvers: &[u8], deleted_at: Option<u64>) -> ContractSettledDeletion {
    ContractSettledDeletion {
        proposed_at: 1_000,
        document_last_modified_at: 10,
        reason: ContractModerationReason {
            code: Some(7),
            text: text.to_string(),
            documents: vec![],
            reason_document_id: None,
        },
        approvals: approvers.iter().copied().map(identity).collect(),
        deleted_at,
    }
}

fn write<'a>(
    contract_id: Identifier,
    document_id: Identifier,
    settled_deletion: ContractSettledDeletion,
    replaces_existing: bool,
) -> DriveOperation<'a> {
    let moderator_id = *settled_deletion
        .approvals
        .last()
        .expect("expected an approval");
    ContractModerationOperation(ContractModerationOperationType::AddSettledDeletion {
        contract_id,
        document_type_name: POST.to_string(),
        document_id,
        settled_deletion,
        replaces_existing,
        moderator_id,
    })
}

fn apply(drive: &Drive, operations: Vec<DriveOperation>, apply: bool) -> FeeResult {
    drive
        .apply_drive_operations(
            operations,
            apply,
            &BlockInfo::default_with_epoch(Epoch::new(3).expect("epoch")),
            None,
            PlatformVersion::latest(),
            Some(&FEE_VERSIONS),
        )
        .expect("expected to apply the operations")
}

fn by_ids(ids: &[Identifier]) -> ContractSettledDeletionsQuery {
    ContractSettledDeletionsQuery {
        document_type_name: POST.to_string(),
        selection: ContractDocumentRemovalsSelection::DocumentIds(ids.to_vec()),
    }
}

fn page(start_after: Option<Identifier>, limit: u16) -> ContractSettledDeletionsQuery {
    ContractSettledDeletionsQuery {
        document_type_name: POST.to_string(),
        selection: ContractDocumentRemovalsSelection::Page { start_after, limit },
    }
}

/// Fetches, proves and verifies one read and checks all three agree.
fn assert_settled_deletions(
    drive: &Drive,
    contract_id: Identifier,
    query: &ContractSettledDeletionsQuery,
    expected: Vec<ContractSettledDeletionEntry>,
) {
    let platform_version = PlatformVersion::latest();
    let fetched = drive
        .fetch_contract_settled_deletions(contract_id, query, None, platform_version)
        .expect("expected to fetch the approvals");
    assert_eq!(fetched, expected, "fetched");

    let proof = drive
        .prove_contract_settled_deletions(contract_id, query, None, platform_version)
        .expect("expected an approvals proof");
    let (proved_root, proved) = Drive::verify_contract_settled_deletions(
        &proof,
        contract_id,
        query,
        false,
        platform_version,
    )
    .expect("expected the proof to verify");
    assert_eq!(proved, expected, "proved");
    let root = drive
        .grove
        .root_hash(None, &platform_version.drive.grove_version)
        .unwrap()
        .expect("expected a root hash");
    assert_eq!(proved_root, root);
}

#[test]
fn should_create_the_approval_trees_with_the_contract() {
    let platform_version = PlatformVersion::latest();

    // A type that says who approves a settled deletion: the tree and the type's.
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = elected_contract_with(&[POST], Some(platform_value!({ "leader": true })));
    insert(&drive, &contract);
    assert!(has_settled_deletions_root(&drive, contract.id()));
    assert!(has_settled_deletions_tree_of(&drive, contract.id(), POST));
    assert!(!has_settled_deletions_tree_of(
        &drive,
        contract.id(),
        "niceDocument"
    ));

    // Deletions within the window only: nothing, so the other tree keeps the shape it had.
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = elected_contract_with(&[POST], None);
    insert(&drive, &contract);
    assert!(!has_settled_deletions_root(&drive, contract.id()));
}

#[test]
fn should_create_the_approval_tree_of_a_document_type_an_update_adds() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = elected_contract_with(&[POST], None);
    insert(&drive, &contract);
    assert!(!has_settled_deletions_root(&drive, contract.id()));

    // The first type that says so brings the tree above it, the next one only its own.
    let mut updated = contract.clone();
    updated.increment_version();
    add_post_type(
        &mut updated,
        "reply",
        Some(platform_value!({ "approvals": 2 })),
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
    assert!(has_settled_deletions_root(&drive, contract.id()));
    assert!(has_settled_deletions_tree_of(
        &drive,
        contract.id(),
        "reply"
    ));
    assert!(!has_settled_deletions_tree_of(&drive, contract.id(), POST));

    let mut updated_again = updated.clone();
    updated_again.increment_version();
    add_post_type(
        &mut updated_again,
        "comment",
        Some(platform_value!({ "leader": true })),
    );
    drive
        .update_contract(
            &updated_again,
            BlockInfo::default(),
            true,
            None,
            platform_version,
            None,
        )
        .expect("expected to update the contract again");
    assert!(has_settled_deletions_tree_of(
        &drive,
        contract.id(),
        "comment"
    ));
    assert!(has_settled_deletions_tree_of(
        &drive,
        contract.id(),
        "reply"
    ));
}

#[test]
fn should_record_replace_read_and_prove_approvals() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = elected_contract_with(&[POST], Some(platform_value!({ "approvals": 3 })));
    insert(&drive, &contract);
    let contract_id = contract.id();

    // Nothing yet: every id absent, the page empty.
    assert_settled_deletions(&drive, contract_id, &by_ids(&[identity(0x31)]), vec![]);
    assert_settled_deletions(&drive, contract_id, &page(None, 10), vec![]);

    let first = ContractSettledDeletionEntry {
        document_id: identity(0x31),
        settled_deletion: approvals("doxxing", &[1], None),
    };
    let second = ContractSettledDeletionEntry {
        document_id: identity(0x32),
        settled_deletion: approvals("spam", &[2], None),
    };
    for entry in [&first, &second] {
        apply(
            &drive,
            vec![write(
                contract_id,
                entry.document_id,
                entry.settled_deletion.clone(),
                false,
            )],
            true,
        );
    }
    assert_settled_deletions(
        &drive,
        contract_id,
        &by_ids(&[identity(0x32), identity(0x99), identity(0x31)]),
        vec![first.clone(), second.clone()],
    );

    // Later approvals rewrite the record in place, the last one marking the deletion.
    let approved = ContractSettledDeletionEntry {
        document_id: first.document_id,
        settled_deletion: approvals("doxxing", &[1, 3], None),
    };
    apply(
        &drive,
        vec![write(
            contract_id,
            approved.document_id,
            approved.settled_deletion.clone(),
            true,
        )],
        true,
    );
    let deleted = ContractSettledDeletionEntry {
        document_id: first.document_id,
        settled_deletion: approvals("doxxing", &[1, 3, 4], Some(2_000)),
    };
    apply(
        &drive,
        vec![write(
            contract_id,
            deleted.document_id,
            deleted.settled_deletion.clone(),
            true,
        )],
        true,
    );
    assert_settled_deletions(
        &drive,
        contract_id,
        &page(None, 10),
        vec![deleted.clone(), second.clone()],
    );
    assert_settled_deletions(
        &drive,
        contract_id,
        &page(Some(identity(0x31)), 10),
        vec![second],
    );

    // A contract nobody has, and a type that says nothing of settled deletions, read as none.
    for (contract_id, query) in [
        (identity(0x77), page(None, 2)),
        (
            contract_id,
            ContractSettledDeletionsQuery {
                document_type_name: "niceDocument".to_string(),
                selection: ContractDocumentRemovalsSelection::Page {
                    start_after: None,
                    limit: 2,
                },
            },
        ),
    ] {
        assert_eq!(
            drive
                .fetch_contract_settled_deletions(contract_id, &query, None, platform_version)
                .expect("expected to read as empty"),
            vec![]
        );
    }

    // One record, as consensus reads it, billed.
    let (fee, read) = drive
        .fetch_contract_settled_deletion_with_fee(
            contract_id,
            POST,
            deleted.document_id,
            &Epoch::new(0).expect("epoch"),
            None,
            platform_version,
        )
        .expect("expected to read one record");
    assert_eq!(read, Some(deleted.settled_deletion));
    assert!(fee.processing_fee > 0);
}

#[test]
fn should_refuse_a_read_of_approvals_out_of_bounds() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = elected_contract_with(&[POST], Some(platform_value!({ "leader": true })));
    insert(&drive, &contract);
    let max = platform_version.drive_abci.query.max_returned_elements;
    let too_many: Vec<Identifier> = (0..=max)
        .map(|n| {
            let mut id = [0u8; 32];
            id[..2].copy_from_slice(&n.to_be_bytes());
            Identifier::from(id)
        })
        .collect();
    for query in [
        by_ids(&[]),
        by_ids(&too_many),
        by_ids(&[identity(1), identity(1)]),
        page(None, 0),
        page(None, max + 1),
    ] {
        drive
            .fetch_contract_settled_deletions(contract.id(), &query, None, platform_version)
            .expect_err("expected the read to be refused");
        drive
            .prove_contract_settled_deletions(contract.id(), &query, None, platform_version)
            .expect_err("expected the proof to be refused");
    }
}

#[test]
fn should_estimate_approvals_at_no_less_than_they_cost() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = elected_contract_with(&[POST], Some(platform_value!({ "approvals": 31 })));
    insert(&drive, &contract);
    let longest = "x".repeat(
        platform_version
            .system_limits
            .max_contract_moderation_reason_length as usize,
    );
    let many: Vec<u8> = (1..=31).collect();

    for (seed, text, approvers) in [
        (1u8, "", &[1u8][..]),
        (2, "spam", &[1, 2, 3][..]),
        (3, longest.as_str(), many.as_slice()),
    ] {
        let operations = || {
            vec![write(
                contract.id(),
                identity(0x40 + seed),
                approvals(text, approvers, Some(5)),
                false,
            )]
        };
        let estimated = apply(&drive, operations(), false);
        let applied = apply(&drive, operations(), true);
        assert!(
            applied.storage_fee > 0,
            "the moderator pays for the approvals"
        );
        // The estimate is for a typical record; one far larger is priced by its own size.
        if text.len() <= 128 && approvers.len() <= 3 {
            assert!(
                estimated.storage_fee >= applied.storage_fee,
                "{} approvals, a {}-byte reason: estimated {} < applied {}",
                approvers.len(),
                text.len(),
                estimated.storage_fee,
                applied.storage_fee
            );
        }
    }
}
