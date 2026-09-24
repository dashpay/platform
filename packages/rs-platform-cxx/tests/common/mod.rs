// Copyright (c) 2026 The Dash Core developers
// Distributed under the MIT software license, see the accompanying
// file COPYING or http://www.opensource.org/licenses/mit-license.php.

//! Shared test setup.
//!
//! `fixture` is a small Platform state built in a real Drive instance and
//! proved the way an evonode proves it: GroveDB proofs from the state, a
//! Tenderdash `StateId` / `CanonicalVote` signed with a BLS quorum key. The
//! replay suite hands those bytes to the client through `dash-sdk`'s mock
//! transport, so the SDK runs the GroveDB replay and the quorum signature
//! check against the key the test pushed exactly as it does against a live
//! node; only the socket is mocked.
//!
//! The fixture is generated once per process for each protocol version the
//! suites run at (see [`Fixture::get`]); one quorum signs them all.
//! `mock_client` wires a `Client` to that transport. Every test installs its
//! own expectation, so the SDK instances are throwaway.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::sync::OnceLock;

use dash_platform_cxx::client::Client;
use dash_platform_cxx::ffi::{Config, QuorumKey, StatusKind};
use dash_sdk::dapi_client::mock::MockResult;
use dash_sdk::dapi_client::transport::{TransportError, TransportRequest};
use dash_sdk::dapi_client::{DapiClientError, DumpData, ExecutionError, ExecutionResponse};
use dash_sdk::dpp::block::block_info::BlockInfo;
use dash_sdk::dpp::bls_signatures::{Bls12381G2Impl, SecretKey as BlsSecretKey, SignatureSchemes};
use dash_sdk::dpp::dashcore::Network;
use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;
use dash_sdk::dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dash_sdk::dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dash_sdk::dpp::data_contract::DataContract;
use dash_sdk::dpp::document::Document;
use dash_sdk::dpp::identity::accessors::{IdentityGettersV0, IdentitySettersV0};
use dash_sdk::dpp::identity::identity_public_key::contract_bounds::ContractBounds;
use dash_sdk::dpp::identity::identity_public_key::methods::hash::IdentityPublicKeyHashMethodsV0;
use dash_sdk::dpp::identity::{Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dash_sdk::dpp::platform_value::{platform_value, Identifier, Value};
use dash_sdk::dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
use dash_sdk::dpp::version::{PlatformVersion, ProtocolVersion, LATEST_VERSION};
use dash_sdk::dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use dash_sdk::platform::proto::{self, Proof, ResponseMetadata};
use dash_sdk::platform::types::identity::{IdentityRequest, IdentityResponse};
use dash_sdk::platform::{Query, QuerySettings};
use dash_sdk::sdk::min_protocol_version;
use dash_sdk::{RequestSettings, Sdk};
use dpp::dashcore::secp256k1::rand::rngs::StdRng;
use dpp::dashcore::secp256k1::rand::{Rng, SeedableRng};
use drive::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use drive::drive::Drive;
use drive::query::vote_poll_vote_state_query::{
    ContestedDocumentVotePollDriveQuery, ContestedDocumentVotePollDriveQueryResultType,
};
use drive::query::DriveDocumentQuery;
use drive::util::object_size_info::{
    DataContractOwnedResolvedInfo, DocumentAndContractInfo, DocumentInfo, OwnedDocumentInfo,
};
use drive::util::storage_flags::StorageFlags;
use drive::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use tenderdash_abci::proto::types::{CanonicalVote, SignedMsgType, StateId};
use tenderdash_abci::signatures::{Hashable, Signable};

/// The protocol version testnet and mainnet run: the SDK's floor for them,
/// which is what a live network serves until it upgrades. Every suite runs
/// at this version and at [`LATEST_VERSION`].
pub const DEPLOYED_VERSION: ProtocolVersion = min_protocol_version(Network::Testnet);
/// The Platform LLMQ type of the fixture network.
pub const PLATFORM_LLMQ_TYPE: u8 = 106;
/// The Tenderdash chain id the fixture quorum signs for.
pub const CHAIN_ID: &str = "dash-testnet-51";
/// The Platform height the fixture state is proved at.
pub const HEIGHT: u64 = 123_456;
/// The core-chain-locked height the fixture state is proved at.
pub const CORE_CHAIN_LOCKED_HEIGHT: u32 = 2_000_000;
/// The DPNS label the fixture identity owns.
pub const OWNED_LABEL: &str = "Alice";
/// A DPNS label under contest between the fixture identities.
pub const CONTESTED_LABEL: &str = "quantum";
/// A DPNS label nothing in the fixture uses.
pub const ABSENT_LABEL: &str = "nobody";
/// The nonce the fixture identity has used against the DPNS contract.
pub const DPNS_NONCE: u64 = 3;
/// The DashPay v2 `corePaymentAddress` on the fixture profile (P2PKH type
/// byte and a hash160).
pub const CORE_PAYMENT_ADDRESS: [u8; 21] = [0x1a; 21];
/// Names the third identity owns: one more than a page.
pub const PAGED_NAME_COUNT: usize = dash_platform_cxx::ops::PAGE_SIZE as usize + 1;

/// The labels of the third identity's names, none of them contested (the
/// `2` keeps them out of the contested pattern).
pub fn paged_label(i: usize) -> String {
    format!("pg2-{i:03}")
}

pub fn config() -> Config {
    Config {
        network: 1,
        tenderdash_chain_id: CHAIN_ID.to_string(),
        platform_llmq_type: PLATFORM_LLMQ_TYPE,
    }
}

/// A DPNS `domain` document as `register_dpns_name` builds it, with the
/// system fields Drive would have filled in.
fn domain_document(
    contract: &DataContract,
    owner: Identifier,
    label: &str,
    rng: &mut StdRng,
    version: &PlatformVersion,
) -> Document {
    let document_type = contract.document_type_for_name("domain").expect("domain");
    let data = platform_value!({
        "parentDomainName": "dash",
        "normalizedParentDomainName": "dash",
        "label": label,
        "normalizedLabel": dash_platform_cxx::helpers::normalize_label(label),
        "preorderSalt": Value::Bytes32(rng.gen()),
        "records": { "identity": owner },
        "subdomainRules": { "allowSubdomains": false },
        "$createdAt": 1_700_000_000_000u64,
        "$updatedAt": 1_700_000_000_000u64,
        "$transferredAt": 1_700_000_000_000u64,
    });
    document_type
        .create_document_from_data(
            data,
            owner,
            HEIGHT,
            CORE_CHAIN_LOCKED_HEIGHT,
            rng.gen(),
            version,
        )
        .expect("domain document")
}

fn insert_document(
    drive: &Drive,
    contract: &DataContract,
    document_type: &str,
    document: &Document,
    version: &PlatformVersion,
) {
    let document_type = contract
        .document_type_for_name(document_type)
        .expect("document type");
    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentInfo::DocumentRefInfo((
                        document,
                        StorageFlags::optional_default_as_cow(),
                    )),
                    owner_id: None,
                },
                contract,
                document_type,
            },
            false,
            BlockInfo::default(),
            true,
            None,
            version,
            None,
        )
        .expect("insert document");
}

