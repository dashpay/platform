//! The records of the documents a contract's moderators deleted: their trees, their writer,
//! the reads and proofs over them, and the storage refund a moderated deletion forfeits.

use crate::drive::contract::moderation::types::{
    ContractDocumentRemovalEntry, ContractDocumentRemovalsQuery, ContractDocumentRemovalsSelection,
};
use crate::drive::contract::paths::{
    contract_document_removals_path, contract_other_path, CONTRACT_BANLIST_KEY,
    CONTRACT_DOCUMENT_REMOVALS_KEY, CONTRACT_OTHER_KEY, CONTRACT_VERSION_KEY,
};
use crate::drive::document::paths::contract_documents_primary_key_path;
use crate::drive::{Drive, RootTree};
use crate::util::batch::DriveOperation::{ContractModerationOperation, DocumentOperation};
use crate::util::batch::{
    ContractFeePotOperationType, ContractModerationOperationType, DocumentOperationType,
    DriveOperation,
};
use crate::util::grove_operations::DirectQueryType;
use crate::util::object_size_info::DocumentInfo::{DocumentOwnedInfo, DocumentRefInfo};
use crate::util::object_size_info::{
    DataContractInfo, DocumentAndContractInfo, DocumentTypeInfo, OwnedDocumentInfo,
};
use crate::util::storage_flags::StorageFlags;
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::Epoch;
use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
use dpp::data_contract::config::moderation::{
    ContractDocumentRemoval, ContractDocumentRestoration, ContractModerationConfig,
    ContractModerationReason, ContractModerators,
};
use dpp::data_contract::document_type::action_fees::{ContractFeePot, ContractFeePotLastClaim};
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
use grovedb::Element;
use once_cell::sync::Lazy;
use std::borrow::Cow;
use std::collections::BTreeMap;

static FEE_VERSIONS: Lazy<CachedEpochIndexFeeVersions> =
    Lazy::new(|| BTreeMap::from([(0, FeeVersion::first())]));

const POST: &str = "post";

fn identity(seed: u8) -> Identifier {
    Identifier::from([seed; 32])
}

fn post_schema(deletable_by_moderators: bool) -> Value {
    platform_value!({
        "type": "object",
        "properties": {
            "text": { "type": "string", "maxLength": 50, "position": 0 },
        },
        "required": ["text"],
        "additionalProperties": false,
        "canBeDeletedByModerators": deletable_by_moderators,
    })
}

/// The fixture contract, declaring moderation when `moderated`, and with each of
/// `deletable_types` as a document type moderators can delete documents of.
fn contract_with(moderated: bool, banlist: bool, deletable_types: &[&str]) -> DataContract {
    let platform_version = PlatformVersion::latest();
    let mut contract =
        get_data_contract_fixture(None, 0, platform_version.protocol_version).data_contract_owned();
    let moderation = moderated.then_some(ContractModerationConfig {
        banlist,
        suspensions: false,
        moderators: ContractModerators::ContractOwner,
        warnings: false,
    });
    contract.set_config(contract.config().clone().with_moderation(moderation));
    for name in deletable_types {
        add_deletable_type(&mut contract, name);
    }
    contract
}

fn add_deletable_type(contract: &mut DataContract, name: &str) {
    contract
        .set_document_schema(
            name,
            post_schema(true),
            true,
            &mut vec![],
            PlatformVersion::latest(),
        )
        .expect("expected to add a document type moderators can delete from");
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

fn has_removals_root(drive: &Drive, contract_id: Identifier) -> bool {
    drive
        .grove_has_raw(
            (&contract_other_path(contract_id.as_slice())).into(),
            &[CONTRACT_DOCUMENT_REMOVALS_KEY],
            DirectQueryType::StatefulDirectQuery,
            None,
            &mut vec![],
            &PlatformVersion::latest().drive,
        )
        .expect("expected to query the contract's other tree")
}

fn has_removals_tree_of(drive: &Drive, contract_id: Identifier, document_type_name: &str) -> bool {
    drive
        .grove_has_raw(
            (&contract_document_removals_path(contract_id.as_slice())).into(),
            document_type_name.as_bytes(),
            DirectQueryType::StatefulDirectQuery,
            None,
            &mut vec![],
            &PlatformVersion::latest().drive,
        )
        .expect("expected to query the contract's removals tree")
}

fn removal(owner: u8, moderator: u8, text: &str, removed_at: u64) -> ContractDocumentRemoval {
    ContractDocumentRemoval {
        document_owner_id: identity(owner),
        moderator_id: identity(moderator),
        reason: ContractModerationReason {
            code: Some(7),
            text: text.to_string(),
            documents: vec![],
            reason_document_id: None,
        },
        removed_at,
        document_hash: [removed_at as u8; 32],
        restoration: None,
    }
}

fn record<'a>(
    contract_id: Identifier,
    document_id: Identifier,
    removal: ContractDocumentRemoval,
) -> DriveOperation<'a> {
    let moderator_id = removal.moderator_id;
    ContractModerationOperation(ContractModerationOperationType::AddDocumentRemoval {
        contract_id,
        document_type_name: POST.to_string(),
        document_id,
        removal,
        replaces_existing: false,
        moderator_id,
    })
}

