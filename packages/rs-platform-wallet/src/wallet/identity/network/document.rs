//! Document create operations on `IdentityWallet`.
//!
//! Lives on `IdentityWallet` (rather than in `rs-sdk-ffi`) for the
//! same reason as `contract.rs`: creating + broadcasting a document is
//! a wallet-level operation. It spans an identity (the owner), needs
//! the wallet's external signer, and broadcasts a document state
//! transition whose signature key is selected from the in-memory
//! wallet manager. Per `swift-sdk/CLAUDE.md`, "anything that spans
//! identities / platform balances / ... belongs in the
//! `platform-wallet` crate"; the Swift side only renders the form,
//! marshals the values, and persists the confirmed document.
//!
//! Mirrors the post-#3541 identity-flow shape:
//!   - The library function takes a `Signer<IdentityPublicKey>`
//!     reference so the FFI's external `KeychainSigner` trampoline can
//!     route signing back to Swift / Keychain without crossing seed
//!     bytes.
//!   - The document content arrives as a properties JSON string and is
//!     turned into a platform `Value` map, then a revision-1 `Document`
//!     via `DocumentType::create_document_from_data` — the same path
//!     `rs-sdk-ffi/src/document/create.rs` uses.
//!   - Broadcast goes through
//!     `dash_sdk::platform::transition::put_document::PutDocument::put_to_platform_and_wait_for_response`
//!     on the platform-wallet runtime (8 MB worker stack) instead of
//!     the rs-sdk-ffi runtime (mobile-tuned default stack) — the same
//!     stack-overflow avoidance `contract.rs` documents for the
//!     post-broadcast GroveDB proof-verification recursion.

use std::collections::BTreeMap;

use async_trait::async_trait;

use dpp::address_funds::AddressWitness;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::document::Document;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::{BinaryData, Value};
use dpp::prelude::{DataContract, Identifier};
use dpp::ProtocolError;

use dash_sdk::platform::transition::put_document::PutDocument;
use dash_sdk::platform::Fetch;

use crate::error::PlatformWalletError;

use super::*;

/// Borrowed-signer adapter — same shape as the local `SignerRef` in
/// `contract.rs` / `dpns.rs` / `transfer.rs`. Lets the
/// `Signer<IdentityPublicKey>` trait bound on the SDK's `PutDocument`
/// extension be satisfied with a `&S` instead of forcing the caller to
/// hand over ownership / wrap in an `Arc` per call.
struct SignerRef<'a, S: ?Sized>(&'a S);

impl<'a, S: ?Sized> std::fmt::Debug for SignerRef<'a, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SignerRef")
    }
}

#[async_trait]
impl<'a, K, S> Signer<K> for SignerRef<'a, S>
where
    K: Send + Sync,
    S: Signer<K> + ?Sized + Send + Sync,
{
    async fn sign(&self, key: &K, data: &[u8]) -> Result<BinaryData, ProtocolError> {
        self.0.sign(key, data).await
    }

    async fn sign_create_witness(
        &self,
        key: &K,
        data: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        self.0.sign_create_witness(key, data).await
    }

    fn can_sign_with(&self, key: &K) -> bool {
        self.0.can_sign_with(key)
    }
}

