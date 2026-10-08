use super::offsets::{history_contract, search_kind};
use super::*;
use dpp::data_contract::accessors::v0::DataContractV0Setters;
use dpp::data_contract::document_type::random_document::CreateRandomDocument;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::{DocumentV0Getters, DocumentV0Setters};
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeySettersV0;
use dpp::identity::{Identity, IdentityPublicKey, Purpose, SecurityLevel};
use dpp::serialization::PlatformDeserializableUntrusted;
use dpp::tests::json_document::json_document_to_document;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use dpp::voting::vote_info_storage::contested_document_vote_poll_stored_info::ContestedDocumentVotePollStoredInfo;
use dpp::voting::vote_polls::VotePoll;
use dpp::voting::votes::resource_vote::{v0::ResourceVoteV0, ResourceVote};
use drive::drive::identity::key::fetch::KeyKindRequestType;
use drive::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use drive::drive::votes::storage_form::contested_document_resource_storage_form::ContestedDocumentResourceVoteStorageForm;
use drive::query::contested_resource_votes_given_by_identity_query::ContestedResourceVotesGivenByIdentityQuery;
use drive::query::contract_lookup_fn_for_contract;
use drive::util::object_size_info::DataContractOwnedResolvedInfo;
use drive::util::object_size_info::DocumentInfo::DocumentRefInfo;
use drive::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use drive::util::storage_flags::StorageFlags;
use std::collections::BTreeMap;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_preserve_supported_offset_pages_and_original_proof_roots() {
    for version in [
        PlatformVersion::get(12).unwrap(),
        PlatformVersion::get(13).unwrap(),
        PlatformVersion::latest(),
    ] {
        let fixture = QueryFixture::new_with_version(version);
        let mut identity =
            Identity::random_identity(5, Some(91823), version).expect("seeded identity");
        for (id, key) in identity.public_keys_mut() {
            key.set_purpose(Purpose::AUTHENTICATION);
            key.set_security_level(if *id == 0 {
                SecurityLevel::MASTER
            } else {
                SecurityLevel::CRITICAL
            });
        }
        let identity_id = identity.id().to_buffer();
        let expected_keys = identity.public_keys().clone();
        fixture
            .platform
            .drive
            .add_new_identity(identity, false, &BlockInfo::default(), true, None, version)
            .expect("insert identity");
        let (contract_id, _) = fixture.store_contract();
        let mut updated = fixture
            .platform
            .drive
            .fetch_contract(contract_id.to_buffer(), None, None, None, version)
            .value
            .expect("fetch contract")
            .expect("stored contract")
            .contract
            .clone();
        let original_bytes = updated
            .serialize_to_bytes_with_platform_version(version)
            .expect("original bytes");
        let original_contract = updated.clone();
        updated.set_version(updated.version() + 1);
        assert_ne!(
            original_bytes,
            updated
                .serialize_to_bytes_with_platform_version(version)
                .expect("updated bytes")
        );
        fixture
            .platform
            .drive
            .apply_contract(
                &updated,
                BlockInfo {
                    time_ms: 2000,
                    height: 11,
                    core_height: 21,
                    ..Default::default()
                },
                true,
                None,
                None,
                version,
            )
            .expect("second historical contract");
        let history_contract = history_contract(&fixture);
        let document_type = history_contract
            .document_type_for_name("profile")
            .expect("profile type");
        let mut history_document = json_document_to_document(
            "../rs-drive/tests/supporting_files/contract/dashpay/profile0.json",
            Some(Identifier::new([9; 32])),
            document_type,
            version,
        )
        .expect("profile fixture");
        // Fixture JSON carries routing metadata; only schema properties are stored.
        history_document.properties_mut().remove("$dataContractId");
        history_document.properties_mut().remove("$type");
        history_document.set_revision(Some(1));
        let document_id = history_document.id().to_buffer();
        let mut expected_documents = BTreeMap::new();
        for (time, display_name) in [
            (1000, "Offset one"),
            (2000, "Offset two"),
            (3000, "Offset three"),
        ] {
            history_document.set("displayName", display_name.into());
            expected_documents.insert(time, history_document.clone());
            fixture
                .platform
                .drive
                .add_document_for_contract(
                    DocumentAndContractInfo {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentRefInfo((
                                &history_document,
                                StorageFlags::optional_default_as_cow(),
                            )),
                            owner_id: None,
                        },
                        contract: &history_contract,
                        document_type,
                    },
                    true,
                    BlockInfo::default_with_time(time),
                    true,
                    None,
                    version,
                    None,
                )
                .expect("insert historical profile");
        }
        let dpns = dpp::tests::fixtures::get_dpns_data_contract_fixture(
            Some(Identifier::new([33; 32])),
            0,
            version.protocol_version,
        )
        .data_contract_owned();
        fixture
            .platform
            .drive
            .apply_contract(&dpns, BlockInfo::default(), true, None, None, version)
            .expect("store DPNS");
        let mut expected_votes = BTreeMap::new();
        let domain_type = dpns.document_type_for_name("domain").expect("domain type");
        for (n, label) in [(0, "offset-alpha"), (1, "offset-beta"), (2, "offset-gamma")] {
            let owner = Identifier::new([40 + n; 32]);
            let mut document = domain_type
                .random_document(Some(u64::from(n)), version)
                .expect("domain document");
            document.set_owner_id(owner);
            document.set("parentDomainName", "dash".into());
            document.set("normalizedParentDomainName", "dash".into());
            document.set("label", label.into());
            document.set("normalizedLabel", label.into());
            document.set("records.identity", owner.into());
            document.set("subdomainRules.allowSubdomains", false.into());
            let poll = ContestedDocumentResourceVotePollWithContractInfo {
                contract: DataContractOwnedResolvedInfo::OwnedDataContract(dpns.clone()),
                document_type_name: "domain".to_owned(),
                index_name: "parentNameAndLabel".to_owned(),
                index_values: vec!["dash".into(), label.into()],
            };
            fixture
                .platform
                .drive
                .add_contested_document(
                    OwnedDocumentInfo {
                        document_info: DocumentRefInfo((
                            &document,
                            StorageFlags::optional_default_as_cow(),
                        )),
                        owner_id: Some(owner.to_buffer()),
                    },
                    poll.clone(),
                    false,
                    Some(
                        ContestedDocumentVotePollStoredInfo::new(BlockInfo::default(), version)
                            .expect("poll info"),
                    ),
                    &BlockInfo::default(),
                    true,
                    None,
                    version,
                )
                .expect("seed contested poll");
            fixture
                .platform
                .drive
                .register_contested_resource_identity_vote(
                    identity_id,
                    1,
                    poll.clone(),
                    ResourceVoteChoice::Abstain,
                    None,
                    &BlockInfo::default(),
                    None,
                    version,
                )
                .expect("register identity vote");
            let poll: dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll = poll.into();
            let poll = VotePoll::ContestedDocumentResourceVotePoll(poll);
            let id = poll.unique_id().expect("poll ID");
            expected_votes.insert(
                id,
                ResourceVote::V0(ResourceVoteV0 {
                    vote_poll: poll,
                    resource_vote_choice: ResourceVoteChoice::Abstain,
                }),
            );
        }
        assert_eq!(expected_votes.len(), 3);

        let stored_root = fixture
            .platform
            .drive
            .grove
            .root_hash(None, &version.drive.grove_version)
            .value
            .expect("stored root");
        let lookup = contract_lookup_fn_for_contract(Arc::new(dpns.clone()));
        let (mut client, server) = fixture.client();
        for offset in [None, Some(0u32), Some(1)] {
            for prove in [false, true] {
                for specific in [false, true] {
                    if prove && offset == Some(1) && !specific {
                        continue;
                    }
                    let drive_kind = if specific {
                        KeyRequestType::SpecificKeys(vec![0, 2, 4])
                    } else {
                        KeyRequestType::AllKeys
                    };
                    let wire_kind = if specific {
                        KeyRequestKind::SpecificKeys(wire::SpecificKeys {
                            key_ids: vec![0, 2, 4],
                        })
                    } else {
                        KeyRequestKind::AllKeys(wire::AllKeys {})
                    };
                    let expected_ids = if specific {
                        vec![0, 2]
                    } else if offset == Some(1) {
                        vec![1, 2]
                    } else {
                        vec![0, 1]
                    };
                    let expected = expected_ids
                        .into_iter()
                        .map(|id| expected_keys[&id].clone())
                        .collect::<Vec<_>>();
                    let request = GetIdentityKeysRequestV0 {
                        identity_id: identity_id.to_vec(),
                        request_type: Some(wire::KeyRequestType {
                            request: Some(wire_kind),
                        }),
                        limit: Some(2),
                        offset,
                        prove,
                    };
                    let response = client
                        .get_identity_keys(wire::GetIdentityKeysRequest::from(request))
                        .await
                        .expect("supported key page")
                        .into_inner();
                    let Some(get_identity_keys_response::Version::V0(v0)) = response.version else {
                        panic!("key V0")
                    };
                    match v0.result.expect("key result") {
                        get_identity_keys_response_v0::Result::Keys(keys) => {
                            assert!(!prove);
                            let actual = keys
                                .keys_bytes
                                .iter()
                                .map(|b| {
                                    IdentityPublicKey::deserialize_from_bytes_untrusted(b)
                                        .expect("key payload")
                                })
                                .collect::<Vec<_>>();
                            assert_eq!(actual, expected);
                        }
                        get_identity_keys_response_v0::Result::Proof(proof) => {
                            assert!(prove);
                            let original = IdentityKeysRequest {
                                identity_id,
                                request_type: drive_kind,
                                limit: Some(2),
                                offset: offset.map(|o| o as u16),
                            };
                            let (root, identity) = Drive::verify_identity_keys_by_identity_id(
                                &proof.grovedb_proof,
                                original,
                                false,
                                false,
                                false,
                                version,
                            )
                            .expect("original selected key proof");
                            assert_eq!(root, stored_root);
                            assert_eq!(
                                identity
                                    .expect("stored identity")
                                    .loaded_public_keys
                                    .into_values()
                                    .collect::<Vec<_>>(),
                                expected
                            );
                        }
                    }
                }
                if prove && offset == Some(1) {
                    continue;
                }
                let expected_contracts = if offset == Some(1) {
                    BTreeMap::from([(1000, original_contract.clone())])
                } else {
                    BTreeMap::from([(1000, original_contract.clone()), (2000, updated.clone())])
                };
                let response = client
                    .get_data_contract_history(wire::GetDataContractHistoryRequest::from(
                        GetDataContractHistoryRequestV0 {
                            id: contract_id.to_vec(),
                            limit: Some(2),
                            offset,
                            start_at_ms: 0,
                            prove,
                        },
                    ))
                    .await
                    .expect("supported contract history")
                    .into_inner();
                let Some(get_data_contract_history_response::Version::V0(v0)) = response.version
                else {
                    panic!("contract history V0")
                };
                match v0.result.expect("history result") {
                    get_data_contract_history_response_v0::Result::DataContractHistory(history) => {
                        assert!(!prove);
                        let expected = expected_contracts
                            .iter()
                            .map(|(date, contract)| {
                                (
                                    *date,
                                    contract
                                        .serialize_to_bytes_with_platform_version(version)
                                        .expect("original contract bytes"),
                                )
                            })
                            .collect::<Vec<_>>();
                        assert_eq!(
                            history
                                .data_contract_entries
                                .into_iter()
                                .map(|e| (e.date, e.value))
                                .collect::<Vec<_>>(),
                            expected
                        );
                    }
                    get_data_contract_history_response_v0::Result::Proof(proof) => {
                        assert!(prove);
                        let (root, contracts) = Drive::verify_contract_history(
                            &proof.grovedb_proof,
                            contract_id.to_buffer(),
                            0,
                            Some(2),
                            offset.map(|o| o as u16),
                            version,
                        )
                        .expect("original contract history proof");
                        assert_eq!(root, stored_root);
                        assert_eq!(contracts.expect("history"), expected_contracts);
                    }
                }
                let expected_page = expected_documents
                    .iter()
                    .skip(offset.unwrap_or(0) as usize)
                    .take(2)
                    .map(|(time, document)| (*time, document.clone()))
                    .collect::<BTreeMap<_, _>>();
                let response = client
                    .get_document_history(wire::GetDocumentHistoryRequest::from(
                        wire::get_document_history_request::GetDocumentHistoryRequestV0 {
                            data_contract_id: history_contract.id().to_vec(),
                            document_type_name: "profile".to_owned(),
                            document_id: document_id.to_vec(),
                            limit: Some(2),
                            offset,
                            start_at_ms: 0,
                            prove,
                        },
                    ))
                    .await
                    .expect("supported document history")
                    .into_inner();
                let Some(wire::get_document_history_response::Version::V0(v0)) = response.version
                else {
                    panic!("document history V0")
                };
                match v0.result.expect("document result") {
   wire::get_document_history_response::get_document_history_response_v0::Result::DocumentHistory(history)=>{assert!(!prove);assert_eq!(history.document_entries.into_iter().map(|e|(e.date,e.value)).collect::<Vec<_>>(),expected_page.iter().map(|(time,document)|(*time,document.serialize(document_type,&history_contract,version).expect("original document bytes"))).collect::<Vec<_>>());},
   wire::get_document_history_response::get_document_history_response_v0::Result::Proof(proof)=>{assert!(prove);let(root,documents)=Drive::verify_document_history(&proof.grovedb_proof,history_contract.id().to_buffer(),"profile",document_type,document_id,0,Some(2),offset.map(|o|o as u16),version).expect("original document history proof");assert_eq!(root,stored_root);assert_eq!(documents.expect("history"),expected_page);},
  }
                let expected_vote_page = expected_votes
                    .iter()
                    .skip(offset.unwrap_or(0) as usize)
                    .take(2)
                    .map(|(id, vote)| (*id, vote.clone()))
                    .collect::<BTreeMap<_, _>>();
                let response = client
                    .get_contested_resource_identity_votes(
                        wire::GetContestedResourceIdentityVotesRequest::from(
                            GetContestedResourceIdentityVotesRequestV0 {
                                identity_id: identity_id.to_vec(),
                                limit: Some(2),
                                offset,
                                order_ascending: true,
                                start_at_vote_poll_id_info: None,
                                prove,
                            },
                        ),
                    )
                    .await
                    .expect("supported identity votes")
                    .into_inner();
                let Some(wire::get_contested_resource_identity_votes_response::Version::V0(v0)) =
                    response.version
                else {
                    panic!("votes V0")
                };
                match v0.result.expect("votes result") {
   wire::get_contested_resource_identity_votes_response::get_contested_resource_identity_votes_response_v0::Result::Votes(v)=>{assert!(!prove);let actual=v.contested_resource_identity_votes.into_iter().map(|vote|{let choice=vote.vote_choice.expect("choice");ContestedDocumentResourceVoteStorageForm {contract_id:vote.contract_id.try_into().expect("contract ID"),document_type_name:vote.document_type_name,index_values:vote.serialized_index_storage_values,resource_vote_choice:(choice.vote_choice_type,choice.identity_id).try_into().expect("choice conversion")}.resolve_with_contract(&dpns,version).expect("vote")}).collect::<Vec<_>>();assert_eq!(actual,expected_vote_page.into_values().collect::<Vec<_>>());assert_eq!(v.finished_results,offset==Some(1));},
   wire::get_contested_resource_identity_votes_response::get_contested_resource_identity_votes_response_v0::Result::Proof(proof)=>{assert!(prove);let query=ContestedResourceVotesGivenByIdentityQuery {identity_id:Identifier::new(identity_id),limit:Some(2),offset:offset.map(|o|o as u16),start_at:None,order_ascending:true};let(root,votes):(_,BTreeMap<Identifier,ResourceVote>)=query.verify_identity_votes_given_proof(&proof.grovedb_proof,lookup.as_ref(),version).expect("original identity votes proof");assert_eq!(root,stored_root);assert_eq!(votes,expected_vote_page);},
  }
            }
        }
        for offset in [None, Some(0u16)] {
            let response = client
                .get_identity_keys(wire::GetIdentityKeysRequest::from(
                    GetIdentityKeysRequestV0 {
                        identity_id: identity_id.to_vec(),
                        request_type: Some(wire::KeyRequestType {
                            request: Some(search_kind()),
                        }),
                        limit: Some(2),
                        offset: offset.map(u32::from),
                        prove: true,
                    },
                ))
                .await
                .expect("unchanged current-key proof")
                .into_inner();
            let Some(get_identity_keys_response::Version::V0(v0)) = response.version else {
                panic!("V0")
            };
            let Some(get_identity_keys_response_v0::Result::Proof(proof)) = v0.result else {
                panic!("proof")
            };
            let query = IdentityKeysRequest {
                identity_id,
                request_type: KeyRequestType::SearchKey(BTreeMap::from([(
                    0,
                    BTreeMap::from([
                        (0, KeyKindRequestType::CurrentKeyOfKindRequest),
                        (1, KeyKindRequestType::CurrentKeyOfKindRequest),
                    ]),
                )])),
                limit: Some(2),
                offset,
            };
            let (root, identity) = Drive::verify_identity_keys_by_identity_id(
                &proof.grovedb_proof,
                query,
                false,
                false,
                false,
                version,
            )
            .expect("original current-key proof");
            assert_eq!(root, stored_root);
            assert!(identity
                .expect("stored identity")
                .loaded_public_keys
                .is_empty());
        }
        server.abort();
        assert_eq!(
            stored_root,
            fixture
                .platform
                .drive
                .grove
                .root_hash(None, &version.drive.grove_version)
                .value
                .expect("unchanged root")
        );
    }
}
