//! Contract groups through the data contract create transition (protocol version 14).

use crate::execution::validation::state_transition::state_transitions::tests::setup_identity;
use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
use crate::rpc::core::MockCoreRPCLike;
use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
use assert_matches::assert_matches;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::basic::BasicError;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::ConsensusError;
use dpp::contract_group::{
    generate_contract_group_id, ContractGroupMember, ContractGroupMembership, ContractGroupOwner,
    ContractGroupRegistration,
};
use dpp::dash_to_credits;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::DataContract;
use dpp::identifier::Identifier;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{Identity, IdentityPublicKey};
use dpp::prelude::IdentityNonce;
use dpp::serialization::PlatformSerializable;
use dpp::state_transition::data_contract_create_transition::methods::DataContractCreateTransitionMethodsV1;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::tests::fixtures::get_data_contract_fixture;
use dpp::tests::json_document::json_document_to_contract_with_ids;
use drive::drive::contract_groups::types::{
    ContractGroupMembersPage, ContractGroupMembersQuery, ContractGroupMembershipsForContract,
};
use drive::grovedb::{Transaction, TransactionArg};
use platform_version::version::PlatformVersion;
use simple_signer::signer::SimpleSigner;
use std::collections::{BTreeMap, BTreeSet};

/// The document types the contract fixture declares; used as document type members.
const FIXTURE_DOCUMENT_TYPE: &str = "niceDocument";

/// The page size used to read a group's members in these tests; every group here is smaller.
const PAGE_LIMIT: u16 = 16;

fn members_page(
    platform: &TempPlatform<MockCoreRPCLike>,
    contract_group_id: Identifier,
    query: ContractGroupMembersQuery,
    transaction: TransactionArg,
) -> ContractGroupMembersPage {
    platform
        .drive
        .fetch_contract_group_members(
            contract_group_id,
            &query,
            PAGE_LIMIT,
            transaction,
            PlatformVersion::latest(),
        )
        .expect("expected to fetch the members page")
}

fn membership(
    contract_group_id: Identifier,
    member: ContractGroupMember,
) -> ContractGroupMembership {
    ContractGroupMembership {
        contract_group_id,
        member,
    }
}

fn single_owner(owner_id: Identifier, name: &str) -> ContractGroupRegistration {
    ContractGroupRegistration {
        owner: ContractGroupOwner::SingleOwner(owner_id),
        name: Some(name.to_string()),
        description: None,
    }
}

fn fixture_contract(owner_id: Identifier, nonce: IdentityNonce) -> DataContract {
    let platform_version = PlatformVersion::latest();
    get_data_contract_fixture(Some(owner_id), nonce, platform_version.protocol_version)
        .data_contract_owned()
}

fn token_contract() -> DataContract {
    json_document_to_contract_with_ids(
        "tests/supporting_files/contract/basic-token/basic-token.json",
        None,
        None,
        false,
        PlatformVersion::latest(),
    )
    .expect("expected the basic token contract")
}

#[allow(clippy::too_many_arguments)]
async fn create_transition_bytes(
    identity: &Identity,
    signer: &SimpleSigner,
    key: &IdentityPublicKey,
    data_contract: DataContract,
    nonce: IdentityNonce,
    contract_group: Option<ContractGroupRegistration>,
    memberships: Vec<ContractGroupMembership>,
) -> Vec<u8> {
    let platform_version = PlatformVersion::latest();
    DataContractCreateTransition::new_from_data_contract_with_contract_group(
        data_contract,
        nonce,
        contract_group,
        memberships,
        &identity.clone().into_partial_identity_info(),
        key.id(),
        signer,
        platform_version,
        None,
    )
    .await
    .expect("expected to build the create transition")
    .serialize_to_bytes()
    .expect("expected to serialize the create transition")
}

