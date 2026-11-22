//! Tests of mocked Fetch trait implementations.

use super::common::{bootstrap_mock_sdk_to_latest, mock_data_contract, mock_document_type};
use dash_sdk::{
    platform::{DocumentQuery, DocumentQuerySdk, Fetch},
    Sdk, SdkBuilder,
};
use dpp::{
    data_contract::{
        accessors::v0::DataContractV0Getters,
        document_type::{
            accessors::DocumentTypeV0Getters, random_document::CreateRandomDocument, DocumentType,
        },
    },
    document::Document,
    identity::{accessors::IdentityGettersV0, IdentityV0},
    prelude::{DataContract, Identifier, Identity},
    version::PlatformVersion,
};

#[tokio::test]
/// Given some identity, when I fetch it using mock API, then I get the same identity
async fn test_mock_fetch_identity() {
    let mut sdk = Sdk::new_mock();

    let expected: Identity = Identity::from(IdentityV0::default());
    let query = expected.id();

    sdk.mock()
        .expect_fetch(query, Some(expected.clone()))
        .await
        .unwrap();

    let retrieved = Identity::fetch(&sdk, query)
        .await
        .unwrap()
        .expect("object should exist");

    assert_eq!(retrieved, expected);
}

#[tokio::test]
/// When I define mock expectation twice for the same request, second call ends with error
async fn test_mock_fetch_duplicate_expectation() {
    let mut sdk = Sdk::new_mock();

    let expected: Identity = Identity::from(IdentityV0::default());
    let expected2 =
        Identity::random_identity(3, Some(2), PlatformVersion::latest()).expect("random identity");

    let query = expected.id();

    sdk.mock()
        .expect_fetch(query, Some(expected.clone()))
        .await
        .expect("first expectation should be added correctly");

    sdk.mock()
        .expect_fetch(query, Some(expected2))
        .await
        .expect_err("conflicting expectation should fail");

    let retrieved = Identity::fetch(&sdk, query)
        .await
        .unwrap()
        .expect("object should exist");

    assert_eq!(retrieved, expected);
}

#[tokio::test]
/// Given some random identity ID, when I fetch it using mock API, then I get None
async fn test_mock_fetch_identity_not_found() {
    let mut sdk = Sdk::new_mock();

    let id = Identifier::random();

    sdk.mock()
        .expect_fetch(id, None as Option<Identity>)
        .await
        .unwrap();

    let retrieved = Identity::fetch(&sdk, id)
        .await
        .expect("fetch should succeed");

    assert!(retrieved.is_none());
}

/// Given some data contract, when I fetch it by ID, I get it.
#[tokio::test]
async fn test_mock_fetch_data_contract() {
    // `mock_data_contract` builds V2 document types that only decode at the latest
    // protocol version; the unpinned default floor would downgrade the type and
    // mismatch. A proven `fetch_current` bootstrap ratchets the auto-detect SDK up
    // to the network's latest version before the data contract round-trips, exactly
    // as production converges.
    let mut sdk = SdkBuilder::new_mock()
        .build()
        .expect("mock Sdk should be created");
    bootstrap_mock_sdk_to_latest(&mut sdk).await;

    let document_type: DocumentType = mock_document_type();
    let expected = mock_data_contract(Some(&document_type));
    let id = expected.id();

    sdk.mock()
        .expect_fetch(id, Some(expected.clone()))
        .await
        .unwrap();

    let retrieved = DataContract::fetch(&sdk, id)
        .await
        .unwrap()
        .expect("object should exist");
    assert_eq!(retrieved, expected);
}

/// Given some data contract, document type name and document, when I fetch expected document using mock Sdk, I get it.
#[tokio::test]
async fn test_mock_fetch_document() {
    use dpp::document::DocumentV0Getters;

    let mut sdk = Sdk::new_mock();
    let document_type: DocumentType = mock_document_type();
    let data_contract = mock_data_contract(Some(&document_type));

    let expected = document_type
        .random_document(None, sdk.version())
        .expect("document should be created");
    let document_id = expected.id();
    let document_type_name = document_type.name();

    // [DocumentQuery::new_with_data_contract_id] will fetch the data contract first, so we need to define an expectation for it.
    sdk.mock()
        .expect_fetch(data_contract.id(), Some(data_contract.clone()))
        .await
        .unwrap();

    let query =
        DocumentQuery::new_with_data_contract_id(&sdk, data_contract.id(), document_type_name)
            .await
            .expect("create document query")
            .with_document_id(&document_id);

    sdk.mock()
        .expect_fetch(query.clone(), Some(expected.clone()))
        .await
        .unwrap();

    let retrieved = Document::fetch(&sdk, query)
        .await
        .unwrap()
        .expect("identity should exist");

    assert_eq!(retrieved, expected);
}