/// The test quorum: one key signs the fixture at every protocol version, so
/// a client trusts it whichever fixture it replays.
pub struct Quorum {
    secret: BlsSecretKey<Bls12381G2Impl>,
    /// The quorum hash in proof byte order.
    pub hash: [u8; 32],
    pub pubkey: [u8; 48],
}

impl Quorum {
    /// The quorum key in the embedder's internal `uint256` byte order, the
    /// order `set_quorum_keys` takes.
    pub fn core_order_key(&self) -> QuorumKey {
        let mut hash = self.hash;
        hash.reverse();
        QuorumKey {
            hash,
            pubkey: self.pubkey,
        }
    }
}

pub fn quorum() -> &'static Quorum {
    static QUORUM: OnceLock<Quorum> = OnceLock::new();
    QUORUM.get_or_init(|| {
        let secret = BlsSecretKey::<Bls12381G2Impl>::from_hash(b"dash-platform-cxx fixture quorum");
        let pubkey = secret
            .public_key()
            .to_bytes()
            .try_into()
            .expect("48-byte key");
        Quorum {
            secret,
            hash: std::array::from_fn(|i| (i as u8).wrapping_mul(7)),
            pubkey,
        }
    })
}

/// The fixture state at one protocol version: two identities, one DPNS
/// name, one contest, one DashPay profile and contact request, one
/// identity-contract nonce.
pub struct Fixture {
    pub version: &'static PlatformVersion,
    drive: Drive,
    dpns: DataContract,
    dashpay: DataContract,
    /// The identity that owns `OWNED_LABEL`, the profile and the contact
    /// request, with keys 0 AUTH/MASTER, 1 AUTH/HIGH, 2 ENC/MEDIUM,
    /// 3 DEC/MEDIUM (the Core key set).
    pub alice: Identity,
    /// The counterparty of the contact request and the other contender.
    pub bob: Identity,
    /// Owns [`PAGED_NAME_COUNT`] names and nothing else.
    pub carol: Identity,
    /// The unique key hash of Alice's key 0.
    pub alice_key0_hash: [u8; 20],
    /// The `corePaymentAddress` on Alice's profile: [`CORE_PAYMENT_ADDRESS`]
    /// where the DashPay contract has the field (v2, protocol version 14),
    /// empty before.
    pub core_payment_address: Vec<u8>,
}

