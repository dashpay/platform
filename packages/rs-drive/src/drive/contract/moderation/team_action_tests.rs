//! The actions a seated moderation team votes on: their trees, the writer of proposals and
//! approvals, the reads and proofs over them, and the refunds of the batch that closes one.

use crate::drive::contract::moderation::types::{
    ContractTeamActionEntry, ContractTeamActionWrite, ContractTeamActionsQuery,
};
use crate::drive::contract::paths::{
    contract_other_path, contract_team_actions_path, CONTRACT_TEAM_ACTIONS_KEY,
    CONTRACT_TEAM_ACTIVE_ACTIONS_KEY, CONTRACT_TEAM_CLOSED_ACTIONS_KEY,
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
    ContractModerationConfig, ContractModerationReason, ContractModerators, ContractTeamAction,
    ContractTeamActionEvent, ElectedModerators, InterimModerators, ModerationAbility,
    DEFAULT_ELECTION_WINDOW_SECONDS,
};
use dpp::data_contract::document_type::random_document::CreateRandomDocument;
use dpp::data_contract::schema::DataContractSchemaMethodsV0;
use dpp::data_contract::DataContract;
use dpp::document::{Document, DocumentV0Getters, DocumentV0Setters};
use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use dpp::fee::fee_result::FeeResult;
use dpp::group::group_action_status::GroupActionStatus;
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
        "required": ["text", "$createdAt", "$updatedAt"],
        "additionalProperties": false,
        "moderatorAbilities": abilities,
    })
}

/// The fixture contract with an elected moderation declaration moderating posts with
/// deletions, whose settled posts the team deletes by `rule` when it is given.
fn elected_contract_with(rule: Option<Value>) -> DataContract {
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
            moderated_document_types: BTreeMap::from([(
                POST.to_string(),
                BTreeSet::from([ModerationAbility::DeleteDocuments]),
            )]),
            interim: InterimModerators::ContractOwner,
            owner_protected: false,
        })),
        warnings: false,
    };
    contract.set_config(contract.config().clone().with_moderation(Some(moderation)));
    contract
        .set_document_schema(
            POST,
            post_schema(rule),
            true,
            &mut vec![],
            PlatformVersion::latest(),
        )
        .expect("expected to add the post type");
    contract
}

fn insert(drive: &Drive, contract: &DataContract) {
    // Estimated first, as the fee validation of the contract's creation estimates it
    for apply in [false, true] {
        drive
            .insert_contract(
                contract,
                BlockInfo::default(),
                apply,
                None,
                PlatformVersion::latest(),
            )
            .expect("expected to insert the contract");
    }
}

fn has(drive: &Drive, path: &[&[u8]], key: &[u8]) -> bool {
    drive
        .grove_has_raw(
            path.into(),
            key,
            DirectQueryType::StatefulDirectQuery,
            None,
            &mut vec![],
            &PlatformVersion::latest().drive,
        )
        .expect("expected to query the tree")
}

/// The proposal of the deletion of `document_id`, by `proposer`, for a reason `text`
fn proposal(proposer: u8, document_id: Identifier, text: &str) -> ContractTeamAction {
    ContractTeamAction {
        proposer_id: identity(proposer),
        proposed_at: 1_000,
        event: ContractTeamActionEvent::DeleteSettledDocument {
            document_type_name: POST.to_string(),
            document_id,
            document_last_modified_at: 10,
            document_revision: Some(1),
            // A seated team's reason always names a reason document its proposal lists
            reason: ContractModerationReason {
                code: Some(7),
                text: text.to_string(),
                documents: vec![],
                reason_document_id: Some(identity(0x7E)),
            },
        },
    }
}

fn signature<'a>(
    contract_id: Identifier,
    action_id: Identifier,
    signer: u8,
    write: ContractTeamActionWrite,
) -> DriveOperation<'a> {
    ContractModerationOperation(ContractModerationOperationType::AddTeamActionSignature {
        contract_id,
        action_id,
        signer_id: identity(signer),
        write,
    })
}

fn propose(action: &ContractTeamAction, closes: bool) -> ContractTeamActionWrite {
    ContractTeamActionWrite::Propose {
        action: action.clone(),
        closes,
    }
}