fn process(
    platform: &TempPlatform<MockCoreRPCLike>,
    transition_bytes: Vec<u8>,
    transaction: &Transaction,
) -> StateTransitionExecutionResult {
    let platform_version = PlatformVersion::latest();
    let platform_state = platform.state.load();
    let processing_result = platform
        .platform
        .process_raw_state_transitions(
            &[transition_bytes],
            &platform_state,
            &BlockInfo::default(),
            transaction,
            platform_version,
            false,
            None,
        )
        .expect("expected to process the state transition");
    let mut results = processing_result.into_execution_results();
    assert_eq!(results.len(), 1, "one transition, one result");
    results.remove(0)
}

fn identity_nonce(
    platform: &TempPlatform<MockCoreRPCLike>,
    identity_id: Identifier,
    transaction: &Transaction,
) -> IdentityNonce {
    platform
        .drive
        .fetch_identity_nonce(
            identity_id.to_buffer(),
            true,
            Some(transaction),
            PlatformVersion::latest(),
        )
        .expect("expected to fetch the identity nonce")
        .unwrap_or_default()
}

#[tokio::test]
async fn should_register_a_contract_group_and_join_it_in_the_same_create() {
    let platform_version = PlatformVersion::latest();
    let mut platform = TestPlatformBuilder::new()
        .build_with_mock_rpc()
        .set_genesis_state();
    let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(2.0));
    let owner_id = identity.id();
    let contract_group_id = generate_contract_group_id(&owner_id, 1);
    let contract = fixture_contract(owner_id, 1);
    let contract_id = DataContract::generate_data_contract_id_v0(owner_id, 1);

    let bytes = create_transition_bytes(
        &identity,
        &signer,
        &key,
        contract,
        1,
        Some(single_owner(owner_id, "dashpay")),
        vec![membership(contract_group_id, ContractGroupMember::Contract)],
    )
    .await;

    let transaction = platform.drive.grove.start_transaction();
    assert_matches!(
        process(&platform, bytes, &transaction),
        StateTransitionExecutionResult::SuccessfulExecution { .. }
    );

    let info = platform
        .drive
        .fetch_contract_group_info(contract_group_id, Some(&transaction), platform_version)
        .expect("expected to fetch the group info")
        .expect("expected the group to be registered");
    assert_eq!(info.owner(), &ContractGroupOwner::SingleOwner(owner_id));
    assert_eq!(info.name(), Some("dashpay"));
    assert_eq!(
        members_page(
            &platform,
            contract_group_id,
            ContractGroupMembersQuery::Contracts { start_after: None },
            Some(&transaction),
        ),
        ContractGroupMembersPage::Contracts(vec![contract_id])
    );
    for query in [
        ContractGroupMembersQuery::DocumentTypes { start_after: None },
        ContractGroupMembersQuery::Tokens { start_after: None },
    ] {
        assert!(members_page(&platform, contract_group_id, query, Some(&transaction)).is_empty());
    }
    let memberships = platform
        .drive
        .fetch_contract_group_memberships_for_contract(
            contract_id,
            Some(&transaction),
            platform_version,
        )
        .expect("expected to fetch the memberships");
    assert_eq!(memberships.contract, BTreeSet::from([contract_group_id]));

    // The contract itself was created as usual.
    assert!(platform
        .drive
        .fetch_contract(
            contract_id.to_buffer(),
            None,
            None,
            Some(&transaction),
            platform_version
        )
        .value
        .expect("expected to fetch the contract")
        .is_some());
}