fn identity_with_core_keys(seed: u64, version: &PlatformVersion) -> Identity {
    let mut rng = StdRng::seed_from_u64(seed);
    let dashpay = SystemDataContract::Dashpay.id();
    let spec = [
        (0u32, Purpose::AUTHENTICATION, SecurityLevel::MASTER, None),
        (1, Purpose::AUTHENTICATION, SecurityLevel::HIGH, None),
        (
            2,
            Purpose::ENCRYPTION,
            SecurityLevel::MEDIUM,
            Some(ContractBounds::SingleContractDocumentType {
                id: dashpay,
                document_type_name: "contactRequest".to_string(),
            }),
        ),
        (
            3,
            Purpose::DECRYPTION,
            SecurityLevel::MEDIUM,
            Some(ContractBounds::SingleContractDocumentType {
                id: dashpay,
                document_type_name: "contactRequest".to_string(),
            }),
        ),
    ];
    let keys: BTreeMap<u32, IdentityPublicKey> = spec
        .into_iter()
        .map(|(id, purpose, level, bounds)| {
            let (key, _) = IdentityPublicKey::random_key_with_known_attributes(
                id,
                &mut rng,
                purpose,
                level,
                KeyType::ECDSA_SECP256K1,
                bounds,
                version,
            )
            .expect("key");
            (id, key)
        })
        .collect();
    let mut identity =
        Identity::new_with_id_and_keys(Identifier::from(rng.gen::<[u8; 32]>()), keys, version)
            .expect("identity");
    identity.set_balance(1_000_000_000);
    identity
}

