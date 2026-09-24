use crate::config::DriveConfig;
use crate::drive::RootTree;
use crate::structure::conformance::{check_conformance, Violation};
use crate::structure::export::{StructureDocument, STRUCTURE_JSON_PATH};
use crate::structure::lint::lint;
use crate::structure::shape::layer_shapes;
use crate::structure::{drive_structure, ElementKind, FlagsKind, StructureNode};
use crate::util::test_helpers::setup::{setup_drive, setup_drive_with_initial_state_structure};
use dpp::block::block_info::BlockInfo;
use dpp::identity::Identity;
use dpp::version::PlatformVersion;
use std::collections::BTreeSet;
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

/// The description as the viewer reads it. Layers reached through fixed keys
/// only get the shape of a fresh chain; layers below a template, such as an
/// identity's, get the shape of the fullest instance the fixtures build.
fn structure_json() -> String {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let structure = drive_structure();
    let origin = format!("genesis@{}", platform_version.protocol_version);
    let mut shapes = layer_shapes(&drive, &structure, &origin, platform_version);
    shapes.extend(fixtures::run_all().shapes());
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

#[test]
fn should_record_a_contract_layer_with_its_documents_on_top() {
    let json: serde_json::Value =
        serde_json::from_str(&structure_json()).expect("expected valid json");

    // A contract's layer is one layer per contract, so its shape comes from a
    // fixture. Documents are read most and sit at the root of the layer; the
    // contract itself and everything else hang below.
    let contract = &json["layer_shapes"]["contracts.contract"];
    assert_eq!(contract["origin"], "fixture contracts_with_documents@14");
    assert_eq!(contract["tree"]["hex"], "01");
    assert_eq!(contract["tree"]["left"]["hex"], "00");
    assert_eq!(contract["tree"]["right"]["hex"], "02");

    // Inside `other` the banlist, read on every document transition of a
    // moderated contract, is in the middle
    let other = &json["layer_shapes"]["contracts.contract.other"]["tree"];
    assert_eq!(other["hex"], "80");
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
    fn should_report_element_flags_of_another_kind() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let identity = Identity::random_identity(2, Some(5), platform_version)
            .expect("expected a random identity");
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

        // An identity's tree carries the epoch it was created in
        let mut structure = drive_structure();
        let identities = structure
            .children
            .iter_mut()
            .find(|child| child.id == "identities")
            .expect("expected the identities node");
        identities.children[0].flags = vec![FlagsKind::None];

        let report = check_conformance(&drive, &structure, None, platform_version)
            .expect("expected to walk the state");

        assert_eq!(
            report.violations,
            vec![Violation::FlagsMismatch {
                node: "identities.identity".to_string(),
                path: vec![vec![RootTree::Identities as u8]],
                expected: vec![FlagsKind::None],
                actual: FlagsKind::Epoch,
            }]
        );
        assert_eq!(
            report.flags.get("identities.identity.keys"),
            Some(&BTreeSet::from([FlagsKind::None]))
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
    use crate::structure::export::{LayerShape, ShapeNode, StateShape};
    use crate::structure::shape::shape_at;
    use crate::structure::{KeySpec, NodeId};
    use crate::util::batch::drive_op_batch::{
        AddressFundsOperationType, ContractFeePotOperationType,
    };
    use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
    use crate::util::batch::ContractModerationOperationType;
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
    use crate::drive::credit_pools::epochs::paths::EpochProposers;
    use dpp::block::epoch::Epoch;
    use dpp::block::finalized_epoch_info::v0::FinalizedEpochInfoV0;
    use dpp::contract_group::{
        generate_contract_group_id, ContractGroupMember, ContractGroupMembership,
        ContractGroupRegistration,
    };
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::accessors::v0::DataContractV0Setters;
    use dpp::data_contract::accessors::v1::DataContractV1Getters;
    use dpp::data_contract::accessors::v1::DataContractV1Setters;
    use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
    use dpp::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
    use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
    use dpp::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Setters;
    use dpp::data_contract::associated_token::token_distribution_rules::accessors::v1::TokenDistributionRulesV1Setters;
    use dpp::data_contract::associated_token::token_once_per_identity_distribution::v0::TokenOncePerIdentityDistributionV0;
    use dpp::data_contract::associated_token::token_once_per_identity_distribution::TokenOncePerIdentityDistribution;
    use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction;
    use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::TokenDistributionRecipient;
    use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;
    use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;
    use dpp::data_contract::associated_token::token_perpetual_distribution::v0::TokenPerpetualDistributionV0;
    use dpp::data_contract::associated_token::token_perpetual_distribution::TokenPerpetualDistribution;
    use dpp::data_contract::associated_token::token_pre_programmed_distribution::v0::TokenPreProgrammedDistributionV0;
    use dpp::data_contract::associated_token::token_pre_programmed_distribution::TokenPreProgrammedDistribution;
    use dpp::data_contract::config::moderation::{
        ContractDocumentRemoval, ContractModerationConfig, ContractModerationReason,
        ContractModerators, ContractWarning, ElectedModerators, InterimModerators,
        ModerationAbility, DEFAULT_ELECTION_WINDOW_SECONDS,
    };
    use dpp::data_contract::config::v0::{DataContractConfigSettersV0, DataContractConfigV0};
    use dpp::data_contract::config::DataContractConfig;
    use dpp::data_contract::document_type::action_fees::{ContractFeePot, ContractFeePotLastClaim};
    use dpp::data_contract::document_type::random_document::CreateRandomDocument;
    use dpp::data_contract::group::v0::GroupV0;
    use dpp::data_contract::group::Group;
    use dpp::data_contract::schema::DataContractSchemaMethodsV0;
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
    use dpp::platform_value::platform_value;
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
        "identities.identity.key_references.transfer.key",
        "identities.identity.key_references.voting.key",
        "saved_block_transactions.compacted.range",
        "saved_block_transactions.compacted_expiration.expiration",
        "saved_block_transactions.address_balances.block",
        "pools.pending_epoch_refunds.epoch",
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

    /// What the fixtures found: every node they reached, and for each layer
    /// below a template the shape of its fullest instance.
    #[derive(Default)]
    pub(super) struct FixtureRun {
        visited: BTreeSet<NodeId>,
        fullest: BTreeMap<NodeId, (usize, LayerShape)>,
        states: BTreeMap<NodeId, BTreeMap<String, StateShape>>,
    }

    fn keys_of(tree: &ShapeNode, keys: &mut BTreeSet<Vec<u8>>) {
        keys.insert(tree.key.clone());
        for child in [&tree.left, &tree.right].into_iter().flatten() {
            keys_of(child, keys);
        }
    }

    impl FixtureRun {
        /// The shapes of the layers below a template. A layer that goes
        /// through states gets one shape per state, which must have been
        /// recorded from an instance holding exactly the keys the state
        /// declares; the state with the most keys is the layer's main shape.
        pub(super) fn shapes(mut self) -> BTreeMap<NodeId, LayerShape> {
            let structure = drive_structure();
            let mut shapes: BTreeMap<NodeId, LayerShape> = self
                .fullest
                .into_iter()
                .map(|(id, (_, shape))| (id, shape))
                .collect();

            structure.walk(&mut |node| {
                if node.states.is_empty() {
                    return;
                }
                let mut recorded = self.states.remove(&node.id).unwrap_or_default();
                let states: Vec<StateShape> = node
                    .states
                    .iter()
                    .map(|state| {
                        let shape = recorded.remove(&state.name).unwrap_or_else(|| {
                            panic!(
                                "no fixture records `{}` in its `{}` state",
                                node.id, state.name
                            )
                        });
                        let declared: BTreeSet<Vec<u8>> = state
                            .keys
                            .iter()
                            .map(|segment| {
                                node.children
                                    .iter()
                                    .find(|child| child.segment == *segment)
                                    .and_then(|child| child.fixed_key_bytes())
                                    .expect("the lint checks that state keys are fixed children")
                                    .to_vec()
                            })
                            .collect();
                        let mut found = BTreeSet::new();
                        keys_of(&shape.tree, &mut found);
                        assert_eq!(
                            found, declared,
                            "`{}` in its `{}` state holds other keys than the state declares",
                            node.id, state.name
                        );
                        shape
                    })
                    .collect();
                let main = states
                    .iter()
                    .zip(&node.states)
                    .max_by_key(|(_, state)| state.keys.len())
                    .map(|(shape, _)| shape.clone())
                    .expect("a node with states has at least two");
                shapes.insert(
                    node.id.clone(),
                    LayerShape {
                        origin: main.origin,
                        tree: main.tree,
                        states,
                    },
                );
            });
            shapes
        }

        /// Records the shape of one instance of a node's layer as the shape
        /// of one of the states the node declares
        fn record_state(
            &mut self,
            drive: &Drive,
            node: &str,
            state: &str,
            path: Vec<Vec<u8>>,
            fixture: &str,
        ) {
            let platform_version = PlatformVersion::latest();
            let tree =
                shape_at(drive, &path, platform_version).expect("expected the shape of the layer");
            self.states.entry(node.to_string()).or_default().insert(
                state.to_string(),
                StateShape {
                    state: state.to_string(),
                    origin: format!("fixture {fixture}@{}", platform_version.protocol_version),
                    tree,
                },
            );
        }
    }

    /// The nodes whose layer sits below a template, so that there is one
    /// instance of it per identity, contract, epoch and so on
    fn layers_below_a_template(structure: &StructureNode) -> BTreeSet<NodeId> {
        fn visit(node: &StructureNode, below_template: bool, found: &mut BTreeSet<NodeId>) {
            let below_template = below_template || matches!(node.key, KeySpec::Dynamic { .. });
            if below_template {
                found.insert(node.id.clone());
            }
            for child in &node.children {
                visit(child, below_template, found);
            }
        }
        let mut found = BTreeSet::new();
        visit(structure, false, &mut found);
        found
    }

    /// Checks the state against the description, and records the shape of
    /// every layer below a template that is fuller here than seen so far
    fn conformance_of(drive: &Drive, fixture: &str, run: &mut FixtureRun) {
        let platform_version = PlatformVersion::latest();
        let structure = drive_structure();
        let report = check_conformance(drive, &structure, None, platform_version)
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

        let templated = layers_below_a_template(&structure);
        for (id, (count, path)) in &report.fullest_layers {
            let fuller = run.fullest.get(id).is_none_or(|(best, _)| count > best);
            if !templated.contains(id) || *count < 2 || !fuller {
                continue;
            }
            let tree = shape_at(drive, path, platform_version)
                .expect("expected the shape of a layer the walker just read");
            let origin = format!("fixture {fixture}@{}", platform_version.protocol_version);
            run.fullest.insert(
                id.clone(),
                (
                    *count,
                    LayerShape {
                        origin,
                        tree,
                        states: vec![],
                    },
                ),
            );
        }
        run.visited.extend(report.visited);
    }

    fn identities(run: &mut FixtureRun) {
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
            let layer = vec![vec![RootTree::Identities as u8], identity_id.to_vec()];
            if seed == 1 {
                run.record_state(
                    &drive,
                    "identities.identity",
                    "created",
                    layer.clone(),
                    "identities",
                );
            }
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
            if seed == 1 {
                run.record_state(
                    &drive,
                    "identities.identity",
                    "used_with_a_contract",
                    layer,
                    "identities",
                );
            }
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
        conformance_of(&drive, "identities", run);
    }

    fn contracts_with_documents(run: &mut FixtureRun) {
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
        conformance_of(&drive, "contracts_with_documents", run);
    }

    /// A contract whose moderators only delete documents: no list, one document type they can
    /// delete from, one removal record. Kept apart from the contract with both lists, whose
    /// other tree is the fullest one and the one whose shape is recorded.
    fn contract_with_document_removals(run: &mut FixtureRun) {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/family/family-contract.json",
            Some([9; 32]),
            None,
            Some(|contract: &mut DataContract| {
                contract.set_config(contract.config().clone().with_moderation(Some(
                    ContractModerationConfig {
                        banlist: false,
                        suspensions: false,
                        moderators: ContractModerators::ContractOwner,
                        warnings: false,
                    },
                )));
                // After the config: the keyword is refused on a contract without moderation.
                contract
                    .set_document_schema(
                        "post",
                        platform_value!({
                            "type": "object",
                            "properties": {
                                "text": { "type": "string", "maxLength": 50, "position": 0 },
                            },
                            "additionalProperties": false,
                            "canBeDeletedByModerators": true,
                        }),
                        true,
                        &mut vec![],
                        PlatformVersion::latest(),
                    )
                    .expect("expected to add a document type moderators can delete from");
            }),
            None,
            Some(platform_version),
        );
        drive
            .apply_drive_operations(
                vec![DriveOperation::ContractModerationOperation(
                    ContractModerationOperationType::AddDocumentRemoval {
                        contract_id: contract.id(),
                        document_type_name: "post".to_string(),
                        document_id: Identifier::from([0x23; 32]),
                        removal: ContractDocumentRemoval {
                            document_owner_id: Identifier::from([0x24; 32]),
                            moderator_id: contract.owner_id(),
                            reason: ContractModerationReason::from_text("spam"),
                            removed_at: 1_000,
                            document_hash: [0x25; 32],
                            restoration: None,
                        },
                        replaces_existing: false,
                        moderator_id: contract.owner_id(),
                    },
                )],
                true,
                &BlockInfo::default(),
                None,
                platform_version,
                None,
            )
            .expect("expected to record a document removal");
        conformance_of(&drive, "contract_with_document_removals", run);
    }

    /// A contract that keeps both barring lists, with one ban and one suspension
    fn moderated_contract(run: &mut FixtureRun) {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/family/family-contract.json",
            Some([9; 32]),
            None,
            Some(|contract: &mut DataContract| {
                contract.set_config(contract.config().clone().with_moderation(Some(
                    ContractModerationConfig {
                        banlist: true,
                        suspensions: true,
                        moderators: ContractModerators::ContractOwner,
                        warnings: false,
                    },
                )))
            }),
            None,
            Some(platform_version),
        );
        drive
            .add_contract_ban(
                contract.id(),
                Identifier::from([0x21; 32]),
                &ContractModerationReason::from_text("spam"),
                contract.owner_id(),
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to ban");
        drive
            .add_contract_suspension(
                contract.id(),
                Identifier::from([0x22; 32]),
                1_000,
                &ContractModerationReason::from_text("flooding"),
                false,
                contract.owner_id(),
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to suspend");
        // Both fee pots hold credits and were claimed once: the pots and the last claims
        // are created on first use.
        let fee_pot_operations = [ContractFeePot::Owner, ContractFeePot::Moderators]
            .into_iter()
            .flat_map(|pot| {
                [
                    ContractFeePotOperationType::AddToPot {
                        contract_id: contract.id(),
                        pot,
                        amount: 1_000,
                    },
                    ContractFeePotOperationType::SetLastClaim {
                        contract_id: contract.id(),
                        pot,
                        last_claim: ContractFeePotLastClaim {
                            epoch_index: 3,
                            time_ms: 1_700_000_000_000,
                            claimant_id: contract.owner_id(),
                        },
                    },
                ]
            })
            .map(DriveOperation::ContractFeePotOperation)
            .collect();
        drive
            .apply_drive_operations(
                fee_pot_operations,
                true,
                &BlockInfo::default(),
                None,
                platform_version,
                None,
            )
            .expect("expected to fill and claim the fee pots");
        conformance_of(&drive, "moderated_contract", run);
    }

    /// A contract whose moderators are elected, keeping the banlist, with one member of its
    /// seated team's moderation action counted
    fn elected_contract(run: &mut FixtureRun) {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/family/family-contract.json",
            Some([11; 32]),
            None,
            Some(|contract: &mut DataContract| {
                contract.set_config(contract.config().clone().with_moderation(Some(
                    ContractModerationConfig {
                        banlist: true,
                        suspensions: false,
                        warnings: false,
                        moderators: ContractModerators::Elected(Box::new(ElectedModerators {
                            join_window: DEFAULT_ELECTION_WINDOW_SECONDS,
                            vote_window: DEFAULT_ELECTION_WINDOW_SECONDS,
                            challenge_cool_down: Some(1_209_600),
                            election_delay: None,
                            max_added_moderators: 0,
                            moderated_document_types: BTreeMap::from([(
                                "person".to_string(),
                                BTreeSet::from([ModerationAbility::Ban]),
                            )]),
                            interim: InterimModerators::NotYetUsable,
                            owner_protected: false,
                        })),
                    },
                )))
            }),
            None,
            Some(platform_version),
        );
        drive
            .apply_drive_operations(
                vec![DriveOperation::ContractModerationOperation(
                    ContractModerationOperationType::SetActionCount {
                        contract_id: contract.id(),
                        identity_id: Identifier::from([0x24; 32]),
                        count: 3,
                    },
                )],
                true,
                &BlockInfo::default(),
                None,
                platform_version,
                None,
            )
            .expect("expected to count a moderation action");
        conformance_of(&drive, "elected_contract", run);
    }

    /// A contract that keeps the banlist and the warning list, with one identity carrying two
    /// warnings
    fn warned_contract(run: &mut FixtureRun) {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/family/family-contract.json",
            Some([10; 32]),
            None,
            Some(|contract: &mut DataContract| {
                contract.set_config(contract.config().clone().with_moderation(Some(
                    ContractModerationConfig {
                        banlist: true,
                        suspensions: false,
                        warnings: true,
                        moderators: ContractModerators::ContractOwner,
                    },
                )))
            }),
            None,
            Some(platform_version),
        );
        drive
            .add_contract_warning(
                contract.id(),
                Identifier::from([0x23; 32]),
                &[
                    ContractWarning {
                        warned_at: 1_000,
                        reason: ContractModerationReason::from_text("first strike"),
                    },
                    ContractWarning {
                        warned_at: 2_000,
                        reason: ContractModerationReason::from_text("second strike"),
                    },
                ],
                false,
                contract.owner_id(),
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to warn");
        conformance_of(&drive, "warned_contract", run);
    }

    fn tokens_and_group_actions(run: &mut FixtureRun) {
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
        conformance_of(&drive, "tokens_and_group_actions", run);
    }

    fn address_balances(run: &mut FixtureRun) {
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
        conformance_of(&drive, "address_balances", run);
    }

    /// An epoch while it runs, then after it was paid out: payout deletes the
    /// proposers and both fee items and keeps the epoch tree. The finished
    /// epoch info is written at payout, so no epoch ever holds all nine keys.
    fn current_then_paid_epoch(run: &mut FixtureRun) {
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
        // The first block of an epoch starts it and pays its fees into the
        // pools in one batch, so a running epoch holds its processing fees
        // from the start. The shape of the layer depends on that.
        batch.push(
            epoch
                .update_processing_fee_pool_operation(1000)
                .expect("expected the processing fee operation"),
        );
        drive
            .grove_apply_batch(batch, false, None, &platform_version.drive)
            .expect("expected to start the epoch");
        conformance_of(&drive, "current_epoch", run);
        run.record_state(
            &drive,
            "pools.epoch",
            "future",
            Epoch::new(1)
                .expect("expected the next epoch")
                .get_path_vec(),
            "current_epoch",
        );
        run.record_state(
            &drive,
            "pools.epoch",
            "running",
            epoch.get_path_vec(),
            "current_epoch",
        );

        // Payout deletes and writes in one batch
        let mut batch = GroveDbOpBatch::new();
        epoch.add_mark_as_paid_operations(&mut batch);
        batch.push(
            drive
                .add_epoch_final_info_operation(
                    &epoch,
                    FinalizedEpochInfoV0 {
                        first_block_time: 3,
                        first_block_height: 2,
                        total_blocks_in_epoch: 3,
                        first_core_block_height: 5,
                        next_epoch_start_core_block_height: 6,
                        total_processing_fees: 1000,
                        total_distributed_storage_fees: 0,
                        total_created_storage_fees: 0,
                        core_block_rewards: 0,
                        block_proposers: BTreeMap::from([(Identifier::from([7; 32]), 3)]),
                        fee_multiplier_permille: 1000,
                        protocol_version: platform_version.protocol_version,
                    }
                    .into(),
                    platform_version,
                )
                .expect("expected the finished epoch info operation"),
        );
        drive
            .grove_apply_batch(batch, false, None, &platform_version.drive)
            .expect("expected to mark the epoch as paid");
        conformance_of(&drive, "paid_epoch", run);
        run.record_state(
            &drive,
            "pools.epoch",
            "paid",
            epoch.get_path_vec(),
            "paid_epoch",
        );
    }

    /// Two contests on the DPNS index of parent domain name and label. In the
    /// second the label is 32 bytes long: as long as the identity id of a
    /// contender, one level above where contenders are, so only what is below
    /// the key tells the two apart. (DPNS only contests shorter labels, but
    /// that rule lives in dpp, and any contested index over identifiers has
    /// 32 byte values.)
    fn contested_documents(run: &mut FixtureRun) {
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
        conformance_of(&drive, "contested_documents", run);
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

    /// A token with a perpetual distribution, a pre-programmed release and a
    /// once per identity rule, before and after each is claimed. The second release stays queued.
    fn token_distributions(run: &mut FixtureRun) {
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
            .set_perpetual_distribution(Some(TokenPerpetualDistribution::V0(
                TokenPerpetualDistributionV0 {
                    distribution_type: RewardDistributionType::BlockBasedDistribution {
                        interval: 10,
                        function: DistributionFunction::FixedAmount { amount: 50 },
                    },
                    distribution_recipient: TokenDistributionRecipient::ContractOwner,
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
        conformance_of(&drive, "token_distributions_unclaimed", run);

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
                .mark_perpetual_release_as_distributed_operations(
                    token_id,
                    recipient,
                    RewardDistributionMoment::BlockBasedMoment(20),
                    &mut None,
                    platform_version,
                )
                .expect("expected the perpetual claim operations"),
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
        conformance_of(&drive, "token_distributions_claimed", run);
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
    /// are bound to a contract that wants one key per purpose, to a contract
    /// that keeps a reference to the latest key of each purpose, to a document
    /// type of that contract, and to a contract group; one of its keys has a
    /// budget.
    fn contract_groups_and_bound_keys(run: &mut FixtureRun) {
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
            bound_key(
                13,
                Purpose::ENCRYPTION,
                ContractBounds::SingleContract {
                    id: contracts[1].id(),
                },
                None,
            ),
            bound_key(
                14,
                Purpose::DECRYPTION,
                ContractBounds::SingleContract {
                    id: contracts[1].id(),
                },
                None,
            ),
        ] {
            identity.add_public_key(key);
        }
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
        // A nonce beside the bound keys, so one entry holds both
        drive
            .merge_identity_contract_nonce(
                identity_id,
                contracts[0].id().to_buffer(),
                1,
                &BlockInfo::default(),
                true,
                None,
                &mut vec![],
                platform_version,
            )
            .expect("expected to set a contract nonce");
        run.record_state(
            &drive,
            "identities.identity",
            "budgeted_key_and_contract",
            vec![vec![RootTree::Identities as u8], identity_id.to_vec()],
            "contract_groups_and_bound_keys",
        );
        conformance_of(&drive, "contract_groups_and_bound_keys", run);
    }

    fn spent_nullifiers(run: &mut FixtureRun) {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        apply_operations(
            &drive,
            drive
                .insert_nullifiers(&[[3; 32], [4; 32]], platform_version)
                .expect("expected the nullifier operations"),
        );
        conformance_of(&drive, "spent_nullifiers", run);
    }

    /// Runs every fixture
    pub(super) fn run_all() -> FixtureRun {
        let mut run = FixtureRun::default();
        identities(&mut run);
        contracts_with_documents(&mut run);
        moderated_contract(&mut run);
        warned_contract(&mut run);
        elected_contract(&mut run);
        contract_with_document_removals(&mut run);
        tokens_and_group_actions(&mut run);
        address_balances(&mut run);
        current_then_paid_epoch(&mut run);
        contested_documents(&mut run);
        token_distributions(&mut run);
        contract_groups_and_bound_keys(&mut run);
        spent_nullifiers(&mut run);
        run
    }

    #[test]
    fn should_match_populated_state_and_reach_every_described_node() {
        let visited = run_all().visited;

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
