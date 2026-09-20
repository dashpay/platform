use crate::config::DriveConfig;
use crate::drive::RootTree;
use crate::structure::conformance::{check_conformance, Violation};
use crate::structure::export::{StructureDocument, STRUCTURE_JSON_PATH};
use crate::structure::lint::lint;
use crate::structure::shape::layer_shapes;
use crate::structure::{drive_structure, ElementKind, StructureNode};
use crate::util::test_helpers::setup::{setup_drive, setup_drive_with_initial_state_structure};
use dpp::version::PlatformVersion;
use std::fs;
use std::path::{Path, PathBuf};

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn should_pass_lint() {
    let problems = lint(&drive_structure(), Some(&repository_root()));

    assert!(
        problems.is_empty(),
        "the structure description is inconsistent:\n  - {}",
        problems.join("\n  - ")
    );
}

#[test]
fn should_match_initial_structure_for_every_protocol_version() {
    let structure = drive_structure();
    let latest = PlatformVersion::latest().protocol_version;

    for protocol_version in 1..=latest {
        let platform_version =
            PlatformVersion::get(protocol_version).expect("expected a platform version");
        // One epoch per era keeps the 50 eras of epoch trees at 50 trees
        let drive = setup_drive(Some(DriveConfig {
            epochs_per_era: 1,
            ..Default::default()
        }));
        drive
            .create_initial_state_structure(None, platform_version)
            .expect("expected to create the initial state structure");

        let report = check_conformance(&drive, &structure, None, platform_version)
            .expect("expected to walk the initial state structure");

        assert!(
            report.violations.is_empty(),
            "protocol version {protocol_version}: the initial state structure differs from the \
             description:\n{}",
            report
                .violations
                .iter()
                .map(|violation| format!("  - {violation}"))
                .collect::<Vec<_>>()
                .join("\n"),
        );
    }
}

/// The description as the viewer reads it, with the shapes of a fresh chain
fn structure_json() -> String {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let structure = drive_structure();
    let origin = format!("genesis@{}", platform_version.protocol_version);
    let shapes = layer_shapes(&drive, &structure, &origin, platform_version);
    let document = StructureDocument::new(structure, platform_version.protocol_version, shapes);

    let mut json =
        serde_json::to_string_pretty(&document).expect("expected to serialize the structure");
    json.push('\n');
    json
}

#[test]
fn should_match_committed_grovedb_structure_json() {
    let path = repository_root().join(STRUCTURE_JSON_PATH);
    let json = structure_json();

    if std::env::var_os("UPDATE_GROVEDB_STRUCTURE").is_some() {
        fs::write(&path, &json).expect("expected to write the structure json");
        return;
    }

    let committed = fs::read_to_string(&path).unwrap_or_default();
    assert!(
        committed == json,
        "{STRUCTURE_JSON_PATH} is out of date. Regenerate it and commit the result:\n\n    \
         UPDATE_GROVEDB_STRUCTURE=1 cargo test -p drive --lib structure::tests\n"
    );
}

#[test]
fn should_record_the_root_layer_as_the_root_tree_diagram_draws_it() {
    let json: serde_json::Value =
        serde_json::from_str(&structure_json()).expect("expected valid json");
    let root = &json["layer_shapes"]["root"]["tree"];

    // DataContractDocuments on top, Identities and Balances below it
    assert_eq!(root["hex"], "40");
    assert_eq!(root["left"]["hex"], "20");
    assert_eq!(root["right"]["hex"], "60");
}

mod walker {
    use super::*;

    #[test]
    fn should_report_an_element_the_description_lacks() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let mut structure = drive_structure();
        structure.children.retain(|child| child.id != "versions");

        let report = check_conformance(&drive, &structure, None, platform_version)
            .expect("expected to walk the initial state structure");

