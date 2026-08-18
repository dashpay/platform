// Copyright (c) 2026 The Dash Core developers
// Distributed under the MIT software license, see the accompanying
// file COPYING or http://www.opensource.org/licenses/mit-license.php.

//! DPP state-transition construction and signing for C++ embedders, built on
//! the real rs-dpp builders.
//!
//! Signing is delegated through a callback so private keys stay in the
//! wallet: the callback receives the key id being signed plus the
//! double-SHA256 digest of the transition's signable bytes and must return
//! a 65-byte compact recoverable ECDSA signature - exactly what
//! `dpp::dashcore::signer::sign` would produce from the raw key
//! (`sign(data, key) == sign_hash(sha256d(data), key)`).

use std::collections::BTreeMap;

use dash_platform_queries::dashpay::{
    build_contact_request_document, ContactRequestDocumentParams,
};
use dash_platform_queries::dpns_usernames::build_dpns_preorder_and_domain_documents;
use dash_platform_queries::transition::put_document::{
    ensure_entropy_matches_document_id, prepare_document_for_transition,
};
use dpp::address_funds::AddressWitness;
use dpp::dashcore::consensus::Decodable;
use dpp::dashcore::hashes::{sha256, Hash};
use dpp::dashcore::{InstantLock, OutPoint, Transaction};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::DataContract;
use dpp::document::{Document, DocumentV0};
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::signer::Signer;
use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
use dpp::identity::state_transition::asset_lock_proof::InstantAssetLockProof;
use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::{BinaryData, Value};
use dpp::prelude::{AssetLockProof, Identifier};
use dpp::serialization::{PlatformSerializable, Signable};
use dpp::state_transition::batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
use dpp::state_transition::batch_transition::BatchTransition;
use dpp::state_transition::identity_create_transition::v0::IdentityCreateTransitionV0;
use dpp::state_transition::public_key_in_creation::accessors::{
    IdentityPublicKeyInCreationV0Getters, IdentityPublicKeyInCreationV0Setters,
};
use dpp::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0;
use dpp::state_transition::StateTransition;
use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
use dpp::util::hash::hash_double;
use dpp::util::strings::convert_to_homograph_safe_chars;
use dpp::ProtocolError;
use platform_version::version::PlatformVersion;

use crate::types::{id32, BuiltTransition, KeyInfo};

/// Signs a 32-byte digest with the wallet key identified by `key_id`,
/// returning a 65-byte compact recoverable ECDSA signature, or `None` on
/// failure (locked wallet, unknown key). `ASSET_LOCK_KEY_ID` selects the
/// one-time asset-lock key of an identity registration.
pub type SignFn<'a> = &'a (dyn Fn(u32, [u8; 32]) -> Option<Vec<u8>> + Sync);

/// Pseudo key id routed to the asset-lock one-time key.
pub const ASSET_LOCK_KEY_ID: u32 = u32::MAX;

const COMPACT_SIG_SIZE: usize = 65;
const COMPRESSED_PUBKEY_SIZE: usize = 33;

/// dpp async `Signer` backed by the digest callback. The async surface is
/// signature-only: the callback is invoked synchronously and the builders
/// are driven by `futures::executor::block_on` on the calling thread.
struct CallbackSigner<'a> {
    sign_fn: SignFn<'a>,
}

impl std::fmt::Debug for CallbackSigner<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CallbackSigner")
    }
}

fn sign_digest(sign_fn: SignFn<'_>, key_id: u32, digest: [u8; 32]) -> Result<Vec<u8>, String> {
    let signature = sign_fn(key_id, digest)
        .ok_or("signing failed (wallet locked or key unavailable)".to_string())?;
    if signature.len() != COMPACT_SIG_SIZE {
        return Err(format!(
            "unexpected signature size {} (want {COMPACT_SIG_SIZE})",
            signature.len()
        ));
    }
    Ok(signature)
}

#[async_trait::async_trait]
impl Signer<IdentityPublicKey> for CallbackSigner<'_> {
    async fn sign(
        &self,
        key: &IdentityPublicKey,
        data: &[u8],
    ) -> Result<BinaryData, ProtocolError> {
        sign_digest(self.sign_fn, key.id(), hash_double(data))
            .map(Into::into)
            .map_err(ProtocolError::Generic)
    }

    async fn sign_create_witness(
        &self,
        key: &IdentityPublicKey,
        data: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        let signature = self.sign(key, data).await?;
        Ok(AddressWitness::P2pkh { signature })
    }

    fn can_sign_with(&self, _key: &IdentityPublicKey) -> bool {
        true
    }
}