fn close(action: &ContractTeamAction, earlier_signers: &[u8]) -> ContractTeamActionWrite {
    close_dropping(action, earlier_signers, &[])
}

fn close_dropping(
    action: &ContractTeamAction,
    earlier_signers: &[u8],
    dropped_signers: &[u8],
) -> ContractTeamActionWrite {
    ContractTeamActionWrite::Close {
        action: action.clone(),
        earlier_signers: earlier_signers.iter().copied().map(identity).collect(),
        dropped_signers: dropped_signers.iter().copied().map(identity).collect(),
    }
}

fn approve_dropping(dropped_signers: &[u8]) -> ContractTeamActionWrite {
    ContractTeamActionWrite::Approve {
        dropped_signers: dropped_signers.iter().copied().map(identity).collect(),
    }
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

fn page(status: GroupActionStatus) -> ContractTeamActionsQuery {
    ContractTeamActionsQuery {
        status,
        start_at: None,
        limit: 10,
    }
}

fn root_hash(drive: &Drive) -> [u8; 32] {
    drive
        .grove
        .root_hash(None, &PlatformVersion::latest().drive.grove_version)
        .unwrap()
        .expect("expected a root hash")
}

/// Fetches, proves and verifies one page and checks all three agree.
fn assert_team_actions(
    drive: &Drive,
    contract_id: Identifier,
    query: &ContractTeamActionsQuery,
    expected: Vec<ContractTeamActionEntry>,
) {
    let platform_version = PlatformVersion::latest();
    let fetched = drive
        .fetch_contract_team_actions(contract_id, query, None, platform_version)
        .expect("expected to fetch the team actions");
    assert_eq!(fetched, expected, "fetched");

    let proof = drive
        .prove_contract_team_actions(contract_id, query, None, platform_version)
        .expect("expected a team actions proof");
    let (proved_root, proved) =
        Drive::verify_contract_team_actions(&proof, contract_id, query, false, platform_version)
            .expect("expected the proof to verify");
    assert_eq!(proved, expected, "proved");
    assert_eq!(proved_root, root_hash(drive));
}

/// Fetches, proves and verifies the approvals of one action and checks all three agree.
fn assert_signers(
    drive: &Drive,
    contract_id: Identifier,
    status: GroupActionStatus,
    action_id: Identifier,
    expected: &[u8],
) {
    let platform_version = PlatformVersion::latest();
    let mut expected: Vec<Identifier> = expected.iter().copied().map(identity).collect();
    expected.sort();
    let fetched = drive
        .fetch_contract_team_action_signers(contract_id, status, action_id, None, platform_version)
        .expect("expected to fetch the approvals");
    assert_eq!(fetched, expected, "fetched");

    let proof = drive
        .prove_contract_team_action_signers(contract_id, status, action_id, None, platform_version)
        .expect("expected an approvals proof");
    let (proved_root, proved) = Drive::verify_contract_team_action_signers(
        &proof,
        contract_id,
        status,
        action_id,
        false,
        platform_version,
    )
    .expect("expected the proof to verify");
    assert_eq!(proved, expected, "proved");
    assert_eq!(proved_root, root_hash(drive));
}

/// Proves one member's approval as a transition's execution is proved, and verifies it.
fn proved_signature(
    drive: &Drive,
    contract_id: Identifier,
    action_id: Identifier,
    signer: u8,
) -> Result<GroupActionStatus, crate::error::Error> {
    let platform_version = PlatformVersion::latest();
    let proof = drive
        .grove_get_proved_path_query(
            &Drive::contract_team_action_signer_query(
                contract_id.to_buffer(),
                action_id.to_buffer(),
                identity(signer).to_buffer(),
            ),
            None,
            &mut vec![],
            &platform_version.drive,
        )
        .expect("expected a proof of the approval");
    Drive::verify_contract_team_action_signature(
        &proof,
        contract_id,
        action_id,
        identity(signer),
        false,
        platform_version,
    )
    .map(|(root, status)| {
        assert_eq!(root, root_hash(drive));
        status
    })
}

#[test]
fn should_create_the_team_action_trees_with_the_contract() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));

    let settled = elected_contract_with(Some(platform_value!({ "leader": true })));
    insert(&drive, &settled);
    assert!(has(
        &drive,
        &contract_other_path(settled.id().as_slice()),
        &[CONTRACT_TEAM_ACTIONS_KEY]
    ));
    for status_key in [
        CONTRACT_TEAM_ACTIVE_ACTIONS_KEY,
        CONTRACT_TEAM_CLOSED_ACTIONS_KEY,
    ] {
        assert!(has(
            &drive,
            &contract_team_actions_path(settled.id().as_slice()),
            status_key
        ));
    }

    // A contract whose settled documents nobody deletes keeps no team actions, and reads as
    // having none
    let mut unsettled = elected_contract_with(None);
    unsettled.set_id(identity(0x77));
    insert(&drive, &unsettled);
    assert!(!has(
        &drive,
        &contract_other_path(unsettled.id().as_slice()),
        &[CONTRACT_TEAM_ACTIONS_KEY]
    ));
    assert!(drive
        .fetch_contract_team_actions(
            unsettled.id(),
            &page(GroupActionStatus::ActionActive),
            None,
            platform_version
        )
        .expect("expected to fetch")
        .is_empty());
    assert_eq!(
        drive
            .fetch_contract_team_action(unsettled.id(), identity(1), None, platform_version)
            .expect("expected to fetch"),
        None
    );
    assert!(drive
        .fetch_contract_team_action_signers(
            unsettled.id(),
            GroupActionStatus::ActionActive,
            identity(1),
            None,
            platform_version
        )
        .expect("expected to fetch")
        .is_empty());
}