        assert_eq!(
            report.violations,
            vec![Violation::UndescribedKey {
                layer: "root".to_string(),
                path: vec![],
                key: vec![RootTree::Versions as u8],
                kind: ElementKind::Tree,
            }]
        );
    }

    #[test]
    fn should_report_an_element_of_another_kind() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let mut structure = drive_structure();
        let balances = structure
            .children
            .iter_mut()
            .find(|child| child.id == "balances")
            .expect("expected the balances node");
        balances.kinds = vec![ElementKind::Tree];

        let report = check_conformance(&drive, &structure, None, platform_version)
            .expect("expected to walk the initial state structure");

        assert_eq!(
            report.violations,
            vec![Violation::KindMismatch {
                node: "balances".to_string(),
                path: vec![],
                expected: vec![ElementKind::Tree],
                actual: ElementKind::SumTree,
            }]
        );
    }

    #[test]
    fn should_report_a_described_node_that_is_missing() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let structure = StructureNode::root()
            .children(drive_structure().children)
            .child(
                StructureNode::fixed("imaginary", &[200], "Imaginary", "")
                    .kind(ElementKind::Tree)
                    .source("packages/rs-drive/src/drive/mod.rs")
                    .describe("A root tree nothing creates."),
            )
            .build();

        let report = check_conformance(&drive, &structure, None, platform_version)
            .expect("expected to walk the initial state structure");

        assert_eq!(
            report.violations,
            vec![Violation::MissingRequired {
                node: "imaginary".to_string(),
                path: vec![],
            }]
        );
    }

    #[test]
    fn should_report_an_element_from_a_later_protocol_version() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let structure = drive_structure();
        let earlier = PlatformVersion::get(13).expect("expected protocol version 13");

        // A state built for the latest version, read as if it were version 13
        let report = check_conformance(&drive, &structure, None, earlier)
            .expect("expected to walk the initial state structure");

        assert!(report.violations.contains(&Violation::OutsideItsVersions {
            node: "contract_groups".to_string(),
            since: 14,
            until: None,
        }));
    }
}

mod fixtures {
    use super::*;
    use crate::drive::Drive;
    use crate::structure::conformance::ConformanceReport;
    use crate::util::batch::drive_op_batch::AddressFundsOperationType;
    use crate::util::batch::DriveOperation;
    use crate::util::test_helpers::setup::setup_document;
    use crate::util::test_helpers::setup_contract;
    use dpp::address_funds::PlatformAddress;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::accessors::v1::DataContractV1Getters;
    use dpp::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
    use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
    use dpp::data_contract::config::v0::{DataContractConfigSettersV0, DataContractConfigV0};
    use dpp::data_contract::config::DataContractConfig;
    use dpp::data_contract::document_type::random_document::CreateRandomDocument;
    use dpp::data_contract::group::v0::GroupV0;
    use dpp::data_contract::group::Group;
    use dpp::data_contract::v1::DataContractV1;
    use dpp::data_contract::DataContract;
    use dpp::group::action_event::GroupActionEvent;
    use dpp::group::group_action::v0::GroupActionV0;
    use dpp::group::group_action::GroupAction;
    use dpp::identifier::Identifier;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;
    use dpp::tokens::status::TokenStatus;
    use dpp::tokens::token_event::TokenEvent;
    use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;
    use std::collections::{BTreeMap, BTreeSet};

