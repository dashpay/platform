use crate::drive::contract_groups::paths::{
    contract_groups_root_path, CONTRACT_GROUPS_GROUPS_KEY, CONTRACT_GROUPS_MEMBERS_KEY,
};
use crate::drive::contract_groups::types::{
    ContractGroupMembersPage, ContractGroupMembersQuery, ContractGroupMembershipsForContract,
};
use crate::drive::identity::key::fetch::{
    IdentityKeysRequest, KeyIDIdentityPublicKeyPairVec, KeyKindRequestType, KeyRequestType,
};
use crate::drive::{Drive, RootTree};
use crate::error::identity::IdentityError;
use crate::error::Error;
use crate::util::grove_operations::DirectQueryType;
use crate::util::test_helpers::setup::{setup_drive, setup_drive_with_initial_state_structure};
use dpp::block::block_info::BlockInfo;
use dpp::contract_group::{
    generate_contract_group_id, ContractGroupInfo, ContractGroupMember, ContractGroupMembership,
    ContractGroupRegistration,
};
use dpp::identifier::Identifier;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::contract_bounds::ContractBounds;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::{Identity, IdentityPublicKey, KeyID, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::BinaryData;
use dpp::version::PlatformVersion;
use grovedb_path::SubtreePath;
use rand::rngs::StdRng;
use rand::SeedableRng;
use std::collections::{BTreeMap, BTreeSet, HashMap};

fn identity(seed: u8) -> Identifier {
    Identifier::from([seed; 32])
}

fn single_owner_info(owner: Identifier, name: &str) -> ContractGroupInfo {
    (
        owner,
        ContractGroupRegistration {
            admins: BTreeSet::new(),
            name: Some(name.to_string()),
            description: Some(format!("{} group", name)),
        },
    )
        .into()
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

fn root_hash(drive: &Drive, platform_version: &PlatformVersion) -> [u8; 32] {
    drive
        .grove
        .root_hash(None, &platform_version.drive.grove_version)
        .unwrap()
        .expect("expected a root hash")
}

/// Fetches, proves and verifies one members page and checks all three agree with `expected`.
fn assert_members_page(
    drive: &Drive,
    contract_group_id: Identifier,
    query: &ContractGroupMembersQuery,
    limit: u16,
    expected: &ContractGroupMembersPage,
    platform_version: &PlatformVersion,
) {
    let fetched = drive
        .fetch_contract_group_members(contract_group_id, query, limit, None, platform_version)
        .expect("expected to fetch the members page");
    assert_eq!(
        &fetched, expected,
        "fetched {:?} of group {}",
        query, contract_group_id
    );

    let proof = drive
        .prove_contract_group_members(contract_group_id, query, limit, None, platform_version)
        .expect("expected a members page proof");
    let (proved_root, proved) = Drive::verify_contract_group_members(
        &proof,
        contract_group_id,
        query,
        limit,
        platform_version,
    )
    .expect("expected to verify the members page proof");
    assert_eq!(proved_root, root_hash(drive, platform_version));
    assert_eq!(
        &proved, expected,
        "proved {:?} of group {}",
        query, contract_group_id
    );
}

fn has_root_tree_key(drive: &Drive, key: &[u8], platform_version: &PlatformVersion) -> bool {
    drive
        .grove_has_raw(
            SubtreePath::empty(),
            key,
            DirectQueryType::StatefulDirectQuery,
            None,
            &mut vec![],
            &platform_version.drive,
        )
        .expect("expected to query the root tree")
}

#[test]
fn should_create_the_contract_groups_root_tree_in_the_initial_structure() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));

    assert!(has_root_tree_key(
        &drive,
        &[RootTree::ContractGroups as u8],
        platform_version
    ));
    for subtree_key in [CONTRACT_GROUPS_GROUPS_KEY, CONTRACT_GROUPS_MEMBERS_KEY] {
        assert!(drive
            .grove_has_raw(
                (&contract_groups_root_path()).into(),
                subtree_key,
                DirectQueryType::StatefulDirectQuery,
                None,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("expected to query the contract groups tree"));
    }
}