/// The replacement of `document_id`'s record by `removal`, paid for by `moderator_id`
fn replace_record<'a>(
    contract_id: Identifier,
    document_id: Identifier,
    removal: ContractDocumentRemoval,
    moderator_id: Identifier,
) -> DriveOperation<'a> {
    ContractModerationOperation(ContractModerationOperationType::AddDocumentRemoval {
        contract_id,
        document_type_name: POST.to_string(),
        document_id,
        removal,
        replaces_existing: true,
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

fn by_ids(ids: &[Identifier]) -> ContractDocumentRemovalsQuery {
    ContractDocumentRemovalsQuery {
        document_type_name: POST.to_string(),
        selection: ContractDocumentRemovalsSelection::DocumentIds(ids.to_vec()),
    }
}

fn page(start_after: Option<Identifier>, limit: u16) -> ContractDocumentRemovalsQuery {
    ContractDocumentRemovalsQuery {
        document_type_name: POST.to_string(),
        selection: ContractDocumentRemovalsSelection::Page { start_after, limit },
    }
}

/// Fetches, proves and verifies one read and checks all three agree.
fn assert_removals(
    drive: &Drive,
    contract_id: Identifier,
    query: &ContractDocumentRemovalsQuery,
    expected: Vec<ContractDocumentRemovalEntry>,
) {
    let platform_version = PlatformVersion::latest();
    let fetched = drive
        .fetch_contract_document_removals(contract_id, query, None, platform_version)
        .expect("expected to fetch the removals");
    assert_eq!(fetched, expected, "fetched");

    let proof = drive
        .prove_contract_document_removals(contract_id, query, None, platform_version)
        .expect("expected a removals proof");
    let (proved_root, proved) = Drive::verify_contract_document_removals(
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
fn should_create_the_removal_trees_with_the_contract() {
    let platform_version = PlatformVersion::latest();

    // Moderated, with a document type moderators can delete from: the tree and the type's.
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = contract_with(true, false, &[POST]);
    insert(&drive, &contract);
    assert!(has_removals_root(&drive, contract.id()));
    assert!(has_removals_tree_of(&drive, contract.id(), POST));
    // A document type that does not set the keyword gets none.
    assert!(!has_removals_tree_of(&drive, contract.id(), "niceDocument"));

    // Moderated with lists only: nothing, so its other tree keeps the shape it had. The
    // update that adds the first such document type brings the tree.
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = contract_with(true, true, &[]);
    insert(&drive, &contract);
    assert!(!has_removals_root(&drive, contract.id()));

    // Not moderated: nothing.
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = contract_with(false, false, &[]);
    insert(&drive, &contract);
    assert!(!has_removals_root(&drive, contract.id()));
}

#[test]
fn should_create_the_removal_tree_of_a_document_type_an_update_adds() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let mut contract = contract_with(true, true, &[POST]);
    insert(&drive, &contract);
    let document_id = identity(0x31);
    apply(
        &drive,
        vec![record(
            contract.id(),
            document_id,
            removal(1, 2, "spam", 10),
        )],
        true,
    );

    add_deletable_type(&mut contract, "reply");
    contract.increment_version();
    // The estimate creates the same tree, and costs at least what the update then pays.
    let estimated = drive
        .update_contract(
            &contract,
            BlockInfo::default(),
            false,
            None,
            platform_version,
            Some(&FEE_VERSIONS),
        )
        .expect("expected to estimate the update");
    let applied = drive
        .update_contract(
            &contract,
            BlockInfo::default(),
            true,
            None,
            platform_version,
            Some(&FEE_VERSIONS),
        )
        .expect("expected to update the contract");
    assert!(estimated.storage_fee >= applied.storage_fee);

    assert!(has_removals_tree_of(&drive, contract.id(), "reply"));
    // The type the contract already had keeps its tree, and what is in it.
    assert_removals(
        &drive,
        contract.id(),
        &by_ids(&[document_id]),
        vec![ContractDocumentRemovalEntry {
            document_id,
            removal: removal(1, 2, "spam", 10),
        }],
    );
}

#[test]
fn should_create_the_removals_tree_with_the_first_document_type_an_update_adds() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    // Lists only: no document type moderators can delete from, so no removals tree.
    let mut contract = contract_with(true, true, &[]);
    insert(&drive, &contract);
    assert!(!has_removals_root(&drive, contract.id()));

    add_deletable_type(&mut contract, POST);
    contract.increment_version();
    drive
        .update_contract(
            &contract,
            BlockInfo::default(),
            true,
            None,
            platform_version,
            Some(&FEE_VERSIONS),
        )
        .expect("expected to update the contract");
    assert!(has_removals_root(&drive, contract.id()));
    assert!(has_removals_tree_of(&drive, contract.id(), POST));

    // And it takes records.
    let document_id = identity(0x31);
    apply(
        &drive,
        vec![record(
            contract.id(),
            document_id,
            removal(1, 2, "spam", 10),
        )],
        true,
    );
    assert_removals(
        &drive,
        contract.id(),
        &by_ids(&[document_id]),
        vec![ContractDocumentRemovalEntry {
            document_id,
            removal: removal(1, 2, "spam", 10),
        }],
    );
}

#[test]
fn should_keep_the_banlist_on_top_of_the_other_tree_when_every_tree_is_created_at_once() {
    // A Merk built from one sorted batch roots at its middle key. The removals tree sorts
    // below the version item, so that a contract keeping both lists, the one whose other tree
    // holds four keys, still has the banlist, which every document transition reads, on top.
    // With fewer keys the version item is on top: nothing reads the removals tree on the way
    // to a list but a client.
    let platform_version = PlatformVersion::latest();
    for (banlist, suspensions, top_of_other) in [
        (false, false, CONTRACT_VERSION_KEY),
        (true, false, CONTRACT_VERSION_KEY),
        (false, true, CONTRACT_VERSION_KEY),
        (true, true, CONTRACT_BANLIST_KEY),
    ] {
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let mut contract = get_data_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        contract.set_config(contract.config().clone().with_moderation(Some(
            ContractModerationConfig {
                banlist,
                suspensions,
                moderators: ContractModerators::ContractOwner,
                warnings: false,
            },
        )));
        add_deletable_type(&mut contract, POST);
        insert(&drive, &contract);

        let contracts_root: &[u8] = Into::<&[u8; 1]>::into(RootTree::DataContractDocuments);
        let other = drive
            .grove
            .get_raw(
                (&[contracts_root, contract.id_ref().as_slice()]).into(),
                &[CONTRACT_OTHER_KEY],
                None,
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .expect("expected the other tree");
        let Element::Tree(root_key, _) = other else {
            panic!("expected a tree, got {other:?}");
        };
        assert_eq!(
            root_key,
            Some(vec![top_of_other]),
            "banlist {banlist}, suspensions {suspensions}"
        );
    }
}

#[test]
fn should_lose_the_top_of_the_other_tree_to_the_version_item_once_both_fee_pots_were_claimed() {
    // The fullest other tree: both lists, the removals tree and the last claim of both
    // fee pots. Four of its six keys then sort below the banlist, and no balanced tree roots
    // at a key with four keys on one side and one on the other: the version item takes the
    // top and the banlist sits one level down. Any key above the banlist would instead cost
    // the banlist the top from the contract's creation on, claims or not. Pinned so that a
    // change of either layout shows here.
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let mut contract =
        get_data_contract_fixture(None, 0, platform_version.protocol_version).data_contract_owned();
    contract.set_config(contract.config().clone().with_moderation(Some(
        ContractModerationConfig {
            banlist: true,
            suspensions: true,
            moderators: ContractModerators::ContractOwner,
            warnings: false,
        },
    )));
    add_deletable_type(&mut contract, POST);
    insert(&drive, &contract);

    let top_of_other = |drive: &Drive| {
        let contracts_root: &[u8] = Into::<&[u8; 1]>::into(RootTree::DataContractDocuments);
        let other = drive
            .grove
            .get_raw(
                (&[contracts_root, contract.id_ref().as_slice()]).into(),
                &[CONTRACT_OTHER_KEY],
                None,
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .expect("expected the other tree");
        match other {
            Element::Tree(root_key, _) => root_key,
            other => panic!("expected a tree, got {other:?}"),
        }
    };
    assert_eq!(top_of_other(&drive), Some(vec![CONTRACT_BANLIST_KEY]));

    // One pot claimed: the banlist stays on top.
    let claim = |pot| {
        apply(
            &drive,
            vec![DriveOperation::ContractFeePotOperation(
                ContractFeePotOperationType::SetLastClaim {
                    contract_id: contract.id(),
                    pot,
                    last_claim: ContractFeePotLastClaim {
                        epoch_index: 3,
                        time_ms: 1_000,
                        claimant_id: contract.owner_id(),
                    },
                },
            )],
            true,
        );
    };
    claim(ContractFeePot::Owner);
    assert_eq!(top_of_other(&drive), Some(vec![CONTRACT_BANLIST_KEY]));

    // Both claimed: the version item.
    claim(ContractFeePot::Moderators);
    assert_eq!(top_of_other(&drive), Some(vec![CONTRACT_VERSION_KEY]));
}

#[test]
fn should_record_read_and_prove_removals() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = contract_with(true, false, &[POST]);
    insert(&drive, &contract);
    let contract_id = contract.id();

    // Nothing yet: every id absent, the page empty.
    assert_removals(&drive, contract_id, &by_ids(&[identity(0x31)]), vec![]);
    assert_removals(&drive, contract_id, &page(None, 10), vec![]);

    let entries: Vec<ContractDocumentRemovalEntry> = [0x31u8, 0x32, 0x33]
        .into_iter()
        .map(|seed| ContractDocumentRemovalEntry {
            document_id: identity(seed),
            removal: removal(seed + 0x10, 2, "spam", 1_000 + seed as u64),
        })
        .collect();
    for entry in &entries {
        apply(
            &drive,
            vec![record(
                contract_id,
                entry.document_id,
                entry.removal.clone(),
            )],
            true,
        );
    }

    // By ids: the ones with a record, in id order, whatever order they were named in; an id
    // with no record is absent from the result, and proved so.
    assert_removals(
        &drive,
        contract_id,
        &by_ids(&[identity(0x33), identity(0x99), identity(0x31)]),
        vec![entries[0].clone(), entries[2].clone()],
    );

    // Pages, in id order, continuing after the cursor.
    assert_removals(&drive, contract_id, &page(None, 2), entries[..2].to_vec());
    assert_removals(
        &drive,
        contract_id,
        &page(Some(identity(0x32)), 2),
        entries[2..].to_vec(),
    );
    assert_removals(&drive, contract_id, &page(Some(identity(0x33)), 2), vec![]);

    // A contract nobody has, and a document type that keeps no records, read as none.
    assert_eq!(
        drive
            .fetch_contract_document_removals(
                identity(0x77),
                &page(None, 2),
                None,
                platform_version
            )
            .expect("expected to read an unknown contract as empty"),
        vec![]
    );
    let other_type = ContractDocumentRemovalsQuery {
        document_type_name: "niceDocument".to_string(),
        selection: ContractDocumentRemovalsSelection::Page {
            start_after: None,
            limit: 2,
        },
    };
    assert_eq!(
        drive
            .fetch_contract_document_removals(contract_id, &other_type, None, platform_version)
            .expect("expected to read a type without records as empty"),
        vec![]
    );
}

#[test]
fn should_refuse_a_read_out_of_bounds() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = contract_with(true, false, &[POST]);
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
            .fetch_contract_document_removals(contract.id(), &query, None, platform_version)
            .expect_err("expected the read to be refused");
        drive
            .prove_contract_document_removals(contract.id(), &query, None, platform_version)
            .expect_err("expected the proof to be refused");
    }
}

#[test]
fn should_estimate_a_record_at_no_less_than_it_costs() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = contract_with(true, false, &[POST]);
    insert(&drive, &contract);
    let longest = "x".repeat(
        platform_version
            .system_limits
            .max_contract_moderation_reason_length as usize,
    );

    for text in ["", "spam", longest.as_str()] {
        let operations = || {
            vec![record(
                contract.id(),
                identity(text.len() as u8),
                removal(1, 2, text, 10),
            )]
        };
        let estimated = apply(&drive, operations(), false);
        let applied = apply(&drive, operations(), true);
        assert!(
            estimated.storage_fee >= applied.storage_fee,
            "a {}-byte reason: estimated {} < applied {}",
            text.len(),
            estimated.storage_fee,
            applied.storage_fee
        );
        // The moderator pays for the reason byte for byte.
        assert!(applied.storage_fee > 0);
    }
}

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

