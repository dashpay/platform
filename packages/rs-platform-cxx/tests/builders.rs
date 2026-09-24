// Copyright (c) 2026 The Dash Core developers
// Distributed under the MIT software license, see the accompanying
// file COPYING or http://www.opensource.org/licenses/mit-license.php.

//! The state-transition builders, checked byte for byte against the
//! constructions `dash-sdk` and dpp perform with in-process private keys:
//! the identity create against `try_from_identity_with_signer_and_private_key`
//! (what `put_to_platform_with_private_key` builds), the DPNS documents
//! against `register_dpns_name`'s inline assembly, and the contact request
//! against `send_contact_request`'s document. The signer stands in for the
//! C++ `WalletSigner`: it receives the signable bytes (or the asset-lock
//! sighash), signs with `dashcore::signer`, and records what it was asked.
//!
//! Every test runs at the protocol version testnet and mainnet run and at
//! this build's latest: the document id derivation, the contested prefund
//! and the DashPay contract all differ between them.
//!
//! The vectors are generated here from the same fixed keys on every run,
//! never hand-written; `first_byte_vectors` checks the one file Core pins
//! against them (`UPDATE_TEST_VECTORS=1` rewrites it).

mod common;

use std::collections::BTreeMap;
use std::sync::Mutex;

use common::{mock_client, offline_sdk, DEPLOYED_VERSION};
use dash_platform_cxx::builders;
use dash_platform_cxx::ffi::{
    self, AssetLockProofInput, BoundsKind, CompactXpub, ContactRequestInput, ContractBounds,
    IdentityKey, NewIdentityKey, ProfileInput,
};
use dash_platform_cxx::signer::{AssetLockSigner, BytesSigner, SignerCallbacks};
use dash_sdk::dpp::dashcore::consensus::serialize;
use dash_sdk::dpp::dashcore::hashes::Hash;
use dash_sdk::dpp::dashcore::secp256k1::{PublicKey, Secp256k1, SecretKey};
use dash_sdk::dpp::dashcore::signer::{sign, sign_hash};
use dash_sdk::dpp::dashcore::{Network, OutPoint, PrivateKey, Txid};
use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;
use dash_sdk::dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dash_sdk::dpp::document::{Document, DocumentV0, DocumentV0Getters, DocumentV0Setters};
use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
use dash_sdk::dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dash_sdk::dpp::identity::identity_public_key::contract_bounds::ContractBounds as DppBounds;
use dash_sdk::dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dash_sdk::dpp::identity::signer::Signer;
use dash_sdk::dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
use dash_sdk::dpp::identity::v0::IdentityV0;
use dash_sdk::dpp::identity::{Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dash_sdk::dpp::key_wallet::bip32::DerivationPath;
use dash_sdk::dpp::key_wallet::signer::Signer as KeyWalletSigner;
use dash_sdk::dpp::native_bls::NativeBlsModule;
use dash_sdk::dpp::platform_value::Value;
use dash_sdk::dpp::prelude::{AssetLockProof, Identifier};
use dash_sdk::dpp::serialization::{
    PlatformDeserializableUntrusted, PlatformSerializable, Signable,
};
use dash_sdk::dpp::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use dash_sdk::dpp::state_transition::batch_transition::batched_transition::document_transition::{
    DocumentTransition, DocumentTransitionV0Methods,
};
use dash_sdk::dpp::state_transition::batch_transition::batched_transition::BatchedTransitionRef;
use dash_sdk::dpp::state_transition::batch_transition::document_create_transition::v0::v0_methods::DocumentCreateTransitionV0Methods;
use dash_sdk::dpp::state_transition::batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
use dash_sdk::dpp::state_transition::batch_transition::BatchTransition;
use dash_sdk::dpp::state_transition::identity_create_transition::methods::IdentityCreateTransitionMethodsV0;
use dash_sdk::dpp::state_transition::identity_create_transition::IdentityCreateTransition;
use dash_sdk::dpp::state_transition::StateTransition;
use dash_sdk::dpp::system_data_contracts::SystemDataContract;
use dash_sdk::dpp::tests::fixtures::instant_asset_lock_proof_fixture;
use dash_sdk::dpp::util::hash::{hash_double, hash_single};
use dash_sdk::dpp::util::strings::convert_to_homograph_safe_chars;
use dash_sdk::dpp::version::{PlatformVersion, ProtocolVersion, LATEST_VERSION};
use dash_sdk::platform::dashpay::{
    ContactRequestInput as SdkContactRequestInput, EcdhProvider, RecipientIdentity,
};
use dpp::data_contract::validate_document::DataContractDocumentValidationMethodsV0;
use platform_encryption::{
    compact_xpub_bytes, decrypt_extended_public_key, derive_shared_key_ecdh,
};
use simple_signer::SingleKeySigner;
use test_case::test_matrix;

/// Fixed test keys: identity keys 0 (master) and 1 (high), the DIP-15
/// encryption pair, and the asset-lock outpoint key.
const MASTER_SK: [u8; 32] = [0x11; 32];
const HIGH_SK: [u8; 32] = [0x22; 32];
const ENCRYPTION_SK: [u8; 32] = [0x44; 32];
const ASSET_LOCK_SK: [u8; 32] = [0x33; 32];

fn pubkey(secret: &[u8; 32]) -> [u8; 33] {
    PublicKey::from_secret_key(
        &Secp256k1::new(),
        &SecretKey::from_byte_array(secret).unwrap(),
    )
    .serialize()
}

/// What the wallet was asked to sign, by key id, or `AssetLock`.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Request {
    Key(u32, Vec<u8>),
    AssetLock([u8; 32]),
}

/// The test wallet: signs with the fixed keys and records every request.
/// It implements the same two callbacks the C++ `WalletSigner` forwards
/// (`SignerCallbacks`), so the builders run exactly as they do under the
/// bridge; `tests/cxx_smoke.cc` covers the C++ side of that forwarding.
struct Wallet {
    requests: Mutex<Vec<Request>>,
    refuse: bool,
    /// Overrides the asset-lock signature's compact header byte.
    signature_header: Option<u8>,
    tokio_runtime_entered: Mutex<bool>,
}

impl Wallet {
    fn new() -> Self {
        Wallet {
            requests: Mutex::new(Vec::new()),
            refuse: false,
            signature_header: None,
            tokio_runtime_entered: Mutex::new(false),
        }
    }