#[test]
fn should_not_create_the_contract_groups_root_tree_before_protocol_version_14() {
    let platform_version = PlatformVersion::get(13).expect("expected protocol version 13");
    let drive = setup_drive(None);
    drive
        .create_initial_state_structure(None, platform_version)
        .expect("expected to create the protocol version 13 structure");

    assert!(!has_root_tree_key(
        &drive,
        &[RootTree::ContractGroups as u8],
        platform_version
    ));
}

#[test]
fn should_register_a_contract_group_and_prove_it_present_or_absent() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let owner = identity(1);
    let contract_group_id = generate_contract_group_id(&owner, 7);
    let info = single_owner_info(owner, "dashpay");
    let contracts_query = ContractGroupMembersQuery::Contracts { start_after: None };

    // Absent before registration, on the fetch and on the proof side.
    assert!(drive
        .fetch_contract_group_info(contract_group_id, None, platform_version)
        .expect("expected to fetch")
        .is_none());
    let absence_proof = drive
        .prove_contract_group_info(contract_group_id, None, platform_version)
        .expect("expected an absence proof");
    let (proved_root, absent) =
        Drive::verify_contract_group_info(&absence_proof, contract_group_id, platform_version)
            .expect("expected to verify the absence proof");
    assert_eq!(proved_root, root_hash(&drive, platform_version));
    assert!(absent.is_none());
    // The members of an absent group read and prove as an empty page.
    assert_members_page(
        &drive,
        contract_group_id,
        &contracts_query,
        10,
        &ContractGroupMembersPage::Contracts(vec![]),
        platform_version,
    );

    let fee = drive
        .insert_contract_group(
            contract_group_id,
            &info,
            &BlockInfo::default(),
            true,
            None,
            platform_version,
        )
        .expect("expected to register the group");
    assert!(fee.storage_fee > 0, "registration writes storage");

    assert_eq!(
        drive
            .fetch_contract_group_info(contract_group_id, None, platform_version)
            .expect("expected to fetch the info"),
        Some(info.clone())
    );
    let proof = drive
        .prove_contract_group_info(contract_group_id, None, platform_version)
        .expect("expected a proof");
    let (proved_root, proved_info) =
        Drive::verify_contract_group_info(&proof, contract_group_id, platform_version)
            .expect("expected to verify the proof");
    assert_eq!(proved_root, root_hash(&drive, platform_version));
    assert_eq!(proved_info, Some(info));

    // A registered group without members has an empty page of every kind.
    for query in [
        contracts_query,
        ContractGroupMembersQuery::DocumentTypes { start_after: None },
        ContractGroupMembersQuery::Tokens { start_after: None },
    ] {
        assert_members_page(
            &drive,
            contract_group_id,
            &query,
            10,
            &ContractGroupMembersPage::empty_for(&query),
            platform_version,
        );
    }
}