    /// Nodes no fixture below reaches yet. The coverage test fails when a
    /// listed node does get reached, so this list can only shrink.
    const UNVERIFIED: &[&str] = &[
        // Token distributions: need a token configured with each rule
        "tokens.distributions.once_per_identity.token",
        "tokens.distributions.once_per_identity.token.identity",
        "tokens.distributions.perpetual.token",
        "tokens.distributions.perpetual.token.info",
        "tokens.distributions.perpetual.token.last_claim",
        "tokens.distributions.perpetual.token.last_claim.identity",
        "tokens.distributions.timed.ms.time",
        "tokens.distributions.timed.ms.time.release",
        "tokens.distributions.pre_programmed.token",
        "tokens.distributions.pre_programmed.token.last_claim",
        "tokens.distributions.pre_programmed.token.last_claim.identity",
        "tokens.distributions.pre_programmed.token.time",
        "tokens.distributions.pre_programmed.token.time.recipient",
        // Identity keys bound to a contract, masternode keys, key budgets
        "identities.identity.contract_info.bound.keys",
        "identities.identity.contract_info.bound.keys.latest",
        "identities.identity.contract_info.bound.keys.purpose",
        "identities.identity.contract_info.bound.keys.purpose.unique",
        "identities.identity.contract_info.bound.keys.purpose.key",
        "identities.identity.key_references.transfer.key",
        "identities.identity.key_references.voting.key",
        "identities.identity.key_budgets",
        "identities.identity.key_budgets.key",
        // Written by block execution in drive-abci
        "saved_block_transactions.compacted.range",
        "saved_block_transactions.compacted_expiration.expiration",
        "saved_block_transactions.address_balances.block",
        "pools.pending_epoch_refunds.epoch",
        "pools.epoch.start_block_core_height",
        "pools.epoch.finished_epoch_info",
        "pools.epoch.start_block_height",
        "pools.epoch.proposers",
        "pools.epoch.proposers.proposer",
        "pools.epoch.processing_fees",
        "pools.epoch.start_time",
        "pools.epoch.fee_multiplier",
        "misc.genesis_core_height",
        "spent_asset_locks.outpoint",
        "withdrawals.queue.transaction",
        "withdrawals.sum_amount.entry",
        "withdrawals.broadcasted.transaction",
        "withdrawals.total_credits_history.snapshot",
        "withdrawals.credit_inflows.inflow",
        // Shielded pool contents
        "shielded_balances.main_pool.nullifiers.nullifier",
        "shielded_balances.main_pool.anchors_by_height.height",
        "shielded_balances.main_pool.anchors_in_pool.anchor",
        // Contested resources: rs-drive has no fixture that runs a vote
        "votes.contested_resource.identity_votes.voter",
        "votes.contested_resource.identity_votes.voter.vote",
        "votes.contested_resource.active_polls.contract",
        "votes.contested_resource.active_polls.contract.document_type",
        "votes.contested_resource.active_polls.contract.document_type.storage",
        "votes.contested_resource.active_polls.contract.document_type.storage.document",
        "votes.contested_resource.active_polls.contract.document_type.indexes",
        "votes.contested_resource.active_polls.contract.document_type.indexes.value",
        "votes.contested_resource.active_polls.contract.document_type.indexes.value.stored_info",
        "votes.contested_resource.active_polls.contract.document_type.indexes.value.abstain",
        "votes.contested_resource.active_polls.contract.document_type.indexes.value.abstain.votes",
        "votes.contested_resource.active_polls.contract.document_type.indexes.value.abstain.votes.voter",
        "votes.contested_resource.active_polls.contract.document_type.indexes.value.lock",
        "votes.contested_resource.active_polls.contract.document_type.indexes.value.lock.votes",
        "votes.contested_resource.active_polls.contract.document_type.indexes.value.lock.votes.voter",
        "votes.contested_resource.active_polls.contract.document_type.indexes.value.contender",
        "votes.contested_resource.active_polls.contract.document_type.indexes.value.contender.document",
        "votes.contested_resource.active_polls.contract.document_type.indexes.value.contender.votes",
        "votes.contested_resource.active_polls.contract.document_type.indexes.value.contender.votes.voter",
        "votes.contested_resource.active_polls.contract.document_type.indexes.value.next_value",
        "votes.end_date_queries.end_date",
        "votes.end_date_queries.end_date.poll",
        // Contract groups
        "contract_groups.groups.group",
        "contract_groups.groups.group.info",
        "contract_groups.groups.group.contracts",
        "contract_groups.groups.group.contracts.contract",
        "contract_groups.groups.group.document_types",
        "contract_groups.groups.group.document_types.member",
        "contract_groups.groups.group.tokens",
        "contract_groups.groups.group.tokens.member",
        "contract_groups.members.contract",
        "contract_groups.members.contract.groups",
        "contract_groups.members.contract.groups.group",
        "contract_groups.members.contract.document_types",
        "contract_groups.members.contract.document_types.document_type",
        "contract_groups.members.contract.document_types.document_type.group",
        "contract_groups.members.contract.tokens",
        "contract_groups.members.contract.tokens.token",
        "contract_groups.members.contract.tokens.token.group",
    ];