fn delete_post<'a>(contract: &'a DataContract, document_id: Identifier) -> DriveOperation<'a> {
    DocumentOperation(DocumentOperationType::DeleteDocument {
        document_id,
        contract_info: DataContractInfo::BorrowedDataContract(contract),
        document_type_info: DocumentTypeInfo::DocumentTypeName(POST.to_string()),
    })
}

fn delete_post_by_moderator<'a>(
    contract: &'a DataContract,
    document_id: Identifier,
) -> DriveOperation<'a> {
    DocumentOperation(DocumentOperationType::DeleteDocumentByModerator {
        document_id,
        contract_info: DataContractInfo::BorrowedDataContract(contract),
        document_type_info: DocumentTypeInfo::DocumentTypeName(POST.to_string()),
    })
}

fn forfeit<'a>() -> DriveOperation<'a> {
    ContractModerationOperation(ContractModerationOperationType::ForfeitStorageRefunds)
}

#[test]
fn should_refund_nobody_for_a_document_a_moderator_deletes() {
    let platform_version = PlatformVersion::latest();
    let author = identity(0x41);
    let moderator = identity(0x42);

    // The author's own deletion: the author is refunded for the storage the post frees.
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = contract_with(true, false, &[POST]);
    insert(&drive, &contract);
    let post = add_post(&drive, &contract, author);
    // An estimate carries no refund, the owner's own deletion included: refunds are computed
    // from the flags of what is really removed. The forfeiture therefore only shows once
    // applied, and the mempool's fee check is the same with or without it.
    let own_estimate = apply(&drive, vec![delete_post(&contract, post.id())], false);
    assert!(own_estimate.fee_refunds.0.is_empty());
    let own = apply(&drive, vec![delete_post(&contract, post.id())], true);
    let refunded: u64 = own
        .fee_refunds
        .get(author.as_bytes())
        .expect("expected the author to be refunded")
        .values()
        .sum();
    assert!(refunded > 0);
    assert_eq!(own.removed_bytes_from_system, 0);

    // A moderator's deletion of the same post: the same bytes leave the system, nobody is
    // refunded, and the moderator pays for the record.
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    insert(&drive, &contract);
    let post = add_post(&drive, &contract, author);
    let operations = || {
        vec![
            delete_post_by_moderator(&contract, post.id()),
            record(
                contract.id(),
                post.id(),
                ContractDocumentRemoval {
                    document_owner_id: author,
                    moderator_id: moderator,
                    reason: ContractModerationReason::from_text("spam"),
                    removed_at: 10,
                    document_hash: [0x43; 32],
                    restoration: None,
                },
            ),
            forfeit(),
        ]
    };
    let estimated = apply(&drive, operations(), false);
    assert!(estimated.fee_refunds.0.is_empty());

    let moderated = apply(&drive, operations(), true);
    assert!(
        moderated.fee_refunds.0.is_empty(),
        "nobody is refunded: {:?}",
        moderated.fee_refunds
    );
    assert!(moderated.removed_bytes_from_system > 0);
    assert!(
        moderated.storage_fee > 0,
        "the moderator pays for the record"
    );

    // The post is gone and its record is there.
    let post_is_stored = drive
        .grove_has_raw(
            (&contract_documents_primary_key_path(contract.id_ref().as_bytes(), POST)).into(),
            post.id().as_slice(),
            DirectQueryType::StatefulDirectQuery,
            None,
            &mut vec![],
            &platform_version.drive,
        )
        .expect("expected to query the posts");
    assert!(!post_is_stored);
    assert_removals(
        &drive,
        contract.id(),
        &by_ids(&[post.id()]),
        vec![ContractDocumentRemovalEntry {
            document_id: post.id(),
            removal: ContractDocumentRemoval {
                document_owner_id: author,
                moderator_id: moderator,
                reason: ContractModerationReason::from_text("spam"),
                removed_at: 10,
                document_hash: [0x43; 32],
                restoration: None,
            },
        }],
    );
}