/// Reconstructs a dpp IdentityPublicKey from the flattened FFI form.
pub fn identity_key_from_info(info: &KeyInfo) -> Result<IdentityPublicKey, String> {
    Ok(IdentityPublicKey::V0(IdentityPublicKeyV0 {
        id: info.id,
        purpose: Purpose::try_from(info.purpose)
            .map_err(|e| format!("bad key purpose {}: {e}", info.purpose))?,
        security_level: SecurityLevel::try_from(info.security_level)
            .map_err(|e| format!("bad key security level {}: {e}", info.security_level))?,
        contract_bounds: None,
        key_type: KeyType::try_from(info.key_type)
            .map_err(|e| format!("bad key type {}: {e}", info.key_type))?,
        read_only: info.read_only,
        data: BinaryData::new(info.data.clone()),
        disabled_at: info.disabled_at,
    }))
}

fn load_contract(contract: SystemDataContract) -> Result<DataContract, String> {
    load_system_data_contract(contract, PlatformVersion::latest())
        .map_err(|e| format!("unable to load system data contract: {e}"))
}

fn built(state_transition: &StateTransition) -> Result<BuiltTransition, String> {
    let bytes = state_transition
        .serialize_to_bytes()
        .map_err(|e| format!("unable to serialize state transition: {e}"))?;
    let hash = sha256::Hash::hash(&bytes).to_byte_array();
    Ok(BuiltTransition { bytes, hash })
}

/// Builds and signs a single-document create batch transition from an
/// assembled document. The upstream put-document guards run first: the
/// entropy must rederive the document id (Drive recomputes it during
/// validation, so a mismatch is caught locally instead of after paying a
/// nonce bump), and the properties are sanitized for the document type.
fn build_document_create_transition(
    contract: &DataContract,
    document_type_name: &str,
    document: Document,
    identity_contract_nonce: u64,
    entropy: [u8; 32],
    key: &KeyInfo,
    sign_fn: SignFn<'_>,
) -> Result<BuiltTransition, String> {
    use dpp::document::DocumentV0Getters;

    let version = PlatformVersion::latest();
    let document_type = contract
        .document_type_for_name(document_type_name)
        .map_err(|e| format!("unknown document type {document_type_name}: {e}"))?;
    ensure_entropy_matches_document_id(
        &contract.id(),
        &document.owner_id(),
        document_type_name,
        &entropy,
        document.id(),
    )
    .map_err(|e| format!("{document_type_name}: {e}"))?;
    let document_type_owned = contract
        .document_type_cloned_for_name(document_type_name)
        .map_err(|e| format!("unknown document type {document_type_name}: {e}"))?;
    let document = prepare_document_for_transition(&document, &document_type_owned);
    let identity_key = identity_key_from_info(key)?;
    let signer = CallbackSigner { sign_fn };
    let state_transition = futures::executor::block_on(
        BatchTransition::new_document_creation_transition_from_document(
            document,
            document_type,
            entropy,
            &identity_key,
            identity_contract_nonce,
            0,
            None,
            &signer,
            version,
            None,
        ),
    )
    .map_err(|e| format!("unable to build {document_type_name} create transition: {e}"))?;
    built(&state_transition)
}

/// Builds and signs a single-document create batch transition from a
/// property map (documents without an upstream pure builder).
#[allow(clippy::too_many_arguments)]
fn build_document_create(
    contract: SystemDataContract,
    document_type_name: &str,
    owner_id: [u8; 32],
    identity_contract_nonce: u64,
    properties: BTreeMap<String, Value>,
    entropy: [u8; 32],
    key: &KeyInfo,
    sign_fn: SignFn<'_>,
) -> Result<BuiltTransition, String> {
    let contract = load_contract(contract)?;
    let owner_id = Identifier::from(owner_id);
    let document_id =
        Document::generate_document_id_v0(&contract.id(), &owner_id, document_type_name, &entropy);
    let document = Document::V0(DocumentV0 {
        id: document_id,
        owner_id,
        properties,
        ..Default::default()
    });
    build_document_create_transition(
        &contract,
        document_type_name,
        document,
        identity_contract_nonce,
        entropy,
        key,
        sign_fn,
    )
}