    fn conformance_of(drive: &Drive, fixture: &str) -> ConformanceReport {
        let platform_version = PlatformVersion::latest();
        let report = check_conformance(drive, &drive_structure(), None, platform_version)
            .expect("expected to walk the state");
        assert!(
            report.violations.is_empty(),
            "fixture `{fixture}` differs from the description:\n{}",
            report
                .violations
                .iter()
                .map(|violation| format!("  - {violation}"))
                .collect::<Vec<_>>()
                .join("\n"),
        );
        report
    }

    fn identities() -> ConformanceReport {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        for seed in 1..4 {
            let identity = Identity::random_identity(5, Some(seed), platform_version)
                .expect("expected a random identity");
            let identity_id = identity.id().to_buffer();
            drive
                .add_new_identity(
                    identity,
                    false,
                    &BlockInfo::default(),
                    true,
                    None,
                    platform_version,
                )
                .expect("expected to add the identity");
            drive
                .merge_identity_contract_nonce(
                    identity_id,
                    [seed as u8; 32],
                    1,
                    &BlockInfo::default(),
                    true,
                    None,
                    &mut vec![],
                    platform_version,
                )
                .expect("expected to set a contract nonce");
        }
        drive
            .add_prefunded_specialized_balance(
                Identifier::from([5; 32]),
                1000,
                None,
                platform_version,
            )
            .expect("expected to add a prefunded balance");
        drive
            .update_validator_proposed_app_version(
                [6; 32],
                platform_version.protocol_version,
                None,
                &platform_version.drive,
            )
            .expect("expected to record a proposed version");
        conformance_of(&drive, "identities")
    }

    fn contracts_with_documents() -> ConformanceReport {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        for (index, path) in [
            "tests/supporting_files/contract/family/family-contract.json",
            "tests/supporting_files/contract/family/family-contract-countable.json",
            "tests/supporting_files/contract/family/family-contract-with-history.json",
            "tests/supporting_files/contract/dashpay/dashpay-contract.json",
            "tests/supporting_files/contract/references/references_with_contract_history.json",
        ]
        .into_iter()
        .enumerate()
        {
            let contract = setup_contract(
                &drive,
                path,
                Some([index as u8 + 1; 32]),
                None,
                // The last contract keeps the history of its own revisions
                Some(move |contract: &mut DataContract| {
                    contract.config_mut().set_keeps_history(index == 4)
                }),
                None,
                Some(platform_version),
            );
            for document_type in contract.document_types().values() {
                for seed in 1..4 {
                    let document = document_type
                        .random_document(Some(seed), platform_version)
                        .expect("expected a random document");
                    setup_document(&drive, &document, &contract, document_type.as_ref(), None);
                }
            }
        }
        conformance_of(&drive, "contracts_with_documents")
    }

    fn tokens_and_group_actions() -> ConformanceReport {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let member_1 = Identifier::from([1; 32]);
        let member_2 = Identifier::from([2; 32]);
        let contract = DataContract::V1(DataContractV1 {
            id: Identifier::from([9; 32]),
            version: 0,
            owner_id: member_1,
            document_types: Default::default(),
            config: DataContractConfig::V0(DataContractConfigV0 {
                can_be_deleted: false,
                readonly: false,
                keeps_history: false,
                documents_keep_history_contract_default: false,
                documents_mutable_contract_default: false,
                documents_can_be_deleted_contract_default: false,
                requires_identity_encryption_bounded_key: None,
                requires_identity_decryption_bounded_key: None,
            }),
            schema_defs: None,
            created_at: None,
            updated_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            created_at_epoch: None,
            updated_at_epoch: None,
            groups: BTreeMap::from([(
                0,
                Group::V0(GroupV0 {
                    members: [(member_1, 1), (member_2, 2)].into(),
                    required_power: 3,
                }),
            )]),
            tokens: BTreeMap::from([(
                0,
                TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive()),
            )]),
            keywords: Vec::new(),
            description: None,
        });
        drive
            .insert_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to insert the contract");

