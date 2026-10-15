use super::*;
use dapi_grpc::platform::v0::get_documents_request::{
    DocumentFieldValue as ProtoDocumentFieldValue, WhereClause as ProtoWhereClause,
    WhereOperator as ProtoWhereOperator,
};
use dapi_grpc::platform::v0::subscribe_to_state_transitions_request::{
    document_filter, state_transition_filter, DocumentFilter as ProtoDocumentFilter,
    StateTransitionFilter as ProtoStateTransitionFilter, Version,
};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::{DataContract, DataContractFactory};
use dpp::platform_value::platform_value;
use dpp::platform_value::string_encoding::Encoding;
use dpp::platform_value::Value;
use dpp::state_transition::address_credit_withdrawal_transition::v0::AddressCreditWithdrawalTransitionV0;
use dpp::state_transition::address_credit_withdrawal_transition::AddressCreditWithdrawalTransition;
use dpp::state_transition::address_funding_from_asset_lock_transition::v0::AddressFundingFromAssetLockTransitionV0;
use dpp::state_transition::address_funding_from_asset_lock_transition::AddressFundingFromAssetLockTransition;
use dpp::state_transition::address_funds_transfer_transition::v0::AddressFundsTransferTransitionV0;
use dpp::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition;
use dpp::state_transition::batch_transition::batched_transition::document_create_transition::v0::DocumentCreateTransitionV0;
use dpp::state_transition::batch_transition::batched_transition::document_create_transition::DocumentCreateTransition;
use dpp::state_transition::batch_transition::batched_transition::document_delete_transition::v0::DocumentDeleteTransitionV0;
use dpp::state_transition::batch_transition::batched_transition::document_delete_transition::DocumentDeleteTransition;
use dpp::state_transition::batch_transition::batched_transition::document_purchase_transition::v0::DocumentPurchaseTransitionV0;
use dpp::state_transition::batch_transition::batched_transition::document_purchase_transition::DocumentPurchaseTransition;
use dpp::state_transition::batch_transition::batched_transition::document_transfer_transition::v0::DocumentTransferTransitionV0;
use dpp::state_transition::batch_transition::batched_transition::document_transfer_transition::DocumentTransferTransition;
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
use dpp::state_transition::batch_transition::batched_transition::BatchedTransition;
use dpp::state_transition::batch_transition::document_base_transition::v1::DocumentBaseTransitionV1;
use dpp::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
use dpp::state_transition::batch_transition::{BatchTransition, BatchTransitionV1};
use dpp::state_transition::identity_create_from_addresses_transition::v0::IdentityCreateFromAddressesTransitionV0;
use dpp::state_transition::identity_create_from_addresses_transition::IdentityCreateFromAddressesTransition;
use dpp::state_transition::identity_create_from_shielded_pool_transition::v0::IdentityCreateFromShieldedPoolTransitionV0;
use dpp::state_transition::identity_create_from_shielded_pool_transition::IdentityCreateFromShieldedPoolTransition;
use dpp::state_transition::identity_credit_transfer_to_addresses_transition::v0::IdentityCreditTransferToAddressesTransitionV0;
use dpp::state_transition::identity_credit_transfer_to_addresses_transition::IdentityCreditTransferToAddressesTransition;
use dpp::state_transition::identity_topup_from_addresses_transition::v0::IdentityTopUpFromAddressesTransitionV0;
use dpp::state_transition::identity_topup_from_addresses_transition::IdentityTopUpFromAddressesTransition;
use dpp::state_transition::shield_from_asset_lock_transition::v0::ShieldFromAssetLockTransitionV0;
use dpp::state_transition::shield_from_asset_lock_transition::v1::ShieldFromAssetLockTransitionV1;
use dpp::state_transition::shield_from_asset_lock_transition::ShieldFromAssetLockTransition;
use dpp::state_transition::shield_transition::v0::ShieldTransitionV0;
use dpp::state_transition::shield_transition::ShieldTransition;
use dpp::state_transition::unshield_transition::v0::UnshieldTransitionV0;
use dpp::state_transition::unshield_transition::UnshieldTransition;
use dpp::state_transition::StateTransition;
use dpp::tests::fixtures::get_data_contract_fixture;
use dpp::version::PlatformVersion;
use drive::query::{ValueClause, WhereClause, WhereOperator};
use std::collections::BTreeMap;
use std::sync::Arc;