#[tokio::test]
async fn should_let_an_owner_add_a_later_contract_by_document_type_and_token() {
    let platform_version = PlatformVersion::latest();
    let mut platform = TestPlatformBuilder::new()
        .build_with_mock_rpc()
        .set_genesis_state();
    let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(2.0));
    let owner_id = identity.id();
    let contract_group_id = generate_contract_group_id(&owner_id, 1);

    let transaction = platform.drive.grove.start_transaction();

    // Register an empty group with the first contract.
    let bytes = create_transition_bytes(
        &identity,
        &signer,
        &key,
        fixture_contract(owner_id, 1),
        1,
        Some(single_owner(owner_id, "suite")),
        vec![],
    )
    .await;
    assert_matches!(
        process(&platform, bytes, &transaction),
        StateTransitionExecutionResult::SuccessfulExecution { .. }
    );

    // A second contract joins by document type.
    let documents_contract_id = DataContract::generate_data_contract_id_v0(owner_id, 2);
    let bytes = create_transition_bytes(
        &identity,
        &signer,
        &key,
        fixture_contract(owner_id, 2),
        2,
        None,
        vec![membership(
            contract_group_id,
            ContractGroupMember::DocumentType(FIXTURE_DOCUMENT_TYPE.to_string()),
        )],
    )
    .await;
    assert_matches!(
        process(&platform, bytes, &transaction),
        StateTransitionExecutionResult::SuccessfulExecution { .. }
    );

    // A third contract joins by token.
    let token_contract_id = DataContract::generate_data_contract_id_v0(owner_id, 3);
    let bytes = create_transition_bytes(
        &identity,
        &signer,
        &key,
        token_contract(),
        3,
        None,
        vec![membership(contract_group_id, ContractGroupMember::Token(0))],
    )
    .await;
    assert_matches!(
        process(&platform, bytes, &transaction),
        StateTransitionExecutionResult::SuccessfulExecution { .. }
    );

    let document_types_query = ContractGroupMembersQuery::DocumentTypes { start_after: None };
    let document_types_page = members_page(
        &platform,
        contract_group_id,
        document_types_query.clone(),
        Some(&transaction),
    );
    assert_eq!(
        document_types_page,
        ContractGroupMembersPage::DocumentTypes(vec![(
            documents_contract_id,
            FIXTURE_DOCUMENT_TYPE.to_string()
        )])
    );
    assert_eq!(
        members_page(
            &platform,
            contract_group_id,
            ContractGroupMembersQuery::Tokens { start_after: None },
            Some(&transaction),
        ),
        ContractGroupMembersPage::Tokens(vec![(token_contract_id, 0)])
    );
    assert!(members_page(
        &platform,
        contract_group_id,
        ContractGroupMembersQuery::Contracts { start_after: None },
        Some(&transaction),
    )
    .is_empty());
    assert_eq!(
        platform
            .drive
            .fetch_contract_group_memberships_for_contract(
                token_contract_id,
                Some(&transaction),
                platform_version,
            )
            .expect("expected to fetch the memberships"),
        ContractGroupMembershipsForContract {
            contract: BTreeSet::new(),
            document_types: BTreeMap::new(),
            tokens: BTreeMap::from([(0, BTreeSet::from([contract_group_id]))]),
        }
    );

    // Proofs are built from committed state and verify against its root.
    platform
        .drive
        .grove
        .commit_transaction(transaction)
        .unwrap()
        .expect("expected to commit the transaction");
    let proof = platform
        .drive
        .prove_contract_group_members(
            contract_group_id,
            &document_types_query,
            PAGE_LIMIT,
            None,
            platform_version,
        )
        .expect("expected a proof");
    let (proved_root, proved) = drive::drive::Drive::verify_contract_group_members(
        &proof,
        contract_group_id,
        &document_types_query,
        PAGE_LIMIT,
        platform_version,
    )
    .expect("expected the proof to verify");
    assert_eq!(proved, document_types_page);
    assert_eq!(
        proved_root,
        platform
            .drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("expected a root hash")
    );
}