    fn refusing() -> Self {
        Wallet {
            refuse: true,
            ..Wallet::new()
        }
    }

    fn with_signature_header(header: u8) -> Self {
        Wallet {
            signature_header: Some(header),
            ..Wallet::new()
        }
    }

    fn secret(key_id: u32) -> Option<[u8; 32]> {
        match key_id {
            0 => Some(MASTER_SK),
            1 => Some(HIGH_SK),
            2 => Some(ENCRYPTION_SK),
            _ => None,
        }
    }

    fn requests(&self) -> Vec<Request> {
        self.requests.lock().unwrap().clone()
    }

    fn note_thread_state(&self) {
        *self.tokio_runtime_entered.lock().unwrap() |=
            tokio::runtime::Handle::try_current().is_ok();
    }
}

impl SignerCallbacks for Wallet {
    fn sign_for_key(&self, key_id: u32, signable: &[u8], out: &mut Vec<u8>) -> bool {
        self.note_thread_state();
        self.requests
            .lock()
            .unwrap()
            .push(Request::Key(key_id, signable.to_vec()));
        if self.refuse {
            return false;
        }
        let Some(secret) = Self::secret(key_id) else {
            return false;
        };
        *out = sign(signable, &secret).expect("sign").to_vec();
        true
    }

    fn sign_asset_lock_sighash(&self, sighash: &[u8; 32], out: &mut Vec<u8>) -> bool {
        self.note_thread_state();
        self.requests
            .lock()
            .unwrap()
            .push(Request::AssetLock(*sighash));
        if self.refuse {
            return false;
        }
        *out = sign_hash(sighash, &ASSET_LOCK_SK).expect("sign").to_vec();
        if let Some(header) = self.signature_header {
            out[0] = header;
        }
        true
    }
}

fn high_key() -> IdentityKey {
    IdentityKey {
        id: 1,
        purpose: Purpose::AUTHENTICATION as u8,
        security_level: SecurityLevel::HIGH as u8,
        key_type: KeyType::ECDSA_SECP256K1 as u8,
        read_only: false,
        data: pubkey(&HIGH_SK).to_vec(),
        disabled_at: 0,
        bounds: ContractBounds::default(),
    }
}

fn dpp_high_key() -> IdentityPublicKey {
    builders::identity_key(&high_key()).expect("key")
}

fn version(protocol_version: ProtocolVersion) -> &'static PlatformVersion {
    PlatformVersion::get(protocol_version).expect("known protocol version")
}

fn owner() -> Identifier {
    Identifier::from([0x77u8; 32])
}

/// The single document transition of a built batch.
fn document_transition(bytes: &[u8]) -> (StateTransition, Identifier, BTreeMap<String, Value>) {
    let state_transition =
        StateTransition::deserialize_from_bytes_untrusted(bytes).expect("deserialize");
    let StateTransition::Batch(batch) = &state_transition else {
        panic!("not a batch");
    };
    let Some(BatchedTransitionRef::Document(transition)) = batch.first_transition() else {
        panic!("no document transition");
    };
    let id = transition.get_id();
    let data = transition.data().cloned().unwrap_or_default();
    (state_transition, id, data)
}

/// Builds the reference batch for `document` the way `put_to_platform`
/// does for a create without caller entropy (`register_dpns_name`'s path):
/// the id is derived from the entropy and nonce under `version`, then the
/// transition is signed with the in-process high key.
fn reference_create(
    version: &'static PlatformVersion,
    contract: SystemDataContract,
    document_type: &str,
    mut document: Document,
    entropy: [u8; 32],
    nonce: u64,
) -> Vec<u8> {
    let contract =
        dash_sdk::dpp::system_data_contracts::load_system_data_contract(contract, version)
            .expect("contract");
    let document_type = contract
        .document_type_for_name(document_type)
        .expect("type");
    document.set_id(
        Document::generate_document_id(
            &contract.id(),
            &document.owner_id(),
            document_type.name(),
            &entropy,
            nonce,
            version,
        )
        .expect("document id"),
    );
    let signer = SingleKeySigner::new_from_slice(&HIGH_SK, Network::Testnet).expect("signer");
    let transition = futures::executor::block_on(
        BatchTransition::new_document_creation_transition_from_document(
            document,
            document_type,
            entropy,
            &dpp_high_key(),
            nonce,
            0,
            None,
            &signer,
            version,
            None,
        ),
    )
    .expect("reference transition");
    transition.serialize_to_bytes().expect("serialize")
}

/// What Drive's advanced-structure validation checks on the document
/// transition `bytes` carries at `version`: a create's id is the one it
/// recomputes (`InvalidDocumentTransitionIdError` otherwise), and the data
/// is valid against the system contract at that version.
fn assert_drive_accepts_structure(bytes: &[u8], version: &'static PlatformVersion) {
    let (state_transition, id, data) = document_transition(bytes);
    let StateTransition::Batch(batch) = &state_transition else {
        unreachable!()
    };
    let Some(BatchedTransitionRef::Document(transition)) = batch.first_transition() else {
        unreachable!()
    };
    if let Some(entropy) = transition.entropy() {
        let expected = Document::generate_document_id(
            &transition.data_contract_id(),
            &owner(),
            transition.document_type_name(),
            &entropy,
            transition.identity_contract_nonce(),
            version,
        )
        .expect("document id");
        assert_ne!(id, Identifier::default(), "the id is set");
        assert_eq!(
            id, expected,
            "consensus recomputes the id the create carries"
        );
    }
    let contract = [SystemDataContract::DPNS, SystemDataContract::Dashpay]
        .into_iter()
        .find(|contract| contract.id() == transition.data_contract_id())
        .expect("a system contract");
    let result = dash_sdk::dpp::system_data_contracts::load_system_data_contract(contract, version)
        .expect("contract")
        .validate_document_properties(transition.document_type_name(), Value::from(data), version)
        .expect("validation runs");
    assert!(result.is_valid(), "{:?}", result.errors);
}

fn check_built(built: &ffi::Built, reference: &[u8], version: &'static PlatformVersion) {
    assert_eq!(
        hex::encode(&built.bytes),
        hex::encode(reference),
        "state transition bytes"
    );
    assert_eq!(
        built.hash,
        hash_single(&built.bytes),
        "hash is sha256 of the bytes"
    );
    let (_, id, _) = document_transition(&built.bytes);
    assert_eq!(
        built.object_id,
        id.to_buffer(),
        "object_id is the document id"
    );
    assert_drive_accepts_structure(&built.bytes, version);
}

