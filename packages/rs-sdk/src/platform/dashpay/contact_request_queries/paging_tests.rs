//! The contact-request sweep against signed pages that the node's document
//! query handler answered at protocol versions 13 and 14, from
//! `tests/vectors/contact_request_paging` (generated and checked for drift by
//! `rs-drive-abci/tests/contact_request_paging.rs`). The mock transport only
//! replays the bytes: every page goes through the SDK's own proof and quorum
//! signature verification, and a request the corpus does not hold fails.
//!
//! The corpus holds 220 requests from one sender to one recipient, in blocks
//! of 5, 150, 3, 2 and 60. A page of 100 cuts the block of 150, and the first
//! continuation is full: its cursor and 99 more.

use super::{contact_request_page_query, CONTACT_REQUEST_PAGE_SIZE};
use crate::platform::FetchMany;
use crate::{Error, Sdk, SdkBuilder};
use dapi_grpc::platform::v0::get_documents_request::get_documents_request_v0::Start;
use dapi_grpc::platform::v0::{GetDocumentsRequest, GetDocumentsResponse};
use dapi_grpc::Message;
use dash_context_provider::{ContextProvider, ContextProviderError};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::{DataContract, TokenConfiguration};
use dpp::document::Document;
use dpp::prelude::Identifier;
use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
use dpp::version::{PlatformVersion, ProtocolVersion};
use rs_dapi_client::{DumpData, ExecutionResponse, RequestSettings};
use std::path::PathBuf;
use std::sync::Arc;

struct Provider {
    contract: Arc<DataContract>,
    quorum_public_key: [u8; 48],
}

impl ContextProvider for Provider {
    fn get_data_contract(
        &self,
        id: &Identifier,
        _version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        Ok((*id == self.contract.id()).then(|| Arc::clone(&self.contract)))
    }

    fn get_token_configuration(
        &self,
        _id: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        Ok(None)
    }

    fn get_quorum_public_key(
        &self,
        _quorum_type: u32,
        _quorum_hash: [u8; 32],
        _core_chain_locked_height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        Ok(self.quorum_public_key)
    }

    fn get_platform_activation_height(&self) -> Result<u32, ContextProviderError> {
        Ok(1)
    }
}

fn corpus_directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/vectors/contact_request_paging")
}

fn manifest() -> serde_json::Value {
    let bytes = std::fs::read(corpus_directory().join("manifest.json")).expect("corpus manifest");
    serde_json::from_slice(&bytes).expect("corpus manifest JSON")
}

fn identifier(value: &serde_json::Value) -> Identifier {
    let bytes: [u8; 32] = hex::decode(value.as_str().expect("hex id"))
        .expect("hex id")
        .try_into()
        .expect("32-byte id");
    Identifier::from(bytes)
}

/// The corpus: its sender, its recipient, and every request between them as
/// `(created_at, id)` in index order.
struct Requests {
    owner: Identifier,
    recipient: Identifier,
    rows: Vec<(u64, Identifier)>,
    foreign: Identifier,
}

impl Requests {
    fn load() -> Self {
        let manifest = manifest();
        let rows = manifest["rows"]
            .as_array()
            .expect("rows")
            .iter()
            .map(|row| (row[0].as_u64().expect("created_at"), identifier(&row[1])))
            .collect();
        Requests {
            owner: identifier(&manifest["owner"]),
            recipient: identifier(&manifest["recipient"]),
            rows,
            foreign: identifier(&manifest["foreign"]),
        }
    }

    fn ids<R>(&self, range: R) -> Vec<Identifier>
    where
        R: std::slice::SliceIndex<[(u64, Identifier)], Output = [(u64, Identifier)]>,
    {
        self.rows[range].iter().map(|(_, id)| *id).collect()
    }
}

/// The DashPay contract and an SDK pinned to `protocol_version` whose
/// transport answers every request the corpus holds at that version.
fn sdk(protocol_version: ProtocolVersion) -> (Sdk, Arc<DataContract>) {
    sdk_trusting(protocol_version, "quorum_public_key")
}

/// [`sdk`], trusting the manifest's quorum key `key`.
fn sdk_trusting(protocol_version: ProtocolVersion, key: &str) -> (Sdk, Arc<DataContract>) {
    let version = PlatformVersion::get(protocol_version).expect("protocol version");
    let contract = Arc::new(
        load_system_data_contract(SystemDataContract::Dashpay, version).expect("DashPay contract"),
    );
    let manifest = manifest();
    let quorum_public_key = manifest[key]
        .as_array()
        .expect("quorum key")
        .iter()
        .map(|byte| byte.as_u64().expect("key byte") as u8)
        .collect::<Vec<_>>()
        .try_into()
        .expect("48-byte key");
    let dumps = tempfile::TempDir::new().expect("dump directory");
    for page in manifest["pages"].as_array().expect("pages") {
        if page["protocol_version"].as_u64() != Some(protocol_version as u64) {
            continue;
        }
        let read = |field: &str| {
            std::fs::read(corpus_directory().join(page[field].as_str().expect("file")))
                .expect("corpus page")
        };
        let request =
            GetDocumentsRequest::decode(read("request").as_slice()).expect("page request");
        let response = Ok(ExecutionResponse {
            inner: GetDocumentsResponse::decode(read("response").as_slice())
                .expect("page response"),
            retries: 0,
            address: "http://127.0.0.1:9000".parse().expect("mock address"),
        });
        let dump = DumpData::new(&request, &response);
        dump.save(&dumps.path().join(dump.filename().expect("dump name")))
            .expect("save page");
    }
    let sdk = SdkBuilder::new_mock()
        .with_version(version)
        .with_context_provider(Provider {
            contract: Arc::clone(&contract),
            quorum_public_key,
        })
        .with_settings(RequestSettings {
            retries: Some(0),
            ..Default::default()
        })
        .with_dump_dir(dumps.path())
        .build()
        .expect("SDK with proof verification");
    (sdk, contract)
}