#[tokio::test]
async fn should_reject_joining_a_group_the_identity_does_not_own_as_a_paid_failure() {
    let mut platform = TestPlatformBuilder::new()
        .build_with_mock_rpc()
        .set_genesis_state();
    let (alice, alice_signer, alice_key) =
        setup_identity(&mut platform, 958, dash_to_credits!(2.0));
    let (bob, bob_signer, bob_key) = setup_identity(&mut platform, 1234, dash_to_credits!(2.0));
    let contract_group_id = generate_contract_group_id(&alice.id(), 1);

    let transaction = platform.drive.grove.start_transaction();
    let bytes = create_transition_bytes(
        &alice,
        &alice_signer,
        &alice_key,
        fixture_contract(alice.id(), 1),
        1,
        Some(single_owner(alice.id(), "alice")),
        vec![],
    )
    .await;
    assert_matches!(
        process(&platform, bytes, &transaction),
        StateTransitionExecutionResult::SuccessfulExecution { .. }
    );

    let nonce_before = identity_nonce(&platform, bob.id(), &transaction);
    let bytes = create_transition_bytes(
        &bob,
        &bob_signer,
        &bob_key,
        fixture_contract(bob.id(), 1),
        1,
        None,
        vec![membership(contract_group_id, ContractGroupMember::Contract)],
    )
    .await;
    assert_matches!(
        process(&platform, bytes, &transaction),
        StateTransitionExecutionResult::PaidConsensusError {
            error: ConsensusError::StateError(StateError::IdentityNotContractGroupOwnerOrAdminError(e)),
            ..
        } if *e.identity_id() == bob.id() && *e.contract_group_id() == contract_group_id
    );
    assert!(
        identity_nonce(&platform, bob.id(), &transaction) > nonce_before,
        "a paid failure bumps the identity nonce"
    );

    // Bob's contract was not created and the group is untouched.
    let bob_contract_id = DataContract::generate_data_contract_id_v0(bob.id(), 1);
    assert!(platform
        .drive
        .fetch_contract(
            bob_contract_id.to_buffer(),
            None,
            None,
            Some(&transaction),
            PlatformVersion::latest()
        )
        .value
        .expect("expected to query the contract")
        .is_none());
    for query in [
        ContractGroupMembersQuery::Contracts { start_after: None },
        ContractGroupMembersQuery::DocumentTypes { start_after: None },
        ContractGroupMembersQuery::Tokens { start_after: None },
    ] {
        assert!(members_page(&platform, contract_group_id, query, Some(&transaction)).is_empty());
    }
}

#[tokio::test]
async fn should_reject_joining_an_unknown_group() {
    let mut platform = TestPlatformBuilder::new()
        .build_with_mock_rpc()
        .set_genesis_state();
    let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(2.0));
    // A group id this identity would derive from a nonce it never used for a registration.
    let unknown_group_id = generate_contract_group_id(&identity.id(), 77);

    let transaction = platform.drive.grove.start_transaction();
    let bytes = create_transition_bytes(
        &identity,
        &signer,
        &key,
        fixture_contract(identity.id(), 1),
        1,
        None,
        vec![membership(unknown_group_id, ContractGroupMember::Contract)],
    )
    .await;
    assert_matches!(
        process(&platform, bytes, &transaction),
        StateTransitionExecutionResult::PaidConsensusError {
            error: ConsensusError::StateError(StateError::ContractGroupNotFoundError(e)),
            ..
        } if *e.contract_group_id() == unknown_group_id
    );
}

#[tokio::test]
async fn should_let_the_owner_and_each_admin_add_members_and_refuse_outsiders() {
    let platform_version = PlatformVersion::latest();
    let mut platform = TestPlatformBuilder::new()
        .build_with_mock_rpc()
        .set_genesis_state();
    let (alice, alice_signer, alice_key) =
        setup_identity(&mut platform, 958, dash_to_credits!(2.0));
    let (bob, bob_signer, bob_key) = setup_identity(&mut platform, 1234, dash_to_credits!(2.0));
    let (carol, carol_signer, carol_key) =
        setup_identity(&mut platform, 4321, dash_to_credits!(2.0));
    let contract_group_id = generate_contract_group_id(&alice.id(), 1);

    let transaction = platform.drive.grove.start_transaction();
    let bytes = create_transition_bytes(
        &alice,
        &alice_signer,
        &alice_key,
        fixture_contract(alice.id(), 1),
        1,
        Some(ContractGroupRegistration {
            owner: ContractGroupOwner::OwnerAndAdmins {
                owner: alice.id(),
                admins: BTreeSet::from([bob.id()]),
            },
            name: None,
            description: Some("owned by alice, administered by bob".to_string()),
        }),
        vec![],
    )
    .await;
    assert_matches!(
        process(&platform, bytes, &transaction),
        StateTransitionExecutionResult::SuccessfulExecution { .. }
    );

    // Bob is an admin and joins alone.
    let bob_contract_id = DataContract::generate_data_contract_id_v0(bob.id(), 1);
    let bytes = create_transition_bytes(
        &bob,
        &bob_signer,
        &bob_key,
        fixture_contract(bob.id(), 1),
        1,
        None,
        vec![membership(contract_group_id, ContractGroupMember::Contract)],
    )
    .await;
    assert_matches!(
        process(&platform, bytes, &transaction),
        StateTransitionExecutionResult::SuccessfulExecution { .. }
    );

    // Carol is neither the owner nor an admin.
    let bytes = create_transition_bytes(
        &carol,
        &carol_signer,
        &carol_key,
        fixture_contract(carol.id(), 1),
        1,
        None,
        vec![membership(contract_group_id, ContractGroupMember::Contract)],
    )
    .await;
    assert_matches!(
        process(&platform, bytes, &transaction),
        StateTransitionExecutionResult::PaidConsensusError {
            error: ConsensusError::StateError(
                StateError::IdentityNotContractGroupOwnerOrAdminError(_)
            ),
            ..
        }
    );

    assert_eq!(
        members_page(
            &platform,
            contract_group_id,
            ContractGroupMembersQuery::Contracts { start_after: None },
            Some(&transaction),
        ),
        ContractGroupMembersPage::Contracts(vec![bob_contract_id])
    );
    let info = platform
        .drive
        .fetch_contract_group_info(contract_group_id, Some(&transaction), platform_version)
        .expect("expected to fetch the group info")
        .expect("expected the group");
    assert_eq!(
        info.description(),
        Some("owned by alice, administered by bob")
    );
}