impl Fixture {
    fn build(version: &'static PlatformVersion) -> Self {
        let drive = setup_drive_with_initial_state_structure(Some(version));
        let dpns = load_system_data_contract(SystemDataContract::DPNS, version).expect("dpns");
        let dashpay =
            load_system_data_contract(SystemDataContract::Dashpay, version).expect("dashpay");
        for contract in [&dpns, &dashpay] {
            drive
                .apply_contract(
                    contract,
                    BlockInfo::default(),
                    true,
                    StorageFlags::optional_default_as_cow(),
                    None,
                    version,
                )
                .expect("apply contract");
        }

        let alice = identity_with_core_keys(1, version);
        let bob = identity_with_core_keys(2, version);
        let carol = identity_with_core_keys(3, version);
        for identity in [&alice, &bob, &carol] {
            drive
                .add_new_identity(
                    identity.clone(),
                    false,
                    &BlockInfo::default(),
                    true,
                    None,
                    version,
                )
                .expect("insert identity");
        }
        drive
            .merge_identity_contract_nonce(
                alice.id().to_buffer(),
                dpns.id().to_buffer(),
                DPNS_NONCE,
                &BlockInfo::default(),
                true,
                None,
                &mut vec![],
                version,
            )
            .expect("nonce");

        let mut rng = StdRng::seed_from_u64(7);
        let owned = domain_document(&dpns, alice.id(), OWNED_LABEL, &mut rng, version);
        insert_document(&drive, &dpns, "domain", &owned, version);
        for i in 0..PAGED_NAME_COUNT {
            let name = domain_document(&dpns, carol.id(), &paged_label(i), &mut rng, version);
            insert_document(&drive, &dpns, "domain", &name, version);
        }

        let vote_poll = ContestedDocumentResourceVotePollWithContractInfo {
            contract: DataContractOwnedResolvedInfo::OwnedDataContract(dpns.clone()),
            document_type_name: "domain".to_string(),
            index_name: "parentNameAndLabel".to_string(),
            index_values: vec![
                Value::Text("dash".to_string()),
                Value::Text(CONTESTED_LABEL.to_string()),
            ],
        };
        for (contender, first) in [(&alice, true), (&bob, false)] {
            let document =
                domain_document(&dpns, contender.id(), CONTESTED_LABEL, &mut rng, version);
            drive
                .add_contested_document(
                    OwnedDocumentInfo {
                        document_info: DocumentInfo::DocumentRefInfo((
                            &document,
                            StorageFlags::optional_default_as_cow(),
                        )),
                        owner_id: Some(contender.id().to_buffer()),
                    },
                    vote_poll.clone(),
                    false,
                    first.then(|| {
                        dash_sdk::dpp::voting::vote_info_storage::contested_document_vote_poll_stored_info::ContestedDocumentVotePollStoredInfo::new(
                            BlockInfo::default(),
                            version,
                        )
                        .expect("stored info")
                    }),
                    &BlockInfo::default(),
                    true,
                    None,
                    version,
                )
                .expect("contested document");
        }
        for (voter, choice, strength) in [
            (
                [0x11u8; 32],
                ResourceVoteChoice::TowardsIdentity(alice.id()),
                1,
            ),
            (
                [0x12u8; 32],
                ResourceVoteChoice::TowardsIdentity(alice.id()),
                4,
            ),
            (
                [0x13u8; 32],
                ResourceVoteChoice::TowardsIdentity(bob.id()),
                1,
            ),
            ([0x14u8; 32], ResourceVoteChoice::Abstain, 1),
            ([0x15u8; 32], ResourceVoteChoice::Lock, 4),
        ] {
            drive
                .register_contested_resource_identity_vote(
                    voter,
                    strength,
                    vote_poll.clone(),
                    choice,
                    None,
                    &BlockInfo::default(),
                    None,
                    version,
                )
                .expect("vote");
        }

        let profile_type = dashpay.document_type_for_name("profile").expect("profile");
        let core_payment_address = if profile_type.properties().contains_key("corePaymentAddress") {
            CORE_PAYMENT_ADDRESS.to_vec()
        } else {
            Vec::new()
        };
        let mut profile_data = platform_value!({
            "displayName": "Alice",
            "publicMessage": "hello",
            "$createdAt": 1_700_000_000_000u64,
            "$updatedAt": 1_700_000_000_000u64,
        });
        if !core_payment_address.is_empty() {
            profile_data
                .set_value(
                    "corePaymentAddress",
                    Value::Bytes(core_payment_address.clone()),
                )
                .expect("payment address");
        }
        let profile = profile_type
            .create_document_from_data(
                profile_data,
                alice.id(),
                HEIGHT,
                CORE_CHAIN_LOCKED_HEIGHT,
                rng.gen(),
                version,
            )
            .expect("profile");
        insert_document(&drive, &dashpay, "profile", &profile, version);
        let contact_type = dashpay
            .document_type_for_name("contactRequest")
            .expect("contactRequest");
        let contact = contact_type
            .create_document_from_data(
                platform_value!({
                    "toUserId": bob.id(),
                    "encryptedPublicKey": Value::Bytes(vec![0x42u8; 96]),
                    "senderKeyIndex": 2u32,
                    "recipientKeyIndex": 3u32,
                    "accountReference": 0x1000_0005u32,
                    "$createdAt": 1_700_000_000_000u64,
                    "$createdAtCoreBlockHeight": CORE_CHAIN_LOCKED_HEIGHT,
                }),
                alice.id(),
                HEIGHT,
                CORE_CHAIN_LOCKED_HEIGHT,
                rng.gen(),
                version,
            )
            .expect("contact request");
        insert_document(&drive, &dashpay, "contactRequest", &contact, version);

        let alice_key0_hash = alice.public_keys()[&0].public_key_hash().expect("key hash");
        Fixture {
            version,
            drive,
            dpns,
            dashpay,
            alice,
            bob,
            carol,
            alice_key0_hash,
            core_payment_address,
        }
    }