// --- DPNS ------------------------------------------------------------------

#[test_matrix([DEPLOYED_VERSION, LATEST_VERSION])]
fn dpns_preorder_matches_register_dpns_name_for_the_same_salt(protocol_version: ProtocolVersion) {
    let version = version(protocol_version);
    let salt = [0x55u8; 32];
    let entropy = [0x66u8; 32];
    let wallet = Wallet::new();
    let built = builders::build_dpns_preorder(
        version,
        owner().to_buffer(),
        7,
        "Alice",
        &salt,
        entropy,
        &high_key(),
        &wallet,
    )
    .expect("preorder");
    // `register_dpns_name`'s document: saltedDomainHash = sha256d(salt ‖
    // normalized label ‖ ".dash"), no other property.
    let mut preimage = salt.to_vec();
    preimage.extend_from_slice(b"a11ce.dash");
    let reference_document = Document::V0(DocumentV0 {
        owner_id: owner(),
        properties: BTreeMap::from([(
            "saltedDomainHash".to_string(),
            Value::Bytes32(hash_double(preimage)),
        )]),
        ..Default::default()
    });
    let reference = reference_create(
        version,
        SystemDataContract::DPNS,
        "preorder",
        reference_document,
        entropy,
        7,
    );
    check_built(&built, &reference, version);
    let requests = wallet.requests();
    assert_eq!(requests.len(), 1);
    let Request::Key(1, signable) = &requests[0] else {
        panic!("signed with the wrong key: {requests:?}");
    };
    let (state_transition, _, _) = document_transition(&built.bytes);
    assert_eq!(
        signable,
        &state_transition.signable_bytes().unwrap(),
        "the wallet saw the signable preimage"
    );
}

#[test_matrix([DEPLOYED_VERSION, LATEST_VERSION])]
fn dpns_domain_matches_register_dpns_name_for_the_same_salt(protocol_version: ProtocolVersion) {
    let version = version(protocol_version);
    let salt = [0x55u8; 32];
    let entropy = [0x66u8; 32];
    let wallet = Wallet::new();
    let built = builders::build_dpns_domain(
        version,
        owner().to_buffer(),
        8,
        "Alice",
        &salt,
        entropy,
        &high_key(),
        &wallet,
    )
    .expect("domain");
    let reference_document = Document::V0(DocumentV0 {
        owner_id: owner(),
        properties: BTreeMap::from([
            (
                "parentDomainName".to_string(),
                Value::Text("dash".to_string()),
            ),
            (
                "normalizedParentDomainName".to_string(),
                Value::Text("dash".to_string()),
            ),
            ("label".to_string(), Value::Text("Alice".to_string())),
            (
                "normalizedLabel".to_string(),
                Value::Text(convert_to_homograph_safe_chars("Alice")),
            ),
            ("preorderSalt".to_string(), Value::Bytes32(salt)),
            (
                "records".to_string(),
                Value::Map(vec![(
                    Value::Text("identity".to_string()),
                    Value::Identifier(owner().to_buffer()),
                )]),
            ),
            (
                "subdomainRules".to_string(),
                Value::Map(vec![(
                    Value::Text("allowSubdomains".to_string()),
                    Value::Bool(false),
                )]),
            ),
        ]),
        ..Default::default()
    });
    let reference = reference_create(
        version,
        SystemDataContract::DPNS,
        "domain",
        reference_document,
        entropy,
        8,
    );
    check_built(&built, &reference, version);
    // "a11ce" is contested (3-19 chars of [a-z01-]): dpp attached the
    // prefund from the domain type's contested index.
    let (state_transition, _, _) = document_transition(&built.bytes);
    let StateTransition::Batch(batch) = &state_transition else {
        unreachable!()
    };
    let Some(BatchedTransitionRef::Document(transition)) = batch.first_transition() else {
        unreachable!()
    };
    let DocumentTransition::Create(create) = transition else {
        panic!("not a create")
    };
    assert_eq!(
        create
            .prefunded_voting_balance()
            .as_ref()
            .map(|(_, credits)| *credits),
        Some(dash_platform_cxx::helpers::contested_vote_fund_credits(
            version
        ))
    );
}

#[test_matrix([DEPLOYED_VERSION, LATEST_VERSION])]
fn uncontested_domain_carries_no_prefund(protocol_version: ProtocolVersion) {
    let version = version(protocol_version);
    let wallet = Wallet::new();
    let built = builders::build_dpns_domain(
        version,
        owner().to_buffer(),
        1,
        "alice-2024",
        &[1u8; 32],
        [2u8; 32],
        &high_key(),
        &wallet,
    )
    .expect("domain");
    let (state_transition, _, _) = document_transition(&built.bytes);
    let StateTransition::Batch(batch) = &state_transition else {
        unreachable!()
    };
    let Some(BatchedTransitionRef::Document(transition)) = batch.first_transition() else {
        unreachable!()
    };
    let DocumentTransition::Create(create) = transition else {
        panic!("not a create")
    };
    assert!(create.prefunded_voting_balance().is_none());
}

// --- profile ---------------------------------------------------------------