#[tokio::test]
async fn should_reject_a_registrant_who_is_not_the_owner_before_paying() {
    let mut platform = TestPlatformBuilder::new()
        .build_with_mock_rpc()
        .set_genesis_state();
    let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(2.0));
    let stranger = Identifier::from([9u8; 32]);

    let transaction = platform.drive.grove.start_transaction();
    let nonce_before = identity_nonce(&platform, identity.id(), &transaction);

    // Being an admin is not enough: the registrant must be the owner.
    for owner in [
        ContractGroupOwner::SingleOwner(stranger),
        ContractGroupOwner::OwnerAndAdmins {
            owner: stranger,
            admins: BTreeSet::from([identity.id()]),
        },
        ContractGroupOwner::OwnerAndAdmins {
            owner: stranger,
            admins: BTreeSet::from([Identifier::from([8u8; 32])]),
        },
    ] {
        let bytes = create_transition_bytes(
            &identity,
            &signer,
            &key,
            fixture_contract(identity.id(), 1),
            1,
            Some(ContractGroupRegistration {
                owner,
                name: None,
                description: None,
            }),
            vec![],
        )
        .await;
        assert_matches!(
            process(&platform, bytes, &transaction),
            StateTransitionExecutionResult::UnpaidConsensusError(ConsensusError::BasicError(
                BasicError::ContractGroupRegistrantNotOwnerError(_)
            ))
        );
    }
    assert_eq!(
        identity_nonce(&platform, identity.id(), &transaction),
        nonce_before,
        "structure failures are not paid"
    );
}