fn id(byte: u8) -> Identifier {
    Identifier::from([byte; 32])
}

fn contract() -> Arc<DataContract> {
    Arc::new(
        get_data_contract_fixture(None, 0, PlatformVersion::latest().protocol_version)
            .data_contract_owned(),
    )
}

fn resolve(
    filters: Vec<StateTransitionFilter>,
    contract: &Arc<DataContract>,
) -> Result<ResolvedFilters, SubscriptionFilterError> {
    let contract = contract.clone();
    ResolvedFilters::resolve(
        filters,
        move |requested| (*requested == contract.id()).then(|| contract.clone()),
        PlatformVersion::latest(),
    )
}

/// `filter` after a round trip through its wire message.
fn over_the_wire(filter: &StateTransitionFilter) -> StateTransitionFilter {
    StateTransitionFilter::from_proto(0, filter.to_proto().unwrap()).unwrap()
}

fn address_transfer(from: PlatformAddress, to: PlatformAddress) -> StateTransition {
    StateTransition::AddressFundsTransfer(AddressFundsTransferTransition::V0(
        AddressFundsTransferTransitionV0 {
            inputs: BTreeMap::from([(from, (1, 5000))]),
            outputs: BTreeMap::from([(to, 4000)]),
            ..Default::default()
        },
    ))
}

fn batch(owner: Identifier, transitions: Vec<BatchedTransition>) -> StateTransition {
    StateTransition::Batch(BatchTransition::V1(BatchTransitionV1 {
        owner_id: owner,
        transitions,
        user_fee_increase: 0,
        signature_public_key_id: 0,
        signature: Default::default(),
    }))
}

fn document_base(contract: &DataContract, doc: u8) -> DocumentBaseTransition {
    DocumentBaseTransition::V1(DocumentBaseTransitionV1 {
        id: id(doc),
        document_type_name: "niceDocument".to_string(),
        data_contract_id: contract.id(),
        identity_contract_nonce: 0,
        token_payment_info: None,
    })
}

fn create_document(contract: &DataContract, doc: u8, name: &str) -> BatchedTransition {
    BatchedTransition::Document(DocumentTransition::Create(DocumentCreateTransition::V0(
        DocumentCreateTransitionV0 {
            base: document_base(contract, doc),
            entropy: [0u8; 32],
            data: BTreeMap::from([("name".to_string(), Value::Text(name.to_string()))]),
            prefunded_voting_balance: None,
        },
    )))
}

fn delete_document(contract: &DataContract, doc: u8) -> BatchedTransition {
    BatchedTransition::Document(DocumentTransition::Delete(DocumentDeleteTransition::V0(
        DocumentDeleteTransitionV0 {
            base: document_base(contract, doc),
        },
    )))
}

fn transfer_document(contract: &DataContract, doc: u8, recipient: Identifier) -> BatchedTransition {
    BatchedTransition::Document(DocumentTransition::Transfer(
        DocumentTransferTransition::V0(DocumentTransferTransitionV0 {
            base: document_base(contract, doc),
            revision: 2,
            recipient_owner_id: recipient,
        }),
    ))
}

fn purchase_document(contract: &DataContract, doc: u8) -> BatchedTransition {
    BatchedTransition::Document(DocumentTransition::Purchase(
        DocumentPurchaseTransition::V0(DocumentPurchaseTransitionV0 {
            base: document_base(contract, doc),
            revision: 2,
            price: 100,
        }),
    ))
}

fn name_equals(name: &str) -> WhereClause {
    WhereClause {
        field: "name".to_string(),
        operator: WhereOperator::Equal,
        value: Value::Text(name.to_string()),
    }
}

/// A filter on `niceDocument` with the one action alternative `action`.
fn nice_documents(contract: &DataContract, action: DocumentActionMatch) -> StateTransitionFilter {
    StateTransitionFilter::Documents(
        DocumentFilter::new(contract.id())
            .with_document_type("niceDocument")
            .with_action(action),
    )
}

fn matches(filters: &ResolvedFilters, state_transition: &StateTransition) -> Option<FilterMatch> {
    filters.matches(state_transition, PlatformVersion::latest())
}