#[tokio::test]
/// Given the fee pots of a contract, when I fetch them by the contract id using mock API, then
/// I get the same pots, a pot paid out in epoch 0 apart from one that never was
async fn should_fetch_mocked_contract_fee_pots_by_contract_id() {
    use dash_sdk::platform::contract_fee_pots::{
        ContractFeePotLastClaim, ContractFeePotState, ContractFeePots, ContractFeePotsQuery,
    };

    let mut sdk = Sdk::new_mock();

    let contract_id = Identifier::from([7u8; 32]);
    let expected = ContractFeePots {
        owner: ContractFeePotState {
            credits: 10_000_000,
            last_claim: None,
        },
        moderators: ContractFeePotState {
            credits: u64::MAX,
            last_claim: Some(ContractFeePotLastClaim {
                epoch_index: 0,
                time_ms: 1_700_000_000_000,
                claimant_id: Identifier::from([9u8; 32]),
            }),
        },
    };

    sdk.mock()
        .expect_fetch(contract_id, Some(expected))
        .await
        .unwrap();

    // The contract id and the named query build the same request, so either finds the pots.
    let by_id = ContractFeePots::fetch(&sdk, contract_id)
        .await
        .unwrap()
        .expect("pots should exist");
    let by_query = ContractFeePots::fetch(&sdk, ContractFeePotsQuery { contract_id })
        .await
        .unwrap()
        .expect("pots should exist");

    assert_eq!(by_id, expected);
    assert_eq!(by_query, expected);
}

#[tokio::test]
/// Given the moderation action counts of an elected contract, when I fetch them by the contract
/// id or the named query using mock API, then I get the same counts back
async fn should_fetch_mocked_moderation_action_counts_by_contract_id() {
    use dash_sdk::platform::contract_moderation::{
        ContractModerationActionCounts, ContractModerationActionCountsQuery,
    };
    use std::collections::BTreeMap;

    let mut sdk = Sdk::new_mock();

    let contract_id = Identifier::from([7u8; 32]);
    let expected = ContractModerationActionCounts(BTreeMap::from([
        (Identifier::from([1u8; 32]), 3),
        (Identifier::from([2u8; 32]), u32::MAX),
    ]));

    sdk.mock()
        .expect_fetch(contract_id, Some(expected.clone()))
        .await
        .unwrap();

    let by_id = ContractModerationActionCounts::fetch(&sdk, contract_id)
        .await
        .unwrap()
        .expect("counts should exist");
    let by_query = ContractModerationActionCounts::fetch(
        &sdk,
        ContractModerationActionCountsQuery { contract_id },
    )
    .await
    .unwrap()
    .expect("counts should exist");

    assert_eq!(by_id, expected);
    assert_eq!(by_query, expected);
}

#[tokio::test]
/// Given a page of team actions, when I fetch it using mock API, then every action comes back
/// with its approval count
async fn should_fetch_mocked_team_actions_with_their_approval_counts() {
    use dash_sdk::platform::contract_moderation::{
        ContractTeamAction, ContractTeamActionEntry, ContractTeamActionEvent, ContractTeamActions,
        ContractTeamActionsPageQuery, GroupActionStatus,
    };
    use dpp::data_contract::config::moderation::ContractModerationReason;

    let mut sdk = Sdk::new_mock();

    let query = ContractTeamActionsPageQuery::new(
        Identifier::from([7u8; 32]),
        GroupActionStatus::ActionActive,
        PlatformVersion::latest(),
    );
    let entry = |seed: u8, approval_count: u32| ContractTeamActionEntry {
        action_id: Identifier::from([seed; 32]),
        action: ContractTeamAction {
            proposer_id: Identifier::from([seed.wrapping_add(1); 32]),
            proposed_at: 1_700_000_000_000,
            event: ContractTeamActionEvent::DeleteSettledDocument {
                document_type_name: "post".to_string(),
                document_id: Identifier::from([seed.wrapping_add(2); 32]),
                document_last_modified_at: 1_600_000_000_000,
                document_revision: Some(2),
                reason: ContractModerationReason::from_text("doxxing"),
            },
        },
        approval_count,
    };
    let expected = ContractTeamActions(vec![entry(1, 1), entry(3, 4)]);

    sdk.mock()
        .expect_fetch(query.clone(), Some(expected.clone()))
        .await
        .unwrap();

    let fetched = ContractTeamActions::fetch(&sdk, query)
        .await
        .unwrap()
        .expect("actions should exist");

    assert_eq!(fetched, expected);
}

