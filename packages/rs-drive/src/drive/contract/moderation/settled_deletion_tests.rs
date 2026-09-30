//! The approvals a seated moderation team gives the deletion of settled documents: their trees,
//! their writer, the reads and proofs over them, and the refunds of the batch that deletes.

use crate::drive::contract::moderation::types::{
    ContractDocumentRemovalsSelection, ContractSettledDeletionEntry, ContractSettledDeletionsQuery,
};
use crate::drive::contract::paths::{
    contract_other_path, contract_settled_deletions_path, CONTRACT_SETTLED_DELETIONS_KEY,
};
use crate::drive::Drive;
use crate::util::batch::DriveOperation::{ContractModerationOperation, DocumentOperation};
use crate::util::batch::{ContractModerationOperationType, DocumentOperationType, DriveOperation};
use crate::util::grove_operations::DirectQueryType;
use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
use crate::util::object_size_info::{
    DataContractInfo, DocumentAndContractInfo, DocumentTypeInfo, OwnedDocumentInfo,
};
use crate::util::storage_flags::StorageFlags;
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::Epoch;
use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
use dpp::data_contract::config::moderation::{
    ContractModerationConfig, ContractModerationReason, ContractModerators,
    ContractSettledDeletion, ElectedModerators, InterimModerators, ModerationAbility,
    DEFAULT_ELECTION_WINDOW_SECONDS,
};
use dpp::data_contract::document_type::random_document::CreateRandomDocument;
use dpp::data_contract::schema::DataContractSchemaMethodsV0;
use dpp::data_contract::DataContract;
use dpp::document::{Document, DocumentV0Getters, DocumentV0Setters};
use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use dpp::fee::fee_result::FeeResult;
use dpp::identifier::Identifier;
use dpp::platform_value::{platform_value, Value};
use dpp::tests::fixtures::get_data_contract_fixture;
use dpp::version::fee::FeeVersion;
use dpp::version::PlatformVersion;
use once_cell::sync::Lazy;
use std::borrow::Cow;
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
        document_revision: Some(1),
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

/// Writes `record` for `document_id` as an approval does, estimated then applied, and checks the
/// estimate is no less than the cost, storage and total alike
fn assert_estimated_at_no_less_than_it_costs(
    drive: &Drive,
    contract: &DataContract,
    document_id: Identifier,
    record: ContractSettledDeletion,
    replaces_existing: bool,
) {
    let operations = || {
        vec![write(
            contract.id(),
            document_id,
            record.clone(),
            replaces_existing,
        )]
    };
    let estimated = apply(drive, operations(), false);
    let applied = apply(drive, operations(), true);
    let what = format!(
        "{} approvals, a {}-byte reason, {}, {}",
        record.approvals.len(),
        record.reason.text.len(),
        if record.deleted_at.is_some() {
            "deleted"
        } else {
            "open"
        },
        if replaces_existing {
            "replacing"
        } else {
            "fresh"
        },
    );
    assert!(
        applied.storage_fee > 0 || replaces_existing,
        "{what}: the moderator pays for a fresh record"
    );
    assert!(
        estimated.storage_fee >= applied.storage_fee,
        "{what}: estimated storage {} < applied {}",
        estimated.storage_fee,
        applied.storage_fee
    );
    assert!(
        estimated.total_base_fee() >= applied.total_base_fee(),
        "{what}: estimated total {} < applied {}",
        estimated.total_base_fee(),
        applied.total_base_fee()
    );
}

#[test]
fn should_estimate_approvals_at_no_less_than_they_cost() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = elected_contract_with(&[POST], Some(platform_value!({ "leader": true })));
    insert(&drive, &contract);
    let longest = "x".repeat(
        platform_version
            .system_limits
            .max_contract_moderation_reason_length as usize,
    );
    let many: Vec<u8> = (1..=31).collect();

    // Every write the approvals of one document make, for a reason empty, typical and as long
    // as allowed: the first approval, each later one growing the record up to as many members
    // as a team holds, and the one that deletes adding its time; and on another document, a
    // fresh, shorter record in place of approvals that lapsed, then the one that deletes.
    for (seed, text) in [(1u8, ""), (2, "spam"), (3, longest.as_str())] {
        let growing = identity(0x40 + seed);
        for count in 1..=many.len() {
            assert_estimated_at_no_less_than_it_costs(
                &drive,
                &contract,
                growing,
                approvals(text, &many[..count], None),
                count > 1,
            );
        }
        assert_estimated_at_no_less_than_it_costs(
            &drive,
            &contract,
            growing,
            approvals(text, &many, Some(5)),
            true,
        );

        let lapsing = identity(0x50 + seed);
        assert_estimated_at_no_less_than_it_costs(
            &drive,
            &contract,
            lapsing,
            approvals(text, &many[..3], None),
            false,
        );
        assert_estimated_at_no_less_than_it_costs(
            &drive,
            &contract,
            lapsing,
            approvals("x", &many[3..4], None),
            true,
        );
        assert_estimated_at_no_less_than_it_costs(
            &drive,
            &contract,
            lapsing,
            approvals("x", &many[3..5], Some(5)),
            true,
        );
    }
}

