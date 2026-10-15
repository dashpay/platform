//! Signed contact-request pages for the SDK sweep's proof tests, answered by
//! the node's document query handler at protocol versions 13 and 14 and
//! signed by a test quorum. The committed corpus lives in
//! `rs-sdk/tests/vectors/contact_request_paging`; set
//! `UPDATE_CONTACT_REQUEST_PAGING_FIXTURES=1` to regenerate it.

#[path = "../../rs-drive/tests/support/contact_request_blocks.rs"]
mod blocks;

use blocks::{ContactBlocks, BLOCK_OF_150, DIRECTIONS, FOREIGN};
use dapi_grpc::platform::v0::get_documents_request::get_documents_request_v0::Start;
use dapi_grpc::platform::v0::{
    get_documents_response, GetDocumentsRequest, GetDocumentsResponse, Proof, ResponseMetadata,
};
use dapi_grpc::Message;
use dash_platform_queries::documents::document_query::DocumentQuery;
use dpp::bls::SecretKey;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::{Document, DocumentV0Getters};
use dpp::platform_value::{platform_value, Identifier};
use dpp::prelude::DataContract;
use dpp::version::{PlatformVersion, ProtocolVersion};
use drive::query::{DriveDocumentQuery, OrderClause, WhereClause, WhereOperator};
use drive_abci::rpc::core::MockCoreRPCLike;
use drive_abci::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tenderdash_abci::proto::types::{CanonicalVote, SignedMsgType, StateId};
use tenderdash_abci::signatures::{Hashable, Signable};

const PAGE_SIZE: u32 = 100;
const HEIGHT: u64 = 1_000;
const TIME_MS: u64 = 1_700_000_500_000;
const CHAIN_ID: &str = "test-chain";
const QUORUM_HASH: [u8; 32] = [2; 32];

/// The secret key that signed the committed corpus.
fn signing_key() -> SecretKey {
    secret_key("2b1ca1d8f00ddc0c9a4d64b6e1c1f0c3a5e4b2d1c0f9e8d7c6b5a4938271605f")
}

fn secret_key(scalar: &str) -> SecretKey {
    let scalar = hex::decode(scalar)
        .expect("hex scalar")
        .try_into()
        .expect("32-byte scalar");
    SecretKey::from_be_bytes(&scalar).expect("valid scalar")
}

#[derive(Serialize)]
struct Page {
    protocol_version: ProtocolVersion,
    name: String,
    request: String,
    response: String,
}

#[derive(Serialize)]
struct Manifest {
    quorum_public_key: Vec<u8>,
    /// A valid key of another quorum.
    wrong_quorum_public_key: Vec<u8>,
    owner: String,
    recipient: String,
    /// Every request between `owner` and `recipient` as `(created_at, id)`,
    /// in index order.
    rows: Vec<(u64, String)>,
    /// A request between two other identities.
    foreign: String,
    pages: Vec<Page>,
}

type Answer = (
    GetDocumentsRequest,
    GetDocumentsResponse,
    Vec<(u64, Identifier)>,
);

/// One page of `field == identity` ordered by `$createdAt`, with the
/// optional `$createdAt >` clause and cursor.
fn page_query(
    contract: &Arc<DataContract>,
    set: &ContactBlocks,
    field: &str,
    after: Option<u64>,
    start: Option<Start>,
) -> DocumentQuery {
    let mut query = DocumentQuery::new(Arc::clone(contract), "contactRequest")
        .expect("contactRequest query")
        .with_where(WhereClause {
            field: field.to_string(),
            operator: WhereOperator::Equal,
            value: platform_value!(set.identity(field)),
        });
    if let Some(after) = after {
        query = query.with_where(WhereClause {
            field: "$createdAt".to_string(),
            operator: WhereOperator::GreaterThan,
            value: platform_value!(after),
        });
    }
    query = query
        .with_order_by(OrderClause {
            field: "$createdAt".to_string(),
            ascending: true,
        })
        .with_limit(PAGE_SIZE);
    query.start = start;
    query
}

fn start_at(id: Identifier) -> Option<Start> {
    Some(Start::StartAt(id.to_vec()))
}

fn start_after(id: Identifier) -> Option<Start> {
    Some(Start::StartAfter(id.to_vec()))
}

