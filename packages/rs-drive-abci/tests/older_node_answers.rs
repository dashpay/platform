//! What a 4.1 node answers to requests that use fields protocol 14 added, and why the SDK
//! must not send them before protocol 14 is verified.
//!
//! A 4.1 node decodes requests with the 4.1 protobuf types, which drop the fields they do
//! not know, and answers the request that is left: the plain page of a composite or chained
//! documents query, or the credit pool for a token's shielded pool. The node is emulated
//! here by this Drive at protocol 13 answering the request without those fields, signed by
//! a test quorum over the genuine protocol 13 root. The SDK verifies the answer against
//! the request it sent and refuses it, at protocol 13 rules and at 14, and the node that
//! sent it is banned. So every such request declares that it needs protocol 14
//! (`TransportRequest::needs_protocol_version`), which keeps the SDK from sending it until a
//! verified answer shows the network runs protocol 14.

use dapi_grpc::platform::v0::get_documents_request::Version as RequestVersion;
use dapi_grpc::platform::v0::get_documents_response::{
    get_documents_response_v1, Version as ResponseVersion,
};
use dapi_grpc::platform::v0::{
    get_most_recent_shielded_anchor_request, get_most_recent_shielded_anchor_response,
    get_shielded_anchors_request, get_shielded_anchors_response,
    get_shielded_encrypted_notes_request, get_shielded_encrypted_notes_response,
    get_shielded_notes_count_request, get_shielded_notes_count_response,
    get_shielded_nullifiers_request, get_shielded_nullifiers_response,
    get_shielded_pool_state_request, get_shielded_pool_state_response, GetDocumentsRequest,
    GetDocumentsResponse, GetMostRecentShieldedAnchorRequest, GetShieldedAnchorsRequest,
    GetShieldedEncryptedNotesRequest, GetShieldedNotesCountRequest, GetShieldedNullifiersRequest,
    GetShieldedPoolStateRequest, Proof, ResponseMetadata,
};
use dash_platform_queries::documents::chained_document_query::ChainedDocumentQuery;
use dash_platform_queries::documents::composite_document_query::CompositeSubQuery;
use dash_platform_queries::documents::document_query::DocumentQuery;
use dpp::block::block_info::BlockInfo;
use dpp::bls_signatures::{Bls12381G2Impl, SecretKey, SignatureSchemes};
use dpp::dashcore::Network;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::{DataContract, DataContractFactory, TokenConfiguration};
use dpp::document::{Document, DocumentV0};
use dpp::identifier::Identifier;
use dpp::platform_value::{platform_value, Value};
use dpp::prelude::CoreBlockHeight;
use dpp::tests::json_document::json_document_to_contract;
use dpp::version::{PlatformVersion, TryFromPlatformVersioned};
use drive::query::{WhereClause, WhereOperator};
use drive::util::object_size_info::DocumentInfo::DocumentRefInfo;
use drive::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use drive_abci::config::PlatformConfig;
use drive_abci::platform_types::platform_state::{PlatformState, PlatformStateV0Methods};
use drive_abci::rpc::core::MockCoreRPCLike;
use drive_abci::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
use drive_proof_verifier::types::{
    Documents, MostRecentShieldedAnchor, ShieldedAnchors, ShieldedEncryptedNotes,
    ShieldedNotesCount, ShieldedNullifierStatuses, ShieldedPoolState,
};
use drive_proof_verifier::{
    ChainedDocuments, CompositeDocuments, ContextProvider, ContextProviderError, FromProof,
};
use rs_dapi_client::transport::TransportRequest;
use std::sync::Arc;
use tenderdash_abci::proto::types::{CanonicalVote, SignedMsgType, StateId};
use tenderdash_abci::signatures::{Hashable, Signable};

const QUORUM_TYPE: u32 = 106;
const QUORUM_HASH: [u8; 32] = [2; 32];
const YAPPR_CONTRACT_PATH: &str =
    "../rs-drive/tests/supporting_files/contract/yappr-likes/yappr-likes-contract.json";

fn v13() -> &'static PlatformVersion {
    PlatformVersion::get(13).expect("protocol 13")
}

fn v14() -> &'static PlatformVersion {
    PlatformVersion::get(14).expect("protocol 14")
}