#[test]
fn should_match_address_filters_by_role() {
    let contract = contract();
    let from = PlatformAddress::P2pkh([1u8; 20]);
    let to = PlatformAddress::P2sh([2u8; 20]);
    let transfer = address_transfer(from, to);

    for (address, role, expected) in [
        (to, Role::Recipient, true),
        (to, Role::Sender, false),
        (from, Role::Sender, true),
        (from, Role::Recipient, false),
        (to, Role::Any, true),
        (PlatformAddress::P2pkh([9u8; 20]), Role::Any, false),
    ] {
        let filters = resolve(
            vec![StateTransitionFilter::Addresses {
                addresses: vec![address],
                role,
            }],
            &contract,
        )
        .unwrap();
        assert_eq!(
            matches(&filters, &transfer).is_some(),
            expected,
            "{address:?} as {role:?}"
        );
    }
}

fn address(byte: u8) -> PlatformAddress {
    PlatformAddress::P2pkh([byte; 20])
}

fn funded_by(senders: &[PlatformAddress]) -> BTreeMap<PlatformAddress, (u32, u64)> {
    senders.iter().map(|sender| (*sender, (1, 5000))).collect()
}

fn change_to(recipients: &[PlatformAddress]) -> Option<(PlatformAddress, u64)> {
    recipients.first().map(|recipient| (*recipient, 1000))
}

/// Each variant that names addresses, with the senders and recipients it must be classified as.
fn address_role_cases() -> Vec<(
    String,
    StateTransition,
    Vec<PlatformAddress>,
    Vec<PlatformAddress>,
)> {
    let (s1, s2, r1, r2) = (address(1), address(2), address(3), address(4));
    let mut cases = Vec::new();
    for (change, recipients) in [("with change", vec![r1]), ("without change", vec![])] {
        let senders = vec![s1, s2];
        cases.push((
            format!("identity create from addresses {change}"),
            StateTransition::IdentityCreateFromAddresses(
                IdentityCreateFromAddressesTransition::V0(
                    IdentityCreateFromAddressesTransitionV0 {
                        inputs: funded_by(&senders),
                        output: change_to(&recipients),
                        ..Default::default()
                    },
                ),
            ),
            senders.clone(),
            recipients.clone(),
        ));
        cases.push((
            format!("identity top up from addresses {change}"),
            StateTransition::IdentityTopUpFromAddresses(IdentityTopUpFromAddressesTransition::V0(
                IdentityTopUpFromAddressesTransitionV0 {
                    inputs: funded_by(&senders),
                    output: change_to(&recipients),
                    ..Default::default()
                },
            )),
            senders.clone(),
            recipients.clone(),
        ));
        cases.push((
            format!("address credit withdrawal {change}"),
            StateTransition::AddressCreditWithdrawal(AddressCreditWithdrawalTransition::V0(
                AddressCreditWithdrawalTransitionV0 {
                    inputs: funded_by(&senders),
                    output: change_to(&recipients),
                    ..Default::default()
                },
            )),
            senders.clone(),
            recipients.clone(),
        ));
        let surplus_output = recipients.first().copied();
        cases.push((
            format!("shield from asset lock v0 {change}"),
            StateTransition::ShieldFromAssetLock(ShieldFromAssetLockTransition::V0(
                ShieldFromAssetLockTransitionV0 {
                    asset_lock_proof: Default::default(),
                    actions: vec![],
                    value_balance: 1000,
                    anchor: [0; 32],
                    proof: vec![],
                    binding_signature: [0; 64],
                    surplus_output,
                    signature: Default::default(),
                },
            )),
            vec![],
            recipients.clone(),
        ));
        cases.push((
            format!("shield from asset lock v1 {change}"),
            StateTransition::ShieldFromAssetLock(ShieldFromAssetLockTransition::V1(
                ShieldFromAssetLockTransitionV1 {
                    asset_lock_proof: Default::default(),
                    actions: vec![],
                    value_balance: 1000,
                    anchor: [0; 32],
                    proof: vec![],
                    binding_signature: [0; 64],
                    surplus_output,
                    signature: Default::default(),
                },
            )),
            vec![],
            recipients,
        ));
    }
    let name = |name: &str| name.to_string();
    cases.extend([
        (
            name("address funding from asset lock"),
            StateTransition::AddressFundingFromAssetLock(
                AddressFundingFromAssetLockTransition::V0(
                    AddressFundingFromAssetLockTransitionV0 {
                        inputs: funded_by(&[s1, s2]),
                        outputs: BTreeMap::from([(r1, None), (r2, Some(500))]),
                        ..Default::default()
                    },
                ),
            ),
            vec![s1, s2],
            vec![r1, r2],
        ),
        (
            name("address funding from asset lock without inputs"),
            StateTransition::AddressFundingFromAssetLock(
                AddressFundingFromAssetLockTransition::V0(
                    AddressFundingFromAssetLockTransitionV0 {
                        outputs: BTreeMap::from([(r1, None)]),
                        ..Default::default()
                    },
                ),
            ),
            vec![],
            vec![r1],
        ),
        (
            name("unshield"),
            StateTransition::Unshield(UnshieldTransition::V0(UnshieldTransitionV0 {
                output_address: r1,
                actions: vec![],
                unshielding_amount: 1000,
                anchor: [0; 32],
                proof: vec![],
                binding_signature: [0; 64],
            })),
            vec![],
            vec![r1],
        ),
        (
            name("shield spends address inputs and credits the pool"),
            StateTransition::Shield(ShieldTransition::V0(ShieldTransitionV0 {
                inputs: funded_by(&[s1, s2]),
                actions: vec![],
                amount: 1000,
                anchor: [0; 32],
                proof: vec![],
                binding_signature: [0; 64],
                fee_strategy: vec![],
                user_fee_increase: 0,
                input_witnesses: vec![],
            })),
            vec![s1, s2],
            vec![],
        ),
        (
            name("identity create from shielded pool"),
            StateTransition::IdentityCreateFromShieldedPool(
                IdentityCreateFromShieldedPoolTransition::V0(
                    IdentityCreateFromShieldedPoolTransitionV0 {
                        public_keys: vec![],
                        denomination: 1000,
                        actions: vec![],
                        anchor: [0; 32],
                        proof: vec![],
                        binding_signature: [0; 64],
                        send_to_address_on_creation_failure: r1,
                        identity_id: id(7),
                    },
                ),
            ),
            vec![],
            vec![r1],
        ),
        (
            name("identity credit transfer to addresses"),
            StateTransition::IdentityCreditTransferToAddresses(
                IdentityCreditTransferToAddressesTransition::V0(
                    IdentityCreditTransferToAddressesTransitionV0 {
                        recipient_addresses: BTreeMap::from([(r1, 100), (r2, 200)]),
                        ..Default::default()
                    },
                ),
            ),
            vec![],
            vec![r1, r2],
        ),
    ]);
    cases
}