/// The insert that brings `post` back: the document as it was, its storage flags naming its
/// owner, as a create's do
fn restore_post<'a>(contract: &'a DataContract, post: &Document) -> DriveOperation<'a> {
    DocumentOperation(DocumentOperationType::AddDocument {
        owned_document_info: OwnedDocumentInfo {
            document_info: DocumentOwnedInfo((
                post.clone(),
                Some(Cow::Owned(StorageFlags::SingleEpochOwned(
                    3,
                    post.owner_id().to_buffer(),
                ))),
            )),
            owner_id: Some(post.owner_id().to_buffer()),
        },
        contract_info: DataContractInfo::BorrowedDataContract(contract),
        document_type_info: DocumentTypeInfo::DocumentTypeName(POST.to_string()),
        override_document: false,
    })
}

fn post_is_stored(drive: &Drive, contract: &DataContract, document_id: Identifier) -> bool {
    drive
        .grove_has_raw(
            (&contract_documents_primary_key_path(contract.id_ref().as_bytes(), POST)).into(),
            document_id.as_slice(),
            DirectQueryType::StatefulDirectQuery,
            None,
            &mut vec![],
            &PlatformVersion::latest().drive,
        )
        .expect("expected to query the posts")
}

#[test]
fn should_restore_a_document_mark_its_record_and_replace_the_record_on_a_second_deletion() {
    let platform_version = PlatformVersion::latest();
    let author = identity(0x41);
    let moderator = identity(0x42);
    let restorer = identity(0x43);
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = contract_with(true, false, &[POST]);
    insert(&drive, &contract);
    let post = add_post(&drive, &contract, author);
    let removal = ContractDocumentRemoval {
        document_owner_id: author,
        moderator_id: moderator,
        reason: ContractModerationReason::from_text("spam"),
        removed_at: 10,
        document_hash: [0x44; 32],
        restoration: None,
    };
    apply(
        &drive,
        vec![
            delete_post_by_moderator(&contract, post.id()),
            record(contract.id(), post.id(), removal.clone()),
            forfeit(),
        ],
        true,
    );
    assert!(!post_is_stored(&drive, &contract, post.id()));

    // The restore: the document back, the record marked in place. The restorer pays for the
    // document and for the bytes the mark adds; nobody is refunded anything.
    let restored = ContractDocumentRemoval {
        restoration: Some(ContractDocumentRestoration {
            moderator_id: restorer,
            restored_at: 20,
        }),
        ..removal.clone()
    };
    let operations = || {
        vec![
            restore_post(&contract, &post),
            replace_record(contract.id(), post.id(), restored.clone(), restorer),
        ]
    };
    let estimated = apply(&drive, operations(), false);
    let applied = apply(&drive, operations(), true);
    assert!(
        applied.storage_fee > 0,
        "the restorer pays for the document"
    );
    assert!(
        estimated.storage_fee >= applied.storage_fee,
        "estimated {} < applied {}",
        estimated.storage_fee,
        applied.storage_fee
    );
    assert!(applied.fee_refunds.0.is_empty());
    assert!(post_is_stored(&drive, &contract, post.id()));
    assert_removals(
        &drive,
        contract.id(),
        &by_ids(&[post.id()]),
        vec![ContractDocumentRemovalEntry {
            document_id: post.id(),
            removal: restored.clone(),
        }],
    );

    // Deleted again by a moderator: a fresh record in place of the restored one, the author
    // refunded nothing again.
    let again = ContractDocumentRemoval {
        moderator_id: restorer,
        removed_at: 30,
        restoration: None,
        ..removal
    };
    let applied = apply(
        &drive,
        vec![
            delete_post_by_moderator(&contract, post.id()),
            replace_record(contract.id(), post.id(), again.clone(), restorer),
            forfeit(),
        ],
        true,
    );
    assert!(applied.fee_refunds.0.is_empty());
    assert!(applied.removed_bytes_from_system > 0);
    assert!(!post_is_stored(&drive, &contract, post.id()));
    assert_removals(
        &drive,
        contract.id(),
        &by_ids(&[post.id()]),
        vec![ContractDocumentRemovalEntry {
            document_id: post.id(),
            removal: again,
        }],
    );
}