/// A mock contract declaring `moderation`, or unmoderated for `None`.
fn mock_contract_with_moderation(
    moderation: Option<dpp::data_contract::config::moderation::ContractModerationConfig>,
) -> DataContract {
    use dpp::data_contract::accessors::v0::DataContractV0Setters;

    let mut contract = mock_data_contract(Some(&mock_document_type()));
    let config = contract.config().clone().with_moderation(moderation);
    contract.set_config(config);
    contract
}

#[tokio::test]
/// Given only the id of a moderated contract, when I build an identity's status query from it,
/// then the query names every list the contract keeps and no other
async fn should_build_a_moderation_status_query_over_the_lists_a_contract_keeps() {
    use dash_sdk::platform::contract_moderation::{
        ContractModerationList, ContractModerationStatusQuery,
    };
    use dpp::data_contract::config::moderation::{ContractModerationConfig, ContractModerators};

    let mut sdk = Sdk::new_mock();
    bootstrap_mock_sdk_to_latest(&mut sdk).await;

    let contract = mock_contract_with_moderation(Some(ContractModerationConfig {
        banlist: true,
        suspensions: false,
        moderators: ContractModerators::ContractOwner,
        warnings: true,
    }));
    let contract_id = contract.id();
    let identity_id = Identifier::from([3u8; 32]);
    sdk.mock()
        .expect_fetch(contract_id, Some(contract))
        .await
        .unwrap();

    let query = ContractModerationStatusQuery::for_contract_id(&sdk, contract_id, identity_id)
        .await
        .expect("a moderated contract has a status query");

    assert_eq!(
        query,
        ContractModerationStatusQuery {
            contract_id,
            identity_id,
            lists: vec![
                ContractModerationList::Banlist,
                ContractModerationList::Warnings
            ],
        }
    );
}

#[tokio::test]
/// Given the id of a contract with no list to read (not moderated, moderated without a list, or
/// absent), when I build a status query from it, then it is refused before any status request
async fn should_refuse_a_moderation_status_query_for_a_contract_without_lists() {
    use dash_sdk::platform::contract_moderation::ContractModerationStatusQuery;
    use dash_sdk::Error;
    use dpp::data_contract::config::moderation::{ContractModerationConfig, ContractModerators};

    let mut sdk = Sdk::new_mock();
    bootstrap_mock_sdk_to_latest(&mut sdk).await;
    let identity_id = Identifier::from([3u8; 32]);

    let unmoderated = mock_contract_with_moderation(None);
    let unmoderated_id = unmoderated.id();
    sdk.mock()
        .expect_fetch(unmoderated_id, Some(unmoderated))
        .await
        .unwrap();
    let refused = ContractModerationStatusQuery::for_contract_id(&sdk, unmoderated_id, identity_id)
        .await
        .expect_err("an unmoderated contract has no status to read");
    assert!(
        matches!(&refused, Error::InvalidRequest(reason) if reason.contains("is not moderated")),
        "{refused}"
    );

    let listless = mock_contract_with_moderation(Some(ContractModerationConfig {
        banlist: false,
        suspensions: false,
        moderators: ContractModerators::ContractOwner,
        warnings: false,
    }));
    let listless_id = listless.id();
    sdk.mock()
        .expect_fetch(listless_id, Some(listless))
        .await
        .unwrap();
    let refused = ContractModerationStatusQuery::for_contract_id(&sdk, listless_id, identity_id)
        .await
        .expect_err("a contract that keeps no list has no status to read");
    assert!(
        matches!(&refused, Error::InvalidRequest(reason) if reason.contains("keeps no moderation list")),
        "{refused}"
    );

    let absent_id = Identifier::from([9u8; 32]);
    sdk.mock()
        .expect_fetch(absent_id, None as Option<DataContract>)
        .await
        .unwrap();
    let refused = ContractModerationStatusQuery::for_contract_id(&sdk, absent_id, identity_id)
        .await
        .expect_err("a contract the network does not hold has no status to read");
    assert!(
        matches!(&refused, Error::MissingDependency(what, _) if what == "DataContract"),
        "{refused}"
    );
}

/// A removal record of document `[seed; 32]` keeping `kept_fields`.
fn mock_removal_entry(
    seed: u8,
    kept_fields: Vec<u8>,
) -> dash_sdk::platform::contract_moderation::ContractDocumentRemovalEntry {
    use dash_sdk::platform::contract_moderation::{
        ContractDocumentRemoval, ContractDocumentRemovalEntry,
    };
    use dpp::data_contract::config::moderation::ContractModerationReason;

    ContractDocumentRemovalEntry {
        document_id: Identifier::from([seed; 32]),
        removal: ContractDocumentRemoval {
            document_owner_id: Identifier::from([20u8; 32]),
            moderator_id: Identifier::from([21u8; 32]),
            reason: ContractModerationReason::from_text("off topic"),
            removed_at: 1_700_000_000_000,
            document_hash: [0xab; 32],
            restoration: None,
            kept_fields,
        },
    }
}