#[test]
fn should_classify_address_roles_for_every_variant_naming_addresses() {
    let contract = contract();
    let stranger = address(99);
    for (name, state_transition, senders, recipients) in address_role_cases() {
        let matched = |address: PlatformAddress, role: Role| {
            let filters = resolve(
                vec![StateTransitionFilter::Addresses {
                    addresses: vec![address],
                    role,
                }],
                &contract,
            )
            .unwrap();
            matches(&filters, &state_transition).is_some()
        };
        for (addresses, own, other) in [
            (&senders, Role::Sender, Role::Recipient),
            (&recipients, Role::Recipient, Role::Sender),
        ] {
            for address in addresses {
                assert!(matched(*address, own), "{name}: {address:?} as {own:?}");
                assert!(
                    !matched(*address, other),
                    "{name}: {address:?} as {other:?}"
                );
                assert!(matched(*address, Role::Any), "{name}: {address:?} as Any");
            }
        }
        for role in [Role::Sender, Role::Recipient, Role::Any] {
            assert!(!matched(stranger, role), "{name}: stranger as {role:?}");
        }
    }
}

#[test]
fn should_report_every_matching_filter_and_batch_position() {
    let contract = contract();
    let state_transition = batch(
        id(1),
        vec![
            create_document(&contract, 10, "alice"),
            create_document(&contract, 11, "bob"),
            transfer_document(&contract, 12, id(5)),
        ],
    );
    let filters = resolve(
        vec![
            nice_documents(
                &contract,
                DocumentActionMatch::new(DocumentAction::Create)
                    .with_new_document_where(name_equals("bob")),
            ),
            StateTransitionFilter::Addresses {
                addresses: vec![PlatformAddress::P2pkh([1u8; 20])],
                role: Role::Any,
            },
            nice_documents(
                &contract,
                DocumentActionMatch::new(DocumentAction::Transfer).with_owner_ids([id(5)]),
            ),
        ],
        &contract,
    )
    .unwrap();

    assert_eq!(
        matches(&filters, &state_transition),
        Some(FilterMatch {
            matched_filters: vec![0, 2],
            matched_batch_positions: vec![1, 2],
        })
    );
}