/// Build the set of `SecurityLevel`s an AUTHENTICATION key may carry to
/// satisfy a document state-transition whose document type requires
/// `requirement`.
///
/// This reproduces the consensus rule the network enforces in
/// `BatchTransition::combined_security_level_requirement`
/// (`rs-dpp/.../batch_transition/methods/v0/mod.rs`): the signing key's
/// security level must be **stronger-or-equal** to the document type's
/// requirement, expressed as the inclusive range `CRITICAL..=requirement`
/// over the `MASTER(0) < CRITICAL(1) < HIGH(2) < MEDIUM(3)` ordering —
/// with `MASTER` handled as its own degenerate `[MASTER]` set. MASTER is
/// otherwise excluded: it is reserved for identity-self-modification and
/// the document-batch purpose requirement (`vec![AUTHENTICATION]`) never
/// admits it for an ordinary document create.
///
/// Picking the key against this exact set (rather than a hardcoded
/// CRITICAL, which the contract-create path uses) is what makes the
/// flow correct for *any* document type — e.g. DPNS `preorder` requires
/// `HIGH`, so both `CRITICAL` and `HIGH` keys qualify, but `MEDIUM` does
/// not.
fn allowed_signing_security_levels(requirement: SecurityLevel) -> Vec<SecurityLevel> {
    if requirement == SecurityLevel::MASTER {
        return vec![SecurityLevel::MASTER];
    }
    // `CRITICAL as u8 == 1`; iterate down to (and including) the
    // requirement. `SecurityLevel::try_from` only fails for values
    // outside 0..=3, and every value in this range is valid by
    // construction.
    (SecurityLevel::CRITICAL as u8..=requirement as u8)
        .filter_map(|level| SecurityLevel::try_from(level).ok())
        .collect()
}

impl IdentityWallet {
    /// Create a new revision-1 document on `contract_id`'s
    /// `document_type_name` owned by `owner_identity_id`, and broadcast
    /// it to Platform.
    ///
    /// The function:
    ///   1. Fetches the on-chain `DataContract` for `contract_id` via
    ///      `self.sdk` and resolves the owned `DocumentType` for
    ///      `document_type_name`.
    ///   2. Parses `properties_json` into a platform `Value` map,
    ///      sanitizes it against the schema (hex/base64 byte arrays,
    ///      base58/hex identifiers — same as `rs-sdk-ffi`'s document
    ///      create), and builds a revision-1 `Document` via
    ///      `DocumentType::create_document_from_data`. Entropy is
    ///      generated by the SDK at broadcast time (we pass `None`), so
    ///      the canonical document id is derived from
    ///      `(contract, owner, type, entropy)` there — the placeholder
    ///      id this build step assigns is overwritten.
    ///   3. Selects the signing `IdentityPublicKey` from the in-memory
    ///      wallet manager: an AUTHENTICATION-purpose, ECDSA_SECP256K1
    ///      key whose security level satisfies the document type's
    ///      `security_level_requirement()` (see
    ///      [`allowed_signing_security_levels`]). Returns a clear error
    ///      if the owner identity isn't loaded or carries no qualifying
    ///      key.
    ///   4. Broadcasts via
    ///      `Document::put_to_platform_and_wait_for_response` on the
    ///      platform-wallet 8 MB-stack worker and returns the confirmed
    ///      `Document` from Platform.
    ///
    /// `properties_json` is a JSON object keyed by property name. Byte-
    /// array fields are supplied as hex (or base64) strings and
    /// identifier fields as base58 (or hex) strings; the schema-driven
    /// sanitize step converts them to the protocol's native `Bytes` /
    /// `Identifier` values. An empty object (`"{}"`) is valid for a
    /// document type with no required properties.
    pub async fn create_document_with_signer<S>(
        &self,
        owner_identity_id: &Identifier,
        contract_id: &Identifier,
        document_type_name: &str,
        properties_json: &str,
        signer: &S,
    ) -> Result<Document, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        let platform_version = self.sdk.version();

