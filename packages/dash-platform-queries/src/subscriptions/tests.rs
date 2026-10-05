use super::*;
use dapi_grpc::platform::v0::subscribe_to_state_transitions_request::Version;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::DataContract;
use dpp::platform_value::string_encoding::Encoding;
use dpp::platform_value::Value;
use dpp::state_transition::address_funds_transfer_transition::v0::AddressFundsTransferTransitionV0;
use dpp::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition;
use dpp::state_transition::batch_transition::batched_transition::document_create_transition::v0::DocumentCreateTransitionV0;
use dpp::state_transition::batch_transition::batched_transition::document_create_transition::DocumentCreateTransition;
use dpp::state_transition::batch_transition::batched_transition::document_delete_transition::v0::DocumentDeleteTransitionV0;
use dpp::state_transition::batch_transition::batched_transition::document_delete_transition::DocumentDeleteTransition;
use dpp::state_transition::batch_transition::batched_transition::document_transfer_transition::v0::DocumentTransferTransitionV0;
use dpp::state_transition::batch_transition::batched_transition::document_transfer_transition::DocumentTransferTransition;
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
use dpp::state_transition::batch_transition::batched_transition::token_transition::TokenTransition;
use dpp::state_transition::batch_transition::batched_transition::BatchedTransition;
use dpp::state_transition::batch_transition::document_base_transition::v1::DocumentBaseTransitionV1;
use dpp::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
use dpp::state_transition::batch_transition::token_base_transition::v0::TokenBaseTransitionV0;
use dpp::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use dpp::state_transition::batch_transition::token_transfer_transition::v0::TokenTransferTransitionV0;
use dpp::state_transition::batch_transition::token_transfer_transition::TokenTransferTransition;
use dpp::state_transition::batch_transition::{BatchTransition, BatchTransitionV1};
use dpp::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;
use dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
use dpp::state_transition::StateTransition;
use dpp::tests::fixtures::get_data_contract_fixture;
use dpp::version::PlatformVersion;
use drive::query::WhereOperator;
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