#[test]
fn should_record_memberships_on_both_sides_and_prove_them() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let owner = identity(1);
    let group_1 = generate_contract_group_id(&owner, 1);
    let group_2 = generate_contract_group_id(&owner, 2);
    for (group_id, name) in [(group_1, "first"), (group_2, "second")] {
        drive
            .insert_contract_group(
                group_id,
                &single_owner_info(owner, name),
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to register the group");
    }

    let contract_1 = identity(10);
    let contract_2 = identity(11);
    let memberships_1 = vec![
        membership(group_1, ContractGroupMember::Contract),
        membership(
            group_2,
            ContractGroupMember::DocumentType("profile".to_string()),
        ),
        membership(
            group_2,
            ContractGroupMember::DocumentType("contactRequest".to_string()),
        ),
        membership(group_2, ContractGroupMember::Token(0)),
        membership(group_1, ContractGroupMember::Token(1)),
    ];
    let fee = drive
        .insert_contract_group_memberships(
            contract_1,
            &memberships_1,
            &BlockInfo::default(),
            true,
            None,
            platform_version,
        )
        .expect("expected to record the memberships");
    assert!(fee.storage_fee > 0, "memberships write storage");

    drive
        .insert_contract_group_memberships(
            contract_2,
            &[membership(group_1, ContractGroupMember::Contract)],
            &BlockInfo::default(),
            true,
            None,
            platform_version,
        )
        .expect("expected to record the second contract's membership");

    // Forward side, one kind at a time.
    let contracts = ContractGroupMembersQuery::Contracts { start_after: None };
    let document_types = ContractGroupMembersQuery::DocumentTypes { start_after: None };
    let tokens = ContractGroupMembersQuery::Tokens { start_after: None };
    for (group_id, query, expected) in [
        (
            group_1,
            &contracts,
            ContractGroupMembersPage::Contracts(vec![contract_1, contract_2]),
        ),
        (
            group_1,
            &document_types,
            ContractGroupMembersPage::DocumentTypes(vec![]),
        ),
        (
            group_1,
            &tokens,
            ContractGroupMembersPage::Tokens(vec![(contract_1, 1)]),
        ),
        (
            group_2,
            &contracts,
            ContractGroupMembersPage::Contracts(vec![]),
        ),
        (
            group_2,
            &document_types,
            ContractGroupMembersPage::DocumentTypes(vec![
                (contract_1, "contactRequest".to_string()),
                (contract_1, "profile".to_string()),
            ]),
        ),
        (
            group_2,
            &tokens,
            ContractGroupMembersPage::Tokens(vec![(contract_1, 0)]),
        ),
    ] {
        assert_members_page(&drive, group_id, query, 10, &expected, platform_version);
    }

    // Backwards side.
    let expected_memberships_1 = ContractGroupMembershipsForContract {
        contract: BTreeSet::from([group_1]),
        document_types: BTreeMap::from([
            ("contactRequest".to_string(), BTreeSet::from([group_2])),
            ("profile".to_string(), BTreeSet::from([group_2])),
        ]),
        tokens: BTreeMap::from([
            (0, BTreeSet::from([group_2])),
            (1, BTreeSet::from([group_1])),
        ]),
    };
    let expected_memberships_2 = ContractGroupMembershipsForContract {
        contract: BTreeSet::from([group_1]),
        document_types: BTreeMap::new(),
        tokens: BTreeMap::new(),
    };
    for (contract_id, expected) in [
        (contract_1, &expected_memberships_1),
        (contract_2, &expected_memberships_2),
        (
            identity(99),
            &ContractGroupMembershipsForContract::default(),
        ),
    ] {
        let fetched = drive
            .fetch_contract_group_memberships_for_contract(contract_id, None, platform_version)
            .expect("expected to fetch memberships");
        assert_eq!(
            &fetched, expected,
            "memberships of contract {}",
            contract_id
        );

        let proof = drive
            .prove_contract_group_memberships_for_contract(contract_id, None, platform_version)
            .expect("expected a proof");
        let (proved_root, proved) = Drive::verify_contract_group_memberships_for_contract(
            &proof,
            contract_id,
            platform_version,
        )
        .expect("expected to verify");
        assert_eq!(proved_root, root_hash(&drive, platform_version));
        assert_eq!(&proved, expected);
    }
    assert_eq!(
        expected_memberships_1.all_contract_group_ids(),
        BTreeSet::from([group_1, group_2])
    );
}