#[test_matrix([DEPLOYED_VERSION, LATEST_VERSION])]
fn profile_create_and_replace(protocol_version: ProtocolVersion) {
    let version = version(protocol_version);
    let entropy = [0x66u8; 32];
    let wallet = Wallet::new();
    let input = ProfileInput {
        display_name: "Alice".to_string(),
        public_message: String::new(),
    };
    let built = builders::build_profile(
        version,
        owner().to_buffer(),
        5,
        &ffi::Profile::default(),
        &input,
        entropy,
        &high_key(),
        &wallet,
    )
    .expect("profile");
    let reference_document = Document::V0(DocumentV0 {
        owner_id: owner(),
        properties: BTreeMap::from([("displayName".to_string(), Value::Text("Alice".to_string()))]),
        ..Default::default()
    });
    let reference = reference_create(
        version,
        SystemDataContract::Dashpay,
        "profile",
        reference_document,
        entropy,
        5,
    );
    check_built(&built, &reference, version);
    let (_, _, data) = document_transition(&built.bytes);
    assert!(
        !data.contains_key("publicMessage"),
        "empty strings are omitted"
    );

    // The replaced document is the one `get_profile` read, with the edits
    // applied: what another wallet set (avatar, payment addresses) and the
    // embedder does not edit stays. The payment addresses arrive with
    // DashPay v2 (protocol version 14); before, no stored profile has them.
    let payment_addresses = dashpay_has_payment_addresses(version);
    let payment = |bytes: Vec<u8>| if payment_addresses { bytes } else { Vec::new() };
    let existing = ffi::Profile {
        document_id: built.object_id,
        owner: owner().to_buffer(),
        revision: 1,
        display_name: "Alice".to_string(),
        avatar_url: "https://example.org/a.png".to_string(),
        avatar_hash: vec![0x11u8; 32],
        avatar_fingerprint: vec![0x22u8; 8],
        core_payment_address: payment(vec![0x33u8; 21]),
        platform_payment_address: payment(vec![0x44u8; 21]),
        shielded_address: payment(vec![0x55u8; 43]),
        created_at: 1,
        updated_at: 1,
        ..Default::default()
    };
    let replace = ProfileInput {
        display_name: "Alice".to_string(),
        public_message: "hello".to_string(),
    };
    let replaced = builders::build_profile(
        version,
        owner().to_buffer(),
        6,
        &existing,
        &replace,
        entropy,
        &high_key(),
        &wallet,
    )
    .expect("replace");
    assert_eq!(replaced.object_id, built.object_id);
    let (state_transition, id, data) = document_transition(&replaced.bytes);
    assert_eq!(id.to_buffer(), built.object_id);
    assert_eq!(data["displayName"], Value::Text("Alice".to_string()));
    assert_eq!(data["publicMessage"], Value::Text("hello".to_string()));
    assert_eq!(
        data["avatarUrl"],
        Value::Text("https://example.org/a.png".to_string())
    );
    assert_eq!(data["avatarHash"].to_bytes().unwrap(), vec![0x11u8; 32]);
    assert_eq!(
        data["avatarFingerprint"].to_bytes().unwrap(),
        vec![0x22u8; 8]
    );
    if payment_addresses {
        assert_eq!(
            data["corePaymentAddress"].to_bytes().unwrap(),
            vec![0x33u8; 21]
        );
        assert_eq!(
            data["platformPaymentAddress"].to_bytes().unwrap(),
            vec![0x44u8; 21]
        );
        assert_eq!(
            data["shieldedAddress"].to_bytes().unwrap(),
            vec![0x55u8; 43]
        );
        assert_eq!(data.len(), 8, "nothing else is added");
    } else {
        assert_eq!(data.len(), 5, "nothing else is added");
    }
    // Byte for byte what `put_to_platform` sends for a document at a
    // revision above the first: a replacement of the sanitized document.
    let reference = reference_replace(
        version,
        Document::V0(DocumentV0 {
            id: Identifier::from(built.object_id),
            owner_id: owner(),
            properties: data.clone(),
            revision: Some(2),
            ..Default::default()
        }),
        6,
    );
    assert_eq!(hex::encode(&replaced.bytes), hex::encode(&reference));
    assert_drive_accepts_structure(&replaced.bytes, version);
    let StateTransition::Batch(batch) = &state_transition else {
        unreachable!()
    };
    let Some(BatchedTransitionRef::Document(transition)) = batch.first_transition() else {
        unreachable!()
    };
    assert_eq!(transition.revision(), Some(2));
    assert_eq!(transition.identity_contract_nonce(), 6);

    // Clearing an edited field drops it; the carried fields stay.
    let cleared = builders::build_profile(
        version,
        owner().to_buffer(),
        7,
        &existing,
        &ProfileInput {
            display_name: String::new(),
            public_message: "hello".to_string(),
        },
        entropy,
        &high_key(),
        &wallet,
    )
    .expect("clear");
    let (_, _, data) = document_transition(&cleared.bytes);
    assert!(!data.contains_key("displayName"));
    assert!(data.contains_key("avatarUrl"));

    // A read profile of another identity, or one without a revision, is
    // not replaced.
    let foreign = ffi::Profile {
        owner: [9u8; 32],
        ..existing.clone()
    };
    assert!(builders::build_profile(
        version,
        owner().to_buffer(),
        5,
        &foreign,
        &replace,
        entropy,
        &high_key(),
        &wallet,
    )
    .is_err());
    let unrevised = ffi::Profile {
        revision: 0,
        ..existing.clone()
    };
    assert!(builders::build_profile(
        version,
        owner().to_buffer(),
        5,
        &unrevised,
        &replace,
        entropy,
        &high_key(),
        &wallet,
    )
    .is_err());

    // A field the network's contract does not have is refused before
    // anything is signed, not after Drive charged for it.
    if !payment_addresses {
        let refused = Wallet::new();
        let error = builders::build_profile(
            version,
            owner().to_buffer(),
            6,
            &ffi::Profile {
                core_payment_address: vec![0x33u8; 21],
                ..existing.clone()
            },
            &replace,
            entropy,
            &high_key(),
            &refused,
        )
        .unwrap_err();
        assert!(error.contains("corePaymentAddress"), "{error}");
        assert!(refused.requests().is_empty());
    }
}

/// Whether the DashPay contract at `version` has the profile payment
/// address fields (DashPay v2, protocol version 14).
fn dashpay_has_payment_addresses(version: &PlatformVersion) -> bool {
    dash_sdk::dpp::system_data_contracts::load_system_data_contract(
        SystemDataContract::Dashpay,
        version,
    )
    .expect("contract")
    .document_type_for_name("profile")
    .expect("profile")
    .properties()
    .contains_key("corePaymentAddress")
}

/// The replacement batch `put_to_platform` builds for `document` (already
/// at its new revision) over the DashPay profile type, signed with the
/// in-process high key.
fn reference_replace(version: &'static PlatformVersion, document: Document, nonce: u64) -> Vec<u8> {
    let contract = dash_sdk::dpp::system_data_contracts::load_system_data_contract(
        SystemDataContract::Dashpay,
        version,
    )
    .expect("contract");
    let document_type = contract.document_type_for_name("profile").expect("type");
    let signer = SingleKeySigner::new_from_slice(&HIGH_SK, Network::Testnet).expect("signer");
    let transition = futures::executor::block_on(
        BatchTransition::new_document_replacement_transition_from_document(
            document,
            document_type,
            &dpp_high_key(),
            nonce,
            0,
            None,
            &signer,
            version,
            None,
        ),
    )
    .expect("reference transition");
    transition.serialize_to_bytes().expect("serialize")
}