        // 1. Fetch the on-chain contract + resolve the document type.
        let data_contract = DataContract::fetch(&self.sdk, *contract_id)
            .await
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to fetch contract {contract_id} for document create: {e}"
                ))
            })?
            .ok_or_else(|| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Data contract {contract_id} not found on Platform; cannot create document"
                ))
            })?;

        // Owned `DocumentType` — `put_to_platform_and_wait_for_response`
        // takes the document type by value.
        let document_type = data_contract
            .document_type_cloned_for_name(document_type_name)
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Document type {document_type_name:?} not found on contract {contract_id}: {e}"
                ))
            })?;

        // 2. Parse properties JSON -> platform Value map, sanitize
        //    against the schema, and build a revision-1 document.
        let properties_value: serde_json::Value =
            serde_json::from_str(properties_json).map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Invalid document properties JSON: {e}"
                ))
            })?;
        let mut properties: BTreeMap<String, Value> = serde_json::from_value(properties_value)
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Document properties must be a JSON object keyed by property name: {e}"
                ))
            })?;
        document_type
            .as_ref()
            .sanitize_document_properties(&mut properties);

        // Entropy is generated by the SDK at broadcast time (we pass
        // `None` to `put_to_platform_and_wait_for_response`), which
        // overwrites the document id with the canonical
        // `(contract, owner, type, entropy)` derivation. The entropy
        // supplied here only feeds the placeholder id, so a fixed zero
        // value is fine — it is never broadcast.
        let document = document_type
            .as_ref()
            .create_document_from_data(
                properties.into(),
                *owner_identity_id,
                0, // block_height — set by Platform
                0, // core_block_height — set by Platform
                [0u8; 32],
                platform_version,
            )
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!("Failed to build document: {e}"))
            })?;

        // 3. Owner identity + signing key from the wallet manager. The
        //    document state transition must be signed by an
        //    AUTHENTICATION + ECDSA key whose security level satisfies
        //    the document type's requirement (NOT a fixed CRITICAL like
        //    the contract-create path).
        let required_level = document_type.security_level_requirement();
        let allowed_levels = allowed_signing_security_levels(required_level);
        let signing_key = {
            let wm = self.wallet_manager.read().await;
            let info = wm.get_wallet_info(&self.wallet_id).ok_or_else(|| {
                PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?;
            let manager = &info.identity_manager;
            let identity = manager
                .identity(owner_identity_id)
                .map(|m| m.identity.clone())
                .ok_or(PlatformWalletError::IdentityNotFound(*owner_identity_id))?;
            identity
                .get_first_public_key_matching(
                    Purpose::AUTHENTICATION,
                    allowed_levels.iter().copied().collect(),
                    [KeyType::ECDSA_SECP256K1].into(),
                    false,
                )
                .ok_or_else(|| {
                    PlatformWalletError::InvalidIdentityData(format!(
                        "No ECDSA authentication key at a security level satisfying \
                         {required_level} found on owner identity {owner_identity_id} \
                         (required to sign a {document_type_name} document state transition)"
                    ))
                })?
                .clone()
        };

        // 4. Broadcast via `PutDocument` on the platform-wallet 8 MB
        //    worker stack. `None` entropy -> the SDK generates entropy
        //    and the canonical document id for this revision-1 create;
        //    `None` token-payment-info -> no token gating.
        let confirmed = document
            .put_to_platform_and_wait_for_response(
                &self.sdk,
                document_type,
                None,
                signing_key,
                None,
                &SignerRef(signer),
                None,
            )
            .await
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to put document to platform: {e}"
                ))
            })?;

        Ok(confirmed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowed_levels_high_requirement_admits_critical_and_high_only() {
        // DPNS `preorder` requires HIGH: CRITICAL + HIGH qualify,
        // MEDIUM does not, MASTER is excluded.
        let levels = allowed_signing_security_levels(SecurityLevel::HIGH);
        assert_eq!(levels, vec![SecurityLevel::CRITICAL, SecurityLevel::HIGH]);
    }

    #[test]
    fn allowed_levels_medium_requirement_admits_critical_high_medium() {
        let levels = allowed_signing_security_levels(SecurityLevel::MEDIUM);
        assert_eq!(
            levels,
            vec![
                SecurityLevel::CRITICAL,
                SecurityLevel::HIGH,
                SecurityLevel::MEDIUM
            ]
        );
    }

    #[test]
    fn allowed_levels_critical_requirement_admits_only_critical() {
        let levels = allowed_signing_security_levels(SecurityLevel::CRITICAL);
        assert_eq!(levels, vec![SecurityLevel::CRITICAL]);
    }

    #[test]
    fn allowed_levels_master_requirement_is_master_only() {
        // Degenerate case mirrored from the consensus rule: a MASTER
        // requirement collapses to the single `[MASTER]` set rather
        // than the CRITICAL..=MASTER range (which would be empty).
        let levels = allowed_signing_security_levels(SecurityLevel::MASTER);
        assert_eq!(levels, vec![SecurityLevel::MASTER]);
    }
}