#[test]
fn should_match_document_filters_on_action_and_batch_owner() {
    let contract = contract();
    let deleting = batch(id(1), vec![delete_document(&contract, 10)]);
    let resolved = |filter: DocumentFilter| {
        resolve(vec![StateTransitionFilter::Documents(filter)], &contract).unwrap()
    };

    let deletes = resolved(
        DocumentFilter::new(contract.id())
            .with_action(DocumentActionMatch::new(DocumentAction::Delete)),
    );
    assert!(matches(&deletes, &deleting).is_some());
    let creates = resolved(
        DocumentFilter::new(contract.id())
            .with_action(DocumentActionMatch::new(DocumentAction::Create)),
    );
    assert!(matches(&creates, &deleting).is_none());
    let someone_else = resolved(DocumentFilter::new(contract.id()).with_batch_owner(id(2)));
    assert!(matches(&someone_else, &deleting).is_none());
    // Not a batch: no document filter matches.
    let transfer = address_transfer(
        PlatformAddress::P2pkh([1u8; 20]),
        PlatformAddress::P2pkh([2u8; 20]),
    );
    assert!(matches(&resolved(DocumentFilter::new(contract.id())), &transfer).is_none());
}

#[test]
fn should_match_a_specific_document_by_id_before_the_transition() {
    let contract = contract();
    let filters = resolve(
        vec![nice_documents(
            &contract,
            DocumentActionMatch::new(DocumentAction::Delete).with_original_document_where(
                WhereClause {
                    field: "$id".to_string(),
                    operator: WhereOperator::Equal,
                    value: Value::Text(id(10).to_string(Encoding::Base58)),
                },
            ),
        )],
        &contract,
    )
    .unwrap();
    let deleting = |doc| batch(id(1), vec![delete_document(&contract, doc)]);
    assert!(matches(&filters, &deleting(10)).is_some());
    assert!(matches(&filters, &deleting(11)).is_none());
}

#[test]
fn should_match_a_document_buyer_by_owner_ids() {
    let contract = contract();
    let buyer = id(6);
    let filters = resolve(
        vec![nice_documents(
            &contract,
            DocumentActionMatch::new(DocumentAction::Purchase).with_owner_ids([buyer]),
        )],
        &contract,
    )
    .unwrap();
    let purchase = |owner| {
        batch(
            owner,
            vec![
                create_document(&contract, 11, "x"),
                purchase_document(&contract, 10),
            ],
        )
    };
    assert_eq!(
        matches(&filters, &purchase(buyer)),
        Some(FilterMatch {
            matched_filters: vec![0],
            matched_batch_positions: vec![1],
        })
    );
    assert!(matches(&filters, &purchase(id(7))).is_none());
}

#[test]
fn should_reject_invalid_document_filters() {
    let contract = contract();
    for filter in [
        // Only `$id` is known of the document before a plain delete.
        nice_documents(
            &contract,
            DocumentActionMatch::new(DocumentAction::Delete)
                .with_original_document_where(name_equals("bob")),
        ),
        // Clauses need a document type.
        StateTransitionFilter::Documents(
            DocumentFilter::new(contract.id()).with_action(
                DocumentActionMatch::new(DocumentAction::Create)
                    .with_new_document_where(name_equals("bob")),
            ),
        ),
        // Owner ids apply only to transfers and purchases.
        nice_documents(
            &contract,
            DocumentActionMatch::new(DocumentAction::Create).with_owner_ids([id(1)]),
        ),
        StateTransitionFilter::Documents(
            DocumentFilter::new(contract.id()).with_document_type("noSuchType"),
        ),
    ] {
        assert!(
            matches!(
                resolve(vec![filter.clone()], &contract),
                Err(SubscriptionFilterError::InvalidFilter { index: 0, .. })
            ),
            "{filter:?}"
        );
    }
    assert_eq!(
        resolve(
            vec![StateTransitionFilter::Documents(DocumentFilter::new(id(
                99
            )))],
            &contract
        )
        .unwrap_err(),
        SubscriptionFilterError::DataContractNotFound(id(99))
    );
}