#[test]
fn should_refund_the_author_who_deletes_a_restored_document() {
    // The restored document's flags name its author, as they did before the deletion: the
    // refund of the author's own deletion is the author's, though a moderator paid to put
    // the document back.
    let platform_version = PlatformVersion::latest();
    let author = identity(0x41);
    let moderator = identity(0x42);
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = contract_with(true, false, &[POST]);
    insert(&drive, &contract);
    let post = add_post(&drive, &contract, author);
    let removal = removal(0x41, 0x42, "spam", 10);
    apply(
        &drive,
        vec![
            delete_post_by_moderator(&contract, post.id()),
            record(contract.id(), post.id(), removal.clone()),
            forfeit(),
        ],
        true,
    );
    let restored = ContractDocumentRemoval {
        restoration: Some(ContractDocumentRestoration {
            moderator_id: moderator,
            restored_at: 20,
        }),
        ..removal
    };
    apply(
        &drive,
        vec![
            restore_post(&contract, &post),
            replace_record(contract.id(), post.id(), restored.clone(), moderator),
        ],
        true,
    );

    let own = apply(&drive, vec![delete_post(&contract, post.id())], true);
    let refunded: u64 = own
        .fee_refunds
        .get(author.as_bytes())
        .expect("expected the author to be refunded")
        .values()
        .sum();
    assert!(refunded > 0);
    assert!(own.fee_refunds.get(moderator.as_bytes()).is_none());
    // The record stays as it was: the author's deletion is not a moderation.
    assert_removals(
        &drive,
        contract.id(),
        &by_ids(&[post.id()]),
        vec![ContractDocumentRemovalEntry {
            document_id: post.id(),
            removal: restored,
        }],
    );
}