    /// The process-wide fixture at `protocol_version`, built on first use.
    pub fn get(protocol_version: ProtocolVersion) -> &'static Fixture {
        static FIXTURES: [OnceLock<Fixture>; LATEST_VERSION as usize + 1] =
            [const { OnceLock::new() }; LATEST_VERSION as usize + 1];
        let version = PlatformVersion::get(protocol_version).expect("known protocol version");
        FIXTURES[protocol_version as usize].get_or_init(|| Fixture::build(version))
    }

    pub fn dpns(&self) -> &DataContract {
        &self.dpns
    }

    pub fn dashpay(&self) -> &DataContract {
        &self.dashpay
    }

    pub fn metadata(&self) -> ResponseMetadata {
        ResponseMetadata {
            height: HEIGHT,
            core_chain_locked_height: CORE_CHAIN_LOCKED_HEIGHT,
            epoch: 0,
            time_ms: now_ms(),
            protocol_version: self.version.protocol_version,
            chain_id: CHAIN_ID.to_string(),
        }
    }

    fn root_hash(&self) -> [u8; 32] {
        self.drive
            .grove
            .root_hash(None, &self.version.drive.grove_version)
            .unwrap()
            .expect("root hash")
    }

    /// Signs the fixture state for `metadata` the way Tenderdash's
    /// Platform quorum does, so the proof verifies only against these
    /// metadata values and this quorum key.
    pub fn proof(&self, grovedb_proof: Vec<u8>, metadata: &ResponseMetadata) -> Proof {
        let state_id = StateId {
            app_version: u64::from(metadata.protocol_version),
            core_chain_locked_height: metadata.core_chain_locked_height,
            time: metadata.time_ms,
            app_hash: self.root_hash().to_vec(),
            height: metadata.height,
        };
        let round = 0;
        let state_id_hash = state_id
            .calculate_msg_hash(&metadata.chain_id, metadata.height as i64, round)
            .expect("state id hash");
        let block_id_hash = vec![0x33u8; 32];
        let vote = CanonicalVote {
            r#type: SignedMsgType::Precommit.into(),
            block_id: block_id_hash.clone(),
            chain_id: metadata.chain_id.clone(),
            height: metadata.height as i64,
            round: i64::from(round),
            state_id: state_id_hash,
        };
        let sign_digest = vote
            .calculate_sign_hash(
                &metadata.chain_id,
                PLATFORM_LLMQ_TYPE,
                &quorum().hash,
                metadata.height as i64,
                round,
            )
            .expect("sign digest");
        let signature = quorum()
            .secret
            .sign(SignatureSchemes::Basic, &sign_digest)
            .expect("sign");
        Proof {
            grovedb_proof,
            quorum_hash: quorum().hash.to_vec(),
            signature: signature.as_raw_value().to_compressed().to_vec(),
            round: round as u32,
            block_id_hash,
            quorum_type: u32::from(PLATFORM_LLMQ_TYPE),
        }
    }

    pub fn prove_identity(&self, id: Identifier) -> Vec<u8> {
        self.drive
            .prove_full_identity(id.to_buffer(), None, &self.version.drive)
            .expect("identity proof")
    }

    pub fn prove_identity_by_key_hash(&self, hash: [u8; 20]) -> Vec<u8> {
        self.drive
            .prove_full_identity_by_unique_public_key_hash(hash, None, self.version)
            .expect("identity by key hash proof")
    }

    pub fn prove_identity_contract_nonce(&self, id: Identifier, contract: Identifier) -> Vec<u8> {
        self.drive
            .prove_identity_contract_nonce(
                id.to_buffer(),
                contract.to_buffer(),
                None,
                &self.version.drive,
            )
            .expect("nonce proof")
    }

    /// Proves the documents a `DocumentQuery` selects; the query must be the
    /// one the shell builds, so the SDK verifies the proof against it.
    pub fn prove_documents(&self, query: &dash_sdk::platform::DocumentQuery) -> Vec<u8> {
        let drive_query = DriveDocumentQuery::try_from(query).expect("drive query");
        drive_query
            .execute_with_proof(&self.drive, None, None, self.version)
            .expect("documents proof")
            .0
    }

    pub fn prove_contested_vote_state(
        &self,
        query: ContestedDocumentVotePollDriveQuery,
    ) -> Vec<u8> {
        query
            .execute_with_proof(&self.drive, None, None, self.version)
            .expect("vote state proof")
            .0
    }

    /// The contested-name query the shell issues for `normalized_label`.
    pub fn contested_query(&self, normalized_label: &str) -> ContestedDocumentVotePollDriveQuery {
        ContestedDocumentVotePollDriveQuery {
            vote_poll: dash_sdk::dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll {
                contract_id: self.dpns.id(),
                document_type_name: "domain".to_string(),
                index_name: "parentNameAndLabel".to_string(),
                index_values: vec![
                    Value::Text("dash".to_string()),
                    Value::Text(normalized_label.to_string()),
                ],
            },
            result_type: ContestedDocumentVotePollDriveQueryResultType::VoteTally,
            allow_include_locked_and_abstaining_vote_tally: true,
            start_at: None,
            limit: Some(100),
            offset: None,
        }
    }
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_millis() as u64
}