#[test]
fn should_build_the_same_operations_for_estimation_and_for_apply() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let owner = identity(1);
    let group_id = generate_contract_group_id(&owner, 3);
    let info = single_owner_info(owner, "estimated");
    let contract_id = identity(20);
    let memberships = vec![
        membership(
            group_id,
            ContractGroupMember::DocumentType("note".to_string()),
        ),
        membership(group_id, ContractGroupMember::Token(2)),
    ];

    let estimated_registration = drive
        .insert_contract_group_operations(
            group_id,
            &info,
            &mut Some(HashMap::new()),
            None,
            platform_version,
        )
        .expect("expected estimated registration operations");
    let applied_registration = drive
        .insert_contract_group_operations(group_id, &info, &mut None, None, platform_version)
        .expect("expected registration operations");
    assert_eq!(
        estimated_registration.len(),
        applied_registration.len(),
        "registration estimation must build every operation the apply path builds"
    );

    let estimated_memberships = drive
        .insert_contract_group_memberships_operations(
            contract_id,
            &memberships,
            &mut Some(HashMap::new()),
            None,
            platform_version,
        )
        .expect("expected estimated membership operations");
    let applied_memberships = drive
        .insert_contract_group_memberships_operations(
            contract_id,
            &memberships,
            &mut None,
            None,
            platform_version,
        )
        .expect("expected membership operations");
    assert_eq!(
        estimated_memberships.len(),
        applied_memberships.len(),
        "membership estimation must build every operation the apply path builds"
    );

    // The estimated fees are at least the applied fees.
    let estimated_fee = drive
        .insert_contract_group(
            group_id,
            &info,
            &BlockInfo::default(),
            false,
            None,
            platform_version,
        )
        .expect("expected an estimate");
    let applied_fee = drive
        .insert_contract_group(
            group_id,
            &info,
            &BlockInfo::default(),
            true,
            None,
            platform_version,
        )
        .expect("expected to apply");
    assert!(
        estimated_fee.storage_fee >= applied_fee.storage_fee,
        "registration estimate {} below actual {}",
        estimated_fee.storage_fee,
        applied_fee.storage_fee
    );

    let estimated_fee = drive
        .insert_contract_group_memberships(
            contract_id,
            &memberships,
            &BlockInfo::default(),
            false,
            None,
            platform_version,
        )
        .expect("expected an estimate");
    let applied_fee = drive
        .insert_contract_group_memberships(
            contract_id,
            &memberships,
            &BlockInfo::default(),
            true,
            None,
            platform_version,
        )
        .expect("expected to apply");
    assert!(
        estimated_fee.storage_fee >= applied_fee.storage_fee,
        "membership estimate {} below actual {}",
        estimated_fee.storage_fee,
        applied_fee.storage_fee
    );
}

#[test]
fn should_refuse_to_register_an_existing_contract_group() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let owner = identity(1);
    let group_id = generate_contract_group_id(&owner, 4);
    let info = single_owner_info(owner, "twice");

    drive
        .insert_contract_group(
            group_id,
            &info,
            &BlockInfo::default(),
            true,
            None,
            platform_version,
        )
        .expect("expected to register");
    let second = drive.insert_contract_group(
        group_id,
        &info,
        &BlockInfo::default(),
        true,
        None,
        platform_version,
    );
    assert!(
        second.is_err(),
        "a second registration must not overwrite the group"
    );
}