#[test]
fn should_delete_for_a_moderator_a_document_its_owner_can_not_delete() {
    // `canBeDeleted` rules what a document's own owner may do. A post nobody can retract, but
    // moderation can remove, is the case the keyword exists for.
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let mut contract = contract_with(true, false, &[]);
    let mut schema = post_schema(true);
    schema
        .insert("canBeDeleted".to_string(), Value::Bool(false))
        .expect("expected to set canBeDeleted");
    contract
        .set_document_schema(POST, schema, true, &mut vec![], platform_version)
        .expect("expected to add the post type");
    insert(&drive, &contract);
    let post = add_post(&drive, &contract, identity(0x41));

    // The owner's deletion is refused by Drive, as it always was.
    drive
        .apply_drive_operations(
            vec![delete_post(&contract, post.id())],
            true,
            &BlockInfo::default(),
            None,
            platform_version,
            Some(&FEE_VERSIONS),
        )
        .expect_err("expected the owner's deletion to be refused");

    // The moderators' is not, estimated or applied.
    apply(
        &drive,
        vec![delete_post_by_moderator(&contract, post.id()), forfeit()],
        false,
    );
    apply(
        &drive,
        vec![delete_post_by_moderator(&contract, post.id()), forfeit()],
        true,
    );
    let post_is_stored = drive
        .grove_has_raw(
            (&contract_documents_primary_key_path(contract.id_ref().as_bytes(), POST)).into(),
            post.id().as_slice(),
            DirectQueryType::StatefulDirectQuery,
            None,
            &mut vec![],
            &platform_version.drive,
        )
        .expect("expected to query the posts");
    assert!(!post_is_stored);
}

#[test]
fn should_keep_refunding_a_batch_without_the_forfeiture_before_protocol_version_14() {
    // Generation 0 of `apply_drive_operations`, which every earlier protocol version runs,
    // knows nothing of the marker: a batch is refunded as it always was.
    let platform_version = PlatformVersion::get(13).expect("expected protocol version 13");
    assert_eq!(
        platform_version
            .drive
            .methods
            .batch_operations
            .apply_drive_operations,
        0
    );
    assert_eq!(
        PlatformVersion::latest()
            .drive
            .methods
            .batch_operations
            .apply_drive_operations,
        1
    );
}