fn key() -> SecretKey<Bls12381G2Impl> {
    SecretKey::from_hash(b"older-node-answers-quorum-fixture")
}

/// Knows no contracts or tokens, and answers the test quorum's public key.
struct Provider;

impl ContextProvider for Provider {
    fn get_data_contract(
        &self,
        _: &Identifier,
        _: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        Ok(None)
    }
    fn get_token_configuration(
        &self,
        _: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        Ok(None)
    }
    fn get_quorum_public_key(
        &self,
        _: u32,
        _: [u8; 32],
        _: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        let bytes: Vec<u8> = (&key().public_key()).into();
        Ok(bytes.try_into().expect("compressed quorum public key"))
    }
    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        Ok(1)
    }
}

/// Signs `proof` and `metadata` over `root` as the quorum signs a block at height 1000.
fn sign(proof: &mut Proof, metadata: &mut ResponseMetadata, root: [u8; 32]) {
    assert_eq!(
        metadata.protocol_version, 13,
        "the node signs protocol 13 state"
    );
    metadata.height = 1000;
    metadata.core_chain_locked_height = 100;
    metadata.time_ms = 1_760_000_000_000;
    metadata.chain_id = "older-node-answers".to_string();
    proof.quorum_hash = QUORUM_HASH.to_vec();
    proof.quorum_type = QUORUM_TYPE;
    proof.round = 0;
    proof.block_id_hash = vec![3; 32];
    let state = StateId {
        app_version: metadata.protocol_version as u64,
        core_chain_locked_height: metadata.core_chain_locked_height,
        time: metadata.time_ms,
        app_hash: root.to_vec(),
        height: metadata.height,
    };
    let vote = CanonicalVote {
        r#type: SignedMsgType::Precommit.into(),
        block_id: proof.block_id_hash.clone(),
        chain_id: metadata.chain_id.clone(),
        height: metadata.height as i64,
        round: proof.round as i64,
        state_id: state
            .calculate_msg_hash(
                &metadata.chain_id,
                metadata.height as i64,
                proof.round as i32,
            )
            .expect("state digest"),
    };
    let digest = vote
        .calculate_sign_hash(
            &metadata.chain_id,
            QUORUM_TYPE as u8,
            &QUORUM_HASH,
            metadata.height as i64,
            proof.round as i32,
        )
        .expect("vote digest");
    proof.signature = key()
        .sign(SignatureSchemes::Basic, &digest)
        .expect("sign the state root")
        .as_raw_value()
        .to_compressed()
        .to_vec();
}

/// A node at protocol 13.
struct Node {
    platform: TempPlatform<MockCoreRPCLike>,
    state: Arc<PlatformState>,
}

impl Node {
    fn new() -> Self {
        let platform = TestPlatformBuilder::new()
            .with_config(PlatformConfig::default_for_network(Network::Testnet))
            .with_initial_protocol_version(13)
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let state = platform.platform.state.load_full();
        assert_eq!(state.current_protocol_version_in_consensus(), 13);
        Self { platform, state }
    }

    fn root(&self) -> [u8; 32] {
        self.platform
            .platform
            .drive
            .grove
            .root_hash(None, &v13().drive.grove_version)
            .unwrap()
            .expect("root")
    }