#[test]
fn should_enforce_filter_limits() {
    let contract = contract();
    let addresses = |count: usize| StateTransitionFilter::Addresses {
        addresses: (0..count)
            .map(|i| {
                let mut hash = [0u8; 20];
                hash[..8].copy_from_slice(&(i as u64).to_be_bytes());
                PlatformAddress::P2pkh(hash)
            })
            .collect(),
        role: Role::Any,
    };
    let owners = |count: usize| {
        nice_documents(
            &contract,
            DocumentActionMatch::new(DocumentAction::Transfer)
                .with_owner_ids((0..count).map(|i| id(i as u8))),
        )
    };
    assert!(resolve(vec![], &contract).is_err());
    assert!(resolve(vec![addresses(1); MAX_FILTERS], &contract).is_ok());
    assert!(resolve(vec![addresses(1); MAX_FILTERS + 1], &contract).is_err());
    assert!(resolve(vec![addresses(0)], &contract).is_err());
    assert!(resolve(vec![addresses(MAX_ADDRESSES_PER_FILTER)], &contract).is_ok());
    assert!(resolve(vec![addresses(MAX_ADDRESSES_PER_FILTER + 1)], &contract).is_err());
    assert!(resolve(vec![owners(MAX_OWNER_IDS_PER_ACTION)], &contract).is_ok());
    assert!(resolve(vec![owners(MAX_OWNER_IDS_PER_ACTION + 1)], &contract).is_err());
}

#[test]
fn should_round_trip_filters_through_their_wire_form() {
    let contract = contract();
    let filters = vec![
        StateTransitionFilter::Documents(
            DocumentFilter::new(contract.id())
                .with_document_type("niceDocument")
                .with_batch_owner(id(3))
                .with_action(
                    DocumentActionMatch::new(DocumentAction::Create)
                        .with_new_document_where(name_equals("bob")),
                )
                .with_action(
                    DocumentActionMatch::new(DocumentAction::Transfer).with_owner_ids([id(4)]),
                )
                .with_action(
                    DocumentActionMatch::new(DocumentAction::UpdatePrice).with_price(ValueClause {
                        operator: WhereOperator::GreaterThan,
                        value: Value::U64(10),
                    }),
                ),
        ),
        StateTransitionFilter::Addresses {
            addresses: vec![
                PlatformAddress::P2pkh([1u8; 20]),
                PlatformAddress::P2sh([2u8; 20]),
            ],
            role: Role::Recipient,
        },
    ];
    let request = subscribe_request(&filters, Some(42)).unwrap();
    let Some(Version::V0(v0)) = request.version else {
        panic!("expected a v0 request");
    };
    assert_eq!(v0.from_block_height, Some(42));
    let decoded = v0
        .filters
        .into_iter()
        .enumerate()
        .map(|(index, filter)| StateTransitionFilter::from_proto(index, filter))
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(decoded, filters);
}

#[test]
fn should_reject_malformed_wire_action_matches() {
    let time_range = ProtoWhereClause {
        field: "$createdAt".to_string(),
        operator: ProtoWhereOperator::InTimeRange as i32,
        value: Some(ProtoDocumentFieldValue::default()),
        ..Default::default()
    };
    for action in [
        document_filter::ActionMatch {
            action: None,
            ..Default::default()
        },
        document_filter::ActionMatch {
            action: Some(document_filter::Action::Create as i32),
            new_document_where: vec![time_range],
            ..Default::default()
        },
    ] {
        let filter = ProtoStateTransitionFilter {
            filter: Some(state_transition_filter::Filter::Documents(
                ProtoDocumentFilter {
                    data_contract_id: id(1).to_vec(),
                    document_type_name: Some("niceDocument".to_string()),
                    actions: vec![action],
                    batch_owner_id: None,
                },
            )),
        };
        assert!(matches!(
            StateTransitionFilter::from_proto(3, filter),
            Err(SubscriptionFilterError::InvalidFilter { index: 3, .. })
        ));
    }
}

#[test]
fn should_resolve_wide_integer_operands_the_same_after_the_wire() {
    let contract = contract();
    for price in [Value::U128(50), Value::I128(1_i128 << 63)] {
        let filter = nice_documents(
            &contract,
            DocumentActionMatch::new(DocumentAction::UpdatePrice).with_price(ValueClause {
                operator: WhereOperator::Equal,
                value: price,
            }),
        );
        resolve(vec![filter.clone()], &contract).expect("resolves natively");
        resolve(vec![over_the_wire(&filter)], &contract).expect("resolves after the wire");
    }
}