#[tokio::test]
/// Given removal records that keep no values, when I read them with their kept values, then
/// each reads as no values and the contract is never fetched (the mock holds no contract)
async fn should_read_removals_keeping_no_values_without_fetching_the_contract() {
    use dash_sdk::platform::contract_moderation::{
        ContractDocumentRemovals, ContractDocumentRemovalsPageQuery,
    };
    use std::collections::BTreeMap;

    let mut sdk = Sdk::new_mock();
    bootstrap_mock_sdk_to_latest(&mut sdk).await;

    let query = ContractDocumentRemovalsPageQuery::new(
        Identifier::from([1u8; 32]),
        "post".to_string(),
        sdk.version(),
    );
    let page = ContractDocumentRemovals(vec![mock_removal_entry(3, vec![])]);
    sdk.mock()
        .expect_fetch(query.clone(), Some(page.clone()))
        .await
        .unwrap();

    let read = query
        .fetch_with_kept_values(&sdk)
        .await
        .expect("records keeping nothing need no contract")
        .expect("the records are there");

    assert_eq!(read.removals, vec![(page.0[0].clone(), BTreeMap::new())]);
    assert_eq!(read.next, None);
}

#[tokio::test]
/// Given a removal record keeping a value of its document, when I read it with its kept values,
/// then the value reads under the document's type, and the page's cursor is rs-sdk's
async fn should_read_the_values_a_removal_record_keeps_under_its_document_type() {
    use dash_sdk::platform::contract_moderation::{
        ContractDocumentRemovals, ContractDocumentRemovalsPageQuery,
    };
    use dpp::data_contract::config::moderation::{
        encode_kept_fields, ContractModerationConfig, ContractModerators,
    };
    use dpp::data_contract::config::DataContractConfig;
    use dpp::data_contract::document_type::DocumentType;
    use dpp::document::DocumentV0;
    use dpp::platform_value::{platform_value, Value};
    use std::collections::BTreeMap;

    let mut sdk = Sdk::new_mock();
    bootstrap_mock_sdk_to_latest(&mut sdk).await;
    let platform_version = PlatformVersion::latest();

    let moderation = ContractModerationConfig {
        banlist: false,
        suspensions: false,
        moderators: ContractModerators::ContractOwner,
        warnings: false,
    };
    let config = DataContractConfig::default_for_version(platform_version)
        .expect("a default config")
        .with_moderation(Some(moderation.clone()));
    let mut contract = mock_contract_with_moderation(Some(moderation));
    let post = DocumentType::try_from_schema(
        contract.id(),
        1,
        config.version(),
        "post",
        platform_value!({
            "type": "object",
            "properties": {
                "hashtag": { "type": "string", "maxLength": 61, "position": 0 },
            },
            "additionalProperties": false,
            "moderatorAbilities": {
                "delete": true,
                "deleteKeepsFields": ["hashtag"],
            },
        }),
        None,
        &BTreeMap::new(),
        &config,
        true,
        &mut vec![],
        platform_version,
    )
    .expect("a post type keeping its hashtag");
    let kept_fields = encode_kept_fields(
        &Document::V0(DocumentV0 {
            id: Identifier::from([3u8; 32]),
            properties: BTreeMap::from([("hashtag".to_string(), Value::Text("dash".into()))]),
            ..Default::default()
        }),
        post.as_ref(),
    )
    .expect("encodes");
    contract
        .document_types_mut()
        .insert("post".to_string(), post);
    let contract_id = contract.id();

    let query =
        ContractDocumentRemovalsPageQuery::new(contract_id, "post".to_string(), sdk.version())
            .with_limit(1);
    let page = ContractDocumentRemovals(vec![mock_removal_entry(3, kept_fields)]);
    sdk.mock()
        .expect_fetch(query.clone(), Some(page.clone()))
        .await
        .unwrap();
    sdk.mock()
        .expect_fetch(contract_id, Some(contract))
        .await
        .unwrap();

    let read = query
        .fetch_with_kept_values(&sdk)
        .await
        .expect("reads")
        .expect("the records are there");

    assert_eq!(
        read.removals,
        vec![(
            page.0[0].clone(),
            BTreeMap::from([("hashtag".to_string(), Value::Text("dash".into()))])
        )]
    );
    // A full page has a page after it, the one rs-sdk's `after` gives
    assert_eq!(read.next, query.after(&page));
    assert!(read.next.is_some());
}