    fn insert(
        &self,
        contract: &DataContract,
        document_type: &str,
        id: [u8; 32],
        owner: [u8; 32],
        properties: Vec<(&str, Value)>,
    ) {
        let document: Document = DocumentV0 {
            id: Identifier::new(id),
            owner_id: Identifier::new(owner),
            properties: properties
                .into_iter()
                .map(|(name, value)| (name.to_string(), value))
                .collect(),
            ..Default::default()
        }
        .into();
        self.platform
            .platform
            .drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&document, None)),
                        owner_id: None,
                    },
                    contract,
                    document_type: contract
                        .document_type_for_name(document_type)
                        .expect("document type"),
                },
                false,
                BlockInfo::default(),
                true,
                None,
                v13(),
                None,
            )
            .expect("insert at protocol 13");
    }

    fn apply(&self, contract: &DataContract) {
        self.platform
            .platform
            .drive
            .apply_contract(contract, BlockInfo::default(), true, None, None, v13())
            .expect("apply the contract at protocol 13");
    }

    /// This node's signed answer to `request` without the fields a 4.1 node drops, read
    /// with `rules`.
    fn answer_as_older_node(
        &self,
        request: &GetDocumentsRequest,
        rules: &PlatformVersion,
    ) -> GetDocumentsResponse {
        let mut stripped = request.clone();
        let Some(RequestVersion::V1(v1)) = stripped.version.as_mut() else {
            panic!("V1 request")
        };
        v1.sub_queries.clear();
        v1.chained = None;
        let result = self
            .platform
            .platform
            .query_documents(stripped, &self.state, rules)
            .expect("query");
        assert!(
            result.errors.is_empty(),
            "query errors: {:?}",
            result.errors
        );
        let mut response = result.data.expect("response");
        let Some(ResponseVersion::V1(v1)) = response.version.as_mut() else {
            panic!("V1 response")
        };
        let mut metadata = v1.metadata.take().expect("metadata");
        let Some(get_documents_response_v1::Result::Proof(proof)) = v1.result.as_mut() else {
            panic!("proved answer")
        };
        sign(proof, &mut metadata, self.root());
        v1.metadata = Some(metadata);
        response
    }
}

/// Posts tagged "dash", their likes and their owners' profiles, as protocol 13 stores them.
fn social_contract() -> DataContract {
    DataContractFactory::new(13)
        .expect("factory")
        .create_with_value_config(
            Identifier::new([9; 32]),
            0,
            platform_value!({
                "post": {
                    "type": "object",
                    "properties": {
                        "hashtag": {"type": "string", "maxLength": 20, "position": 0},
                    },
                    "required": ["hashtag"],
                    "indices": [{"name": "byHashtag", "properties": [{"hashtag": "asc"}]}],
                    "additionalProperties": false,
                },
                "like": {
                    "type": "object",
                    "properties": {
                        "postId": {"type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
                                   "contentMediaType": "application/x.dash.dpp.identifier", "position": 0},
                    },
                    "required": ["postId"],
                    "indices": [{"name": "byPost", "properties": [{"postId": "asc"}], "countable": "countable"}],
                    "additionalProperties": false,
                },
                "profile": {
                    "type": "object",
                    "properties": {
                        "displayName": {"type": "string", "maxLength": 20, "position": 0},
                    },
                    "required": ["displayName"],
                    "indices": [{"name": "byOwner", "properties": [{"$ownerId": "asc"}], "unique": true}],
                    "additionalProperties": false,
                },
            }),
            None,
            None,
        )
        .expect("contract at protocol 13")
        .data_contract_owned()
}

/// The answer of a 4.1 node to a composite documents query is its plain page, which the
/// SDK cannot verify as the composite it asked for.
#[test]
fn should_refuse_an_older_node_answer_to_a_composite_query() {
    let node = Node::new();
    let contract = social_contract();
    node.apply(&contract);
    node.insert(
        &contract,
        "post",
        [0xA1; 32],
        [0x11; 32],
        vec![("hashtag", Value::Text("dash".into()))],
    );
    node.insert(
        &contract,
        "post",
        [0xB2; 32],
        [0x22; 32],
        vec![("hashtag", Value::Text("dash".into()))],
    );
    node.insert(
        &contract,
        "like",
        [0x51; 32],
        [0x11; 32],
        vec![("postId", Value::Identifier([0xA1; 32]))],
    );
    node.insert(
        &contract,
        "like",
        [0x52; 32],
        [0x22; 32],
        vec![("postId", Value::Identifier([0xA1; 32]))],
    );
    node.insert(
        &contract,
        "profile",
        [0x61; 32],
        [0x11; 32],
        vec![("displayName", Value::Text("one".into()))],
    );
    let contract = Arc::new(contract);
    let query = DocumentQuery::new(contract.clone(), "post")
        .expect("post")
        .with_where(WhereClause {
            field: "hashtag".into(),
            operator: WhereOperator::Equal,
            value: Value::Text("dash".into()),
        })
        .with_limit(10)
        .with_sub_query(
            CompositeSubQuery::count(contract.clone(), "like")
                .expect("like")
                .bound_to_page("$id", "postId"),
        )
        .with_sub_query(
            CompositeSubQuery::documents(contract, "profile")
                .expect("profile")
                .bound_to_page("$ownerId", "$ownerId"),
        );
    for sender in [v13(), v14()] {
        let request = GetDocumentsRequest::try_from_platform_versioned(query.clone(), sender)
            .expect("encode the composite query");
        let answer = node.answer_as_older_node(&request, v13());
        // The answer is a genuine, verifiable answer to the plain page.
        let mut page = query.clone();
        page.sub_queries.clear();
        let documents = <Documents as FromProof<DocumentQuery>>::maybe_from_proof(
            page,
            answer.clone(),
            Network::Testnet,
            v13(),
            &Provider,
        )
        .expect("the answer proves the plain page")
        .expect("documents");
        assert_eq!(documents.len(), 2);
        for rules in [v13(), v14()] {
            let verified = <CompositeDocuments as FromProof<DocumentQuery>>::maybe_from_proof(
                query.clone(),
                answer.clone(),
                Network::Testnet,
                rules,
                &Provider,
            );
            assert!(
                verified.is_err(),
                "sent at {}, verified under {}: {verified:?}",
                sender.protocol_version,
                rules.protocol_version
            );
        }
        assert_eq!(request.needs_protocol_version(), Some(14));
    }
}