#[test]
fn should_record_read_and_prove_team_actions_as_they_close() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = elected_contract_with(Some(platform_value!({ "approvals": 3 })));
    insert(&drive, &contract);
    let contract_id = contract.id();
    let (first, second) = (identity(0xA1), identity(0xA2));
    let first_proposal = proposal(1, identity(0x51), "doxxing");
    let second_proposal = proposal(4, identity(0x52), "spam");

    // The first is proposed by 1 and approved by 2, still short of three approvals; the second
    // is proposed by a member who meets its rule alone, so it closes at once
    apply(
        &drive,
        vec![
            signature(contract_id, first, 1, propose(&first_proposal, false)),
            signature(contract_id, second, 4, propose(&second_proposal, true)),
        ],
        true,
    );
    apply(
        &drive,
        vec![signature(
            contract_id,
            first,
            2,
            ContractTeamActionWrite::Approve {
                dropped_signers: vec![],
            },
        )],
        true,
    );

    assert_eq!(
        drive
            .fetch_contract_team_action(contract_id, first, None, platform_version)
            .expect("expected to fetch"),
        Some((GroupActionStatus::ActionActive, first_proposal.clone()))
    );
    assert_eq!(
        drive
            .fetch_contract_team_action(contract_id, identity(0xA9), None, platform_version)
            .expect("expected to fetch"),
        None
    );
    assert_team_actions(
        &drive,
        contract_id,
        &page(GroupActionStatus::ActionActive),
        vec![ContractTeamActionEntry {
            action_id: first,
            action: first_proposal.clone(),
            approval_count: 2,
        }],
    );
    assert_signers(
        &drive,
        contract_id,
        GroupActionStatus::ActionActive,
        first,
        &[1, 2],
    );
    assert_signers(
        &drive,
        contract_id,
        GroupActionStatus::ActionClosed,
        second,
        &[4],
    );
    assert_eq!(
        proved_signature(&drive, contract_id, first, 2).expect("expected the approval"),
        GroupActionStatus::ActionActive
    );
    assert_eq!(
        proved_signature(&drive, contract_id, second, 4).expect("expected the approval"),
        GroupActionStatus::ActionClosed
    );
    proved_signature(&drive, contract_id, first, 3)
        .expect_err("expected no approval by a member who never gave one");

    // The third approval closes the first: it moves, with every approval and its info, to the
    // closed actions, and nothing of it stays active
    apply(
        &drive,
        vec![signature(
            contract_id,
            first,
            3,
            close(&first_proposal, &[1, 2]),
        )],
        true,
    );
    assert_eq!(
        drive
            .fetch_contract_team_action(contract_id, first, None, platform_version)
            .expect("expected to fetch"),
        Some((GroupActionStatus::ActionClosed, first_proposal.clone()))
    );
    assert_team_actions(
        &drive,
        contract_id,
        &page(GroupActionStatus::ActionActive),
        vec![],
    );
    assert_team_actions(
        &drive,
        contract_id,
        &page(GroupActionStatus::ActionClosed),
        vec![
            ContractTeamActionEntry {
                action_id: first,
                action: first_proposal.clone(),
                approval_count: 3,
            },
            ContractTeamActionEntry {
                action_id: second,
                action: second_proposal.clone(),
                approval_count: 1,
            },
        ],
    );
    assert_signers(
        &drive,
        contract_id,
        GroupActionStatus::ActionActive,
        first,
        &[],
    );
    assert_signers(
        &drive,
        contract_id,
        GroupActionStatus::ActionClosed,
        first,
        &[1, 2, 3],
    );
    // An approval given while the action was active proves the same once it closed
    assert_eq!(
        proved_signature(&drive, contract_id, first, 2).expect("expected the approval"),
        GroupActionStatus::ActionClosed
    );

    // A page from an action id, the id included or not
    assert_team_actions(
        &drive,
        contract_id,
        &ContractTeamActionsQuery {
            status: GroupActionStatus::ActionClosed,
            start_at: Some((first, false)),
            limit: 10,
        },
        vec![ContractTeamActionEntry {
            action_id: second,
            action: second_proposal,
            approval_count: 1,
        }],
    );
    // A page of one action holds that action whole: its info and its approvals
    assert_team_actions(
        &drive,
        contract_id,
        &ContractTeamActionsQuery {
            status: GroupActionStatus::ActionClosed,
            start_at: None,
            limit: 1,
        },
        vec![ContractTeamActionEntry {
            action_id: first,
            action: first_proposal,
            approval_count: 3,
        }],
    );
}