// --- contact request -------------------------------------------------------

fn identity_with_keys(
    id: Identifier,
    keys: &[(u32, Purpose, SecurityLevel, [u8; 33])],
) -> Identity {
    let public_keys = keys
        .iter()
        .map(|(key_id, purpose, level, data)| {
            let bounds = matches!(purpose, Purpose::ENCRYPTION | Purpose::DECRYPTION).then(|| {
                DppBounds::SingleContractDocumentType {
                    id: SystemDataContract::Dashpay.id(),
                    document_type_name: "contactRequest".to_string(),
                }
            });
            let key: IdentityPublicKey = IdentityPublicKeyV0 {
                id: *key_id,
                purpose: *purpose,
                security_level: *level,
                contract_bounds: bounds,
                key_type: KeyType::ECDSA_SECP256K1,
                read_only: false,
                data: data.to_vec().into(),
                disabled_at: None,
            }
            .into();
            (*key_id, key)
        })
        .collect();
    IdentityV0 {
        id,
        public_keys,
        balance: 0,
        revision: 0,
    }
    .into()
}

fn ffi_identity(identity: &Identity) -> ffi::Identity {
    ffi::Identity {
        id: identity.id().to_buffer(),
        balance: identity.balance(),
        revision: identity.revision(),
        keys: identity
            .public_keys()
            .values()
            .map(|key| IdentityKey {
                id: key.id(),
                purpose: key.purpose() as u8,
                security_level: key.security_level() as u8,
                key_type: key.key_type() as u8,
                read_only: key.read_only(),
                data: key.data().to_vec(),
                disabled_at: 0,
                bounds: match key.contract_bounds() {
                    Some(DppBounds::SingleContractDocumentType {
                        id,
                        document_type_name,
                    }) => ContractBounds {
                        kind: BoundsKind::SingleContractDocumentType,
                        contract_id: id.to_buffer(),
                        document_type: document_type_name.clone(),
                    },
                    _ => ContractBounds::default(),
                },
            })
            .collect(),
    }
}

#[test_matrix([DEPLOYED_VERSION, LATEST_VERSION])]
fn contact_request_matches_send_contact_request_for_the_same_secret(
    protocol_version: ProtocolVersion,
) {
    let version = version(protocol_version);
    let sender_sk = SecretKey::from_byte_array(&ENCRYPTION_SK).unwrap();
    let recipient_sk = SecretKey::from_byte_array(&[0x88u8; 32]).unwrap();
    let secp = Secp256k1::new();
    let recipient_pk = PublicKey::from_secret_key(&secp, &recipient_sk);
    let sender = identity_with_keys(
        owner(),
        &[
            (
                0,
                Purpose::AUTHENTICATION,
                SecurityLevel::MASTER,
                pubkey(&MASTER_SK),
            ),
            (
                1,
                Purpose::AUTHENTICATION,
                SecurityLevel::HIGH,
                pubkey(&HIGH_SK),
            ),
            (
                2,
                Purpose::ENCRYPTION,
                SecurityLevel::MEDIUM,
                pubkey(&ENCRYPTION_SK),
            ),
        ],
    );
    let recipient = identity_with_keys(
        Identifier::from([0x99u8; 32]),
        &[
            (
                0,
                Purpose::AUTHENTICATION,
                SecurityLevel::MASTER,
                pubkey(&[0x87u8; 32]),
            ),
            (
                2,
                Purpose::ENCRYPTION,
                SecurityLevel::MEDIUM,
                pubkey(&[0x86u8; 32]),
            ),
            (
                3,
                Purpose::DECRYPTION,
                SecurityLevel::MEDIUM,
                recipient_pk.serialize(),
            ),
        ],
    );
    // Core derives the ECDH secret and the compact xpub; the shell never
    // sees the sender's ENCRYPTION private key.
    let shared_secret = derive_shared_key_ecdh(&sender_sk, &recipient_pk);
    let xpub = CompactXpub {
        parent_fingerprint: [1, 2, 3, 4],
        chain_code: [0x5au8; 32],
        public_key: pubkey(&[0x5bu8; 32]),
    };
    let input = ContactRequestInput {
        to_user_id: recipient.id().to_buffer(),
        sender_key_index: 2,
        recipient_key_index: 3,
        recipient_pubkey: recipient_pk.serialize(),
        account_reference: 0x1000_0005,
        compact_xpub: xpub.clone(),
        shared_secret,
        account_label: "savings".to_string(),
    };

    let client = mock_client();
    let sdk = offline_sdk(&client, version);
    let wallet = Wallet::new();
    let built = builders::build_contact_request(
        &sdk,
        version,
        &ffi_identity(&sender),
        &ffi_identity(&recipient),
        9,
        &input,
        &high_key(),
        &wallet,
    )
    .expect("contact request");
    let (state_transition, id, data) = document_transition(&built.bytes);
    assert_eq!(built.object_id, id.to_buffer());
    assert_eq!(state_transition.signature_public_key_id(), Some(1));

    // The document is what the SDK mints: the encryption uses a random IV
    // per call, so compare structurally and by decrypting with the same
    // secret, then compare the transition around it byte for byte by
    // rebuilding through the SDK's own path with the minted properties.
    assert_eq!(
        data["toUserId"],
        Value::Identifier(recipient.id().to_buffer())
    );
    assert_eq!(data["senderKeyIndex"], Value::U32(2));
    assert_eq!(data["recipientKeyIndex"], Value::U32(3));
    assert_eq!(data["accountReference"], Value::U32(0x1000_0005));
    let encrypted_key = data["encryptedPublicKey"].to_bytes().unwrap();
    assert_eq!(encrypted_key.len(), 96);
    assert_eq!(
        decrypt_extended_public_key(&shared_secret, &encrypted_key).unwrap(),
        compact_xpub_bytes(xpub.parent_fingerprint, xpub.chain_code, xpub.public_key)
    );
    let label = data["encryptedAccountLabel"].to_bytes().unwrap();
    assert!((48..=80).contains(&label.len()));
    assert_eq!(
        platform_encryption::decrypt_account_label(&shared_secret, &label).unwrap(),
        "savings"
    );

    // Same properties and entropy through `send_contact_request`'s
    // assembly (the document put path) must give the same bytes.
    let StateTransition::Batch(batch) = &state_transition else {
        unreachable!()
    };
    let Some(BatchedTransitionRef::Document(transition)) = batch.first_transition() else {
        unreachable!()
    };
    let entropy: [u8; 32] = transition.entropy().unwrap().try_into().unwrap();
    let reference_document = Document::V0(DocumentV0 {
        owner_id: sender.id(),
        properties: data.clone(),
        ..Default::default()
    });
    let reference = reference_create(
        version,
        SystemDataContract::Dashpay,
        "contactRequest",
        reference_document,
        entropy,
        9,
    );
    assert_eq!(hex::encode(&built.bytes), hex::encode(&reference));
    assert_drive_accepts_structure(&built.bytes, version);

    // And `create_contact_request` itself, driven with the same inputs
    // through the SDK, yields the same property set (modulo the random IVs).
    let sdk_input = SdkContactRequestInput {
        sender_identity: sender.clone(),
        recipient: RecipientIdentity::Identity(recipient.clone()),
        sender_key_index: 2,
        recipient_key_index: 3,
        account_reference: 0x1000_0005,
        account_label: Some("savings".to_string()),
        auto_accept_proof: None,
    };
    type UnusedSdkSide =
        fn(&IdentityPublicKey, u32) -> std::future::Ready<Result<SecretKey, dash_sdk::Error>>;
    let ecdh: EcdhProvider<UnusedSdkSide, _, _, _> = EcdhProvider::ClientSide {
        get_shared_secret: move |_: &PublicKey| async move { Ok(shared_secret) },
    };
    let xpub_bytes =
        compact_xpub_bytes(xpub.parent_fingerprint, xpub.chain_code, xpub.public_key).to_vec();
    let minted = futures::executor::block_on(sdk.create_contact_request(sdk_input, ecdh, |_| {
        let xpub_bytes = xpub_bytes.clone();
        async move { Ok(xpub_bytes) }
    }))
    .expect("sdk mint");
    let mut expected_keys: Vec<&String> = minted.properties.keys().collect();
    expected_keys.sort();
    let mut built_keys: Vec<&String> = data.keys().collect();
    built_keys.sort();
    assert_eq!(built_keys, expected_keys);

    // The wrong recipient key refuses before anything is signed.
    let wrong = ContactRequestInput {
        recipient_key_index: 2,
        ..input.clone()
    };
    let refused = Wallet::new();
    let error = builders::build_contact_request(
        &sdk,
        version,
        &ffi_identity(&sender),
        &ffi_identity(&recipient),
        9,
        &wrong,
        &high_key(),
        &refused,
    )
    .unwrap_err();
    assert!(error.contains("recipient key"), "{error}");
    assert!(refused.requests().is_empty());
}