/// The salted domain hash the DPNS preorder blinds the name behind:
/// sha256d(salt || "<normalized label>.dash"). Matches the preimage
/// `build_dpns_preorder_and_domain_documents` hashes into the preorder's
/// `saltedDomainHash` property.
fn salted_domain_hash(salt: [u8; 32], label: &str) -> [u8; 32] {
    let normalized = convert_to_homograph_safe_chars(label);
    let mut preimage = salt.to_vec();
    preimage.extend((normalized + ".dash").as_bytes());
    hash_double(&preimage)
}

/// DPNS preorder create. The documents come from the upstream pure builder
/// (dash-platform-queries); the salted domain hash doubles as the document
/// entropy — it is already blinded and unique per (name, salt), so rebuilds
/// of the same registration stay byte-identical.
pub fn build_dpns_preorder(
    identity_id: &[u8],
    identity_contract_nonce: u64,
    label: &str,
    preorder_salt: &[u8],
    key: &KeyInfo,
    sign_fn: SignFn<'_>,
) -> Result<BuiltTransition, String> {
    let owner = id32(identity_id, "identity id")?;
    let salt = id32(preorder_salt, "preorder salt")?;
    let entropy = salted_domain_hash(salt, label);
    let contract = load_contract(SystemDataContract::DPNS)?;
    let (preorder, _domain) = build_dpns_preorder_and_domain_documents(
        &contract,
        Identifier::from(owner),
        label,
        entropy,
        salt,
    )
    .map_err(|e| format!("unable to build DPNS preorder document: {e}"))?;
    build_document_create_transition(
        &contract,
        "preorder",
        preorder,
        identity_contract_nonce,
        entropy,
        key,
        sign_fn,
    )
}