/// The answer of a 4.1 node to a chained documents query is its inner query alone, which
/// the SDK cannot verify as the join it asked for. Only contracts registered under protocol
/// 14 rules declare a join, so the node reads this one with those rules; the state and the
/// signature stay protocol 13's.
#[test]
fn should_refuse_an_older_node_answer_to_a_chained_query() {
    let node = Node::new();
    let contract = json_document_to_contract(YAPPR_CONTRACT_PATH, false, v14())
        .expect("parse the yappr-likes contract");
    node.apply(&contract);
    let owner = [0x11; 32];
    for post in [[0xA1; 32], [0xB2; 32]] {
        node.insert(
            &contract,
            "post",
            post,
            owner,
            vec![
                ("hashtag", Value::Text("dash".into())),
                ("message", Value::Text("post".into())),
            ],
        );
        node.insert(
            &contract,
            "like",
            [post[0] ^ 0xFF; 32],
            owner,
            vec![
                ("hashtag", Value::Text("dash".into())),
                ("postId", Value::Identifier(post)),
            ],
        );
    }
    let inner = DocumentQuery::new(Arc::new(contract), "like")
        .expect("like")
        .with_where(WhereClause {
            field: "$ownerId".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Identifier(owner),
        })
        .with_limit(10);
    let query = ChainedDocumentQuery::new(inner.clone(), "postId", "post");
    for sender in [v13(), v14()] {
        let request = GetDocumentsRequest::try_from_platform_versioned(query.clone(), sender)
            .expect("encode the chained query");
        let answer = node.answer_as_older_node(&request, v14());
        // The answer is a genuine, verifiable answer to the inner query, under the rules of
        // the protocol version that signed it.
        let likes = <Documents as FromProof<DocumentQuery>>::maybe_from_proof(
            inner.clone(),
            answer.clone(),
            Network::Testnet,
            v13(),
            &Provider,
        )
        .expect("the answer proves the inner query")
        .expect("documents");
        assert_eq!(likes.len(), 2);
        for rules in [v13(), v14()] {
            let verified = <ChainedDocuments as FromProof<ChainedDocumentQuery>>::maybe_from_proof(
                query.clone(),
                answer.clone(),
                Network::Testnet,
                rules,
                &Provider,
            );
            assert!(
                verified.is_err(),
                "sent at {}, verified under {}: {verified:?}",
                sender.protocol_version,
                rules.protocol_version
            );
        }
        assert_eq!(request.needs_protocol_version(), Some(14));
    }
}