// --- identity create -------------------------------------------------------

fn new_keys() -> Vec<NewIdentityKey> {
    vec![
        NewIdentityKey {
            id: 0,
            purpose: Purpose::AUTHENTICATION as u8,
            security_level: SecurityLevel::MASTER as u8,
            pubkey: pubkey(&MASTER_SK),
            bounds: ContractBounds::default(),
        },
        NewIdentityKey {
            id: 1,
            purpose: Purpose::AUTHENTICATION as u8,
            security_level: SecurityLevel::HIGH as u8,
            pubkey: pubkey(&HIGH_SK),
            bounds: ContractBounds::default(),
        },
        NewIdentityKey {
            id: 2,
            purpose: Purpose::ENCRYPTION as u8,
            security_level: SecurityLevel::MEDIUM as u8,
            pubkey: pubkey(&ENCRYPTION_SK),
            bounds: ContractBounds {
                kind: BoundsKind::SingleContractDocumentType,
                contract_id: SystemDataContract::Dashpay.id().to_buffer(),
                document_type: "contactRequest".to_string(),
            },
        },
    ]
}

fn instant_proof() -> AssetLockProof {
    let one_time = PrivateKey::from_byte_array(&ASSET_LOCK_SK, Network::Testnet).unwrap();
    instant_asset_lock_proof_fixture(Some(one_time), None)
}

fn proof_input(proof: &AssetLockProof) -> AssetLockProofInput {
    match proof {
        AssetLockProof::Instant(instant) => AssetLockProofInput {
            is_instant: true,
            transaction: serialize(&instant.transaction),
            instant_lock: serialize(&instant.instant_lock),
            output_index: instant.output_index(),
            core_chain_locked_height: 0,
            out_point: [0u8; 36],
        },
        AssetLockProof::Chain(chain) => AssetLockProofInput {
            is_instant: false,
            transaction: Vec::new(),
            instant_lock: Vec::new(),
            output_index: 0,
            core_chain_locked_height: chain.core_chain_locked_height,
            out_point: chain.out_point.into(),
        },
    }
}

/// A reference signer over all identity keys, for dpp's in-process path.
#[derive(Debug)]
struct AllKeysSigner;

#[async_trait::async_trait]
impl Signer<IdentityPublicKey> for AllKeysSigner {
    async fn sign(
        &self,
        key: &IdentityPublicKey,
        data: &[u8],
    ) -> Result<dash_sdk::dpp::platform_value::BinaryData, dash_sdk::dpp::ProtocolError> {
        let secret = Wallet::secret(key.id()).expect("known key");
        Ok(sign(data, &secret).unwrap().to_vec().into())
    }
    async fn sign_create_witness(
        &self,
        key: &IdentityPublicKey,
        data: &[u8],
    ) -> Result<dash_sdk::dpp::address_funds::AddressWitness, dash_sdk::dpp::ProtocolError> {
        Ok(dash_sdk::dpp::address_funds::AddressWitness::P2pkh {
            signature: self.sign(key, data).await?,
        })
    }
    fn can_sign_with(&self, _key: &IdentityPublicKey) -> bool {
        true
    }
}