#[test]
fn should_page_through_members_with_a_cursor_and_bound_the_limit() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let owner = identity(1);
    let group_id = generate_contract_group_id(&owner, 5);
    drive
        .insert_contract_group(
            group_id,
            &single_owner_info(owner, "paged"),
            &BlockInfo::default(),
            true,
            None,
            platform_version,
        )
        .expect("expected to register the group");

    let whole_contracts = [identity(10), identity(11), identity(12)];
    for contract_id in whole_contracts {
        drive
            .insert_contract_group_memberships(
                contract_id,
                &[membership(group_id, ContractGroupMember::Contract)],
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to record the membership");
    }
    let typed_contract = identity(20);
    drive
        .insert_contract_group_memberships(
            typed_contract,
            &[
                membership(group_id, ContractGroupMember::DocumentType("a".to_string())),
                membership(group_id, ContractGroupMember::DocumentType("b".to_string())),
                membership(group_id, ContractGroupMember::DocumentType("c".to_string())),
                membership(group_id, ContractGroupMember::Token(0)),
                membership(group_id, ContractGroupMember::Token(1)),
            ],
            &BlockInfo::default(),
            true,
            None,
            platform_version,
        )
        .expect("expected to record the memberships");
    let other_typed_contract = identity(21);
    drive
        .insert_contract_group_memberships(
            other_typed_contract,
            &[membership(
                group_id,
                ContractGroupMember::DocumentType("a".to_string()),
            )],
            &BlockInfo::default(),
            true,
            None,
            platform_version,
        )
        .expect("expected to record the membership");

    // Walks every page of one kind with the cursor each page hands back, proving each page.
    let walk = |first: ContractGroupMembersQuery, limit: u16| -> Vec<ContractGroupMembersPage> {
        let mut pages = vec![];
        let mut query = first;
        loop {
            let page = drive
                .fetch_contract_group_members(group_id, &query, limit, None, platform_version)
                .expect("expected to fetch a page");
            assert_members_page(&drive, group_id, &query, limit, &page, platform_version);
            assert!(
                page.len() <= limit as usize,
                "a page never exceeds its limit"
            );
            match page.next_query() {
                Some(next) => {
                    pages.push(page);
                    query = next;
                }
                None => {
                    assert!(page.is_empty());
                    return pages;
                }
            }
        }
    };

    assert_eq!(
        walk(
            ContractGroupMembersQuery::Contracts { start_after: None },
            2
        ),
        vec![
            ContractGroupMembersPage::Contracts(vec![identity(10), identity(11)]),
            ContractGroupMembersPage::Contracts(vec![identity(12)]),
        ]
    );
    // The cursor continues inside a contract's document types, then moves to the next contract.
    assert_eq!(
        walk(
            ContractGroupMembersQuery::DocumentTypes { start_after: None },
            2
        ),
        vec![
            ContractGroupMembersPage::DocumentTypes(vec![
                (typed_contract, "a".to_string()),
                (typed_contract, "b".to_string()),
            ]),
            ContractGroupMembersPage::DocumentTypes(vec![
                (typed_contract, "c".to_string()),
                (other_typed_contract, "a".to_string()),
            ]),
        ]
    );
    assert_eq!(
        walk(ContractGroupMembersQuery::Tokens { start_after: None }, 1),
        vec![
            ContractGroupMembersPage::Tokens(vec![(typed_contract, 0)]),
            ContractGroupMembersPage::Tokens(vec![(typed_contract, 1)]),
        ]
    );

    // A page is never unbounded: zero and over-the-maximum limits are refused on both the
    // fetch and the proof side.
    let query = ContractGroupMembersQuery::Contracts { start_after: None };
    for limit in [0, drive.config.max_query_limit + 1] {
        assert!(drive
            .fetch_contract_group_members(group_id, &query, limit, None, platform_version)
            .is_err());
        assert!(drive
            .prove_contract_group_members(group_id, &query, limit, None, platform_version)
            .is_err());
    }
}

/// A continuation cursor often sits on the last entry a contract has in the group. Descending
/// into that contract finds nothing, and GroveDB would charge that empty descent against the
/// limit unless told otherwise, which with a limit of one ended pagination before the next
/// contract. The fetch and the proof both keep the limit, so a limit of one reaches everything.
#[test]
fn should_page_document_types_past_a_contract_with_no_entries_left() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let owner = identity(1);
    let group_id = generate_contract_group_id(&owner, 6);
    drive
        .insert_contract_group(
            group_id,
            &single_owner_info(owner, "sparse"),
            &BlockInfo::default(),
            true,
            None,
            platform_version,
        )
        .expect("expected to register the group");

    // Four contracts with one document type each: every cursor lands on a contract's last entry.
    let expected: Vec<(Identifier, String)> = [(30, "t1"), (31, "t2"), (32, "t3"), (33, "t4")]
        .into_iter()
        .map(|(seed, name)| (identity(seed), name.to_string()))
        .collect();
    for (contract_id, name) in &expected {
        drive
            .insert_contract_group_memberships(
                *contract_id,
                &[membership(
                    group_id,
                    ContractGroupMember::DocumentType(name.clone()),
                )],
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to record the membership");
    }

    let mut query = ContractGroupMembersQuery::DocumentTypes { start_after: None };
    let mut collected = vec![];
    loop {
        let page = drive
            .fetch_contract_group_members(group_id, &query, 1, None, platform_version)
            .expect("expected to fetch a page");
        assert_members_page(&drive, group_id, &query, 1, &page, platform_version);
        let ContractGroupMembersPage::DocumentTypes(entries) = &page else {
            panic!("expected a document types page");
        };
        collected.extend(entries.iter().cloned());
        match page.next_query() {
            Some(next) => query = next,
            None => break,
        }
    }
    assert_eq!(
        collected, expected,
        "a limit of one must still reach every entry"
    );
}