/// A post by `owner_id`, stored with its owner's flags, as a create leaves it
fn add_post(drive: &Drive, contract: &DataContract, owner_id: Identifier) -> Document {
    let platform_version = PlatformVersion::latest();
    let document_type = contract
        .document_type_for_name(POST)
        .expect("expected the post document type");
    let mut document = document_type
        .random_document(Some(5), platform_version)
        .expect("expected a random post");
    document.set_owner_id(owner_id);
    let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpochOwned(
        0,
        owner_id.to_buffer(),
    )));
    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((&document, storage_flags)),
                    owner_id: None,
                },
                contract,
                document_type,
            },
            false,
            BlockInfo::default(),
            true,
            None,
            platform_version,
            None,
        )
        .expect("expected to add the post");
    document
}

/// The batch of the leader's approval that starts afresh with `text` and deletes `post_id`:
/// the post, the approvals record it rewrites, and the forfeiture of a moderator's deletion
fn delete_by_approval(
    drive: &Drive,
    contract: &DataContract,
    post_id: Identifier,
    text: &str,
) -> FeeResult {
    apply(
        drive,
        vec![
            DocumentOperation(DocumentOperationType::ForceDeleteDocument {
                document_id: post_id,
                contract_info: DataContractInfo::BorrowedDataContract(contract),
                document_type_info: DocumentTypeInfo::DocumentTypeName(POST.to_string()),
            }),
            write(
                contract.id(),
                post_id,
                approvals(text, &[9], Some(2_000)),
                true,
            ),
            ContractModerationOperation(ContractModerationOperationType::ForfeitStorageRefunds),
        ],
        true,
    )
}

/// The refunds of the leader's approval that deletes a post by `author` over lapsed approvals
/// `[1, 2, 3]`, paid for last by the third approver
fn refunds_of_a_deciding_approval(author: Identifier) -> FeeResult {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = elected_contract_with(&[POST], Some(platform_value!({ "leader": true })));
    insert(&drive, &contract);
    let post = add_post(&drive, &contract, author);
    apply(
        &drive,
        vec![write(
            contract.id(),
            post.id(),
            approvals("doxxing", &[1, 2, 3], None),
            false,
        )],
        true,
    );
    delete_by_approval(&drive, &contract, post.id(), "doxxing")
}

#[test]
fn should_refund_an_approver_the_bytes_the_deciding_approval_frees() {
    // The leader's record is shorter than the one it replaces. The author forfeits the post's
    // refund; the third approver keeps the refund of the record bytes the replacement frees.
    let applied = refunds_of_a_deciding_approval(identity(0x41));
    assert_eq!(
        applied.fee_refunds.0.keys().copied().collect::<Vec<_>>(),
        vec![identity(3).to_buffer()]
    );
    assert!(applied.removed_bytes_from_system > 0);
}

#[test]
fn should_refund_an_approver_who_wrote_the_post_only_the_record_bytes() {
    // The approver who paid for the record also paid for the post: the post's refund is
    // forfeited all the same, and the approver gets back the record bytes alone, as much as an
    // approver who did not write it.
    let own_post = refunds_of_a_deciding_approval(identity(3));
    let other_post = refunds_of_a_deciding_approval(identity(0x41));
    assert_eq!(own_post.fee_refunds, other_post.fee_refunds);
    assert!(own_post.removed_bytes_from_system > 0);
}

#[test]
fn should_refund_whoever_the_record_flags_name_though_no_longer_an_approver() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = elected_contract_with(&[POST], Some(platform_value!({ "leader": true })));
    insert(&drive, &contract);
    let post = add_post(&drive, &contract, identity(0x41));

    // The first approver's record lapses, and a fresh one of the same size replaces it: GroveDB
    // keeps the first approver's flags, though the approvals now name another member.
    let reason = |letter: &str| letter.repeat(60);
    apply(
        &drive,
        vec![write(
            contract.id(),
            post.id(),
            approvals(&reason("a"), &[1], None),
            false,
        )],
        true,
    );
    apply(
        &drive,
        vec![write(
            contract.id(),
            post.id(),
            approvals(&reason("b"), &[3], None),
            true,
        )],
        true,
    );

    // The leader's approval, for a shorter reason, deletes the post: the bytes it frees go back
    // to the member the record's flags name.
    let applied = delete_by_approval(&drive, &contract, post.id(), "spam");
    assert_eq!(
        applied.fee_refunds.0.keys().copied().collect::<Vec<_>>(),
        vec![identity(1).to_buffer()]
    );
}