fn reference_identity_create(
    version: &'static PlatformVersion,
    proof: AssetLockProof,
) -> (Vec<u8>, Identifier) {
    let keys = new_keys()
        .into_iter()
        .map(|key| {
            let dpp_key: IdentityPublicKey = IdentityPublicKeyV0 {
                id: key.id,
                purpose: Purpose::try_from(key.purpose).unwrap(),
                security_level: SecurityLevel::try_from(key.security_level).unwrap(),
                contract_bounds: (key.bounds.kind == BoundsKind::SingleContractDocumentType).then(
                    || DppBounds::SingleContractDocumentType {
                        id: Identifier::from(key.bounds.contract_id),
                        document_type_name: key.bounds.document_type.clone(),
                    },
                ),
                key_type: KeyType::ECDSA_SECP256K1,
                read_only: false,
                data: key.pubkey.to_vec().into(),
                disabled_at: None,
            }
            .into();
            (key.id, dpp_key)
        })
        .collect();
    let identity_id = proof.create_identifier().unwrap();
    let identity: Identity = IdentityV0 {
        id: identity_id,
        public_keys: keys,
        balance: 0,
        revision: 0,
    }
    .into();
    // `put_to_platform_with_private_key`'s construction.
    let transition = futures::executor::block_on(
        IdentityCreateTransition::try_from_identity_with_signer_and_private_key(
            &identity,
            proof,
            &ASSET_LOCK_SK,
            &AllKeysSigner,
            &NativeBlsModule,
            0,
            version,
        ),
    )
    .expect("reference identity create");
    (transition.serialize_to_bytes().unwrap(), identity_id)
}

#[test_matrix([DEPLOYED_VERSION, LATEST_VERSION])]
fn identity_create_matches_the_private_key_path_byte_for_byte(protocol_version: ProtocolVersion) {
    let version = version(protocol_version);
    for proof in [
        instant_proof(),
        AssetLockProof::Chain(ChainAssetLockProof {
            core_chain_locked_height: 2_000_000,
            out_point: OutPoint::new(Txid::from_byte_array([0xabu8; 32]), 1),
        }),
    ] {
        let (reference, identity_id) = reference_identity_create(version, proof.clone());
        let wallet = Wallet::new();
        let built =
            builders::build_identity_create(version, &proof_input(&proof), &new_keys(), &wallet)
                .expect("identity create");
        assert_eq!(hex::encode(&built.bytes), hex::encode(&reference));
        assert_eq!(built.object_id, identity_id.to_buffer());
        assert_eq!(built.hash, hash_single(&built.bytes));
        // Every key signed the same preimage; the asset lock key signed its
        // double SHA256, once.
        let state_transition =
            StateTransition::deserialize_from_bytes_untrusted(&built.bytes).unwrap();
        let signable = state_transition.signable_bytes().unwrap();
        let requests = wallet.requests();
        assert_eq!(requests.len(), 4, "{requests:?}");
        for key_id in 0..3 {
            assert!(requests.contains(&Request::Key(key_id, signable.clone())));
        }
        assert!(requests.contains(&Request::AssetLock(hash_double(&signable))));
    }
}

#[test_matrix([DEPLOYED_VERSION, LATEST_VERSION])]
fn identity_create_refuses_bad_input_before_signing(protocol_version: ProtocolVersion) {
    let version = version(protocol_version);
    let wallet = Wallet::new();
    let proof = proof_input(&instant_proof());
    assert!(builders::build_identity_create(version, &proof, &[], &wallet).is_err());
    let mut duplicated = new_keys();
    duplicated[1].id = 0;
    assert!(builders::build_identity_create(version, &proof, &duplicated, &wallet).is_err());
    let garbage = AssetLockProofInput {
        transaction: vec![0u8; 8],
        ..proof.clone()
    };
    assert!(builders::build_identity_create(version, &garbage, &new_keys(), &wallet).is_err());
    assert!(wallet.requests().is_empty(), "nothing was signed");
}

// --- signer adapters -------------------------------------------------------

#[test_matrix([DEPLOYED_VERSION, LATEST_VERSION])]
fn signer_refusals_and_bad_signatures_are_reported(protocol_version: ProtocolVersion) {
    let version = version(protocol_version);
    let wallet = Wallet::refusing();
    let error = builders::build_dpns_preorder(
        version,
        owner().to_buffer(),
        1,
        "alice",
        &[1u8; 32],
        [2u8; 32],
        &high_key(),
        &wallet,
    )
    .unwrap_err();
    assert!(error.contains("refused"), "{error}");
    let error = builders::build_identity_create(
        version,
        &proof_input(&instant_proof()),
        &new_keys(),
        &wallet,
    )
    .unwrap_err();
    assert!(error.contains("refused"), "{error}");

    // A key the wallet cannot produce ECDSA signatures for is refused
    // before the callback.
    let wallet = Wallet::new();
    let mut bls_key = high_key();
    bls_key.key_type = KeyType::BLS12_381 as u8;
    bls_key.data = vec![0u8; 48];
    let error = builders::build_dpns_preorder(
        version,
        owner().to_buffer(),
        1,
        "alice",
        &[1u8; 32],
        [2u8; 32],
        &bls_key,
        &wallet,
    )
    .unwrap_err();
    assert!(error.contains("ECDSA"), "{error}");
    assert!(wallet.requests().is_empty());
}

#[test]
fn asset_lock_signer_recovers_the_public_key() {
    let wallet = Wallet::new();
    let sighash = [0x42u8; 32];
    let (signature, recovered) = futures::executor::block_on(
        AssetLockSigner(&wallet).sign_ecdsa(&DerivationPath::master(), sighash),
    )
    .expect("asset lock signature");
    let secp = Secp256k1::new();
    let expected =
        PublicKey::from_secret_key(&secp, &SecretKey::from_byte_array(&ASSET_LOCK_SK).unwrap());
    assert_eq!(recovered, expected);
    // r‖s equals what the compact signature carries.
    let compact = sign_hash(&sighash, &ASSET_LOCK_SK).unwrap();
    assert_eq!(
        signature.serialize_compact().to_vec(),
        compact[1..].to_vec()
    );
    assert!(futures::executor::block_on(
        AssetLockSigner(&wallet).public_key(&DerivationPath::master())
    )
    .is_err());
    assert!(BytesSigner(&wallet).can_sign_with(&dpp_high_key()));

    // Only a compressed-key recovery header (31..=34) is accepted: the
    // asset lock's outpoint key is compressed.
    let uncompressed = Wallet::with_signature_header(27);
    let error = futures::executor::block_on(
        AssetLockSigner(&uncompressed).sign_ecdsa(&DerivationPath::master(), sighash),
    )
    .unwrap_err();
    assert!(error.contains("recovery header"), "{error}");
    let garbage = Wallet::with_signature_header(0xff);
    assert!(futures::executor::block_on(
        AssetLockSigner(&garbage).sign_ecdsa(&DerivationPath::master(), sighash)
    )
    .is_err());
    // Every compressed header parses; only the right recovery id recovers
    // the key, which is what `sign_with_core_signer` then checks.
    let genuine = sign_hash(&sighash, &ASSET_LOCK_SK).unwrap()[0];
    for header in 31..=34u8 {
        let wallet = Wallet::with_signature_header(header);
        let recovered = futures::executor::block_on(
            AssetLockSigner(&wallet).sign_ecdsa(&DerivationPath::master(), sighash),
        );
        match recovered {
            Ok((_, key)) => assert_eq!(key == expected, header == genuine, "header {header}"),
            Err(error) => assert!(
                header != genuine && error.contains("recover"),
                "header {header}: {error}"
            ),
        }
    }
}