        let token_id = contract.token_id(0).expect("expected a token id");
        drive
            .token_mint(
                token_id.to_buffer(),
                member_1.to_buffer(),
                1000,
                true,
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to mint");
        drive
            .token_freeze(
                token_id,
                member_2,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to freeze");
        drive
            .token_apply_status(
                token_id.to_buffer(),
                TokenStatus::new(true, platform_version).expect("expected a token status"),
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to pause");
        drive
            .token_set_direct_purchase_price(
                token_id.to_buffer(),
                Some(TokenPricingSchedule::SinglePrice(10)),
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to set a price");

        // One action left active, one signed by both members and so closed
        for (action_id, close) in [([7; 32], false), ([8; 32], true)] {
            let action_id = Identifier::from(action_id);
            let action = GroupAction::V0(GroupActionV0 {
                contract_id: contract.id(),
                proposer_id: member_1,
                token_contract_position: 0,
                event: GroupActionEvent::TokenEvent(TokenEvent::Mint(100, member_1, None)),
            });
            drive
                .add_group_action(
                    contract.id(),
                    0,
                    Some(action),
                    false,
                    action_id,
                    member_1,
                    1,
                    &BlockInfo::default(),
                    true,
                    None,
                    platform_version,
                )
                .expect("expected to propose the action");
            if close {
                drive
                    .add_group_action(
                        contract.id(),
                        0,
                        None,
                        true,
                        action_id,
                        member_2,
                        2,
                        &BlockInfo::default(),
                        true,
                        None,
                        platform_version,
                    )
                    .expect("expected to close the action");
            }
        }
        conformance_of(&drive, "tokens_and_group_actions")
    }

    fn address_balances() -> ConformanceReport {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let operations = vec![
            DriveOperation::AddressFundsOperation(AddressFundsOperationType::SetBalanceToAddress {
                address: PlatformAddress::P2pkh([10; 20]),
                nonce: 5,
                balance: 1_000_000,
            }),
            DriveOperation::AddressFundsOperation(AddressFundsOperationType::SetBalanceToAddress {
                address: PlatformAddress::P2sh([11; 20]),
                nonce: 7,
                balance: 2_000_000,
            }),
        ];
        drive
            .apply_drive_operations(
                operations,
                true,
                &BlockInfo::default(),
                None,
                platform_version,
                None,
            )
            .expect("expected to set address balances");
        conformance_of(&drive, "address_balances")
    }

    #[test]
    fn should_match_populated_state_and_reach_every_described_node() {
        let mut visited = BTreeSet::new();
        for report in [
            identities(),
            contracts_with_documents(),
            tokens_and_group_actions(),
            address_balances(),
        ] {
            visited.extend(report.visited);
        }

        let mut unreached = vec![];
        let mut wrongly_listed = vec![];
        drive_structure().walk(&mut |node| {
            let reached = visited.contains(&node.id);
            let listed = UNVERIFIED.contains(&node.id.as_str());
            if !reached && !listed {
                unreached.push(node.id.clone());
            }
            if reached && listed {
                wrongly_listed.push(node.id.clone());
            }
        });

        assert!(
            wrongly_listed.is_empty(),
            "these nodes are reached now, take them off UNVERIFIED:\n  {}",
            wrongly_listed.join("\n  ")
        );
        assert!(
            unreached.is_empty(),
            "no fixture reaches these nodes; add one, or list them in UNVERIFIED:\n  {}",
            unreached.join("\n  ")
        );
    }
}