#[test]
fn should_refuse_a_page_of_team_actions_out_of_bounds() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = elected_contract_with(Some(platform_value!({ "leader": true })));
    insert(&drive, &contract);
    let max = platform_version.drive_abci.query.max_returned_elements;
    for limit in [0, max + 1] {
        let query = ContractTeamActionsQuery {
            status: GroupActionStatus::ActionActive,
            start_at: None,
            limit,
        };
        drive
            .fetch_contract_team_actions(contract.id(), &query, None, platform_version)
            .expect_err("expected the read to be refused");
        drive
            .prove_contract_team_actions(contract.id(), &query, None, platform_version)
            .expect_err("expected the proof to be refused");
    }
}

/// What GroveDB's estimate may leave out of the storage a write of `sum_items` sum items and
/// `sum_trees` sum trees is charged: it estimates a sum item by its serialized size and a sum tree
/// by its layer's average, and charges their fixed costs when applied, a few bytes more each, as
/// for a token group's approvals. Nothing else may be under-estimated.
fn sum_element_allowance(sum_items: u64, sum_trees: u64) -> u64 {
    const SUM_ITEM_GAP_BYTES: u64 = 8;
    const SUM_TREE_GAP_BYTES: u64 = 9;
    (sum_items * SUM_ITEM_GAP_BYTES + sum_trees * SUM_TREE_GAP_BYTES)
        * FeeVersion::first()
            .storage
            .storage_disk_usage_credit_per_byte
}