fn key_bound_to_contract_group(
    key_id: KeyID,
    purpose: Purpose,
    security_level: SecurityLevel,
    contract_group_id: Identifier,
    platform_version: &PlatformVersion,
) -> IdentityPublicKey {
    let mut rng = StdRng::seed_from_u64(key_id as u64 + 900);
    IdentityPublicKeyV0 {
        id: key_id,
        purpose,
        security_level,
        contract_bounds: Some(ContractBounds::ContractGroup {
            id: contract_group_id,
        }),
        key_type: KeyType::ECDSA_SECP256K1,
        read_only: false,
        data: BinaryData::new(
            KeyType::ECDSA_SECP256K1
                .random_public_key_data(&mut rng, platform_version)
                .expect("expected a random key"),
        ),
        disabled_at: None,
    }
    .into()
}

fn group_bound_keys_request(
    identity_id: Identifier,
    contract_group_id: Identifier,
    kind: KeyKindRequestType,
) -> IdentityKeysRequest {
    IdentityKeysRequest {
        identity_id: identity_id.to_buffer(),
        request_type: KeyRequestType::ContractGroupBoundKey(
            contract_group_id.to_buffer(),
            Purpose::AUTHENTICATION,
            kind,
        ),
        limit: None,
        offset: None,
    }
}

#[test]
fn should_store_and_fetch_an_authentication_key_bound_to_a_contract_group() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(None);
    let contract_group_id = generate_contract_group_id(&identity(1), 1);
    drive
        .insert_contract_group(
            contract_group_id,
            &single_owner_info(identity(1), "wallet"),
            &BlockInfo::default(),
            true,
            None,
            platform_version,
        )
        .expect("expected to register the group");

    let mut new_identity = Identity::random_identity(3, Some(4791), platform_version)
        .expect("expected a random identity");
    let bound_key = key_bound_to_contract_group(
        7,
        Purpose::AUTHENTICATION,
        SecurityLevel::HIGH,
        contract_group_id,
        platform_version,
    );
    new_identity.add_public_key(bound_key.clone());

    let estimated = drive
        .add_new_identity(
            new_identity.clone(),
            false,
            &BlockInfo::default(),
            false,
            None,
            platform_version,
        )
        .expect("expected to estimate the identity");
    let actual = drive
        .add_new_identity(
            new_identity.clone(),
            false,
            &BlockInfo::default(),
            true,
            None,
            platform_version,
        )
        .expect("expected to insert the identity");
    assert!(
        estimated.processing_fee >= actual.processing_fee
            && estimated.storage_fee >= actual.storage_fee,
        "estimate {:?} must cover actual {:?}",
        estimated,
        actual
    );

    // The current key of the group, through the alias, and the listing, which skips the alias.
    let current = drive
        .fetch_identity_keys::<KeyIDIdentityPublicKeyPairVec>(
            IdentityKeysRequest::new_contract_group_authentication_keys_query(
                new_identity.id().to_buffer(),
                contract_group_id.to_buffer(),
            ),
            None,
            platform_version,
        )
        .expect("expected the current key");
    assert_eq!(current, vec![(7, bound_key.clone())]);
    let all = drive
        .fetch_identity_keys::<KeyIDIdentityPublicKeyPairVec>(
            group_bound_keys_request(
                new_identity.id(),
                contract_group_id,
                KeyKindRequestType::AllKeysOfKindRequest,
            ),
            None,
            platform_version,
        )
        .expect("expected every key of the group");
    assert_eq!(all, vec![(7, bound_key)]);

    // Another group holds nothing for this identity.
    let other = drive.fetch_identity_keys::<KeyIDIdentityPublicKeyPairVec>(
        group_bound_keys_request(
            new_identity.id(),
            generate_contract_group_id(&identity(1), 2),
            KeyKindRequestType::AllKeysOfKindRequest,
        ),
        None,
        platform_version,
    );
    assert!(
        other.map(|keys| keys.is_empty()).unwrap_or(true),
        "no keys are bound to a group the identity never named"
    );
}