#[test]
fn should_bound_price_in_candidates() {
    let contract = contract();
    let price_in = |values: Vec<Value>| {
        nice_documents(
            &contract,
            DocumentActionMatch::new(DocumentAction::UpdatePrice).with_price(ValueClause {
                operator: WhereOperator::In,
                value: Value::Array(values),
            }),
        )
    };
    let too_many = price_in((0..20_000u64).map(|n| Value::U64(n % 3)).collect());
    let duplicates = price_in(vec![Value::U64(1), Value::U64(1)]);
    // The same price in two encodings: distinct values, but one price once normalized.
    let mixed_duplicates = price_in(vec![Value::I64(1), Value::U64(1)]);
    for filter in [too_many, duplicates, mixed_duplicates] {
        assert!(resolve(vec![filter.clone()], &contract).is_err());
        assert!(resolve(vec![over_the_wire(&filter)], &contract).is_err());
    }
    assert!(resolve(
        vec![price_in(vec![Value::U64(1), Value::U64(2)])],
        &contract
    )
    .is_ok());
}

/// A contract with one document type, `rating`: a u8 `stars`, a float `value`, a byte
/// array `blob`, an identifier `author` and a u16 typed array `ranks`.
fn rating_contract() -> Arc<DataContract> {
    let documents = platform_value!({
        "rating": {
            "type": "object",
            "properties": {
                "stars": { "type": "integer", "minimum": 0, "maximum": 255, "position": 0 },
                "value": { "type": "number", "position": 1 },
                "blob": { "type": "array", "byteArray": true, "maxItems": 16u32, "position": 2 },
                "author": {
                    "type": "array", "byteArray": true, "minItems": 32u32, "maxItems": 32u32,
                    "contentMediaType": "application/x.dash.dpp.identifier", "position": 3
                },
                "ranks": {
                    "type": "array", "maxItems": 8u32,
                    "items": { "type": "integer", "minimum": 0, "maximum": 65535 },
                    "position": 4
                }
            },
            "additionalProperties": false
        }
    });
    Arc::new(
        DataContractFactory::new(PlatformVersion::latest().protocol_version)
            .expect("factory")
            .create_with_value_config(id(1), 1, documents, None, None)
            .expect("the contract parses")
            .data_contract_owned(),
    )
}

/// A filter on creates of `rating` with `clauses`.
fn rating_creates(contract: &DataContract, clauses: Vec<WhereClause>) -> StateTransitionFilter {
    let action = clauses.into_iter().fold(
        DocumentActionMatch::new(DocumentAction::Create),
        DocumentActionMatch::with_new_document_where,
    );
    StateTransitionFilter::Documents(
        DocumentFilter::new(contract.id())
            .with_document_type("rating")
            .with_action(action),
    )
}

/// A batch creating a `rating` whose `field` is `value`.
fn rating_created(contract: &DataContract, field: &str, value: Value) -> StateTransition {
    batch(
        id(1),
        vec![BatchedTransition::Document(DocumentTransition::Create(
            DocumentCreateTransition::V0(DocumentCreateTransitionV0 {
                base: DocumentBaseTransition::V1(DocumentBaseTransitionV1 {
                    id: id(10),
                    document_type_name: "rating".to_string(),
                    data_contract_id: contract.id(),
                    identity_contract_nonce: 0,
                    token_payment_info: None,
                }),
                entropy: [0u8; 32],
                data: BTreeMap::from([(field.to_string(), value)]),
                prefunded_voting_balance: None,
            }),
        ))],
    )
}

fn clause(field: &str, operator: WhereOperator, value: Value) -> WhereClause {
    WhereClause {
        field: field.to_string(),
        operator,
        value,
    }
}

#[test]
fn should_refuse_text_on_an_integer_field_from_the_wire_without_panicking() {
    let contract = rating_contract();
    // Its `Display` would split the text inside the final character.
    let text = Value::Text("a".repeat(19) + "é");
    let filter = rating_creates(&contract, vec![clause("stars", WhereOperator::Equal, text)]);
    assert!(matches!(
        resolve(vec![over_the_wire(&filter)], &contract),
        Err(SubscriptionFilterError::InvalidFilter { index: 0, .. })
    ));
}