/// DPNS domain create. The preorder salt is drawn fresh per registration
/// attempt, making it a suitable deterministic document entropy for the
/// paired domain create. The contested-name vote-resolution prefund is
/// computed by rs-dpp from the contested unique index of the DPNS domain
/// type during DocumentCreateTransition::from_document; no explicit
/// handling needed.
#[allow(clippy::too_many_arguments)]
pub fn build_dpns_domain(
    identity_id: &[u8],
    identity_contract_nonce: u64,
    label: &str,
    normalized_label: &str,
    parent_domain: &str,
    preorder_salt: &[u8],
    key: &KeyInfo,
    sign_fn: SignFn<'_>,
) -> Result<BuiltTransition, String> {
    if convert_to_homograph_safe_chars(label) != normalized_label {
        return Err("normalized label does not match label".to_string());
    }
    // The upstream builder registers under the "dash" TLD; that is the only
    // parent domain that exists.
    if parent_domain != "dash" {
        return Err(format!("unsupported parent domain {parent_domain:?}"));
    }
    let owner = id32(identity_id, "identity id")?;
    let salt = id32(preorder_salt, "preorder salt")?;
    let contract = load_contract(SystemDataContract::DPNS)?;
    let (_preorder, domain) = build_dpns_preorder_and_domain_documents(
        &contract,
        Identifier::from(owner),
        label,
        salt,
        salt,
    )
    .map_err(|e| format!("unable to build DPNS domain document: {e}"))?;
    build_document_create_transition(
        &contract,
        "domain",
        domain,
        identity_contract_nonce,
        salt,
        key,
        sign_fn,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn build_profile(
    identity_id: &[u8],
    identity_contract_nonce: u64,
    display_name: &str,
    public_message: &str,
    avatar_url: &str,
    avatar_hash: &[u8],
    avatar_fingerprint: &[u8],
    revision: u64,
    existing_document_id: Option<&[u8]>,
    entropy: &[u8],
    key: &KeyInfo,
    sign_fn: SignFn<'_>,
) -> Result<BuiltTransition, String> {
    let owner = id32(identity_id, "identity id")?;
    let entropy = id32(entropy, "entropy")?;
    if !avatar_hash.is_empty() && avatar_hash.len() != 32 {
        return Err("avatar hash must be 32 bytes".to_string());
    }
    if !avatar_fingerprint.is_empty() && avatar_fingerprint.len() != 8 {
        return Err("avatar fingerprint must be 8 bytes".to_string());
    }
    // All profile fields are optional in the DashPay contract; $createdAt /
    // $updatedAt are assigned by the chain and never appear in the
    // transition.
    let mut properties = BTreeMap::new();
    if !display_name.is_empty() {
        properties.insert(
            "displayName".to_string(),
            Value::Text(display_name.to_string()),
        );
    }
    if !public_message.is_empty() {
        properties.insert(
            "publicMessage".to_string(),
            Value::Text(public_message.to_string()),
        );
    }
    if !avatar_url.is_empty() {
        properties.insert("avatarUrl".to_string(), Value::Text(avatar_url.to_string()));
    }
    if !avatar_hash.is_empty() {
        properties.insert(
            "avatarHash".to_string(),
            Value::Bytes32(id32(avatar_hash, "avatar hash")?),
        );
    }
    if !avatar_fingerprint.is_empty() {
        properties.insert(
            "avatarFingerprint".to_string(),
            Value::Bytes(avatar_fingerprint.to_vec()),
        );
    }

    let Some(existing_document_id) = existing_document_id else {
        if revision != 1 {
            return Err("profile create requires revision 1".to_string());
        }
        return build_document_create(
            SystemDataContract::Dashpay,
            "profile",
            owner,
            identity_contract_nonce,
            properties,
            entropy,
            key,
            sign_fn,
        );
    };

    if revision < 2 {
        return Err("profile replace requires revision > 1".to_string());
    }
    let version = PlatformVersion::latest();
    let contract = load_contract(SystemDataContract::Dashpay)?;
    let document_type = contract
        .document_type_for_name("profile")
        .map_err(|e| format!("unknown document type profile: {e}"))?;
    let document = Document::V0(DocumentV0 {
        id: Identifier::from(id32(existing_document_id, "existing document id")?),
        owner_id: Identifier::from(owner),
        properties,
        revision: Some(revision),
        ..Default::default()
    });
    let identity_key = identity_key_from_info(key)?;
    let signer = CallbackSigner { sign_fn };
    let state_transition = futures::executor::block_on(
        BatchTransition::new_document_replacement_transition_from_document(
            document,
            document_type,
            &identity_key,
            identity_contract_nonce,
            0,
            None,
            &signer,
            version,
            None,
        ),
    )
    .map_err(|e| format!("unable to build profile replace transition: {e}"))?;
    built(&state_transition)
}

#[allow(clippy::too_many_arguments)]
pub fn build_contact_request(
    identity_id: &[u8],
    identity_contract_nonce: u64,
    to_user_id: &[u8],
    encrypted_public_key: &[u8],
    sender_key_index: u32,
    recipient_key_index: u32,
    account_reference: u32,
    encrypted_account_label: &[u8],
    entropy: &[u8],
    key: &KeyInfo,
    sign_fn: SignFn<'_>,
) -> Result<BuiltTransition, String> {
    let owner = id32(identity_id, "identity id")?;
    let to_user = id32(to_user_id, "to-user id")?;
    let entropy = id32(entropy, "entropy")?;
    let contract = load_contract(SystemDataContract::Dashpay)?;
    // The upstream builder validates the crypto-material sizes and assembles
    // the DIP-15 property map; $createdAt and $createdAtCoreBlockHeight are
    // chain-assigned system fields, so they do not enter the transition.
    let (document_id, properties) = build_contact_request_document(
        &contract,
        ContactRequestDocumentParams {
            sender_id: Identifier::from(owner),
            recipient_id: Identifier::from(to_user),
            sender_key_index,
            recipient_key_index,
            account_reference,
            encrypted_public_key: encrypted_public_key.to_vec(),
            encrypted_account_label: (!encrypted_account_label.is_empty())
                .then(|| encrypted_account_label.to_vec()),
            auto_accept_proof: None,
            entropy,
        },
    )
    .map_err(|e| format!("unable to build contact request document: {e}"))?;
    let document = Document::V0(DocumentV0 {
        id: document_id,
        owner_id: Identifier::from(owner),
        properties,
        ..Default::default()
    });
    build_document_create_transition(
        &contract,
        "contactRequest",
        document,
        identity_contract_nonce,
        entropy,
        key,
        sign_fn,
    )
}

/// Asset lock proof input for identity registration.
pub enum AssetLockProofInput {
    Instant {
        /// Serialized asset lock transaction.
        transaction: Vec<u8>,
        /// Serialized islock message.
        instant_lock: Vec<u8>,
        /// Index of the asset-lock OP_RETURN output in tx.vout.
        output_index: u32,
    },
    Chain {
        core_chain_locked_height: u32,
        /// txid || vout (LE u32), consensus encoding.
        out_point: [u8; 36],
    },
}

/// A public key to register with a new identity. The private key never
/// crosses; each key proves ownership through the signing callback.
pub struct NewIdentityKey {
    pub id: u32,
    pub purpose: u8,
    pub security_level: u8,
    /// Compressed secp256k1 public key (33 bytes).
    pub pubkey: Vec<u8>,
}

/// IdentityCreateTransition assembled manually so that both the identity
/// keys and the asset-lock one-time key stay behind the signing callback.
/// Mirrors rs-dpp `try_from_identity_with_signer_and_private_key`: all
/// per-key signatures and the outer asset-lock signature cover the same
/// signable bytes (key signatures, the outer signature and the identity id
/// are all excluded from the signable form).
pub fn build_identity_create(
    proof: AssetLockProofInput,
    keys: &[NewIdentityKey],
    sign_fn: SignFn<'_>,
) -> Result<BuiltTransition, String> {
    if keys.is_empty() {
        return Err("no identity keys provided".to_string());
    }

    let asset_lock_proof = match proof {
        AssetLockProofInput::Instant {
            transaction,
            instant_lock,
            output_index,
        } => {
            let transaction = Transaction::consensus_decode(&mut transaction.as_slice())
                .map_err(|e| format!("bad asset lock transaction: {e}"))?;
            let instant_lock = InstantLock::consensus_decode(&mut instant_lock.as_slice())
                .map_err(|e| format!("bad instant lock: {e}"))?;
            AssetLockProof::Instant(InstantAssetLockProof::new(
                instant_lock,
                transaction,
                output_index,
            ))
        }
        AssetLockProofInput::Chain {
            core_chain_locked_height,
            out_point,
        } => {
            let out_point = OutPoint::consensus_decode(&mut out_point.as_slice())
                .map_err(|e| format!("bad asset lock outpoint: {e}"))?;
            AssetLockProof::Chain(ChainAssetLockProof {
                core_chain_locked_height,
                out_point,
            })
        }
    };

    // rs-dpp registers Identity::public_keys() (a BTreeMap keyed by id), so
    // the keys serialize in ascending id order with no duplicates.
    let mut sorted: Vec<&NewIdentityKey> = keys.iter().collect();
    sorted.sort_by_key(|key| key.id);
    let mut public_keys = Vec::with_capacity(sorted.len());
    for (i, key) in sorted.iter().enumerate() {
        if i > 0 && key.id == sorted[i - 1].id {
            return Err(format!("duplicate identity key id {}", key.id));
        }
        if key.pubkey.len() != COMPRESSED_PUBKEY_SIZE {
            return Err(format!(
                "identity key {}: unexpected public key size {}",
                key.id,
                key.pubkey.len()
            ));
        }
        public_keys.push(
            IdentityPublicKeyInCreationV0 {
                id: key.id,
                key_type: KeyType::ECDSA_SECP256K1,
                purpose: Purpose::try_from(key.purpose)
                    .map_err(|e| format!("identity key {}: bad purpose: {e}", key.id))?,
                security_level: SecurityLevel::try_from(key.security_level)
                    .map_err(|e| format!("identity key {}: bad security level: {e}", key.id))?,
                contract_bounds: None,
                read_only: false,
                data: BinaryData::new(key.pubkey.clone()),
                signature: Default::default(),
            }
            .into(),
        );
    }

    let identity_id = asset_lock_proof
        .create_identifier()
        .map_err(|e| format!("unable to derive identity id: {e}"))?;
    let mut transition = IdentityCreateTransitionV0 {
        public_keys,
        asset_lock_proof,
        user_fee_increase: 0,
        identity_id,
        ..Default::default()
    };

    // Every registered key proves ownership by signing the same digest as
    // the asset-lock key: the double-SHA256 of the signable bytes. Key
    // signatures are excluded from the signable form, so setting them does
    // not change it.
    let state_transition: StateTransition = transition.clone().into();
    let signable = state_transition
        .signable_bytes()
        .map_err(|e| format!("unable to compute signable bytes: {e}"))?;
    let digest = hash_double(&signable);
    for key in transition.public_keys.iter_mut() {
        let signature = sign_digest(sign_fn, key.id(), digest)
            .map_err(|e| format!("identity key {}: {e}", key.id()))?;
        key.set_signature(BinaryData::new(signature));
    }
    let mut state_transition: StateTransition = transition.into();
    let signature = sign_digest(sign_fn, ASSET_LOCK_KEY_ID, digest)
        .map_err(|e| format!("asset lock key: {e}"))?;
    if !state_transition.set_signature(BinaryData::new(signature)) {
        return Err("unable to set asset lock signature".to_string());
    }
    built(&state_transition)
}