fn ids(documents: &super::ContactRequestDocuments) -> Vec<Identifier> {
    documents.keys().copied().collect()
}

#[tokio::test]
async fn should_read_every_request_once_through_a_block_cut_by_a_page_boundary() {
    let requests = Requests::load();
    let first_block = requests.rows[0].0;
    for protocol_version in [13, 14] {
        let (sdk, _) = sdk(protocol_version);
        let received = sdk
            .fetch_received_contact_requests(requests.recipient, None)
            .await
            .expect("received requests");
        assert_eq!(
            ids(&received),
            requests.ids(0..),
            "protocol {protocol_version}: {} received",
            received.len()
        );
        let sent = sdk
            .fetch_sent_contact_requests(requests.owner, None)
            .await
            .expect("sent requests");
        assert_eq!(
            ids(&sent),
            requests.ids(0..),
            "protocol {protocol_version}: {} sent",
            sent.len()
        );
        let received_after = sdk
            .fetch_received_contact_requests(requests.recipient, Some(first_block))
            .await
            .expect("received requests after the first block");
        assert_eq!(
            ids(&received_after),
            requests.ids(5..),
            "protocol {protocol_version}: {} received after the first block",
            received_after.len()
        );
    }
}

/// The control: up to protocol version 13 a verified `startAfter` page at
/// the same cursor skips the other 55 requests of the cut block.
#[tokio::test]
async fn should_verify_a_start_after_page_that_skips_the_rest_of_the_cut_block_at_13() {
    let requests = Requests::load();
    let (sdk, contract) = sdk(13);
    let cursor = requests.rows[99].1;
    let mut query =
        contact_request_page_query(contract, "toUserId", requests.recipient, None, None);
    query.start = Some(Start::StartAfter(cursor.to_vec()));
    let page = Document::fetch_many(&sdk, query)
        .await
        .expect("a verified startAfter page");
    assert_eq!(ids(&page), requests.ids(155..));
}

#[tokio::test]
async fn should_refuse_a_cursor_that_is_not_a_request_of_the_query() {
    let requests = Requests::load();
    for protocol_version in [13, 14] {
        let (sdk, contract) = sdk(protocol_version);
        let page = sdk
            .fetch_contact_request_page(
                &contract,
                "toUserId",
                requests.recipient,
                None,
                Some(requests.foreign),
            )
            .await;
        assert!(
            matches!(page, Err(Error::InvalidProvedResponse(_))),
            "protocol {protocol_version}: {:?}",
            page.map(|page| page.raw_len)
        );
    }
}

/// With a high-water mark, a first page only returns requests created after
/// it, so a cursor created at or before it is not one of its rows.
#[tokio::test]
async fn should_refuse_a_cursor_not_created_after_the_high_water_mark() {
    let requests = Requests::load();
    let (cursor_created_at, cursor) = requests.rows[99];
    for protocol_version in [13, 14] {
        let (sdk, contract) = sdk(protocol_version);
        let page = sdk
            .fetch_contact_request_page(
                &contract,
                "toUserId",
                requests.recipient,
                Some(cursor_created_at),
                Some(cursor),
            )
            .await;
        assert!(
            matches!(page, Err(Error::InvalidProvedResponse(_))),
            "protocol {protocol_version}: {:?}",
            page.map(|page| page.raw_len)
        );
        let page = sdk
            .fetch_contact_request_page(
                &contract,
                "toUserId",
                requests.recipient,
                Some(cursor_created_at - 1),
                Some(cursor),
            )
            .await
            .expect("a cursor created after the high-water mark");
        // A full raw page holds only 99 new requests: the sweep continues.
        assert_eq!(ids(&page.documents), requests.ids(100..199));
        assert_eq!(page.raw_len, CONTACT_REQUEST_PAGE_SIZE as usize);
        assert_eq!(page.last_id, Some(requests.rows[198].1));
    }
}

#[tokio::test]
async fn should_refuse_pages_signed_by_another_quorum() {
    let requests = Requests::load();
    for protocol_version in [13, 14] {
        let (sdk, _) = sdk_trusting(protocol_version, "wrong_quorum_public_key");
        let received = sdk
            .fetch_received_contact_requests(requests.recipient, None)
            .await;
        assert!(
            matches!(received, Err(Error::Proof(_))),
            "protocol {protocol_version}: {:?}",
            received.map(|documents| documents.len())
        );
    }
}