#[test]
fn should_refuse_a_key_bound_to_a_missing_contract_group_or_with_another_purpose() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(None);
    let contract_group_id = generate_contract_group_id(&identity(1), 1);
    drive
        .insert_contract_group(
            contract_group_id,
            &single_owner_info(identity(1), "wallet"),
            &BlockInfo::default(),
            true,
            None,
            platform_version,
        )
        .expect("expected to register the group");

    for (case, purpose, security_level, group) in [
        (
            "missing group",
            Purpose::AUTHENTICATION,
            SecurityLevel::HIGH,
            generate_contract_group_id(&identity(1), 9),
        ),
        (
            "encryption key",
            Purpose::ENCRYPTION,
            SecurityLevel::MEDIUM,
            contract_group_id,
        ),
    ] {
        let mut new_identity = Identity::random_identity(3, Some(81), platform_version)
            .expect("expected a random identity");
        new_identity.add_public_key(key_bound_to_contract_group(
            7,
            purpose,
            security_level,
            group,
            platform_version,
        ));
        let result = drive.add_new_identity(
            new_identity,
            false,
            &BlockInfo::default(),
            true,
            None,
            platform_version,
        );
        assert!(
            matches!(
                result,
                Err(Error::Identity(IdentityError::IdentityKeyBoundsError(_)))
            ),
            "{case}: {result:?}"
        );
    }
}

#[test]
fn should_keep_a_disabled_group_bound_key_reachable_through_the_group_request() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(None);
    let contract_group_id = generate_contract_group_id(&identity(1), 1);
    drive
        .insert_contract_group(
            contract_group_id,
            &single_owner_info(identity(1), "wallet"),
            &BlockInfo::default(),
            true,
            None,
            platform_version,
        )
        .expect("expected to register the group");
    let mut new_identity = Identity::random_identity(3, Some(515), platform_version)
        .expect("expected a random identity");
    new_identity.add_public_key(key_bound_to_contract_group(
        7,
        Purpose::AUTHENTICATION,
        SecurityLevel::HIGH,
        contract_group_id,
        platform_version,
    ));
    drive
        .add_new_identity(
            new_identity.clone(),
            false,
            &BlockInfo::default(),
            true,
            None,
            platform_version,
        )
        .expect("expected to insert the identity");

    drive
        .disable_identity_keys(
            new_identity.id().to_buffer(),
            vec![7],
            1_000,
            &BlockInfo::default(),
            true,
            None,
            platform_version,
        )
        .expect("expected to disable the bound key");

    let current = drive
        .fetch_identity_keys::<KeyIDIdentityPublicKeyPairVec>(
            IdentityKeysRequest::new_contract_group_authentication_keys_query(
                new_identity.id().to_buffer(),
                contract_group_id.to_buffer(),
            ),
            None,
            platform_version,
        )
        .expect("expected the current key");
    assert_eq!(current.len(), 1);
    assert_eq!(current[0].1.disabled_at(), Some(1_000));
}