#[tokio::test]
async fn should_reject_malformed_registrations_and_memberships_in_basic_structure() {
    let platform_version = PlatformVersion::latest();
    let mut platform = TestPlatformBuilder::new()
        .build_with_mock_rpc()
        .set_genesis_state();
    let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(2.0));
    let owner_id = identity.id();
    let group_id = generate_contract_group_id(&owner_id, 1);
    let limits = &platform_version.system_limits;

    let transaction = platform.drive.grove.start_transaction();

    let registration_cases: Vec<(ContractGroupRegistration, fn(&BasicError) -> bool)> = vec![
        (
            ContractGroupRegistration {
                owner: ContractGroupOwner::OwnerAndAdmins {
                    owner: owner_id,
                    admins: BTreeSet::new(),
                },
                name: None,
                description: None,
            },
            |e| matches!(e, BasicError::InvalidContractGroupAdminsError(_)),
        ),
        (
            ContractGroupRegistration {
                owner: ContractGroupOwner::OwnerAndAdmins {
                    owner: owner_id,
                    admins: BTreeSet::from([owner_id]),
                },
                name: None,
                description: None,
            },
            |e| matches!(e, BasicError::InvalidContractGroupAdminsError(_)),
        ),
        (
            ContractGroupRegistration {
                owner: ContractGroupOwner::OwnerAndAdmins {
                    owner: owner_id,
                    admins: (0..=limits.max_contract_group_admins)
                        .map(|i| Identifier::from([i as u8 + 1; 32]))
                        .collect(),
                },
                name: None,
                description: None,
            },
            |e| matches!(e, BasicError::InvalidContractGroupAdminsError(_)),
        ),
        (
            ContractGroupRegistration {
                owner: ContractGroupOwner::SingleOwner(owner_id),
                name: Some(String::new()),
                description: None,
            },
            |e| matches!(e, BasicError::InvalidContractGroupNameLengthError(_)),
        ),
        (
            ContractGroupRegistration {
                owner: ContractGroupOwner::SingleOwner(owner_id),
                name: Some("n".repeat(limits.max_contract_group_name_length as usize + 1)),
                description: None,
            },
            |e| matches!(e, BasicError::InvalidContractGroupNameLengthError(_)),
        ),
        (
            ContractGroupRegistration {
                owner: ContractGroupOwner::SingleOwner(owner_id),
                name: None,
                description: Some(
                    "d".repeat(limits.max_contract_group_description_length as usize + 1),
                ),
            },
            |e| matches!(e, BasicError::InvalidContractGroupDescriptionLengthError(_)),
        ),
    ];
    for (registration, is_expected) in registration_cases {
        let bytes = create_transition_bytes(
            &identity,
            &signer,
            &key,
            fixture_contract(owner_id, 1),
            1,
            Some(registration.clone()),
            vec![],
        )
        .await;
        match process(&platform, bytes, &transaction) {
            StateTransitionExecutionResult::UnpaidConsensusError(ConsensusError::BasicError(
                error,
            )) if is_expected(&error) => {}
            other => panic!("registration {:?} produced {:?}", registration, other),
        }
    }

    let membership_cases: Vec<(Vec<ContractGroupMembership>, fn(&BasicError) -> bool)> = vec![
        (
            vec![
                membership(group_id, ContractGroupMember::Contract),
                membership(group_id, ContractGroupMember::Contract),
            ],
            |e| matches!(e, BasicError::DuplicateContractGroupMembershipError(_)),
        ),
        (
            vec![
                membership(group_id, ContractGroupMember::Contract),
                membership(
                    group_id,
                    ContractGroupMember::DocumentType(FIXTURE_DOCUMENT_TYPE.to_string()),
                ),
            ],
            |e| matches!(e, BasicError::RedundantContractGroupMembershipError(_)),
        ),
        (
            vec![membership(
                group_id,
                ContractGroupMember::DocumentType("missing".to_string()),
            )],
            |e| matches!(e, BasicError::ContractGroupMemberNotInContractError(_)),
        ),
        (
            vec![membership(group_id, ContractGroupMember::Token(0))],
            |e| matches!(e, BasicError::ContractGroupMemberNotInContractError(_)),
        ),
        (
            (0..=limits.max_contract_group_memberships_per_contract)
                .map(|i| {
                    membership(
                        generate_contract_group_id(&owner_id, 100 + i as u64),
                        ContractGroupMember::Contract,
                    )
                })
                .collect(),
            |e| matches!(e, BasicError::ContractGroupMembershipsOverLimitError(_)),
        ),
    ];
    for (memberships, is_expected) in membership_cases {
        let bytes = create_transition_bytes(
            &identity,
            &signer,
            &key,
            fixture_contract(owner_id, 1),
            1,
            Some(single_owner(owner_id, "checks")),
            memberships.clone(),
        )
        .await;
        match process(&platform, bytes, &transaction) {
            StateTransitionExecutionResult::UnpaidConsensusError(ConsensusError::BasicError(
                error,
            )) if is_expected(&error) => {}
            other => panic!("memberships {:?} produced {:?}", memberships, other),
        }
    }
}