/// The answer of a 4.1 node to a token's shielded pool request is the credit pool's, which
/// the SDK cannot verify as the token's pool it asked for. Without a token the request is
/// the one a 4.1 SDK sends, served by every node.
#[test]
fn should_refuse_an_older_node_answer_to_a_token_pool_request() {
    let node = Node::new();
    let token_id = Some(vec![0x77; 32]);

    /// Asks `$request` for a token's pool, answers it as a 4.1 node through `$query` (the
    /// credit pool), and verifies the answer as `$object` under protocol 13 and 14 rules.
    macro_rules! token_pool_case {
        ($request:ident, $request_module:ident, $request_v0:ident, $response_module:ident,
         $response_v0:ident, $query:ident, $object:ty, { $($field:ident: $value:expr),* $(,)? }) => {{
            use $response_module::$response_v0::Result as Answer;
            use $response_module::Version as ResponseVersion;
            let request = |token_id: Option<Vec<u8>>| $request {
                version: Some($request_module::Version::V0($request_module::$request_v0 {
                    prove: true,
                    token_id,
                    $($field: $value,)*
                })),
            };
            let result = node
                .platform
                .platform
                .$query(request(None), &node.state, v13())
                .expect("query");
            assert!(result.errors.is_empty(), "query errors: {:?}", result.errors);
            let mut answer = result.data.expect("response");
            let Some(ResponseVersion::V0(v0)) = answer.version.as_mut() else {
                panic!("V0 response")
            };
            let mut metadata = v0.metadata.take().expect("metadata");
            let Some(Answer::Proof(proof)) = v0.result.as_mut() else {
                panic!("proved answer")
            };
            sign(proof, &mut metadata, node.root());
            v0.metadata = Some(metadata);
            // The answer is a genuine, verifiable answer about the credit pool.
            <$object as FromProof<$request>>::maybe_from_proof(
                request(None),
                answer.clone(),
                Network::Testnet,
                v13(),
                &Provider,
            )
            .expect("the answer proves the credit pool");
            for rules in [v13(), v14()] {
                let verified = <$object as FromProof<$request>>::maybe_from_proof(
                    request(token_id.clone()),
                    answer.clone(),
                    Network::Testnet,
                    rules,
                    &Provider,
                );
                assert!(
                    verified.is_err(),
                    "{} under {}: {verified:?}",
                    stringify!($request),
                    rules.protocol_version
                );
            }
            assert_eq!(request(token_id.clone()).needs_protocol_version(), Some(14));
            assert_eq!(request(None).needs_protocol_version(), None);
        }};
    }

    token_pool_case!(
        GetShieldedPoolStateRequest,
        get_shielded_pool_state_request,
        GetShieldedPoolStateRequestV0,
        get_shielded_pool_state_response,
        get_shielded_pool_state_response_v0,
        query_shielded_pool_state,
        ShieldedPoolState,
        {}
    );
    token_pool_case!(
        GetShieldedNotesCountRequest,
        get_shielded_notes_count_request,
        GetShieldedNotesCountRequestV0,
        get_shielded_notes_count_response,
        get_shielded_notes_count_response_v0,
        query_shielded_notes_count,
        ShieldedNotesCount,
        {}
    );
    token_pool_case!(
        GetShieldedAnchorsRequest,
        get_shielded_anchors_request,
        GetShieldedAnchorsRequestV0,
        get_shielded_anchors_response,
        get_shielded_anchors_response_v0,
        query_shielded_anchors,
        ShieldedAnchors,
        {}
    );
    token_pool_case!(
        GetMostRecentShieldedAnchorRequest,
        get_most_recent_shielded_anchor_request,
        GetMostRecentShieldedAnchorRequestV0,
        get_most_recent_shielded_anchor_response,
        get_most_recent_shielded_anchor_response_v0,
        query_most_recent_shielded_anchor,
        MostRecentShieldedAnchor,
        {}
    );
    token_pool_case!(
        GetShieldedEncryptedNotesRequest, get_shielded_encrypted_notes_request,
        GetShieldedEncryptedNotesRequestV0, get_shielded_encrypted_notes_response, get_shielded_encrypted_notes_response_v0,
        query_shielded_encrypted_notes, ShieldedEncryptedNotes,
        { start_index: 0, count: 16 }
    );
    token_pool_case!(
        GetShieldedNullifiersRequest, get_shielded_nullifiers_request,
        GetShieldedNullifiersRequestV0, get_shielded_nullifiers_response, get_shielded_nullifiers_response_v0,
        query_shielded_nullifiers, ShieldedNullifierStatuses,
        { nullifiers: vec![vec![1; 32]] }
    );
}