fn grovedb_proof(response: &GetDocumentsResponse) -> &[u8] {
    use get_documents_response::{get_documents_response_v0, get_documents_response_v1, Version};
    match &response.version {
        Some(Version::V0(v0)) => match &v0.result {
            Some(get_documents_response_v0::Result::Proof(proof)) => &proof.grovedb_proof,
            other => panic!("v0 response without a proof: {other:?}"),
        },
        Some(Version::V1(v1)) => match &v1.result {
            Some(get_documents_response_v1::Result::Proof(proof)) => &proof.grovedb_proof,
            other => panic!("v1 response without a proof: {other:?}"),
        },
        None => panic!("unversioned response"),
    }
}

/// The handler's proof, signed by the test quorum over the state root.
fn signed(
    response: GetDocumentsResponse,
    root: [u8; 32],
    protocol_version: ProtocolVersion,
) -> GetDocumentsResponse {
    use get_documents_response::{
        get_documents_response_v0, get_documents_response_v1, GetDocumentsResponseV0,
        GetDocumentsResponseV1, Version,
    };
    let metadata = ResponseMetadata {
        height: HEIGHT,
        core_chain_locked_height: 1,
        epoch: 0,
        time_ms: TIME_MS,
        protocol_version,
        chain_id: CHAIN_ID.into(),
    };
    let mut proof = Proof {
        grovedb_proof: grovedb_proof(&response).to_vec(),
        quorum_hash: QUORUM_HASH.to_vec(),
        signature: vec![],
        round: 1,
        block_id_hash: vec![3; 32],
        quorum_type: 1,
    };
    let state = StateId {
        app_version: protocol_version as u64,
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
            .expect("signed state digest"),
    };
    let digest = vote
        .calculate_sign_hash(
            &metadata.chain_id,
            proof.quorum_type as u8,
            &QUORUM_HASH,
            metadata.height as i64,
            proof.round as i32,
        )
        .expect("vote digest");
    proof.signature = signing_key()
        .sign(&digest)
        .expect("quorum signature")
        .to_bytes()
        .to_vec();
    let version = match response.version {
        Some(Version::V0(_)) => Version::V0(GetDocumentsResponseV0 {
            result: Some(get_documents_response_v0::Result::Proof(proof)),
            metadata: Some(metadata),
        }),
        _ => Version::V1(GetDocumentsResponseV1 {
            result: Some(get_documents_response_v1::Result::Proof(proof)),
            metadata: Some(metadata),
        }),
    };
    GetDocumentsResponse {
        version: Some(version),
    }
}

fn check_file(directory: &Path, filename: &str, expected: &[u8], update: bool) {
    let file = directory.join(filename);
    if update {
        fs::write(&file, expected).expect("write fixture");
    }
    let committed = fs::read(&file).expect("committed fixture");
    assert_eq!(committed, expected, "fixture drift: {filename}");
}

/// A node at one protocol version holding every set of [`blocks::ALL`].
struct Node {
    platform: TempPlatform<MockCoreRPCLike>,
    contract: Arc<DataContract>,
    version: &'static PlatformVersion,
    root: [u8; 32],
}

impl Node {
    fn new(protocol_version: ProtocolVersion) -> Self {
        let version = PlatformVersion::get(protocol_version).expect("protocol version");
        let platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(protocol_version)
            .build_with_mock_rpc();
        platform
            .drive
            .create_initial_state_structure(None, version)
            .expect("Drive structure");
        let contract = Arc::new(blocks::populate(&platform.drive, version));
        let root = platform
            .drive
            .grove
            .root_hash(None, &version.drive.grove_version)
            .unwrap()
            .expect("state root");
        Node {
            platform,
            contract,
            version,
            root,
        }
    }

    /// The handler's signed answer to `query`, with the page's rows as
    /// `(created_at, id)` read back from its proof.
    fn answer(&self, query: DocumentQuery) -> Answer {
        let drive_query = DriveDocumentQuery::try_from(&query).expect("drive query");
        let request = query
            .clone()
            .try_into_request_for_version(self.version)
            .expect("wire request");
        let state = self.platform.state.load();
        let result = self
            .platform
            .query_documents(request.clone(), &state, self.version)
            .expect("document query");
        assert!(result.is_valid(), "{:?}", result.errors);
        let response = signed(
            result.into_data().expect("response"),
            self.root,
            self.version.protocol_version,
        );
        let (root, documents) = drive_query
            .verify_proof_keep_serialized(grovedb_proof(&response), self.version)
            .expect("proof of the page");
        assert_eq!(root, self.root);
        let document_type = self
            .contract
            .document_type_for_name("contactRequest")
            .expect("contactRequest type");
        let rows = documents
            .iter()
            .map(|bytes| {
                let document = Document::from_bytes(bytes, document_type, self.version)
                    .expect("contact request");
                (document.created_at().expect("$createdAt"), document.id())
            })
            .collect();
        (request, response, rows)
    }
}