#[test]
fn should_group_range_bounds_of_different_integer_widths() {
    let contract = rating_contract();
    let filter = rating_creates(
        &contract,
        vec![
            clause("stars", WhereOperator::GreaterThanOrEquals, Value::I64(1)),
            clause("stars", WhereOperator::LessThanOrEquals, Value::U64(5)),
        ],
    );
    for filter in [filter.clone(), over_the_wire(&filter)] {
        let filters = resolve(vec![filter], &contract).expect("the bounds group");
        let rated = |stars| rating_created(&contract, "stars", Value::U8(stars));
        assert!(matches(&filters, &rated(3)).is_some());
        assert!(matches(&filters, &rated(7)).is_none());
    }
}

#[test]
fn should_refuse_to_send_an_integer_wider_than_64_bits() {
    let contract = rating_contract();
    let filter = rating_creates(
        &contract,
        vec![clause("value", WhereOperator::Equal, Value::U128(1 << 64))],
    );
    resolve(vec![filter.clone()], &contract).expect("resolves natively, as a float");
    let error = filter.to_proto().expect_err("no wire form");
    assert!(error.to_string().contains("64 bits"), "{error}");
}

#[test]
fn should_resolve_a_byte_list_operand_the_same_after_the_wire() {
    let contract = rating_contract();
    let filter = rating_creates(
        &contract,
        vec![clause(
            "blob",
            WhereOperator::Equal,
            Value::Array(vec![Value::U8(1), Value::U8(2)]),
        )],
    );
    for filter in [filter.clone(), over_the_wire(&filter)] {
        let filters = resolve(vec![filter], &contract).expect("a byte list");
        let carrying = |bytes| rating_created(&contract, "blob", Value::Bytes(bytes));
        assert!(matches(&filters, &carrying(vec![1, 2])).is_some());
        assert!(matches(&filters, &carrying(vec![1, 3])).is_none());
    }
}

/// `byte` 32 times, as a list of `u8`s.
fn identifier_byte_list(byte: u8) -> Value {
    Value::Array(vec![Value::U8(byte); 32])
}

#[test]
fn should_resolve_an_id_byte_list_operand_the_same_after_the_wire() {
    let contract = contract();
    let filter = nice_documents(
        &contract,
        DocumentActionMatch::new(DocumentAction::Delete).with_original_document_where(clause(
            "$id",
            WhereOperator::Equal,
            identifier_byte_list(10),
        )),
    );
    for filter in [filter.clone(), over_the_wire(&filter)] {
        let filters = resolve(vec![filter], &contract).expect("an identifier byte list");
        let deleting = |doc| batch(id(1), vec![delete_document(&contract, doc)]);
        assert!(matches(&filters, &deleting(10)).is_some());
        assert!(matches(&filters, &deleting(11)).is_none());
    }
}

#[test]
fn should_resolve_an_identifier_byte_list_operand_the_same_after_the_wire() {
    let contract = rating_contract();
    let filter = rating_creates(
        &contract,
        vec![clause(
            "author",
            WhereOperator::Equal,
            identifier_byte_list(5),
        )],
    );
    for filter in [filter.clone(), over_the_wire(&filter)] {
        let filters = resolve(vec![filter], &contract).expect("an identifier byte list");
        let by = |author| rating_created(&contract, "author", id(author).into());
        assert!(matches(&filters, &by(5)).is_some());
        assert!(matches(&filters, &by(6)).is_none());
    }
    // Still exactly 32 bytes.
    let short = rating_creates(
        &contract,
        vec![clause(
            "author",
            WhereOperator::Equal,
            Value::Array(vec![Value::U8(5); 31]),
        )],
    );
    for filter in [short.clone(), over_the_wire(&short)] {
        assert!(resolve(vec![filter], &contract).is_err());
    }
}

#[test]
fn should_match_a_u16_array_against_a_list_given_as_bytes_after_the_wire() {
    let contract = rating_contract();
    let filter = rating_creates(
        &contract,
        vec![clause(
            "ranks",
            WhereOperator::ContainsAny,
            Value::Bytes(vec![1, 2]),
        )],
    );
    for filter in [filter.clone(), over_the_wire(&filter)] {
        let filters = resolve(vec![filter], &contract).expect("u16 candidates given as bytes");
        let ranked = |ranks: &[u16]| {
            let ranks = ranks.iter().copied().map(Value::U16).collect();
            rating_created(&contract, "ranks", Value::Array(ranks))
        };
        assert!(matches(&filters, &ranked(&[1, 500])).is_some());
        assert!(matches(&filters, &ranked(&[3])).is_none());
    }
}