/// A client with the test quorum's key pushed and no ChainLock anchor.
pub fn bare_client() -> Client {
    let client = Client::new(&config()).expect("client");
    client
        .provider()
        .set_quorum_keys(&[quorum().core_order_key()]);
    client
}

/// [`bare_client`] with a ChainLock anchor at `chainlock_height`.
pub fn mock_client_at(chainlock_height: u32) -> Client {
    let client = bare_client();
    client
        .provider()
        .set_local_core_chain_locked_height(chainlock_height);
    client
}

/// A client wired to `dash-sdk`'s mock transport with the fixture's
/// quorum key and a fresh ChainLock anchor at the proved core height.
pub fn mock_client() -> Client {
    mock_client_at(CORE_CHAIN_LOCKED_HEIGHT)
}

/// A mock-transport SDK at `version`, with no expectations: enough for the
/// builders, which only read the compiled-in contracts through the client's
/// provider.
pub fn offline_sdk(client: &Client, version: &'static PlatformVersion) -> Sdk {
    client
        .mock_sdk_builder()
        .with_initial_version(version)
        .build()
        .expect("mock sdk")
}

/// Installs the fixture's identity proof for Alice under `metadata`, after
/// `tamper` had its way with the proof.
pub fn install_identity(
    fixture: &Fixture,
    client: &Client,
    metadata: ResponseMetadata,
    tamper: impl FnOnce(&mut Proof),
) {
    let mut proof = fixture.proof(fixture.prove_identity(fixture.alice.id()), &metadata);
    tamper(&mut proof);
    install_ok(
        fixture,
        client,
        &identity_request(fixture.alice.id()),
        identity_response(proof, metadata),
    );
}

/// Asserts a status kind, printing the message on a mismatch.
#[track_caller]
pub fn assert_kind(status: &dash_platform_cxx::ffi::Status, kind: StatusKind) {
    assert_eq!(status.kind, kind, "{}", status.message);
}

