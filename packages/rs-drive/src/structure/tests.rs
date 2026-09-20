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
    use crate::drive::credit_pools::epochs::operations_factory::EpochOperations;
    use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
    use crate::drive::Drive;
    use crate::fees::op::LowLevelDriveOperation;
    use crate::structure::conformance::ConformanceReport;
    use crate::util::batch::drive_op_batch::AddressFundsOperationType;
    use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
    use crate::util::batch::DriveOperation;
    use crate::util::batch::GroveDbOpBatch;
    use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
    use crate::util::object_size_info::{
        DataContractOwnedResolvedInfo, DocumentAndContractInfo, OwnedDocumentInfo,
    };
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::setup::setup_document;
    use crate::util::test_helpers::setup_contract;
    use dpp::address_funds::PlatformAddress;
    use dpp::block::block_info::BlockInfo;
    use dpp::block::epoch::Epoch;
    use dpp::contract_group::{
        generate_contract_group_id, ContractGroupMember, ContractGroupMembership,
        ContractGroupRegistration,
    };
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::accessors::v1::DataContractV1Getters;
    use dpp::data_contract::accessors::v1::DataContractV1Setters;
    use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
    use dpp::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
    use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
    use dpp::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Setters;
    use dpp::data_contract::associated_token::token_distribution_rules::accessors::v1::TokenDistributionRulesV1Setters;
    use dpp::data_contract::associated_token::token_once_per_identity_distribution::v0::TokenOncePerIdentityDistributionV0;
    use dpp::data_contract::associated_token::token_once_per_identity_distribution::TokenOncePerIdentityDistribution;
    use dpp::data_contract::associated_token::token_pre_programmed_distribution::v0::TokenPreProgrammedDistributionV0;
    use dpp::data_contract::associated_token::token_pre_programmed_distribution::TokenPreProgrammedDistribution;
    use dpp::data_contract::config::v0::{DataContractConfigSettersV0, DataContractConfigV0};
    use dpp::data_contract::config::DataContractConfig;
    use dpp::data_contract::document_type::random_document::CreateRandomDocument;
    use dpp::data_contract::group::v0::GroupV0;
    use dpp::data_contract::group::Group;
    use dpp::data_contract::storage_requirements::keys_for_document_type::StorageKeyRequirements;
    use dpp::data_contract::v1::DataContractV1;
    use dpp::data_contract::DataContract;
    use dpp::document::{DocumentV0Getters, DocumentV0Setters};
    use dpp::fee::Credits;
    use dpp::group::action_event::GroupActionEvent;
    use dpp::group::group_action::v0::GroupActionV0;
    use dpp::group::group_action::GroupAction;
    use dpp::identifier::Identifier;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::contract_bounds::ContractBounds;
    use dpp::identity::identity_public_key::v1::IdentityPublicKeyV1;
    use dpp::identity::Identity;
    use dpp::identity::{IdentityPublicKey, KeyID, KeyType, Purpose, SecurityLevel};
    use dpp::platform_value::BinaryData;
    use dpp::platform_value::Value;
    use dpp::tests::fixtures::get_dashpay_contract_fixture;
    use dpp::tokens::status::TokenStatus;
    use dpp::tokens::token_event::TokenEvent;
    use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;
    use dpp::voting::vote_info_storage::contested_document_vote_poll_stored_info::ContestedDocumentVotePollStoredInfo;
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    use std::collections::{BTreeMap, BTreeSet};

    /// Nodes no fixture below reaches, but the drive-abci strategy tests do:
    /// they check the state of every chain they run against the description
    /// (`assert_state_conforms_to_structure` in their `execution.rs`), and
    /// between them they write these.
    const REACHED_BY_STRATEGY_TESTS: &[&str] = &[
        "tokens.distributions.perpetual.token",
        "tokens.distributions.perpetual.token.info",
        "tokens.distributions.perpetual.token.last_claim",
        "tokens.distributions.perpetual.token.last_claim.identity",
        "identities.identity.key_references.transfer.key",
        "identities.identity.key_references.voting.key",
        "saved_block_transactions.compacted.range",
        "saved_block_transactions.compacted_expiration.expiration",
        "saved_block_transactions.address_balances.block",
        "pools.pending_epoch_refunds.epoch",
        "pools.epoch.finished_epoch_info",
        "pools.epoch.processing_fees",
        "misc.genesis_core_height",
        "spent_asset_locks.outpoint",
        "withdrawals.queue.transaction",
        "withdrawals.sum_amount.entry",
        "withdrawals.broadcasted.transaction",
        "withdrawals.total_credits_history.snapshot",
        "withdrawals.credit_inflows.inflow",
        "shielded_balances.main_pool.anchors_by_height.height",
        "shielded_balances.main_pool.anchors_in_pool.anchor",
        "votes.contested_resource.identity_votes.voter",
        "votes.contested_resource.identity_votes.voter.vote",
        "votes.contested_resource.active_polls.contract.document_type.indexes.value.abstain.votes.voter",
        "votes.contested_resource.active_polls.contract.document_type.indexes.value.lock.votes.voter",
        "votes.contested_resource.active_polls.contract.document_type.indexes.value.contender.votes.voter",
    ];

    /// Nodes nothing reaches, so their description has not been checked
    /// against a real GroveDB. Empty, and meant to stay empty: whoever
    /// describes a node can write a fixture that creates it. The coverage test
    /// fails when a listed node does get reached.
    const UNVERIFIED: &[&str] = &[];

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

    /// An epoch while it runs, then after it was paid out: payout deletes the
    /// proposers and both fee items and keeps the epoch tree.
    fn current_then_paid_epoch() -> ConformanceReport {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let epoch = Epoch::new(0).expect("expected the genesis epoch");

        let mut batch = GroveDbOpBatch::new();
        epoch.add_init_current_operations(
            1000,
            2,
            5,
            3,
            platform_version.protocol_version,
            &mut batch,
        );
        batch.push(epoch.update_proposer_block_count_operation(&[7; 32], 3));
        drive
            .grove_apply_batch(batch, false, None, &platform_version.drive)
            .expect("expected to start the epoch");
        let mut report = conformance_of(&drive, "current_epoch");

        let mut batch = GroveDbOpBatch::new();
        epoch.add_mark_as_paid_operations(&mut batch);
        drive
            .grove_apply_batch(batch, false, None, &platform_version.drive)
            .expect("expected to mark the epoch as paid");
        report
            .visited
            .extend(conformance_of(&drive, "paid_epoch").visited);
        report
    }

    /// Two contests on the DPNS index of parent domain name and label. In the
    /// second the label is 32 bytes long: as long as the identity id of a
    /// contender, one level above where contenders are, so only what is below
    /// the key tells the two apart. (DPNS only contests shorter labels, but
    /// that rule lives in dpp, and any contested index over identifiers has
    /// 32 byte values.)
    fn contested_documents() -> ConformanceReport {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/dpns/dpns-contract-contested-unique-index.json",
            None,
            None,
            None::<fn(&mut DataContract)>,
            None,
            Some(platform_version),
        );
        let document_type = contract
            .document_type_for_name("domain")
            .expect("expected the domain document type");

        let thirty_two_bytes = "b".repeat(32);
        for (seed, parent, label) in [
            (1, "dash", "quantum"),
            (2, "dash", thirty_two_bytes.as_str()),
        ] {
            let mut document = document_type
                .random_document(Some(seed), platform_version)
                .expect("expected a random document");
            document.set(
                "normalizedParentDomainName",
                Value::Text(parent.to_string()),
            );
            document.set("normalizedLabel", Value::Text(label.to_string()));
            drive
                .add_contested_document_for_contract(
                    DocumentAndContractInfo {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentRefInfo((&document, None)),
                            owner_id: Some(document.owner_id().to_buffer()),
                        },
                        contract: &contract,
                        document_type,
                    },
                    ContestedDocumentResourceVotePollWithContractInfo {
                        contract: DataContractOwnedResolvedInfo::OwnedDataContract(
                            contract.clone(),
                        ),
                        document_type_name: "domain".to_string(),
                        index_name: "parentNameAndLabel".to_string(),
                        index_values: vec![
                            Value::Text(parent.to_string()),
                            Value::Text(label.to_string()),
                        ],
                    },
                    false,
                    BlockInfo::default(),
                    true,
                    Some(
                        ContestedDocumentVotePollStoredInfo::new(
                            BlockInfo::default(),
                            platform_version,
                        )
                        .expect("expected the stored info of a new poll"),
                    ),
                    None,
                    platform_version,
                )
                .expect("expected to add the contested document");
        }
        conformance_of(&drive, "contested_documents")
    }

    fn apply_operations(drive: &Drive, operations: Vec<LowLevelDriveOperation>) {
        drive
            .apply_batch_low_level_drive_operations(
                None,
                None,
                operations,
                &mut vec![],
                &PlatformVersion::latest().drive,
            )
            .expect("expected to apply the operations");
    }

    /// A token with a pre-programmed release and a once per identity rule,
    /// before and after each is claimed. The second release stays queued.
    fn token_distributions() -> ConformanceReport {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let recipient = [7; 32];

        let mut configuration = TokenConfiguration::V0(
            TokenConfigurationV0::default_most_restrictive().with_base_supply(0),
        );
        configuration
            .distribution_rules_mut()
            .set_pre_programmed_distribution(Some(TokenPreProgrammedDistribution::V0(
                TokenPreProgrammedDistributionV0 {
                    distributions: BTreeMap::from([
                        (100, BTreeMap::from([(Identifier::from(recipient), 445)])),
                        (200, BTreeMap::from([(Identifier::from(recipient), 5)])),
                    ]),
                },
            )));
        configuration
            .distribution_rules_mut()
            .set_once_per_identity_distribution(Some(TokenOncePerIdentityDistribution::V0(
                TokenOncePerIdentityDistributionV0 { amount: 100 },
            )));
        let mut contract = get_dashpay_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        contract.set_tokens(BTreeMap::from([(0, configuration)]));
        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to insert the token contract");
        let mut report = conformance_of(&drive, "token_distributions_unclaimed");

        let token_id = contract.token_id(0).expect("expected a token").to_buffer();
        apply_operations(
            &drive,
            drive
                .mark_pre_programmed_release_as_distributed_operations(
                    token_id,
                    recipient,
                    100,
                    &BlockInfo::default(),
                    &mut None,
                    None,
                    platform_version,
                )
                .expect("expected the pre-programmed claim operations"),
        );
        apply_operations(
            &drive,
            drive
                .mark_once_per_identity_release_as_distributed_operations(
                    token_id,
                    recipient,
                    1000,
                    &BlockInfo::default(),
                    &mut None,
                    platform_version,
                )
                .expect("expected the once per identity claim operations"),
        );
        report
            .visited
            .extend(conformance_of(&drive, "token_distributions_claimed").visited);
        report
    }

    /// A key bound to a contract, a document type or a contract group. A key
    /// given a budget is a version 1 key and gets a budget entry.
    fn bound_key(
        key_id: KeyID,
        purpose: Purpose,
        bounds: ContractBounds,
        total_budget: Option<Credits>,
    ) -> IdentityPublicKey {
        let mut rng = StdRng::seed_from_u64(key_id as u64 + 900);
        IdentityPublicKeyV1 {
            id: key_id,
            purpose,
            security_level: if purpose == Purpose::AUTHENTICATION {
                SecurityLevel::HIGH
            } else {
                SecurityLevel::MEDIUM
            },
            contract_bounds: Some(bounds),
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(
                KeyType::ECDSA_SECP256K1
                    .random_public_key_data(&mut rng, PlatformVersion::latest())
                    .expect("expected a random key"),
            ),
            disabled_at: None,
            total_budget,
            expires_at: None,
        }
        .into()
    }

    /// Contract groups with every kind of member, and an identity whose keys
    /// are bound to a contract that wants one key per purpose, to a document
    /// type of a contract that allows several, and to a contract group; one
    /// of its keys has a budget.
    ///
    /// Not covered, because Drive cannot write it: an encryption or decryption
    /// key bound to a whole contract that keeps a reference to the latest key.
    /// Its sibling reference is placed beside the purpose subtrees, one level
    /// above the key it names, and the insert fails.
    fn contract_groups_and_bound_keys() -> ConformanceReport {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let owner = Identifier::from([1; 32]);

        let mut contracts = vec![];
        for (seed, requirement) in [
            (1, StorageKeyRequirements::Unique),
            (2, StorageKeyRequirements::MultipleReferenceToLatest),
        ] {
            let mut contract =
                get_dashpay_contract_fixture(None, seed, platform_version.protocol_version)
                    .data_contract_owned();
            contract
                .config_mut()
                .set_requires_identity_encryption_bounded_key(Some(requirement));
            contract
                .config_mut()
                .set_requires_identity_decryption_bounded_key(Some(requirement));
            drive
                .apply_contract(
                    &contract,
                    BlockInfo::default(),
                    true,
                    StorageFlags::optional_default_as_cow(),
                    None,
                    platform_version,
                )
                .expect("expected to insert the contract");
            contracts.push(contract);
        }

        let group_id = generate_contract_group_id(&owner, 1);
        drive
            .insert_contract_group(
                group_id,
                &(
                    owner,
                    ContractGroupRegistration {
                        admins: BTreeSet::new(),
                        name: Some("wallet".to_string()),
                        description: None,
                    },
                )
                    .into(),
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to register the contract group");
        let member = |member| ContractGroupMembership {
            contract_group_id: group_id,
            member,
        };
        drive
            .insert_contract_group_memberships(
                contracts[0].id(),
                &[
                    member(ContractGroupMember::Contract),
                    member(ContractGroupMember::DocumentType("profile".to_string())),
                    member(ContractGroupMember::Token(0)),
                ],
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to record the memberships");

        let mut identity = Identity::random_identity(3, Some(77), platform_version)
            .expect("expected a random identity");
        for key in [
            bound_key(
                10,
                Purpose::ENCRYPTION,
                ContractBounds::SingleContract {
                    id: contracts[0].id(),
                },
                None,
            ),
            bound_key(
                11,
                Purpose::DECRYPTION,
                ContractBounds::SingleContractDocumentType {
                    id: contracts[1].id(),
                    document_type_name: "contactRequest".to_string(),
                },
                None,
            ),
            bound_key(
                12,
                Purpose::AUTHENTICATION,
                ContractBounds::ContractGroup { id: group_id },
                Some(5_000_000),
            ),
        ] {
            identity.add_public_key(key);
        }
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
        conformance_of(&drive, "contract_groups_and_bound_keys")
    }

    fn spent_nullifiers() -> ConformanceReport {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        apply_operations(
            &drive,
            drive
                .insert_nullifiers(&[[3; 32], [4; 32]], platform_version)
                .expect("expected the nullifier operations"),
        );
        conformance_of(&drive, "spent_nullifiers")
    }

    #[test]
    fn should_match_populated_state_and_reach_every_described_node() {
        let mut visited = BTreeSet::new();
        for report in [
            identities(),
            contracts_with_documents(),
            tokens_and_group_actions(),
            address_balances(),
            current_then_paid_epoch(),
            contested_documents(),
            token_distributions(),
            contract_groups_and_bound_keys(),
            spent_nullifiers(),
        ] {
            visited.extend(report.visited);
        }

        let mut unreached = vec![];
        let mut wrongly_listed = vec![];
        drive_structure().walk(&mut |node| {
            let reached = visited.contains(&node.id);
            let listed = UNVERIFIED.contains(&node.id.as_str())
                || REACHED_BY_STRATEGY_TESTS.contains(&node.id.as_str());
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