#[test]
fn should_match_committed_contact_request_paging_responses() {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../rs-sdk/tests/vectors/contact_request_paging");
    let update = std::env::var("UPDATE_CONTACT_REQUEST_PAGING_FIXTURES").as_deref() == Ok("1");
    if update {
        fs::create_dir_all(&directory).expect("fixture corpus directory");
    }
    let set = &BLOCK_OF_150;
    let rows = set.rows();
    let first_block = set.blocks[0].0;
    let foreign = FOREIGN.rows()[0].1;
    let mut pages = Vec::new();
    for protocol_version in [13, 14] {
        let node = Node::new(protocol_version);
        let mut save = |name: &str, (request, response, rows): Answer| {
            let base = format!("pv{protocol_version}-{name}");
            for (kind, bytes) in [
                ("request", request.encode_to_vec()),
                ("response", response.encode_to_vec()),
            ] {
                check_file(&directory, &format!("{base}.{kind}.bin"), &bytes, update);
            }
            pages.push(Page {
                protocol_version,
                name: name.to_string(),
                request: format!("{base}.request.bin"),
                response: format!("{base}.response.bin"),
            });
            rows
        };
        let query = |field, after, start| page_query(&node.contract, set, field, after, start);

        // Pages a sweep at the corpus's page size, saving each page, and
        // returns every page's rows with continuation cursor rows dropped.
        let mut sweep = |name: &str, field, after| {
            let mut read = Vec::new();
            let mut cursor: Option<Identifier> = None;
            let mut pages = 0;
            loop {
                pages += 1;
                let after = if cursor.is_none() { after } else { None };
                let page = save(
                    &format!("{name}-{pages}"),
                    node.answer(query(field, after, cursor.and_then(start_at))),
                );
                let mut page_rows = page.iter();
                if let Some(cursor) = cursor {
                    assert_eq!(page_rows.next().map(|(_, id)| *id), Some(cursor));
                }
                read.extend(page_rows.copied());
                if page.len() < PAGE_SIZE as usize {
                    return (pages, read);
                }
                cursor = page.last().map(|(_, id)| *id);
            }
        };

        for field in DIRECTIONS {
            let direction = if field == "toUserId" {
                "received"
            } else {
                "sent"
            };
            // A full first page that cuts the block of 150, a full
            // continuation at its last row, then a short one.
            assert_eq!(sweep(direction, field, None), (3, rows.clone()));
        }
        // A first page after the first block, then continuations without the
        // `$createdAt` clause.
        assert_eq!(
            sweep("received-after", "toUserId", Some(first_block)),
            (3, rows[5..].to_vec())
        );

        if protocol_version == 13 {
            // The controls, as the sweep sent them before this corpus: a
            // `startAfter` continuation, with the first page's `$createdAt`
            // clause kept when there is one, skips the rest of the cut block.
            for (name, field, after, cursor) in [
                ("received-2-start-after", "toUserId", None, rows[99].1),
                ("sent-2-start-after", "$ownerId", None, rows[99].1),
                (
                    "received-after-2-start-after",
                    "toUserId",
                    Some(first_block),
                    rows[104].1,
                ),
            ] {
                let control = save(name, node.answer(query(field, after, start_after(cursor))));
                assert_eq!(control, rows[155..]);
            }
        }

        // A cursor that is not a row of this query: the node answers from
        // the cursor's position, without the cursor.
        let foreign_page = save(
            "received-foreign-cursor",
            node.answer(query("toUserId", None, start_at(foreign))),
        );
        assert!(!foreign_page.is_empty());
        assert!(foreign_page.iter().all(|(_, id)| *id != foreign));
    }

    let manifest = Manifest {
        quorum_public_key: signing_key().public_key().to_bytes().to_vec(),
        wrong_quorum_public_key: secret_key(
            "1f2e3d4c5b6a79881726354453627180918a7b6c5d4e3f20112233445566778f",
        )
        .public_key()
        .to_bytes()
        .to_vec(),
        owner: hex::encode(set.owner),
        recipient: hex::encode(set.recipient),
        rows: rows
            .iter()
            .map(|(created_at, id)| (*created_at, hex::encode(id.as_slice())))
            .collect(),
        foreign: hex::encode(foreign.as_slice()),
        pages,
    };
    let mut bytes = serde_json::to_vec_pretty(&manifest).expect("fixture manifest");
    bytes.push(b'\n');
    check_file(&directory, "manifest.json", &bytes, update);
}