/// Installs a mock SDK that answers `request` with a successful `response`
/// on `client`, seeded at the fixture's protocol version: the state of a
/// network SDK after its first verified response.
pub fn install_ok<R>(fixture: &Fixture, client: &Client, request: &R, response: R::Response)
where
    R: TransportRequest,
    R::Response: Clone,
{
    install_seeded(
        client,
        request,
        Ok(execution_response(response)),
        Some(fixture.version),
    );
}

/// [`install_ok`] with the SDK at the per-network protocol floor it seeds
/// itself with before any response has been verified.
pub fn install_at_floor<R>(client: &Client, request: &R, response: R::Response)
where
    R: TransportRequest,
    R::Response: Clone,
{
    install_seeded(client, request, Ok(execution_response(response)), None);
}

/// [`install_ok`] with the node refusing `request` with the gRPC `status`.
pub fn install_refusal<R>(
    fixture: &Fixture,
    client: &Client,
    request: &R,
    status: dash_sdk::dapi_grpc::tonic::Status,
) where
    R: TransportRequest,
    R::Response: Clone,
{
    let error = ExecutionError {
        inner: DapiClientError::Transport(TransportError::Grpc(status)),
        retries: 0,
        address: None,
    };
    install_seeded(client, request, Err(error), Some(fixture.version));
}

/// A successful mock-transport answer.
pub fn execution_response<T>(inner: T) -> ExecutionResponse<T> {
    ExecutionResponse {
        inner,
        retries: 0,
        address: "https://127.0.0.1:1".parse().expect("address"),
    }
}

/// The expectation travels through a dump directory because that is the
/// only way to seed the mock transport from outside `dash-sdk`.
fn install_seeded<R>(
    client: &Client,
    request: &R,
    response: MockResult<R>,
    initial_version: Option<&'static PlatformVersion>,
) where
    R: TransportRequest,
    R::Response: Clone,
{
    // Tests run in parallel; a wall-clock name could hand one test
    // another's expectation.
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "dash-platform-cxx-mock-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).expect("dump dir");
    let dump = DumpData::new(request, &response);
    dump.save(&dir.join(dump.filename().expect("dump file name")))
        .expect("save expectation");
    let mut builder = client.mock_sdk_builder().with_dump_dir(&dir);
    if let Some(version) = initial_version {
        builder = builder.with_initial_version(version);
    }
    let sdk: Sdk = builder.build().expect("mock sdk");
    let _ = std::fs::remove_dir_all(&dir);
    client.set_sdk(sdk);
}

/// `Identity::fetch` sends its request through the SDK's `IdentityRequest`
/// wrapper, so the mock keys on the wrapper, not the proto message.
pub fn identity_request(id: Identifier) -> IdentityRequest {
    IdentityRequest::GetIdentity(proto::GetIdentityRequest {
        version: Some(proto::get_identity_request::Version::V0(
            proto::get_identity_request::GetIdentityRequestV0 {
                id: id.to_vec(),
                prove: true,
            },
        )),
    })
}

pub fn identity_response(proof: Proof, metadata: ResponseMetadata) -> IdentityResponse {
    IdentityResponse::GetIdentity(identity_proto_response(
        proto::get_identity_response::get_identity_response_v0::Result::Proof(proof),
        metadata,
    ))
}

pub fn identity_proto_response(
    result: proto::get_identity_response::get_identity_response_v0::Result,
    metadata: ResponseMetadata,
) -> proto::GetIdentityResponse {
    proto::GetIdentityResponse {
        version: Some(proto::get_identity_response::Version::V0(
            proto::get_identity_response::GetIdentityResponseV0 {
                metadata: Some(metadata),
                result: Some(result),
            },
        )),
    }
}