/// Applies `operations`, which insert `sum_items` sum items and create `sum_trees` sum trees,
/// estimated then applied, and checks the estimate is no less than the cost: the total, which
/// the balance check reads, and the storage but for the sum elements' known gap
/// ([`sum_element_allowance`]).
fn assert_estimated_at_no_less_than_it_costs(
    drive: &Drive,
    what: &str,
    sum_items: u64,
    sum_trees: u64,
    operations: impl Fn() -> Vec<DriveOperation<'static>>,
) {
    let estimated = apply(drive, operations(), false);
    let applied = apply(drive, operations(), true);
    assert!(
        applied.storage_fee > 0,
        "{what}: the member pays for its approval"
    );
    assert!(
        estimated.storage_fee + sum_element_allowance(sum_items, sum_trees) >= applied.storage_fee,
        "{what}: estimated storage {} (+ allowance {}) < applied {}",
        estimated.storage_fee,
        sum_element_allowance(sum_items, sum_trees),
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
fn should_estimate_team_actions_at_no_less_than_they_cost() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = elected_contract_with(Some(platform_value!({ "approvals": 16 })));
    insert(&drive, &contract);
    let contract_id = contract.id();
    let longest = "x".repeat(
        platform_version
            .system_limits
            .max_contract_moderation_reason_length as usize,
    );

    // For a reason empty, typical and as long as allowed: a proposal, every approval up to as
    // many as the declared team holds (its leader and 15 elected members), the one that closes
    // it, and a proposal that closes at once
    for (seed, text) in [(1u8, ""), (2, "spam"), (3, longest.as_str())] {
        let growing = identity(0x40 + seed);
        let proposed = proposal(1, identity(0x60 + seed), text);
        assert_estimated_at_no_less_than_it_costs(
            &drive,
            &format!("proposal, {text:?}"),
            1,
            1,
            || {
                vec![signature(
                    contract_id,
                    growing,
                    1,
                    propose(&proposed, false),
                )]
            },
        );
        for signer in 2..16u8 {
            assert_estimated_at_no_less_than_it_costs(
                &drive,
                &format!("approval {signer}, {text:?}"),
                1,
                0,
                || {
                    vec![signature(
                        contract_id,
                        growing,
                        signer,
                        ContractTeamActionWrite::Approve {
                            dropped_signers: vec![],
                        },
                    )]
                },
            );
        }
        let earlier: Vec<u8> = (1..16).collect();
        assert_estimated_at_no_less_than_it_costs(
            &drive,
            &format!("closing, {text:?}"),
            16,
            1,
            || {
                vec![signature(
                    contract_id,
                    growing,
                    16,
                    close(&proposed, &earlier),
                )]
            },
        );

        let at_once = identity(0x50 + seed);
        let proposed = proposal(9, identity(0x70 + seed), text);
        assert_estimated_at_no_less_than_it_costs(
            &drive,
            &format!("closing proposal, {text:?}"),
            1,
            1,
            || vec![signature(contract_id, at_once, 9, propose(&proposed, true))],
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

/// The refunds of the approval by 3 that closes the deletion of a post by `author` which 1
/// proposed and 2 approved: the post's forfeited deletion, and the action closed, moving the
/// earlier approvals
fn refunds_of_a_closing_approval(author: Identifier) -> FeeResult {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = elected_contract_with(Some(platform_value!({ "approvals": 3 })));
    insert(&drive, &contract);
    let post = add_post(&drive, &contract, author);
    let action_id = identity(0xA1);
    let proposed = proposal(1, post.id(), "doxxing");
    apply(
        &drive,
        vec![signature(
            contract.id(),
            action_id,
            1,
            propose(&proposed, false),
        )],
        true,
    );
    apply(
        &drive,
        vec![signature(
            contract.id(),
            action_id,
            2,
            ContractTeamActionWrite::Approve {
                dropped_signers: vec![],
            },
        )],
        true,
    );
    apply(
        &drive,
        vec![
            DocumentOperation(DocumentOperationType::ForceDeleteDocument {
                document_id: post.id(),
                contract_info: DataContractInfo::BorrowedDataContract(&contract),
                document_type_info: DocumentTypeInfo::DocumentTypeName(POST.to_string()),
            }),
            signature(contract.id(), action_id, 3, close(&proposed, &[1, 2])),
            ContractModerationOperation(ContractModerationOperationType::ForfeitStorageRefunds),
        ],
        true,
    )
}

#[test]
fn should_refund_the_earlier_approvers_when_the_closing_approval_moves_them() {
    // The author forfeits the post's refund; the proposer is refunded its action and its
    // approval, the second approver its approval, as they move to the closed actions without
    // flags.
    let applied = refunds_of_a_closing_approval(identity(0x41));
    assert_eq!(
        applied
            .fee_refunds
            .0
            .keys()
            .copied()
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([identity(1).to_buffer(), identity(2).to_buffer()])
    );
    assert!(applied.removed_bytes_from_system > 0);
}

#[test]
fn should_refund_an_approver_who_wrote_the_post_only_its_approval() {
    // The approver who wrote the post forfeits the post's refund all the same, and gets back
    // its approval alone, as much as when another member wrote the post.
    let own_post = refunds_of_a_closing_approval(identity(2));
    let other_post = refunds_of_a_closing_approval(identity(0x41));
    assert_eq!(own_post.fee_refunds, other_post.fee_refunds);
}

#[test]
fn should_drop_the_approvals_of_members_who_left_and_refund_them() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = elected_contract_with(Some(platform_value!({ "approvals": 3 })));
    insert(&drive, &contract);
    let contract_id = contract.id();
    let action_id = identity(0xA1);
    let proposed = proposal(1, identity(0x51), "doxxing");
    apply(
        &drive,
        vec![signature(
            contract_id,
            action_id,
            1,
            propose(&proposed, false),
        )],
        true,
    );
    apply(
        &drive,
        vec![signature(contract_id, action_id, 2, approve_dropping(&[]))],
        true,
    );

    // 2 left the team: the next approval drops it, and refunds it its approval
    let dropping = apply(
        &drive,
        vec![signature(contract_id, action_id, 3, approve_dropping(&[2]))],
        true,
    );
    assert_eq!(
        dropping.fee_refunds.0.keys().copied().collect::<Vec<_>>(),
        vec![identity(2).to_buffer()]
    );
    assert_signers(
        &drive,
        contract_id,
        GroupActionStatus::ActionActive,
        action_id,
        &[1, 3],
    );
    // Back on the team, 2 approves again
    apply(
        &drive,
        vec![signature(contract_id, action_id, 2, approve_dropping(&[]))],
        true,
    );

    // 3 left: the approval that closes the action drops it, and moves the others
    apply(
        &drive,
        vec![signature(
            contract_id,
            action_id,
            4,
            close_dropping(&proposed, &[1, 2], &[3]),
        )],
        true,
    );
    assert_signers(
        &drive,
        contract_id,
        GroupActionStatus::ActionClosed,
        action_id,
        &[1, 2, 4],
    );
    assert_signers(
        &drive,
        contract_id,
        GroupActionStatus::ActionActive,
        action_id,
        &[],
    );
}

#[test]
fn should_bill_the_reads_of_an_approval() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = elected_contract_with(Some(platform_value!({ "approvals": 3 })));
    insert(&drive, &contract);
    let action_id = identity(0xA1);
    let proposed = proposal(1, identity(0x51), "doxxing");
    apply(
        &drive,
        vec![signature(
            contract.id(),
            action_id,
            1,
            propose(&proposed, false),
        )],
        true,
    );
    let epoch = Epoch::new(3).expect("epoch");

    // The reads an approval's validation makes, the action and its approvals, are billed
    let (action_fee, found) = drive
        .fetch_contract_team_action_with_fee(
            contract.id(),
            action_id,
            &epoch,
            None,
            platform_version,
        )
        .expect("expected to read the action");
    assert_eq!(found, Some((GroupActionStatus::ActionActive, proposed)));
    assert!(action_fee.processing_fee > 0);
    let (signers_fee, signers) = drive
        .fetch_contract_team_action_signers_with_fee(
            contract.id(),
            GroupActionStatus::ActionActive,
            action_id,
            &epoch,
            None,
            platform_version,
        )
        .expect("expected to read the approvals");
    assert_eq!(signers, vec![identity(1)]);
    assert!(signers_fee.processing_fee > 0);

    // An action the team never proposed is read, and billed, as none
    let (missing_fee, missing) = drive
        .fetch_contract_team_action_with_fee(
            contract.id(),
            identity(0xA9),
            &epoch,
            None,
            platform_version,
        )
        .expect("expected to read no action");
    assert_eq!(missing, None);
    assert!(missing_fee.processing_fee > 0);
}

#[test]
fn should_read_a_contract_nobody_has_as_keeping_no_team_actions() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let nobody = identity(0x99);
    assert_eq!(
        drive
            .fetch_contract_team_action(nobody, identity(1), None, platform_version)
            .expect("expected to fetch"),
        None
    );
    assert!(drive
        .fetch_contract_team_actions(
            nobody,
            &page(GroupActionStatus::ActionClosed),
            None,
            platform_version
        )
        .expect("expected to fetch")
        .is_empty());
    assert!(drive
        .fetch_contract_team_action_signers(
            nobody,
            GroupActionStatus::ActionActive,
            identity(1),
            None,
            platform_version
        )
        .expect("expected to fetch")
        .is_empty());
}