fn credit_transfer(sender: Identifier, recipient: Identifier) -> StateTransition {
    StateTransition::IdentityCreditTransfer(IdentityCreditTransferTransition::V0(
        IdentityCreditTransferTransitionV0 {
            identity_id: sender,
            recipient_id: recipient,
            amount: 1000,
            ..Default::default()
        },
    ))
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

fn document_base(contract: &DataContract, document_type: &str, doc: u8) -> DocumentBaseTransition {
    DocumentBaseTransition::V1(DocumentBaseTransitionV1 {
        id: id(doc),
        document_type_name: document_type.to_string(),
        data_contract_id: contract.id(),
        identity_contract_nonce: 0,
        token_payment_info: None,
    })
}

fn create_document(contract: &DataContract, doc: u8, name: &str) -> BatchedTransition {
    BatchedTransition::Document(DocumentTransition::Create(DocumentCreateTransition::V0(
        DocumentCreateTransitionV0 {
            base: document_base(contract, "niceDocument", doc),
            entropy: [0u8; 32],
            data: BTreeMap::from([("name".to_string(), Value::Text(name.to_string()))]),
            prefunded_voting_balance: None,
        },
    )))
}

fn delete_document(contract: &DataContract, doc: u8) -> BatchedTransition {
    BatchedTransition::Document(DocumentTransition::Delete(DocumentDeleteTransition::V0(
        DocumentDeleteTransitionV0 {
            base: document_base(contract, "niceDocument", doc),
        },
    )))
}

fn transfer_document(contract: &DataContract, doc: u8, recipient: Identifier) -> BatchedTransition {
    BatchedTransition::Document(DocumentTransition::Transfer(
        DocumentTransferTransition::V0(DocumentTransferTransitionV0 {
            base: document_base(contract, "niceDocument", doc),
            revision: 2,
            recipient_owner_id: recipient,
        }),
    ))
}

fn transfer_token(token: Identifier, recipient: Identifier) -> BatchedTransition {
    BatchedTransition::Token(TokenTransition::Transfer(TokenTransferTransition::V0(
        TokenTransferTransitionV0 {
            base: TokenBaseTransition::V0(TokenBaseTransitionV0 {
                identity_contract_nonce: 0,
                token_contract_position: 0,
                data_contract_id: id(200),
                token_id: token,
                using_group_info: None,
            }),
            amount: 10,
            recipient_id: recipient,
            public_note: None,
            shared_encrypted_note: None,
            private_encrypted_note: None,
        },
    )))
}

fn name_equals(name: &str) -> drive::query::WhereClause {
    drive::query::WhereClause {
        field: "name".to_string(),
        operator: WhereOperator::Equal,
        value: Value::Text(name.to_string()),
    }
}

fn matches(filters: &ResolvedFilters, state_transition: &StateTransition) -> Option<FilterMatch> {
    filters.matches(state_transition, PlatformVersion::latest())
}

#[test]
fn should_match_identity_filters_by_role() {
    let contract = contract();
    let transfer = credit_transfer(id(1), id(2));

    let recipient = resolve(
        vec![StateTransitionFilter::Identities {
            identity_ids: vec![id(2)],
            role: Role::Recipient,
        }],
        &contract,
    )
    .unwrap();
    assert!(matches(&recipient, &transfer).is_some());

    let sender_side = resolve(
        vec![StateTransitionFilter::Identities {
            identity_ids: vec![id(2)],
            role: Role::Sender,
        }],
        &contract,
    )
    .unwrap();
    assert!(matches(&sender_side, &transfer).is_none());

    let any = resolve(
        vec![StateTransitionFilter::Identities {
            identity_ids: vec![id(1)],
            role: Role::Any,
        }],
        &contract,
    )
    .unwrap();
    assert!(matches(&any, &transfer).is_some());
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

#[test]
fn should_report_every_matching_filter_and_batch_position() {
    let contract = contract();
    let owner = id(1);
    let state_transition = batch(
        owner,
        vec![
            create_document(&contract, 10, "alice"),
            create_document(&contract, 11, "bob"),
            transfer_token(id(100), id(5)),
        ],
    );
    let filters = resolve(
        vec![
            StateTransitionFilter::Documents(
                DocumentFilter::new(contract.id())
                    .with_document_type("niceDocument")
                    .with_action(
                        DocumentActionMatch::new(DocumentAction::Create)
                            .with_new_document_where(name_equals("bob")),
                    ),
            ),
            StateTransitionFilter::Identities {
                identity_ids: vec![id(7)],
                role: Role::Any,
            },
            StateTransitionFilter::Tokens {
                token_ids: vec![id(100)],
                identity_ids: vec![id(5)],
                role: Role::Recipient,
            },
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
    let owner = id(1);
    let deleting = batch(owner, vec![delete_document(&contract, 10)]);

    let deletes = resolve(
        vec![StateTransitionFilter::Documents(
            DocumentFilter::new(contract.id())
                .with_action(DocumentActionMatch::new(DocumentAction::Delete)),
        )],
        &contract,
    )
    .unwrap();
    assert!(matches(&deletes, &deleting).is_some());

    let creates = resolve(
        vec![StateTransitionFilter::Documents(
            DocumentFilter::new(contract.id())
                .with_action(DocumentActionMatch::new(DocumentAction::Create)),
        )],
        &contract,
    )
    .unwrap();
    assert!(matches(&creates, &deleting).is_none());

    let someone_else = resolve(
        vec![StateTransitionFilter::Documents(
            DocumentFilter::new(contract.id()).with_batch_owner(id(2)),
        )],
        &contract,
    )
    .unwrap();
    assert!(matches(&someone_else, &deleting).is_none());
}

#[test]
fn should_match_a_specific_document_by_id_before_the_transition() {
    let contract = contract();
    let filters = resolve(
        vec![StateTransitionFilter::Documents(
            DocumentFilter::new(contract.id())
                .with_document_type("niceDocument")
                .with_action(
                    DocumentActionMatch::new(DocumentAction::Delete).with_original_document_where(
                        drive::query::WhereClause {
                            field: "$id".to_string(),
                            operator: WhereOperator::Equal,
                            value: Value::Text(id(10).to_string(Encoding::Base58)),
                        },
                    ),
                ),
        )],
        &contract,
    )
    .unwrap();
    assert!(matches(
        &filters,
        &batch(id(1), vec![delete_document(&contract, 10)])
    )
    .is_some());
    assert!(matches(
        &filters,
        &batch(id(1), vec![delete_document(&contract, 11)])
    )
    .is_none());
}

#[test]
fn should_match_document_and_token_recipients_with_identity_filters() {
    let contract = contract();
    let me = id(5);
    let filters = resolve(
        vec![StateTransitionFilter::Identities {
            identity_ids: vec![me],
            role: Role::Recipient,
        }],
        &contract,
    )
    .unwrap();
    assert!(matches(
        &filters,
        &batch(id(1), vec![transfer_document(&contract, 10, me)])
    )
    .is_some());
    assert!(matches(&filters, &batch(id(1), vec![transfer_token(id(100), me)])).is_some());
    assert!(matches(
        &filters,
        &batch(id(1), vec![transfer_token(id(100), id(6))])
    )
    .is_none());
}

#[test]
fn should_reject_clauses_on_the_original_document_other_than_id() {
    let contract = contract();
    let error = resolve(
        vec![StateTransitionFilter::Documents(
            DocumentFilter::new(contract.id())
                .with_document_type("niceDocument")
                .with_action(
                    DocumentActionMatch::new(DocumentAction::Delete)
                        .with_original_document_where(name_equals("bob")),
                ),
        )],
        &contract,
    )
    .unwrap_err();
    assert!(matches!(
        error,
        SubscriptionFilterError::InvalidFilter { index: 0, .. }
    ));
}

#[test]
fn should_reject_clauses_without_a_document_type() {
    let contract = contract();
    let error = resolve(
        vec![StateTransitionFilter::Documents(
            DocumentFilter::new(contract.id()).with_action(
                DocumentActionMatch::new(DocumentAction::Create)
                    .with_new_document_where(name_equals("bob")),
            ),
        )],
        &contract,
    )
    .unwrap_err();
    assert!(matches!(
        error,
        SubscriptionFilterError::InvalidFilter { .. }
    ));
}

#[test]
fn should_reject_unknown_contracts_and_document_types() {
    let contract = contract();
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
    assert!(resolve(
        vec![StateTransitionFilter::Documents(
            DocumentFilter::new(contract.id()).with_document_type("noSuchType")
        )],
        &contract
    )
    .is_err());
}

#[test]
fn should_enforce_filter_limits() {
    let contract = contract();
    let identities = |count: usize| StateTransitionFilter::Identities {
        identity_ids: (0..count).map(|i| id(i as u8)).collect(),
        role: Role::Any,
    };
    assert!(resolve(vec![], &contract).is_err());
    assert!(resolve(vec![identities(1); MAX_FILTERS + 1], &contract).is_err());
    assert!(resolve(vec![identities(0)], &contract).is_err());
    assert!(resolve(vec![identities(MAX_IDS_PER_FILTER + 1)], &contract).is_err());
    assert!(resolve(
        vec![StateTransitionFilter::Tokens {
            token_ids: vec![],
            identity_ids: vec![],
            role: Role::Any,
        }],
        &contract
    )
    .is_err());
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
                ),
        ),
        StateTransitionFilter::Addresses {
            addresses: vec![
                PlatformAddress::P2pkh([1u8; 20]),
                PlatformAddress::P2sh([2u8; 20]),
            ],
            role: Role::Recipient,
        },
        StateTransitionFilter::Tokens {
            token_ids: vec![id(100)],
            identity_ids: vec![],
            role: Role::Sender,
        },
        StateTransitionFilter::DataContracts {
            data_contract_ids: vec![contract.id()],
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

fn contract_update(contract: DataContract) -> StateTransition {
    use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
    use dpp::version::TryFromPlatformVersioned;
    StateTransition::DataContractUpdate(
        DataContractUpdateTransition::try_from_platform_versioned(
            (contract, 1),
            PlatformVersion::latest(),
        )
        .expect("update transition"),
    )
}

#[test]
fn should_rebind_document_filters_to_the_contract_an_update_carries() {
    use dpp::data_contract::accessors::v0::DataContractV0Setters;
    let contract = contract();
    let mut filters = resolve(
        vec![StateTransitionFilter::Documents(
            DocumentFilter::new(contract.id()).with_document_type("niceDocument"),
        )],
        &contract,
    )
    .unwrap();
    assert_eq!(
        filters.data_contract(contract.id()).unwrap().version(),
        contract.version()
    );

    let mut updated = (*contract).clone();
    updated.set_version(contract.version() + 1);
    let update = contract_update(updated);
    assert_eq!(filters.followed_data_contract(&update), Some(contract.id()));
    filters
        .follow(&update, PlatformVersion::latest())
        .expect("update converts");
    assert_eq!(
        filters.data_contract(contract.id()).unwrap().version(),
        contract.version() + 1
    );
    // Still matches documents of the type under the new version.
    let state_transition = batch(id(1), vec![create_document(&contract, 10, "bob")]);
    assert!(matches(&filters, &state_transition).is_some());
}

#[test]
fn should_ignore_updates_of_contracts_no_filter_is_bound_to() {
    use dpp::data_contract::accessors::v0::DataContractV0Setters;
    let contract = contract();
    let mut filters = resolve(
        vec![StateTransitionFilter::Identities {
            identity_ids: vec![id(7)],
            role: Role::Any,
        }],
        &contract,
    )
    .unwrap();
    let mut other = (*contract).clone();
    other.set_id(id(42));
    let update = contract_update(other);
    assert_eq!(filters.followed_data_contract(&update), None);
    filters
        .follow(&update, PlatformVersion::latest())
        .expect("nothing to follow");
}

#[test]
fn should_reject_a_wire_action_match_without_its_action() {
    use dapi_grpc::platform::v0::subscribe_to_state_transitions_request::{
        document_filter, state_transition_filter, DocumentFilter as ProtoDocumentFilter,
        StateTransitionFilter as ProtoStateTransitionFilter,
    };
    let filter = ProtoStateTransitionFilter {
        filter: Some(state_transition_filter::Filter::Documents(
            ProtoDocumentFilter {
                data_contract_id: id(1).to_vec(),
                document_type_name: Some("niceDocument".to_string()),
                actions: vec![document_filter::ActionMatch {
                    action: None,
                    ..Default::default()
                }],
                batch_owner_id: None,
            },
        )),
    };
    assert!(matches!(
        StateTransitionFilter::from_proto(3, filter),
        Err(SubscriptionFilterError::InvalidFilter { index: 3, .. })
    ));
}

fn rating_contract() -> Arc<DataContract> {
    use dpp::data_contract::DataContractFactory;
    use dpp::platform_value::platform_value;
    let documents = platform_value!({
        "rating": {
            "type": "object",
            "properties": {
                "stars": { "type": "integer", "minimum": 0, "maximum": 255, "position": 0 }
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

#[test]
fn should_group_range_bounds_given_with_different_integer_widths() {
    let contract = rating_contract();
    let bound = |operator, value| drive::query::WhereClause {
        field: "stars".to_string(),
        operator,
        value,
    };
    let filter = StateTransitionFilter::Documents(
        DocumentFilter::new(contract.id())
            .with_document_type("rating")
            .with_action(
                DocumentActionMatch::new(DocumentAction::Create)
                    .with_new_document_where(bound(
                        WhereOperator::GreaterThanOrEquals,
                        Value::I64(1),
                    ))
                    .with_new_document_where(bound(WhereOperator::LessThanOrEquals, Value::U64(5))),
            ),
    );
    // Natively, and after the wire round trip that keeps int64 and uint64 apart.
    let wire = StateTransitionFilter::from_proto(0, filter.to_proto().unwrap()).unwrap();
    for filter in [filter, wire] {
        let filters = resolve(vec![filter], &contract).expect("mixed-width bounds group");
        let rated = |stars: u64| {
            batch(
                id(2),
                vec![BatchedTransition::Document(DocumentTransition::Create(
                    DocumentCreateTransition::V0(DocumentCreateTransitionV0 {
                        base: document_base(&contract, "rating", 10),
                        entropy: [0u8; 32],
                        data: BTreeMap::from([("stars".to_string(), Value::U64(stars))]),
                        prefunded_voting_balance: None,
                    }),
                ))],
            )
        };
        assert!(matches(&filters, &rated(3)).is_some());
        assert!(matches(&filters, &rated(9)).is_none());
    }
}