// --- threading -------------------------------------------------------------

#[test_matrix([DEPLOYED_VERSION, LATEST_VERSION])]
fn builders_never_enter_a_tokio_runtime(protocol_version: ProtocolVersion) {
    let version = version(protocol_version);
    let wallet = Wallet::new();
    assert!(
        tokio::runtime::Handle::try_current().is_err(),
        "the test runs without a runtime"
    );
    let client = mock_client();
    let sdk = offline_sdk(&client, version);
    let sender = identity_with_keys(
        owner(),
        &[
            (
                1,
                Purpose::AUTHENTICATION,
                SecurityLevel::HIGH,
                pubkey(&HIGH_SK),
            ),
            (
                2,
                Purpose::ENCRYPTION,
                SecurityLevel::MEDIUM,
                pubkey(&ENCRYPTION_SK),
            ),
        ],
    );
    let recipient_pk = PublicKey::from_secret_key(
        &Secp256k1::new(),
        &SecretKey::from_byte_array(&[0x88u8; 32]).unwrap(),
    );
    let recipient = identity_with_keys(
        Identifier::from([0x99u8; 32]),
        &[(
            3,
            Purpose::DECRYPTION,
            SecurityLevel::MEDIUM,
            recipient_pk.serialize(),
        )],
    );
    let contact = ContactRequestInput {
        to_user_id: recipient.id().to_buffer(),
        sender_key_index: 2,
        recipient_key_index: 3,
        recipient_pubkey: recipient_pk.serialize(),
        account_reference: 0,
        compact_xpub: CompactXpub {
            parent_fingerprint: [0; 4],
            chain_code: [1; 32],
            public_key: pubkey(&[0x5bu8; 32]),
        },
        shared_secret: [7u8; 32],
        account_label: String::new(),
    };
    builders::build_dpns_preorder(
        version,
        owner().to_buffer(),
        1,
        "alice",
        &[1u8; 32],
        [2u8; 32],
        &high_key(),
        &wallet,
    )
    .unwrap();
    builders::build_dpns_domain(
        version,
        owner().to_buffer(),
        2,
        "alice",
        &[1u8; 32],
        [2u8; 32],
        &high_key(),
        &wallet,
    )
    .unwrap();
    builders::build_profile(
        version,
        owner().to_buffer(),
        3,
        &ffi::Profile::default(),
        &ProfileInput {
            display_name: "a".into(),
            ..Default::default()
        },
        [2u8; 32],
        &high_key(),
        &wallet,
    )
    .unwrap();
    builders::build_contact_request(
        &sdk,
        version,
        &ffi_identity(&sender),
        &ffi_identity(&recipient),
        4,
        &contact,
        &high_key(),
        &wallet,
    )
    .unwrap();
    builders::build_identity_create(
        version,
        &proof_input(&instant_proof()),
        &new_keys(),
        &wallet,
    )
    .unwrap();
    assert!(
        !*wallet.tokio_runtime_entered.lock().unwrap(),
        "a builder called the signer from inside a tokio runtime"
    );
    assert_eq!(wallet.requests().len(), 4 + 4);
}

// --- first-byte vector -----------------------------------------------------

/// Writes `test_data/state_transition_first_byte.json`: the first byte of
/// a serialized `StateTransition` is bincode's variant index of the outer
/// enum, which Core's `WalletSigner` checks against the operation kind
/// before it signs. Regenerated from real transitions, never hand-edited.
#[test_matrix([DEPLOYED_VERSION, LATEST_VERSION])]
fn first_byte_vectors(protocol_version: ProtocolVersion) {
    let version = version(protocol_version);
    let wallet = Wallet::new();
    let batch = builders::build_dpns_preorder(
        version,
        owner().to_buffer(),
        1,
        "alice",
        &[1u8; 32],
        [2u8; 32],
        &high_key(),
        &wallet,
    )
    .unwrap();
    let identity_create = builders::build_identity_create(
        version,
        &proof_input(&instant_proof()),
        &new_keys(),
        &wallet,
    )
    .unwrap();
    // Pinned from the latest version's transitions; the variant indexes are
    // the same at every version the suite runs.
    let vectors = serde_json::json!({
        "description": "First byte of a serialized StateTransition: bincode's variant index of the StateTransition enum (declaration order in rs-dpp state_transition/mod.rs), not StateTransitionType. Core's WalletSigner asserts it matches the SigningOperation kind before signing.",
        "protocol_version": version.protocol_version,
        "batch": { "first_byte": batch.bytes[0], "example_hex": hex::encode(&batch.bytes[..8]) },
        "identity_create": { "first_byte": identity_create.bytes[0], "example_hex": hex::encode(&identity_create.bytes[..8]) },
    });
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("test_data/state_transition_first_byte.json");
    if protocol_version == LATEST_VERSION {
        let rendered = serde_json::to_string_pretty(&vectors).unwrap() + "\n";
        if std::env::var_os("UPDATE_TEST_VECTORS").is_some() {
            std::fs::write(&path, &rendered).unwrap();
        }
        assert_eq!(
            std::fs::read_to_string(&path).unwrap_or_default(),
            rendered,
            "the checked-in vector file is stale: rerun with UPDATE_TEST_VECTORS=1"
        );
    }
    let pinned: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(pinned["batch"]["first_byte"], batch.bytes[0]);
    assert_eq!(
        pinned["identity_create"]["first_byte"],
        identity_create.bytes[0]
    );
    // Pinned here as well, so a regeneration that changes them fails loudly.
    assert_eq!(
        batch.bytes[0], 2,
        "Batch is the third StateTransition variant"
    );
    assert_eq!(
        identity_create.bytes[0], 3,
        "IdentityCreate is the fourth StateTransition variant"
    );
}