pub fn identity_by_key_hash_request(hash: [u8; 20]) -> IdentityRequest {
    IdentityRequest::GetIdentityByPublicKeyHash(proto::GetIdentityByPublicKeyHashRequest {
        version: Some(proto::get_identity_by_public_key_hash_request::Version::V0(
            proto::get_identity_by_public_key_hash_request::GetIdentityByPublicKeyHashRequestV0 {
                public_key_hash: hash.to_vec(),
                prove: true,
            },
        )),
    })
}

pub fn identity_by_key_hash_response(proof: Proof, metadata: ResponseMetadata) -> IdentityResponse {
    IdentityResponse::GetIdentityByPublicKeyHash(proto::GetIdentityByPublicKeyHashResponse {
        version: Some(proto::get_identity_by_public_key_hash_response::Version::V0(
            proto::get_identity_by_public_key_hash_response::GetIdentityByPublicKeyHashResponseV0 {
                metadata: Some(metadata),
                result: Some(
                    proto::get_identity_by_public_key_hash_response::get_identity_by_public_key_hash_response_v0::Result::Proof(proof),
                ),
            },
        )),
    })
}

pub fn nonce_request(
    id: Identifier,
    contract: Identifier,
) -> proto::GetIdentityContractNonceRequest {
    proto::GetIdentityContractNonceRequest {
        version: Some(proto::get_identity_contract_nonce_request::Version::V0(
            proto::get_identity_contract_nonce_request::GetIdentityContractNonceRequestV0 {
                identity_id: id.to_vec(),
                contract_id: contract.to_vec(),
                prove: true,
            },
        )),
    }
}

pub fn nonce_response(
    proof: Proof,
    metadata: ResponseMetadata,
) -> proto::GetIdentityContractNonceResponse {
    proto::GetIdentityContractNonceResponse {
        version: Some(proto::get_identity_contract_nonce_response::Version::V0(
            proto::get_identity_contract_nonce_response::GetIdentityContractNonceResponseV0 {
                metadata: Some(metadata),
                result: Some(
                    proto::get_identity_contract_nonce_response::get_identity_contract_nonce_response_v0::Result::Proof(proof),
                ),
            },
        )),
    }
}

pub fn documents_response(proof: Proof, metadata: ResponseMetadata) -> proto::GetDocumentsResponse {
    proto::GetDocumentsResponse {
        version: Some(proto::get_documents_response::Version::V0(
            proto::get_documents_response::GetDocumentsResponseV0 {
                metadata: Some(metadata),
                result: Some(
                    proto::get_documents_response::get_documents_response_v0::Result::Proof(proof),
                ),
            },
        )),
    }
}

pub fn contested_response(
    proof: Proof,
    metadata: ResponseMetadata,
) -> proto::GetContestedResourceVoteStateResponse {
    proto::GetContestedResourceVoteStateResponse {
        version: Some(proto::get_contested_resource_vote_state_response::Version::V0(
            proto::get_contested_resource_vote_state_response::GetContestedResourceVoteStateResponseV0 {
                metadata: Some(metadata),
                result: Some(
                    proto::get_contested_resource_vote_state_response::get_contested_resource_vote_state_response_v0::Result::Proof(proof),
                ),
            },
        )),
    }
}

/// The encoder settings of an SDK at the fixture's protocol version, with
/// proofs on; the same `Query` impls the shell's SDK encodes with.
fn query_settings(fixture: &Fixture) -> QuerySettings<'static> {
    static REQUEST_SETTINGS: RequestSettings = RequestSettings::default();
    QuerySettings {
        request_settings: &REQUEST_SETTINGS,
        protocol_version: fixture.version,
        prove: true,
    }
}

/// The wire request the shell sends for a `DocumentQuery`.
pub fn documents_request(
    fixture: &Fixture,
    query: &dash_sdk::platform::DocumentQuery,
) -> proto::GetDocumentsRequest {
    query
        .query(&query_settings(fixture))
        .expect("documents request")
}

/// The wire request the shell sends for a contested-name vote state query.
pub fn contested_request(
    fixture: &Fixture,
    query: &ContestedDocumentVotePollDriveQuery,
) -> proto::GetContestedResourceVoteStateRequest {
    query
        .query(&query_settings(fixture))
        .expect("contested request")
}
