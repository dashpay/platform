// Copyright (c) 2026 The Dash Core developers
// Distributed under the MIT software license, see the accompanying
// file COPYING or http://www.opensource.org/licenses/mit-license.php.

//! State-transition assembly on the embedder's thread: dpp builds and signs
//! the transitions, this module only turns bridge inputs into dpp inputs
//! and assembles the three DashPay / DPNS documents `dash-sdk` does not yet
//! expose as pure functions (its `register_dpns_name` draws the preorder
//! salt itself, which a crash-safe flow cannot use, and it has no profile
//! helper). Contact requests go through `Sdk::create_contact_request`, so
//! the mint-side key policy, the DIP-15 size checks and the encryption are
//! the SDK's.
//!
//! The dpp builders are async only in signature; they are driven by
//! `futures::executor::block_on`, so no runtime is entered and the signer
//! is called synchronously on the calling thread.

use std::collections::BTreeMap;

use dash_sdk::dpp::dashcore::consensus::Decodable;
use dash_sdk::dpp::dashcore::secp256k1::rand::{rngs::StdRng, Rng, SeedableRng};
use dash_sdk::dpp::dashcore::secp256k1::PublicKey;
use dash_sdk::dpp::dashcore::{InstantLock, OutPoint, Transaction};
use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;
use dash_sdk::dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dash_sdk::dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dash_sdk::dpp::data_contract::DataContract;
use dash_sdk::dpp::document::{Document, DocumentV0, DocumentV0Getters, INITIAL_REVISION};
use dash_sdk::dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dash_sdk::dpp::identity::identity_public_key::contract_bounds::ContractBounds;
use dash_sdk::dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dash_sdk::dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
use dash_sdk::dpp::identity::state_transition::asset_lock_proof::InstantAssetLockProof;
use dash_sdk::dpp::identity::v0::IdentityV0;
use dash_sdk::dpp::identity::{Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dash_sdk::dpp::key_wallet::bip32::DerivationPath;
use dash_sdk::dpp::native_bls::NativeBlsModule;
use dash_sdk::dpp::platform_value::Value;
use dash_sdk::dpp::prelude::{AssetLockProof, Identifier};
use dash_sdk::dpp::serialization::PlatformSerializable;
use dash_sdk::dpp::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use dash_sdk::dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransitionV0Methods;
use dash_sdk::dpp::state_transition::batch_transition::batched_transition::BatchedTransitionRef;
use dash_sdk::dpp::state_transition::batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
use dash_sdk::dpp::state_transition::batch_transition::BatchTransition;
use dash_sdk::dpp::state_transition::identity_create_transition::methods::IdentityCreateTransitionMethodsV0;
use dash_sdk::dpp::state_transition::identity_create_transition::IdentityCreateTransition;
use dash_sdk::dpp::state_transition::StateTransition;
use dash_sdk::dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
use dash_sdk::dpp::util::hash::{hash_double, hash_single};
use dash_sdk::dpp::util::strings::convert_to_homograph_safe_chars;
use dash_sdk::dpp::version::PlatformVersion;
use dash_sdk::platform::dashpay::{ContactRequestInput, EcdhProvider, RecipientIdentity};
use dash_sdk::Sdk;
use futures::executor::block_on;
use zeroize::Zeroizing;

use crate::ffi;
use crate::helpers::compact_xpub_to_bytes;
use crate::signer::{AssetLockSigner, BytesSigner, SignerCallbacks};

/// Both are consensus-bounded Core messages; anything larger than a block
/// is not one.
const MAX_CORE_MESSAGE_BYTES: usize = 2 * 1024 * 1024;

/// Fresh document entropy. Drawn per build and never persisted: a rebuilt
/// transition gets a new id (from protocol version 14 the id also commits
/// to the identity contract nonce), so the unique indexes, not byte
/// identity, protect against duplicates.
pub fn fresh_entropy() -> [u8; 32] {
    StdRng::from_entropy().gen()
}

fn contract_bounds(bounds: &ffi::ContractBounds) -> Result<Option<ContractBounds>, String> {
    let id = Identifier::from(bounds.contract_id);
    Ok(match bounds.kind {
        ffi::BoundsKind::NoBounds => None,
        ffi::BoundsKind::SingleContract => Some(ContractBounds::SingleContract { id }),
        ffi::BoundsKind::SingleContractDocumentType => {
            Some(ContractBounds::SingleContractDocumentType {
                id,
                document_type_name: bounds.document_type.clone(),
            })
        }
        ffi::BoundsKind::ContractGroup => Some(ContractBounds::ContractGroup { id }),
        other => return Err(format!("unknown contract bounds kind {}", other.repr)),
    })
}

/// A dpp key from the bridge form the embedder read back from
/// `get_identity`.
pub fn identity_key(key: &ffi::IdentityKey) -> Result<IdentityPublicKey, String> {
    Ok(IdentityPublicKeyV0 {
        id: key.id,
        purpose: Purpose::try_from(key.purpose)
            .map_err(|e| format!("key {}: bad purpose: {e}", key.id))?,
        security_level: SecurityLevel::try_from(key.security_level)
            .map_err(|e| format!("key {}: bad security level: {e}", key.id))?,
        contract_bounds: contract_bounds(&key.bounds)?,
        key_type: KeyType::try_from(key.key_type)
            .map_err(|e| format!("key {}: bad key type: {e}", key.id))?,
        read_only: key.read_only,
        data: key.data.clone().into(),
        disabled_at: (key.disabled_at != 0).then_some(key.disabled_at),
    }
    .into())
}

/// A dpp identity from the bridge form; only the id and keys matter to the
/// builders.
pub fn identity(identity: &ffi::Identity) -> Result<Identity, String> {
    let public_keys = identity
        .keys
        .iter()
        .map(|key| identity_key(key).map(|key| (key.id(), key)))
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    Ok(IdentityV0 {
        id: Identifier::from(identity.id),
        public_keys,
        balance: identity.balance,
        revision: identity.revision,
    }
    .into())
}

fn system_contract(
    contract: SystemDataContract,
    version: &PlatformVersion,
) -> Result<DataContract, String> {
    load_system_data_contract(contract, version)
        .map_err(|e| format!("unable to load the {contract:?} contract: {e}"))
}

/// Serializes a signed transition. `object_id` names the identity or
/// document it creates or replaces.
fn built(state_transition: &StateTransition, object_id: Identifier) -> Result<ffi::Built, String> {
    let bytes = state_transition
        .serialize_to_bytes()
        .map_err(|e| format!("unable to serialize the state transition: {e}"))?;
    Ok(ffi::Built {
        hash: hash_single(&bytes),
        bytes,
        object_id: object_id.to_buffer(),
    })
}

/// A document with the given properties and no system fields: its id is
/// set from the entropy and nonce when the create transition is built (see
/// [`build_system_document`]), and Drive fills the timestamps.
fn new_document(owner: Identifier, properties: BTreeMap<String, Value>) -> Document {
    Document::V0(DocumentV0 {
        owner_id: owner,
        properties,
        ..Default::default()
    })
}

/// The one document transition of a batch built here.
fn document_id(state_transition: &StateTransition) -> Result<Identifier, String> {
    let StateTransition::Batch(batch) = state_transition else {
        return Err("expected a batch transition".to_string());
    };
    match batch.first_transition() {
        Some(BatchedTransitionRef::Document(transition)) => Ok(transition.get_id()),
        _ => Err("the batch carries no document transition".to_string()),
    }
}

enum Kind {
    Create { entropy: [u8; 32] },
    Replace,
}

/// Builds and signs a single-document batch transition over a compiled-in
/// system contract's document type. The properties are sanitized for the
/// document type first, and a create gets the id derived from its entropy
/// and nonce at `version`, as `dash-sdk`'s put-document path does: up to
/// protocol version 13 dpp sends the document's id as it is (Drive
/// recomputes it from the entropy alone and refuses a mismatch), from 14 it
/// derives the id itself and this only anticipates it.
#[allow(clippy::too_many_arguments)]
fn build_system_document(
    version: &PlatformVersion,
    contract: SystemDataContract,
    type_name: &str,
    mut document: Document,
    nonce: u64,
    kind: Kind,
    key: &ffi::IdentityKey,
    signer: &dyn SignerCallbacks,
) -> Result<ffi::Built, String> {
    let contract = system_contract(contract, version)?;
    let document_type = contract
        .document_type_for_name(type_name)
        .map_err(|e| e.to_string())?;
    // The contract at `version` may predate a property (DashPay's payment
    // addresses arrive with protocol version 14); Drive would refuse the
    // document after charging for it.
    if let Some(unknown) = document
        .properties()
        .keys()
        .find(|name| !document_type.properties().contains_key(*name))
    {
        return Err(format!(
            "the {type_name} document type has no property {unknown} at protocol version {}",
            version.protocol_version
        ));
    }
    document_type.sanitize_document_properties(document.properties_mut());
    let identity_key = identity_key(key)?;
    let signer = BytesSigner(signer);
    let state_transition = match kind {
        Kind::Create { entropy } => {
            document
                .set_id_for_creation(document_type, &entropy, nonce, version)
                .map_err(|e| format!("unable to derive the document id: {e}"))?;
            block_on(
                BatchTransition::new_document_creation_transition_from_document(
                    document,
                    document_type,
                    entropy,
                    &identity_key,
                    nonce,
                    0,
                    None,
                    &signer,
                    version,
                    None,
                ),
            )
        }
        Kind::Replace => block_on(
            BatchTransition::new_document_replacement_transition_from_document(
                document,
                document_type,
                &identity_key,
                nonce,
                0,
                None,
                &signer,
                version,
                None,
            ),
        ),
    }
    .map_err(|e| {
        format!(
            "unable to build the {} transition: {e}",
            document_type.name()
        )
    })?;
    let id = document_id(&state_transition)?;
    built(&state_transition, id)
}

/// The DPNS preorder commitment: double SHA256 of `salt ‖ normalized label
/// ‖ ".dash"`, as `register_dpns_name` computes it.
pub fn salted_domain_hash(normalized_label: &str, salt: &[u8; 32]) -> [u8; 32] {
    let mut buffer = salt.to_vec();
    buffer.extend_from_slice(normalized_label.as_bytes());
    buffer.extend_from_slice(b".dash");
    hash_double(buffer)
}

/// The DPNS `preorder` document `register_dpns_name` would build for
/// `label` and `salt`.
pub fn dpns_preorder_document(owner: Identifier, label: &str, salt: &[u8; 32]) -> Document {
    let normalized_label = convert_to_homograph_safe_chars(label);
    new_document(
        owner,
        BTreeMap::from([(
            "saltedDomainHash".to_string(),
            Value::Bytes32(salted_domain_hash(&normalized_label, salt)),
        )]),
    )
}

/// The DPNS `domain` document `register_dpns_name` would build for `label`
/// and `salt`, under the "dash" parent, pointing at `owner`.
pub fn dpns_domain_document(owner: Identifier, label: &str, salt: &[u8; 32]) -> Document {
    new_document(
        owner,
        BTreeMap::from([
            (
                "parentDomainName".to_string(),
                Value::Text("dash".to_string()),
            ),
            (
                "normalizedParentDomainName".to_string(),
                Value::Text("dash".to_string()),
            ),
            ("label".to_string(), Value::Text(label.to_string())),
            (
                "normalizedLabel".to_string(),
                Value::Text(convert_to_homograph_safe_chars(label)),
            ),
            ("preorderSalt".to_string(), Value::Bytes32(*salt)),
            (
                "records".to_string(),
                Value::Map(vec![(
                    Value::Text("identity".to_string()),
                    Value::Identifier(owner.to_buffer()),
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
    )
}

#[allow(clippy::too_many_arguments)]
pub fn build_dpns_preorder(
    version: &PlatformVersion,
    owner: [u8; 32],
    nonce: u64,
    label: &str,
    salt: &[u8; 32],
    entropy: [u8; 32],
    key: &ffi::IdentityKey,
    signer: &dyn SignerCallbacks,
) -> Result<ffi::Built, String> {
    build_system_document(
        version,
        SystemDataContract::DPNS,
        "preorder",
        dpns_preorder_document(Identifier::from(owner), label, salt),
        nonce,
        Kind::Create { entropy },
        key,
        signer,
    )
}

/// The contested-name prefund is attached by dpp from the domain type's
/// contested unique index; nothing here decides whether a name is contested.
#[allow(clippy::too_many_arguments)]
pub fn build_dpns_domain(
    version: &PlatformVersion,
    owner: [u8; 32],
    nonce: u64,
    label: &str,
    salt: &[u8; 32],
    entropy: [u8; 32],
    key: &ffi::IdentityKey,
    signer: &dyn SignerCallbacks,
) -> Result<ffi::Built, String> {
    build_system_document(
        version,
        SystemDataContract::DPNS,
        "domain",
        dpns_domain_document(Identifier::from(owner), label, salt),
        nonce,
        Kind::Create { entropy },
        key,
        signer,
    )
}

/// A DashPay `profile`: created when `existing` has no document id, else
/// replaced at its revision + 1. A replace transition carries the whole
/// document, so the fields the embedder does not edit (avatar, payment
/// addresses) are carried over from `existing` as `get_profile` read them;
/// an empty edited string leaves the field out.
#[allow(clippy::too_many_arguments)]
pub fn build_profile(
    version: &PlatformVersion,
    owner: [u8; 32],
    nonce: u64,
    existing: &ffi::Profile,
    input: &ffi::ProfileInput,
    entropy: [u8; 32],
    key: &ffi::IdentityKey,
    signer: &dyn SignerCallbacks,
) -> Result<ffi::Built, String> {
    let texts = [
        ("displayName", &input.display_name),
        ("publicMessage", &input.public_message),
        ("avatarUrl", &existing.avatar_url),
    ]
    .into_iter()
    .filter(|(_, text)| !text.is_empty())
    .map(|(name, text)| (name.to_string(), Value::Text(text.clone())));
    let bytes = [
        ("avatarHash", &existing.avatar_hash),
        ("avatarFingerprint", &existing.avatar_fingerprint),
        ("corePaymentAddress", &existing.core_payment_address),
        ("platformPaymentAddress", &existing.platform_payment_address),
        ("shieldedAddress", &existing.shielded_address),
    ]
    .into_iter()
    .filter(|(_, bytes)| !bytes.is_empty())
    .map(|(name, bytes)| (name.to_string(), Value::Bytes(bytes.clone())));
    let properties = texts.chain(bytes).collect();
    let owner = Identifier::from(owner);
    if existing.document_id == [0u8; 32] {
        return build_system_document(
            version,
            SystemDataContract::Dashpay,
            "profile",
            new_document(owner, properties),
            nonce,
            Kind::Create { entropy },
            key,
            signer,
        );
    }
    if existing.owner != owner.to_buffer() {
        return Err("the profile to replace belongs to another identity".to_string());
    }
    if existing.revision < INITIAL_REVISION {
        return Err("the profile to replace carries no revision".to_string());
    }
    let document = Document::V0(DocumentV0 {
        id: Identifier::from(existing.document_id),
        owner_id: owner,
        properties,
        revision: Some(existing.revision + 1),
        ..Default::default()
    });
    build_system_document(
        version,
        SystemDataContract::Dashpay,
        "profile",
        document,
        nonce,
        Kind::Replace,
        key,
        signer,
    )
}

/// A DashPay `contactRequest`, minted by `Sdk::create_contact_request` with
/// the embedder's ECDH secret (`EcdhProvider::ClientSide`) and continued
/// into the batch transition exactly as `send_contact_request` does. The
/// contract comes from the context provider, so nothing touches the
/// network. The SDK's own version only picks the contract it mints with;
/// the transition is built under `version`, the one a verified read has
/// shown the network to run.
#[allow(clippy::too_many_arguments)]
pub fn build_contact_request(
    sdk: &Sdk,
    version: &PlatformVersion,
    sender: &ffi::Identity,
    recipient: &ffi::Identity,
    nonce: u64,
    input: &ffi::ContactRequestInput,
    key: &ffi::IdentityKey,
    signer: &dyn SignerCallbacks,
) -> Result<ffi::Built, String> {
    let expected_recipient_pubkey = PublicKey::from_slice(&input.recipient_pubkey)
        .map_err(|e| format!("bad recipient public key: {e}"))?;
    let shared_secret = Zeroizing::new(input.shared_secret);
    let xpub = Zeroizing::new(compact_xpub_to_bytes(&input.compact_xpub));

    let contact_request = ContactRequestInput {
        sender_identity: identity(sender)?,
        recipient: RecipientIdentity::Identity(identity(recipient)?),
        sender_key_index: input.sender_key_index,
        recipient_key_index: input.recipient_key_index,
        account_reference: input.account_reference,
        account_label: (!input.account_label.is_empty()).then(|| input.account_label.clone()),
        auto_accept_proof: None,
    };
    // The SDK-side ECDH variant is never used; a never-called `fn` satisfies
    // its type parameters.
    type UnusedSdkSideEcdh = fn(
        &IdentityPublicKey,
        u32,
    ) -> std::future::Ready<
        Result<dash_sdk::dpp::dashcore::secp256k1::SecretKey, dash_sdk::Error>,
    >;
    let ecdh: EcdhProvider<UnusedSdkSideEcdh, _, _, _> = EcdhProvider::ClientSide {
        // The secret was derived against `recipient_pubkey`; the SDK must
        // have selected that same key from the recipient identity, or the
        // request would encrypt to a key the recipient cannot use.
        get_shared_secret: move |peer: &PublicKey| {
            let matches = *peer == expected_recipient_pubkey;
            async move {
                if !matches {
                    return Err(dash_sdk::Error::Generic(
                        "the recipient key at recipient_key_index is not the key the shared \
                         secret was derived against"
                            .to_string(),
                    ));
                }
                // The SDK takes the secret by value and drops its copy after
                // encrypting; only this crate's copies are zeroized.
                Ok(*shared_secret)
            }
        },
    };
    let result = block_on(
        sdk.create_contact_request(contact_request, ecdh, |_account| {
            let xpub = xpub.clone();
            async move { Ok(xpub.to_vec()) }
        }),
    )
    .map_err(|e| format!("unable to create the contact request: {e}"))?;

    let document = Document::V0(DocumentV0 {
        id: result.id,
        owner_id: result.owner_id,
        properties: result.properties,
        ..Default::default()
    });
    build_system_document(
        version,
        SystemDataContract::Dashpay,
        "contactRequest",
        document,
        nonce,
        Kind::Create {
            entropy: result.entropy.0,
        },
        key,
        signer,
    )
}

fn asset_lock_proof(input: &ffi::AssetLockProofInput) -> Result<AssetLockProof, String> {
    if input.is_instant {
        if input.transaction.len() > MAX_CORE_MESSAGE_BYTES
            || input.instant_lock.len() > MAX_CORE_MESSAGE_BYTES
        {
            return Err("the asset lock transaction or instant lock exceeds 2 MiB".to_string());
        }
        let transaction = Transaction::consensus_decode(&mut input.transaction.as_slice())
            .map_err(|e| format!("bad asset lock transaction: {e}"))?;
        let instant_lock = InstantLock::consensus_decode(&mut input.instant_lock.as_slice())
            .map_err(|e| format!("bad instant lock: {e}"))?;
        return Ok(AssetLockProof::Instant(InstantAssetLockProof::new(
            instant_lock,
            transaction,
            input.output_index,
        )));
    }
    let out_point = OutPoint::consensus_decode(&mut input.out_point.as_slice())
        .map_err(|e| format!("bad asset lock outpoint: {e}"))?;
    Ok(AssetLockProof::Chain(ChainAssetLockProof {
        core_chain_locked_height: input.core_chain_locked_height,
        out_point,
    }))
}

/// An `IdentityCreateTransition` through dpp's
/// `try_from_identity_with_signers`: every registered key signs the
/// transition through `SignForKey`, the asset-lock outpoint key through
/// `SignAssetLockSighash`. The identity id is derived from the asset lock.
pub fn build_identity_create(
    version: &PlatformVersion,
    proof: &ffi::AssetLockProofInput,
    keys: &[ffi::NewIdentityKey],
    signer: &dyn SignerCallbacks,
) -> Result<ffi::Built, String> {
    if keys.is_empty() {
        return Err("an identity needs at least one key".to_string());
    }
    let proof = asset_lock_proof(proof)?;
    let identity_id = proof
        .create_identifier()
        .map_err(|e| format!("unable to derive the identity id: {e}"))?;
    let mut public_keys = BTreeMap::new();
    for key in keys {
        let identity_key = identity_key(&ffi::IdentityKey {
            id: key.id,
            purpose: key.purpose,
            security_level: key.security_level,
            key_type: KeyType::ECDSA_SECP256K1 as u8,
            read_only: false,
            data: key.pubkey.to_vec(),
            disabled_at: 0,
            bounds: key.bounds.clone(),
        })?;
        if public_keys.insert(key.id, identity_key).is_some() {
            return Err(format!("duplicate identity key id {}", key.id));
        }
    }
    let identity: Identity = IdentityV0 {
        id: identity_id,
        public_keys,
        balance: 0,
        revision: 0,
    }
    .into();
    let state_transition = block_on(IdentityCreateTransition::try_from_identity_with_signers(
        &identity,
        proof,
        &DerivationPath::master(),
        &BytesSigner(signer),
        &AssetLockSigner(signer),
        &NativeBlsModule,
        0,
        version,
    ))
    .map_err(|e| format!("unable to build the identity create transition: {e}"))?;
    built(&state_transition, identity_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn salted_domain_hash_matches_register_dpns_name() {
        // The SDK hashes `salt ‖ normalized_label ‖ ".dash"` with sha256d.
        let salt = [0x11u8; 32];
        let mut preimage = salt.to_vec();
        preimage.extend_from_slice(b"a11ce.dash");
        assert_eq!(
            salted_domain_hash("a11ce", &salt),
            dash_sdk::dpp::util::hash::hash_double(preimage)
        );
    }

    #[test]
    fn bounds_and_keys_round_trip() {
        let key = ffi::IdentityKey {
            id: 2,
            purpose: Purpose::ENCRYPTION as u8,
            security_level: SecurityLevel::MEDIUM as u8,
            key_type: KeyType::ECDSA_SECP256K1 as u8,
            data: vec![2u8; 33],
            bounds: ffi::ContractBounds {
                kind: ffi::BoundsKind::SingleContractDocumentType,
                contract_id: SystemDataContract::Dashpay.id().to_buffer(),
                document_type: "contactRequest".to_string(),
            },
            ..Default::default()
        };
        let dpp_key = identity_key(&key).expect("key");
        assert_eq!(
            dpp_key.contract_bounds(),
            Some(&ContractBounds::SingleContractDocumentType {
                id: SystemDataContract::Dashpay.id(),
                document_type_name: "contactRequest".to_string(),
            })
        );
        assert!(identity_key(&ffi::IdentityKey {
            purpose: 200,
            ..key.clone()
        })
        .is_err());
        assert!(identity_key(&ffi::IdentityKey {
            bounds: ffi::ContractBounds {
                kind: ffi::BoundsKind { repr: 9 },
                ..Default::default()
            },
            ..key
        })
        .is_err());
    }

    #[test]
    fn entropy_is_fresh_per_call() {
        assert_ne!(fresh_entropy(), fresh_entropy());
    }
}
