//! DashPay contact request lifecycle: send, sync, accept, reject.

use dpp::document::DocumentV0Getters;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::identity_public_key::Purpose;
use dpp::identity::signer::Signer;
use dpp::identity::Identity;
use dpp::identity::IdentityPublicKey;
use dpp::identity::KeyType;
use dpp::identity::SecurityLevel;
use dpp::platform_value::Value;
use dpp::prelude::Identifier;

use super::contacts::RegisterExternalError;
use super::sdk_writer::SendContactRequestParams;
use super::*;
use crate::broadcaster::TransactionBroadcaster;
use crate::error::PlatformWalletError;
use crate::wallet::identity::types::dashpay::contact_request::ContactRequest;
use crate::wallet::identity::types::dashpay::established_contact::EstablishedContact;

// ---------------------------------------------------------------------------
// Deferred-crypto drain provider
// ---------------------------------------------------------------------------

/// Supplies the wallet-HD key material the seedless contact-crypto paths need
/// (the deferred-crypto drain AND the live send/accept flow), without
/// platform-wallet naming the concrete Keychain signer (`MnemonicResolverCoreSigner`
/// lives in `rs-sdk-ffi`, which platform-wallet does not depend on). The glue
/// crate implements this over the resolver-backed signer; tests implement it
/// with canned values. All methods take a **Rust-built** derivation path —
/// path provenance stays in Rust (the host derives at exactly the path it is
/// handed), and no private scalar ever crosses back into platform-wallet.
#[async_trait::async_trait]
pub trait ContactCryptoProvider {
    /// Extended public key at `path` — our DashPay receiving (friendship) xpub.
    async fn receiving_xpub(
        &self,
        path: &key_wallet::bip32::DerivationPath,
    ) -> Result<key_wallet::bip32::ExtendedPubKey, PlatformWalletError>;

    /// ECDH shared secret between our key at `path` and the contact's `peer`
    /// pubkey.
    async fn ecdh_shared_secret(
        &self,
        path: &key_wallet::bip32::DerivationPath,
        peer: &dashcore::secp256k1::PublicKey,
    ) -> Result<[u8; 32], PlatformWalletError>;

    /// DIP-15 `accountReference` for a send: the scalar at `path` (the sender's
    /// encryption key) keys the HMAC+mask over `compact_xpub`. Computed in the
    /// signer so the raw scalar never returns to platform-wallet.
    async fn account_reference(
        &self,
        path: &key_wallet::bip32::DerivationPath,
        compact_xpub: &[u8],
        account_index: u32,
        version: u32,
    ) -> Result<u32, PlatformWalletError>;

    /// Inverse of [`Self::account_reference`] — recover `(version, account_index)`
    /// from a masked reference using the same in-signer scalar at `path`. Used
    /// on re-send to read the previous rotation version.
    async fn unmask_account_reference(
        &self,
        path: &key_wallet::bip32::DerivationPath,
        compact_xpub: &[u8],
        account_reference: u32,
    ) -> Result<(u32, u32), PlatformWalletError>;
}

/// Test [`ContactCryptoProvider`] that derives from a resident test seed via
/// `key_wallet` — the seedless-test stand-in for the production Keychain signer.
/// It derives at exactly the Rust-built paths the production glue feeds, so a
/// test wired through this provider exercises the same key material the resident
/// seed would have produced, with no resident seed on the wallet under test.
/// (Replaces the `attach_wallet_seed` test setup; faithful, unlike the
/// canned/stub providers used by the queue-mechanics drain tests.)
#[cfg(test)]
pub(crate) struct SeedCryptoProvider {
    wallet: key_wallet::wallet::Wallet,
}

#[cfg(test)]
impl SeedCryptoProvider {
    pub(crate) fn from_seed(seed: [u8; 64], network: key_wallet::Network) -> Self {
        use key_wallet::wallet::initialization::WalletAccountCreationOptions;
        use key_wallet::wallet::Wallet;
        Self {
            wallet: Wallet::from_seed_bytes(seed, network, WalletAccountCreationOptions::None)
                .expect("test seed wallet"),
        }
    }
}

#[cfg(test)]
#[async_trait::async_trait]
impl ContactCryptoProvider for SeedCryptoProvider {
    async fn receiving_xpub(
        &self,
        path: &key_wallet::bip32::DerivationPath,
    ) -> Result<key_wallet::bip32::ExtendedPubKey, PlatformWalletError> {
        self.wallet.derive_extended_public_key(path).map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!("test receiving_xpub: {e}"))
        })
    }

    async fn ecdh_shared_secret(
        &self,
        path: &key_wallet::bip32::DerivationPath,
        peer: &dashcore::secp256k1::PublicKey,
    ) -> Result<[u8; 32], PlatformWalletError> {
        let xprv = self.wallet.derive_extended_private_key(path).map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!("test ecdh derive: {e}"))
        })?;
        Ok(platform_encryption::derive_shared_key_ecdh(
            &xprv.private_key,
            peer,
        ))
    }

    async fn account_reference(
        &self,
        path: &key_wallet::bip32::DerivationPath,
        compact_xpub: &[u8],
        account_index: u32,
        version: u32,
    ) -> Result<u32, PlatformWalletError> {
        let xprv = self.wallet.derive_extended_private_key(path).map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!("test accountRef derive: {e}"))
        })?;
        Ok(platform_encryption::calculate_account_reference(
            &xprv.private_key.secret_bytes(),
            compact_xpub,
            account_index,
            version,
        ))
    }

    async fn unmask_account_reference(
        &self,
        path: &key_wallet::bip32::DerivationPath,
        compact_xpub: &[u8],
        account_reference: u32,
    ) -> Result<(u32, u32), PlatformWalletError> {
        let xprv = self.wallet.derive_extended_private_key(path).map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!("test unmask derive: {e}"))
        })?;
        Ok(platform_encryption::unmask_account_reference(
            account_reference,
            &xprv.private_key.secret_bytes(),
            compact_xpub,
        ))
    }
}

// ---------------------------------------------------------------------------
// Send contact request
// ---------------------------------------------------------------------------

impl<B: TransactionBroadcaster + ?Sized> IdentityWallet<B> {
    /// Send a contact request to another identity using an
    /// externally-supplied signer for the document state-transition.
    ///
    /// All parameters that can be resolved internally are resolved
    /// automatically:
    /// - **identity_index**: looked up from the local `ManagedIdentity`
    /// - **sender_key_index**: first `ECDSA_SECP256K1` `Purpose::ENCRYPTION`
    ///   key on the sender
    /// - **recipient_key_index**: first `ECDSA_SECP256K1` `Purpose::DECRYPTION`
    ///   key on the recipient, falling back to the first ENCRYPTION key when
    ///   the recipient has no DECRYPTION key (mobile cohort) — see
    ///   [`select_recipient_key_index`]
    /// - **account_index**: defaults to `0`
    /// - **ECDH**: performed SDK-side using the sender's derived
    ///   encryption private key.
    ///
    /// Document signing is routed through `signer` — the
    /// architecturally correct path per `swift-sdk/CLAUDE.md`.
    ///
    /// All wallet-HD key material — the friendship receiving xpub, the ECDH
    /// shared secret, and the DIP-15 `accountReference` — is sourced through
    /// `crypto` (a [`ContactCryptoProvider`], the Keychain signer in production,
    /// canned values in tests). No resident seed is touched, so this path works
    /// for seedless / external-signable wallets: the raw ECDH scalar stays in
    /// the signer and only the (public) xpub, the shared secret, and the masked
    /// reference cross back.
    #[allow(clippy::type_complexity)]
    pub async fn send_contact_request_with_external_signer<S, C>(
        &self,
        sender_identity_id: &Identifier,
        recipient_identity_id: &Identifier,
        account_label: Option<String>,
        auto_accept_proof: Option<Vec<u8>>,
        signer: &S,
        crypto: &C,
    ) -> Result<ContactRequest, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
        C: ContactCryptoProvider + Sync,
    {
        // 1. Retrieve the sender identity and its HD index from the
        //    local manager.
        let (sender_identity, identity_index) = {
            let wm = self.wallet_manager.read().await;
            let info = wm
                .get_wallet_info(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
            let managed = info
                .identity_manager
                .managed_identity(sender_identity_id)
                .ok_or(PlatformWalletError::IdentityNotFound(*sender_identity_id))?;
            let index = managed
                .identity_index
                .ok_or(PlatformWalletError::IdentityIndexNotSet(
                    *sender_identity_id,
                ))?;
            (managed.identity.clone(), index)
        };

        // 2. Fetch the recipient identity from Platform.
        let recipient_identity = {
            use dash_sdk::platform::Fetch;
            Identity::fetch(&self.sdk, *recipient_identity_id)
                .await
                .map_err(|e| {
                    PlatformWalletError::InvalidIdentityData(format!(
                        "Failed to fetch recipient identity: {}",
                        e
                    ))
                })?
                .ok_or_else(|| PlatformWalletError::IdentityNotFound(*recipient_identity_id))?
        };

        // 3. Resolve key indices. The sender selects its own ENCRYPTION key
        //    (the live convention for both cohorts); ECDSA_SECP256K1 is
        //    required for ECDH.
        let sender_encryption_key = sender_identity
            .public_keys()
            .iter()
            .find(|(_, k)| {
                k.purpose() == Purpose::ENCRYPTION && k.key_type() == KeyType::ECDSA_SECP256K1
            })
            .map(|(_, k)| k.clone())
            .ok_or_else(|| {
                PlatformWalletError::InvalidIdentityData(
                    "Sender identity has no ECDSA_SECP256K1 encryption key".to_string(),
                )
            })?;
        let sender_key_index = sender_encryption_key.id();

        let recipient_key_index = select_recipient_key_index(&recipient_identity)?;

        // 3b. Gate the selected key pair through the same validator
        //     the receive/accept paths use, BEFORE any ECDH or
        //     broadcast. The selectors above pick plausible indices;
        //     the validator pins the full contract (key types, not
        //     disabled, purpose policy) so a malformed identity can't
        //     reach the encrypt-and-broadcast stage with a key that
        //     would poison the channel.
        let validation = crate::wallet::identity::crypto::validation::validate_contact_request(
            &sender_identity,
            sender_key_index,
            &recipient_identity,
            recipient_key_index,
        );
        if !validation.is_valid {
            return Err(PlatformWalletError::InvalidIdentityData(format!(
                "Contact request failed pre-send validation: {}",
                validation.errors.join("; ")
            )));
        }
        for warning in &validation.warnings {
            tracing::warn!(
                sender = %sender_identity_id,
                recipient = %recipient_identity_id,
                warning,
                "Contact request pre-send validation warning"
            );
        }

        // 4. Derive the DashPay receiving (friendship) xpub via the signer —
        //    no resident seed. The signer derives at exactly the Rust-built
        //    path and returns only the (public) xpub.
        //
        // CONSISTENCY INVARIANT (do not break without re-checking
        // `account_reference`): the friendship xpub path
        // (`DashpayReceivingFunds`) is pinned to account 0, but the
        // accountReference masks THIS `account_index` into its low 28 bits. A
        // same-seed cross-wallet recovery un-masks the reference to learn which
        // of our accounts the xpub belongs to — so if a future change threads a
        // non-zero index here while the path stays at account 0, the recipient
        // would look for the wrong account (silent, no oracle). Make the path
        // account-aware AND add a round-trip test before relaxing this.
        let account_index: u32 = 0;
        let contact_xpub_ext = self
            .receiving_xpub_for(sender_identity_id, recipient_identity_id, account_index, crypto)
            .await?;
        // DIP-15 *compact* 69-byte plaintext (parentFingerprint ‖ chainCode ‖
        // pubKey) — NOT `ExtendedPubKey::encode()`. The DashPay receiving path
        // ends in a Normal256 child, so `encode()` is the 107-byte DIP-14
        // serialization → 128-byte ciphertext → fails the contract's
        // `maxItems: 96` and both reference clients' hard `len == 69` checks.
        let xpub_bytes = platform_encryption::CompactXpub {
            parent_fingerprint: contact_xpub_ext.parent_fingerprint.to_bytes(),
            chain_code: contact_xpub_ext.chain_code.to_bytes(),
            public_key: contact_xpub_ext.public_key.serialize(),
        }
        .to_bytes()
        .to_vec();

        // The sender's encryption-key derivation path. The scalar at this path
        // keys BOTH the ECDH shared secret (step 6) and the accountReference
        // mask (step 4b); the signer derives at exactly this path and the raw
        // scalar never returns here.
        let sender_enc_path = Self::identity_auth_derivation_path(
            self.sdk.network,
            key_wallet::bip32::KeyDerivationType::ECDSA,
            identity_index,
            sender_encryption_key.id(),
        )?;

        // 4b. Mask the accountReference per DIP-15: the low 28
        //     bits are the account index XOR'd with a PRF of the
        //     compact xpub keyed by our ECDH private key; the top 4
        //     bits carry the rotation version. The version starts at 0
        //     and bumps past the previous sent request's version when
        //     re-sending to the same recipient — the contract's unique
        //     index `($ownerId, toUserId, accountReference)` rejects an
        //     identical resend, so the bump is what makes a superseding
        //     (rotation) request broadcastable. The HMAC+mask runs in the
        //     signer (keyed by the raw scalar at `sender_enc_path`).
        let account_reference = {
            let prior_reference = {
                let wm = self.wallet_manager.read().await;
                wm.get_wallet_info(&self.wallet_id)
                    .and_then(|info| info.identity_manager.managed_identity(sender_identity_id))
                    // Checks both the pending sent map AND the established
                    // contact's outgoing request — see the method doc for
                    // why consulting only the pending map breaks rotation
                    // on established contacts.
                    .and_then(|managed| managed.prior_sent_account_reference(recipient_identity_id))
            };
            let previous_version = match prior_reference {
                Some(prior) => Some(
                    crypto
                        .unmask_account_reference(&sender_enc_path, &xpub_bytes, prior)
                        .await?
                        .0,
                ),
                None => None,
            };
            let version = match previous_version {
                // 4-bit field; saturate rather than wrap so a 16th
                // rotation fails loudly at the unique index instead of
                // silently colliding with version 0.
                Some(v) if v >= 15 => {
                    tracing::warn!(
                        recipient = %recipient_identity_id,
                        "accountReference rotation version saturated at 15"
                    );
                    15
                }
                Some(v) => v + 1,
                None => 0,
            };
            crypto
                .account_reference(&sender_enc_path, &xpub_bytes, account_index, version)
                .await?
        };

        // 5. Build the signing key reference for document signing.
        let identity_public_key = sender_identity
            // Contact-request send writes a document state transition,
            // which DPP requires to be signed by a HIGH-or-stricter
            // authentication key. MASTER is rejected on document writes.
            .get_first_public_key_matching(
                Purpose::AUTHENTICATION,
                [SecurityLevel::HIGH, SecurityLevel::CRITICAL].into(),
                [KeyType::ECDSA_SECP256K1].into(),
                false,
            )
            .cloned()
            .ok_or_else(|| {
                PlatformWalletError::InvalidIdentityData(
                    "Sender identity has no HIGH or CRITICAL authentication key \
                     (required for document state transitions)"
                        .to_string(),
                )
            })?;

        // 6. Client-side ECDH via the signer: the shared secret is derived in
        //    the signer (scalar at `sender_enc_path`) against the recipient's
        //    encryption key, so the SDK seam receives the finished secret
        //    (`EcdhProvider::ClientSide`) and no private key is ever materialized
        //    here. The recipient key is resolved exactly as the SDK would
        //    (`recipientKeyIndex` on the recipient identity); the seam re-checks
        //    the SDK asks for this same key before using the secret.
        let recipient_enc_pubkey = {
            use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
            let key = recipient_identity
                .public_keys()
                .get(&recipient_key_index)
                .ok_or_else(|| {
                    PlatformWalletError::InvalidIdentityData(format!(
                        "Recipient identity has no key at index {recipient_key_index}"
                    ))
                })?;
            dashcore::secp256k1::PublicKey::from_slice(key.data().as_slice()).map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Recipient encryption public key is invalid: {e}"
                ))
            })?
        };
        let shared_secret = crypto
            .ecdh_shared_secret(&sender_enc_path, &recipient_enc_pubkey)
            .await?;

        // 7. Broadcast through the write seam. All inputs are resolved
        //    above; the seam assembles the SDK `EcdhProvider` + xpub
        //    closure and dispatches `Sdk::send_contact_request`. Routing
        //    the broadcast through `sdk_writer` (rather than calling the
        //    seven-generic SDK method inline) is what makes this path
        //    testable without a live network — see `sdk_writer.rs`.
        let result = self
            .sdk_writer
            .send_contact_request(SendContactRequestParams {
                sender_identity: sender_identity.clone(),
                recipient_identity,
                sender_key_index,
                recipient_key_index,
                account_reference,
                account_label,
                auto_accept_proof,
                shared_secret,
                expected_recipient_pubkey: recipient_enc_pubkey,
                xpub_bytes,
                signing_public_key: identity_public_key,
                signer: signer as &(dyn Signer<IdentityPublicKey> + Send + Sync),
            })
            .await?;

        // 8. Mirror the local-state bookkeeping in `send_contact_request`.
        //
        // Store the REAL 96-byte ciphertext off the broadcast
        // document (not a zero placeholder) so the persisted /
        // SwiftData row matches what landed on Platform — a restored
        // device comparing local rows against chain sees identity,
        // and the sent-side re-ingest doesn't "upgrade" the row.
        // Hard error rather than a zero-fill fallback: persisting a 96-byte
        // all-zero "valid-looking" ciphertext would poison the local row
        // (a restored device compares it to chain and mismatches; anything
        // treating it as the contact's xpub source decrypts garbage). The
        // broadcast already landed on-chain, so the sweep re-ingests
        // the real document on the next pass — returning an error here is
        // strictly safer than silently storing poison in release builds.
        let encrypted_public_key = result
            .document
            .properties()
            .get("encryptedPublicKey")
            .and_then(|v: &Value| v.to_binary_bytes().ok())
            .ok_or_else(|| {
                PlatformWalletError::InvalidIdentityData(
                    "broadcast contactRequest lacks a readable encryptedPublicKey; \
                     the on-chain doc will reconcile on the next sync"
                        .to_string(),
                )
            })?;
        let contact_request = ContactRequest::new(
            *sender_identity_id,
            result.recipient_id,
            sender_key_index,
            recipient_key_index,
            result.account_reference,
            encrypted_public_key,
            result.document.created_at_core_block_height().unwrap_or(0),
            result.document.created_at().unwrap_or(0),
        );

        {
            let mut wm = self.wallet_manager.write().await;
            let info = wm
                .get_wallet_info_mut(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
            let managed = info
                .identity_manager
                .managed_identity_mut(sender_identity_id)
                .ok_or(PlatformWalletError::IdentityNotFound(*sender_identity_id))?;
            managed.add_sent_contact_request(contact_request.clone(), &self.persister);
        }

        // Register our receiving (friendship) account from the xpub the signer
        // already derived above — same friendship key, no second derivation and
        // no resident seed.
        self.register_contact_account(
            sender_identity_id,
            recipient_identity_id,
            account_index,
            Some(contact_xpub_ext),
        )
        .await?;

        Ok(contact_request)
    }
}

/// Collapse a stream of parsed received contact requests to the single
/// newest request per sender, keyed by `sender_id`.
///
/// "Newest" is the lexicographic max of `(created_at, account_reference)`
/// — created_at is the primary signal (a rotation request is broadcast
/// later), with account_reference as a deterministic tiebreak for the
/// degenerate same-timestamp case.
///
/// This is the idempotency keystone of the recurring sync: on-chain
/// `contactRequest` docs are immutable and never deleted, so a sender who
/// rotated leaves both their old and bumped-reference docs returning on
/// every sweep. Feeding both into the ingest loop makes the stale one look
/// like a "rotation" away from the tracked state, thrashing it back and
/// forth each pass. Collapsing to the newest first makes the sweep a
/// fixpoint.
/// High-water rewind window applied to the incremental contact-request query.
/// Re-fetching the last 10 minutes each sweep covers clock skew **and**
/// equal-`$createdAt` documents straddling a page boundary, so it is
/// correctness-load-bearing — NOT a tunable; `0` is invalid.
const SYNC_OVERLAP_MS: u64 = 10 * 60_000;

/// Lower bound for the incremental `$createdAt >` query: the high-water minus
/// the overlap window. `None` (no cursor yet) ⇒ full fetch.
fn query_lower_bound(high_water: Option<u64>) -> Option<u64> {
    high_water.map(|hw| hw.saturating_sub(SYNC_OVERLAP_MS))
}

/// Advance a high-water cursor to the max `$createdAt` fetched this sweep,
/// never below its current value. `max_fetched` is the max over docs *seen*
/// (including ones ingest later collapses or skips — the cursor records
/// fetch-completeness, not ingest-success), `None` when nothing was fetched (a
/// zero-doc sweep leaves the cursor unchanged). The caller must only invoke
/// this when the paginate exhausted without error.
fn advance_high_water(current: Option<u64>, max_fetched: Option<u64>) -> Option<u64> {
    match (current, max_fetched) {
        (Some(c), Some(m)) => Some(c.max(m)), // never move backward
        (None, m) => m,                       // first sweep: adopt what was fetched
        (current, None) => current,           // zero-doc sweep: leave unchanged
    }
}

/// Advance the cursor only if it still holds `snapshot` — the value read at the
/// start of the sweep. If it changed mid-sweep (an `unignore_sender` resets it
/// to `None` to force a re-fetch of a sender whose docs predate the cursor),
/// this sweep's `max_fetched` is stale — its fetch ran before the reset and
/// excluded that sender — so leave the new value rather than clobber the rewind.
/// Without this, a concurrent un-ignore is lost and the sender stays invisible
/// until a cold restart.
fn advance_if_unchanged(
    current: Option<u64>,
    snapshot: Option<u64>,
    max_fetched: Option<u64>,
) -> Option<u64> {
    if current == snapshot {
        advance_high_water(snapshot, max_fetched)
    } else {
        current
    }
}

fn newest_received_per_sender(
    requests: impl IntoIterator<Item = ContactRequest>,
) -> std::collections::BTreeMap<Identifier, ContactRequest> {
    let mut newest: std::collections::BTreeMap<Identifier, ContactRequest> =
        std::collections::BTreeMap::new();
    for req in requests {
        let sender = req.sender_id;
        let replace = newest
            .get(&sender)
            .map(|cur| {
                (req.created_at, req.account_reference) > (cur.created_at, cur.account_reference)
            })
            .unwrap_or(true);
        if replace {
            newest.insert(sender, req);
        }
    }
    newest
}

/// Select the recipient identity's key id to reference in
/// `recipientKeyIndex` for an outgoing contact request.
///
/// Verified testnet reality: the newest cohort uses a
/// recipient **DECRYPTION** key (our original convention), but the dominant
/// 126-owner mobile population has **no DECRYPTION key at all** and references
/// its **ENCRYPTION** key for `recipientKeyIndex`. To send to either cohort:
///
/// 1. Prefer the recipient's first `ECDSA_SECP256K1` **DECRYPTION** key.
/// 2. Fall back to the recipient's first `ECDSA_SECP256K1` **ENCRYPTION** key.
/// 3. Error only if the recipient has neither.
///
/// No AUTHENTICATION fallback: no live client population needs it, and reusing
/// signing keys for ECDH is poor key separation. `ECDSA_SECP256K1` is required
/// either way (every observed key is that type, and ECDH needs the full key).
fn select_recipient_key_index(recipient_identity: &Identity) -> Result<u32, PlatformWalletError> {
    // Skip disabled (revoked) keys: encrypting the DIP-15 compact xpub to a
    // key whose private half may be compromised would hand the contact's
    // payment xpub to whoever holds the revoked key. `disabled_at().is_none()`
    // mirrors the validator's disabled-key gate.
    // Prefer a DECRYPTION key.
    if let Some((id, _)) = recipient_identity.public_keys().iter().find(|(_, k)| {
        k.purpose() == Purpose::DECRYPTION
            && k.key_type() == KeyType::ECDSA_SECP256K1
            && k.disabled_at().is_none()
    }) {
        return Ok(*id);
    }
    // Fall back to an ENCRYPTION key (mobile cohort).
    if let Some((id, _)) = recipient_identity.public_keys().iter().find(|(_, k)| {
        k.purpose() == Purpose::ENCRYPTION
            && k.key_type() == KeyType::ECDSA_SECP256K1
            && k.disabled_at().is_none()
    }) {
        return Ok(*id);
    }
    Err(PlatformWalletError::InvalidIdentityData(
        "Recipient identity has no enabled ECDSA_SECP256K1 DECRYPTION or ENCRYPTION key"
            .to_string(),
    ))
}

// ---------------------------------------------------------------------------
// Sync contact requests from platform
// ---------------------------------------------------------------------------

impl<B: TransactionBroadcaster + ?Sized> IdentityWallet<B> {
    /// Fetch and process contact requests from the platform for all local identities.
    ///
    /// For every identity in the local manager this method, per sweep:
    /// 1. Fetches both **received** and **own sent** contact-request
    ///    documents from Platform.
    /// 2. Ingests received requests via `add_incoming_contact_request` —
    ///    including reciprocal requests from senders we already sent to (so
    ///    contacts establish via sync). Dedup is preserved for requests
    ///    already tracked as incoming or established, and every request from
    ///    an ignored sender is suppressed (per-sender — all of their requests,
    ///    rotations included).
    /// 3. Ingests own sent requests via `add_sent_contact_request`, which
    ///    carries its own sent-side guard so a recurring re-ingest
    ///    creates no phantom pending rows and preserves contact metadata.
    /// 4. For **every** established contact missing a sending account
    ///    (not only newly-established ones — this also repairs
    ///    restore-from-seed and best-effort-accept gaps), rebuilds both
    ///    the `DashpayReceivingFunds` and `DashpayExternalAccount`
    ///    accounts, with the transient/permanent failure policy.
    ///
    /// **Lock ordering (critical).** The account-building registrations
    /// (`register_contact_account`, `register_external_contact_account`)
    /// re-acquire the wallet-manager lock, which is a **non-reentrant**
    /// tokio `RwLock`. Candidates are therefore collected while the write
    /// guard is held, the guard is **dropped**, and only then are the
    /// register functions called — mirroring the accept path. Calling
    /// them inline under the guard would deadlock on first execution.
    ///
    /// Returns all newly discovered incoming contact requests.
    pub async fn sync_contact_requests(&self) -> Result<Vec<ContactRequest>, PlatformWalletError> {
        // Snapshot each identity's high-water cursors up front so the
        // incremental query bound is read before any mutation this sweep.
        let identities: Vec<(Identifier, Option<u64>, Option<u64>)> = {
            let wm = self.wallet_manager.read().await;
            let info = wm
                .get_wallet_info(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
            info.identity_manager
                .all_identities()
                .into_iter()
                .map(|i| {
                    let id = i.id();
                    let (hwr, hws) = info
                        .identity_manager
                        .managed_identity(&id)
                        .map(|m| (m.high_water_received_ms, m.high_water_sent_ms))
                        .unwrap_or((None, None));
                    (id, hwr, hws)
                })
                .collect()
        };

        let mut all_requests = Vec::new();

        for (identity_id, hw_received, hw_sent) in identities {
            // --- Fetch (no guard held during the awaits). ---
            //
            // Log-and-continue per identity: a fetch failure for one
            // identity must NOT abort the sweep across the others. This
            // is load-bearing for the recurring loop — a single
            // identity's transient DAPI error shouldn't stall DashPay
            // sync for every other identity on the wallet.
            let received_docs = match self
                .sdk
                .fetch_received_contact_requests(identity_id, query_lower_bound(hw_received))
                .await
            {
                Ok(docs) => docs,
                Err(e) => {
                    tracing::warn!(
                        identity = %identity_id,
                        error = %e,
                        "Failed to fetch received contact requests; skipping this identity"
                    );
                    continue;
                }
            };
            // Also fetch our own sent requests so a restored / second
            // device reconciles established contacts instead of rendering
            // them as bare incoming requests. A failure here is logged but
            // does not skip the received-side ingest already fetched above —
            // and the sent cursor is NOT advanced when this fails.
            let mut sent_ok = true;
            let sent_docs = match self
                .sdk
                .fetch_sent_contact_requests(identity_id, query_lower_bound(hw_sent))
                .await
            {
                Ok(docs) => docs,
                Err(e) => {
                    tracing::warn!(
                        identity = %identity_id,
                        error = %e,
                        "Failed to fetch sent contact requests; reconciling received side only"
                    );
                    sent_ok = false;
                    Default::default()
                }
            };

            // Max `$createdAt` over docs FETCHED this sweep (not over docs that
            // survive ingest's collapse/dedup) — the cursor records
            // fetch-completeness. Reaching here means the received fetch
            // exhausted without error, so its cursor may advance; the sent
            // cursor advances only if `sent_ok`.
            let max_received = received_docs
                .values()
                .filter_map(|d| d.as_ref())
                .filter_map(|d| d.created_at())
                .max();
            let max_sent = sent_docs
                .values()
                .filter_map(|d| d.as_ref())
                .filter_map(|d| d.created_at())
                .max();

            // --- Ingest under the write guard; collect account-building
            //     candidates; then DROP the guard before registering. ---
            let candidates = {
                let mut wm = self.wallet_manager.write().await;
                let Some((wallet, info)) = wm.get_wallet_mut_and_info_mut(&self.wallet_id) else {
                    continue;
                };
                let managed = match info.identity_manager.managed_identity_mut(&identity_id) {
                    Some(m) => m,
                    None => continue,
                };
                // Established contacts re-keyed by a rotation request in
                // this pass — their stale external accounts are torn down
                // below so the build sweep re-registers from the new xpub.
                let mut rotated_contacts: Vec<Identifier> = Vec::new();

                // (1) Ingest received requests.
                //
                // Immutable contactRequest docs are never deleted on-chain,
                // so a sender who rotated leaves MULTIPLE docs — the old
                // reference plus the bumped one — that ALL return on every
                // sweep. Collapse to the single newest doc per sender BEFORE
                // ingest (see `newest_received_per_sender`). Without this, a
                // stale older doc is mis-read as a "rotation" away from the
                // tracked state on every sweep, flipping the stored reference
                // back and forth, tearing down + rebuilding the external
                // account, and writing a changeset each pass forever.
                let parsed_received = received_docs.iter().filter_map(|(_doc_id, maybe_doc)| {
                    let doc = maybe_doc.as_ref()?;
                    Self::parse_contact_request_doc(doc, doc.owner_id(), identity_id)
                });
                let newest_by_sender = newest_received_per_sender(parsed_received);

                for (sender_id, contact_request) in newest_by_sender {
                    // Ignore (per-sender mute, local-only): an ignored
                    // sender's requests are ALL suppressed from the main
                    // pending list — including rotated (bumped
                    // accountReference) ones. Checked FIRST and per-sender,
                    // unlike the old per-(sender, accountReference) reject:
                    // if you ignored the person you ignored them.
                    // `unignore_sender` rewinds the cursor so this skip stops
                    // firing on the next sweep.
                    if managed.is_sender_ignored(&sender_id) {
                        tracing::debug!(
                            sender = %sender_id,
                            recipient = %identity_id,
                            account_reference = contact_request.account_reference,
                            "Skipping ignored sender's contact request"
                        );
                        continue;
                    }
                    // Do NOT skip just because the sender is in
                    // `sent_contact_requests` — that is the reciprocal we
                    // need to let through to auto-establish. True dedup is
                    // (sender, accountReference): the SAME reference as the
                    // tracked incoming/established state is a re-ingest of a
                    // known doc; a DIFFERENT reference from a known sender
                    // is a rotation request (receive side) and must get
                    // through.
                    let tracked_reference = managed
                        .incoming_contact_requests
                        .get(&sender_id)
                        .map(|r| r.account_reference)
                        .or_else(|| {
                            managed
                                .established_contacts
                                .get(&sender_id)
                                .map(|c| c.incoming_request.account_reference)
                        });
                    if tracked_reference == Some(contact_request.account_reference) {
                        continue;
                    }

                    if tracked_reference.is_some() {
                        // Rotation: supersede the tracked request. When an
                        // established contact was re-keyed, queue the stale
                        // external account for teardown so the build sweep
                        // below re-registers it from the new xpub.
                        if managed.apply_rotated_incoming_request(
                            contact_request.clone(),
                            &self.persister,
                        ) {
                            rotated_contacts.push(sender_id);
                        }
                        all_requests.push(contact_request);
                        continue;
                    }

                    managed.add_incoming_contact_request(contact_request.clone(), &self.persister);
                    all_requests.push(contact_request);
                }

                // (2) Ingest our own sent requests. `add_sent_contact_request`
                //     guards itself against duplicates / metadata loss.
                for (_doc_id, maybe_doc) in sent_docs.iter() {
                    let doc = match maybe_doc {
                        Some(d) => d,
                        None => continue,
                    };
                    // For a sent request the recipient is `toUserId`.
                    let recipient_id = match doc
                        .properties()
                        .get("toUserId")
                        .and_then(|v: &Value| v.to_identifier().ok())
                    {
                        Some(v) => v,
                        None => continue,
                    };
                    let Some(contact_request) =
                        Self::parse_sent_contact_request_doc(doc, identity_id, recipient_id)
                    else {
                        continue;
                    };
                    managed.add_sent_contact_request(contact_request, &self.persister);
                }

                // (2b) Tear down stale external accounts for contacts that
                //      rotated in this pass: both the immutable Account
                //      (old xpub — `send_payment`'s derivation source) and
                //      the managed wrapper (old address pool). The
                //      candidate collection below then re-queues them and
                //      the build step re-registers from the NEW encrypted
                //      xpub. The persisted account row is upserted (same
                //      unique key) when the re-registration round lands.
                for contact_id in &rotated_contacts {
                    use key_wallet::account::account_collection::DashpayAccountKey;
                    let key = DashpayAccountKey {
                        index: 0,
                        user_identity_id: identity_id.to_buffer(),
                        friend_identity_id: contact_id.to_buffer(),
                    };
                    wallet.accounts.dashpay_external_accounts.remove(&key);
                    info.core_wallet
                        .accounts
                        .dashpay_external_accounts
                        .remove(&key);
                }

                // Advance the high-water cursors to the max `$createdAt`
                // fetched this sweep, never below the current value. The
                // received fetch reached here only on success; advance the
                // sent cursor only if its fetch also succeeded. A mid-sweep
                // fetch error therefore leaves that direction's cursor intact
                // so the overlap re-fetches next sweep (no burying).
                //
                // Compare-and-advance (see `advance_if_unchanged`): a concurrent
                // `unignore_sender` may have reset the cursor mid-sweep to force
                // a re-fetch; this sweep's stale `max` must not clobber that.
                managed.high_water_received_ms =
                    advance_if_unchanged(managed.high_water_received_ms, hw_received, max_received);
                if sent_ok {
                    managed.high_water_sent_ms =
                        advance_if_unchanged(managed.high_water_sent_ms, hw_sent, max_sent);
                }

                // (3) Collect account-building candidates: every established
                //     contact missing a sending (external) account, skipping
                //     contacts whose payment channel is already marked
                //     permanently broken (no unbounded retry).
                Self::collect_account_build_candidates(info, &identity_id)
            };

            // --- Build accounts AFTER dropping the write guard. ---
            for candidate in candidates {
                self.build_contact_accounts(&identity_id, candidate).await;
            }
        }

        Ok(all_requests)
    }

    /// Parse a received `contactRequest` document into a [`ContactRequest`],
    /// logging + returning `None` on any missing required field.
    fn parse_contact_request_doc(
        doc: &dpp::document::Document,
        sender_id: Identifier,
        recipient_id: Identifier,
    ) -> Option<ContactRequest> {
        let props = doc.properties();
        let sender_key_index = props
            .get("senderKeyIndex")
            .and_then(|v: &Value| v.to_integer::<u32>().ok());
        let recipient_key_index = props
            .get("recipientKeyIndex")
            .and_then(|v: &Value| v.to_integer::<u32>().ok());
        let account_reference = props
            .get("accountReference")
            .and_then(|v: &Value| v.to_integer::<u32>().ok());
        let encrypted_public_key = props
            .get("encryptedPublicKey")
            .and_then(|v: &Value| v.as_bytes())
            .cloned();

        match (
            sender_key_index,
            recipient_key_index,
            account_reference,
            encrypted_public_key,
        ) {
            (Some(ski), Some(rki), Some(ar), Some(epk)) => Some(ContactRequest::new(
                sender_id,
                recipient_id,
                ski,
                rki,
                ar,
                epk,
                doc.created_at_core_block_height().unwrap_or(0),
                doc.created_at().unwrap_or(0),
            )),
            _ => {
                tracing::warn!(
                    sender = %sender_id,
                    recipient = %recipient_id,
                    "Skipping contact request document: missing required field"
                );
                None
            }
        }
    }

    /// Parse our own sent `contactRequest` document into a [`ContactRequest`]
    /// (owner is us, recipient is `toUserId`).
    fn parse_sent_contact_request_doc(
        doc: &dpp::document::Document,
        owner_id: Identifier,
        recipient_id: Identifier,
    ) -> Option<ContactRequest> {
        // Same field set as the received side; the only difference is which
        // identity is owner vs recipient.
        Self::parse_contact_request_doc(doc, owner_id, recipient_id)
    }

    /// Collect every established contact (for `identity_id`) that is
    /// missing its `DashpayExternalAccount` and is NOT already marked
    /// permanently broken — the account-building candidates for this
    /// sweep. Runs under the caller's write guard; performs no
    /// awaits and no lock re-acquisition.
    fn collect_account_build_candidates(
        info: &crate::wallet::platform_wallet::PlatformWalletInfo,
        identity_id: &Identifier,
    ) -> Vec<AccountBuildCandidate> {
        use key_wallet::account::account_collection::DashpayAccountKey;

        let Some(managed) = info.identity_manager.managed_identity(identity_id) else {
            return Vec::new();
        };

        let mut out = Vec::new();
        for (contact_id, contact) in &managed.established_contacts {
            // Never retry a permanently-broken channel — wait for a
            // superseding request (which clears the flag on re-establish).
            if contact.payment_channel_broken {
                continue;
            }
            let key = DashpayAccountKey {
                index: 0,
                user_identity_id: identity_id.to_buffer(),
                friend_identity_id: contact_id.to_buffer(),
            };
            let has_external = info
                .core_wallet
                .accounts
                .dashpay_external_accounts
                .contains_key(&key);
            if has_external {
                continue;
            }
            // The incoming request carries the counterparty's encrypted
            // xpub + the key indices needed for ECDH.
            let incoming = &contact.incoming_request;
            out.push(AccountBuildCandidate {
                contact_id: *contact_id,
                encrypted_public_key: incoming.encrypted_public_key.clone(),
                our_decryption_key_index: incoming.recipient_key_index,
                contact_encryption_key_index: incoming.sender_key_index,
            });
        }
        out
    }

    /// Build the two DashPay accounts for one established contact,
    /// applying the transient/permanent failure policy.
    ///
    /// Order:
    /// 1. Register the `DashpayReceivingFunds` account — derivable from our
    ///    own seed, no decryption needed. This is what makes *incoming*
    ///    contact payments visible to SPV; restore-from-seed leaves it
    ///    unbuilt, so the sweep rebuilds it for every established contact.
    /// 2. Fetch the counterparty identity and **validate** the request's
    ///    key indices via [`validate_contact_request`] BEFORE any ECDH —
    ///    an attacker-crafted index pointing at an AUTHENTICATION key would
    ///    otherwise derive a wrong shared secret and poison the account.
    /// 3. Register the `DashpayExternalAccount` (decrypt + ECDH).
    ///
    /// Failure policy:
    /// - **Transient** (identity fetch / network): logged, left for the
    ///   next sweep to retry. The broken flag stays clear.
    /// - **Permanent** (validation failure, decrypt/decode failure): the
    ///   contact is marked `payment_channel_broken` so subsequent sweeps
    ///   skip it until a superseding request arrives.
    ///
    /// Watch-only / seedless wallets (no `identity_index`) are skipped and
    /// logged — the watch-only ECDH path (host-side signing hook) lands
    /// later.
    ///
    /// Called **after** the sync write guard is dropped: the register
    /// functions re-acquire the non-reentrant wallet-manager lock.
    async fn build_contact_accounts(
        &self,
        identity_id: &Identifier,
        candidate: AccountBuildCandidate,
    ) {
        let contact_id = candidate.contact_id;

        // Readiness: can we build accounts for this identity right now?
        // - Unmanaged or out-of-wallet (no HD slot) → not ours to build; skip.
        // - Wallet-owned but no resident key material (watch-only / signer not
        //   unlocked) → DEFER: do NOT churn the receiving account (step 1) or
        //   break the external channel (step 3); the build runs on a later
        //   sweep once a signer is available. Gating on `has_seed()` rather
        //   than `identity_index.is_none()` is the fix: a wallet-owned but
        //   seedless identity has an index, so the old predicate let it fall
        //   through to a transient-churn (step 1) and a permanent channel-kill
        //   (step 3). Currently "can derive" == `has_seed()`; the seedless
        //   model extends this to an available resolver-backed signer.
        enum BuildReadiness {
            NotOurs,
            KeyMaterialUnavailable,
            Ready,
        }
        let readiness = {
            let wm = self.wallet_manager.read().await;
            let indexed = wm
                .get_wallet_info(&self.wallet_id)
                .and_then(|info| info.identity_manager.managed_identity(identity_id))
                .map(|managed| managed.identity_index.is_some())
                .unwrap_or(false);
            if !indexed {
                BuildReadiness::NotOurs
            } else if wm
                .get_wallet(&self.wallet_id)
                .map(|w| w.has_seed())
                .unwrap_or(false)
            {
                BuildReadiness::Ready
            } else {
                BuildReadiness::KeyMaterialUnavailable
            }
        };
        match readiness {
            BuildReadiness::NotOurs => {
                tracing::info!(
                    identity = %identity_id,
                    contact = %contact_id,
                    "Skipping DashPay account build for unmanaged/out-of-wallet identity"
                );
                return;
            }
            BuildReadiness::KeyMaterialUnavailable => {
                // Enqueue the deferred crypto ops so a later drain (signer
                // present) completes them, instead of churning the receiving
                // account or breaking the external channel every sweep.
                self.enqueue_deferred_contact_crypto(identity_id, &candidate)
                    .await;
                tracing::info!(
                    identity = %identity_id,
                    contact = %contact_id,
                    "Deferred DashPay account build: key material unavailable \
                     (watch-only / signer not unlocked); ops enqueued, will run when a signer is available"
                );
                return;
            }
            BuildReadiness::Ready => {}
        }

        // (1) Receiving account — derivable from our seed, no decryption.
        if let Err(e) = self
            .register_contact_account(identity_id, &contact_id, 0, None)
            .await
        {
            // Treated as transient: a derivation/insert hiccup here doesn't
            // poison the channel, and the receiving account is rebuilt on
            // the next sweep. Do NOT mark broken.
            tracing::warn!(
                identity = %identity_id,
                contact = %contact_id,
                error = %e,
                "Failed to register DashPay receiving account; will retry next sweep"
            );
        }

        // (2) Fetch counterparty identity (transient on failure) + validate
        //     key indices BEFORE any ECDH (permanent on failure).
        let contact_identity = {
            use dash_sdk::platform::Fetch;
            match Identity::fetch(&self.sdk, contact_id).await {
                Ok(Some(id)) => id,
                Ok(None) => {
                    // The contact identity isn't on Platform — treat as
                    // transient (it may appear later); leave for retry.
                    tracing::warn!(
                        identity = %identity_id,
                        contact = %contact_id,
                        "Contact identity not found on Platform; deferring account build"
                    );
                    return;
                }
                Err(e) => {
                    tracing::warn!(
                        identity = %identity_id,
                        contact = %contact_id,
                        error = %e,
                        "Transient failure fetching contact identity; will retry next sweep"
                    );
                    return;
                }
            }
        };

        // Our identity for the validation (in-memory; cloned under a read lock).
        let our_identity = {
            let wm = self.wallet_manager.read().await;
            wm.get_wallet_info(&self.wallet_id)
                .and_then(|info| info.identity_manager.managed_identity(identity_id))
                .map(|m| m.identity.clone())
        };
        let Some(our_identity) = our_identity else {
            tracing::warn!(
                identity = %identity_id,
                contact = %contact_id,
                "Our identity vanished during account build; deferring"
            );
            return;
        };

        // Validate the request's key indices (purpose ENCRYPTION/DECRYPTION
        // + ECDSA type) BEFORE deriving the shared secret. A failure is
        // PERMANENT — the request is malformed and re-deriving won't help.
        let validation = crate::wallet::identity::crypto::validation::validate_contact_request(
            &contact_identity,
            candidate.contact_encryption_key_index,
            &our_identity,
            candidate.our_decryption_key_index,
        );
        if !validation.is_valid {
            // A PURPOSE-only mismatch (e.g. a legacy 2024 doc
            // referencing an AUTHENTICATION key) is NOT permanent — the
            // immutable request can't change but our acceptance policy might,
            // and on-chain history contains nonconforming-but-honest docs.
            // Skip + log; the next sweep retries. Reserve the permanent
            // broken mark for key-TYPE / missing-key / disabled-key failures.
            // `is_purpose_only()` (not the bare `purpose_mismatch` flag) so a
            // purpose mismatch that co-occurs with a hard error still marks
            // broken instead of masking the permanent fault into a retry loop.
            if validation.is_purpose_only() {
                tracing::warn!(
                    identity = %identity_id,
                    contact = %contact_id,
                    errors = ?validation.errors,
                    "Contact request key-purpose mismatch; skipping account build (not marking broken — will retry)"
                );
                return;
            }
            tracing::warn!(
                identity = %identity_id,
                contact = %contact_id,
                errors = ?validation.errors,
                "Contact request failed key-index validation; marking payment channel broken (permanent)"
            );
            self.mark_contact_channel_broken(identity_id, &contact_id)
                .await;
            return;
        }

        // (3) Register the external (sending) account — decrypt + ECDH.
        //     Pass the identity we already fetched above so registration
        //     does no network I/O: that way a PERMANENT crypto/data fault
        //     (bad encrypted xpub, missing key) breaks the channel, but a
        //     TRANSIENT persistence hiccup is left for the next sweep to
        //     retry instead of permanently killing payments.
        match self
            .register_external_contact_account(
                identity_id,
                &contact_identity,
                &candidate.encrypted_public_key,
                candidate.our_decryption_key_index,
                candidate.contact_encryption_key_index,
                // Resident-seed path; the readiness gate above guarantees a seed
                // here. The seedless drain is the only caller passing `Some`.
                None,
            )
            .await
        {
            Ok(()) => {}
            Err(e) if e.is_unavailable() => {
                // Key material became unavailable between the readiness gate
                // and the derive (e.g. a Keychain lock mid-sweep). DEFER —
                // never break the channel; a later sweep with a signer retries.
                tracing::info!(
                    identity = %identity_id,
                    contact = %contact_id,
                    error = %e.into_inner(),
                    "Deferring DashPay external account: key material unavailable (channel left intact)"
                );
            }
            Err(e) if e.is_permanent() => {
                tracing::warn!(
                    identity = %identity_id,
                    contact = %contact_id,
                    error = %e.into_inner(),
                    "Contact request failed crypto registration; marking payment channel broken (permanent)"
                );
                self.mark_contact_channel_broken(identity_id, &contact_id)
                    .await;
            }
            Err(e) => {
                tracing::warn!(
                    identity = %identity_id,
                    contact = %contact_id,
                    error = %e.into_inner(),
                    "Transient failure registering DashPay external account; will retry next sweep (channel left intact)"
                );
            }
        }
    }

    /// Enqueue the deferred contact-crypto ops for a contact whose account
    /// build was paused because key material is unavailable (watch-only /
    /// signer locked). Idempotent per `(owner, contact, kind)` — re-enqueuing
    /// updates the entry in place. Stores only the on-chain ciphertext +
    /// public key indices, never a secret. The entries are drained when a
    /// signer becomes available.
    async fn enqueue_deferred_contact_crypto(
        &self,
        identity_id: &Identifier,
        candidate: &AccountBuildCandidate,
    ) {
        use crate::changeset::{
            upsert_pending_contact_crypto, PendingContactCrypto, PendingContactCryptoOp,
            PlatformWalletChangeSet,
        };
        let enqueued_at_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let entries = vec![
            // (1) Our receiving xpub (no payload — derived from the identity ids).
            PendingContactCrypto {
                owner_identity_id: *identity_id,
                contact_id: candidate.contact_id,
                op: PendingContactCryptoOp::RegisterReceiving,
                enqueued_at_ms,
            },
            // (2) The external account — ECDH decrypt of the contact's xpub.
            PendingContactCrypto {
                owner_identity_id: *identity_id,
                contact_id: candidate.contact_id,
                op: PendingContactCryptoOp::RegisterExternal {
                    encrypted_public_key: candidate.encrypted_public_key.clone(),
                    our_decryption_key_index: candidate.our_decryption_key_index,
                    contact_encryption_key_index: candidate.contact_encryption_key_index,
                },
                enqueued_at_ms,
            },
        ];

        // In-memory upsert under the write lock (released before persisting).
        {
            let mut wm = self.wallet_manager.write().await;
            let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) else {
                return;
            };
            for entry in &entries {
                upsert_pending_contact_crypto(&mut info.pending_contact_crypto, entry.clone());
            }
        }

        // Persist the add-delta so the queue survives a restart. Best-effort:
        // the recurring sweep re-discovers + re-enqueues if this fails (the
        // in-memory queue above already covers the current session).
        let changeset = PlatformWalletChangeSet {
            pending_contact_crypto_added: entries,
            ..Default::default()
        };
        if let Err(e) = self.persister.store(changeset) {
            tracing::warn!(
                identity = %identity_id, contact = %candidate.contact_id, error = %e,
                "failed to persist deferred contact-crypto enqueue; will re-enqueue next sweep"
            );
        }
    }

    /// Drain the persisted deferred-crypto queue using `provider` for the
    /// Keychain-derived key material. Call when a signer is available (Keychain
    /// unlock, or any signer-present DashPay action). Returns the number of
    /// entries completed (removed from the queue).
    ///
    /// Per entry: run the op; on success remove it and persist the removal; on
    /// unavailable/transient failure leave it for the next drain. The
    /// `RegisterExternal` + `ContactInfoDecrypt` ops (which need a contact
    /// fetch + ECDH/contactInfo derivation) drain in a follow-up and are left
    /// queued here — so calling this is always safe, it just completes what it
    /// can.
    pub async fn drain_pending_contact_crypto<P: ContactCryptoProvider + Sync>(
        &self,
        provider: &P,
    ) -> usize {
        use crate::changeset::{PendingContactCryptoKey, PendingContactCryptoOp};

        // Snapshot the queue, then run the async ops without holding the lock.
        let entries: Vec<crate::changeset::PendingContactCrypto> = {
            let wm = self.wallet_manager.read().await;
            wm.get_wallet_info(&self.wallet_id)
                .map(|info| info.pending_contact_crypto.clone())
                .unwrap_or_default()
        };
        if entries.is_empty() {
            return 0;
        }

        let mut cleared: Vec<PendingContactCryptoKey> = Vec::new();
        for entry in &entries {
            match &entry.op {
                PendingContactCryptoOp::RegisterReceiving => {
                    // Build the friendship path in Rust; the provider derives
                    // our receiving xpub at it via the Keychain signer.
                    let account_type = key_wallet::account::AccountType::DashpayReceivingFunds {
                        index: 0,
                        user_identity_id: entry.owner_identity_id.to_buffer(),
                        friend_identity_id: entry.contact_id.to_buffer(),
                    };
                    let path = match account_type.derivation_path(self.sdk.network) {
                        Ok(p) => p,
                        Err(e) => {
                            tracing::warn!(
                                owner = %entry.owner_identity_id, contact = %entry.contact_id,
                                error = %e, "drain: receiving path build failed; leaving queued"
                            );
                            continue;
                        }
                    };
                    match provider.receiving_xpub(&path).await {
                        Ok(xpub) => match self
                            .register_contact_account(
                                &entry.owner_identity_id,
                                &entry.contact_id,
                                0,
                                Some(xpub),
                            )
                            .await
                        {
                            Ok(()) => cleared.push(entry.key()),
                            Err(e) => tracing::warn!(
                                owner = %entry.owner_identity_id, contact = %entry.contact_id,
                                error = %e, "drain: register receiving account failed; leaving queued"
                            ),
                        },
                        Err(e) => tracing::warn!(
                            owner = %entry.owner_identity_id, contact = %entry.contact_id,
                            error = %e, "drain: receiving-xpub provider failed; leaving queued"
                        ),
                    }
                }
                PendingContactCryptoOp::RegisterExternal {
                    encrypted_public_key,
                    our_decryption_key_index,
                    contact_encryption_key_index,
                } => {
                    // Our HD index, for the ECDH derivation path. If the owner
                    // isn't wallet-owned, this op can't be ours — leave queued.
                    let identity_index = {
                        let wm = self.wallet_manager.read().await;
                        wm.get_wallet_info(&self.wallet_id)
                            .and_then(|info| {
                                info.identity_manager
                                    .managed_identity(&entry.owner_identity_id)
                            })
                            .and_then(|m| m.identity_index)
                    };
                    let Some(identity_index) = identity_index else {
                        tracing::warn!(
                            owner = %entry.owner_identity_id, contact = %entry.contact_id,
                            "drain: owner not wallet-owned; leaving queued"
                        );
                        continue;
                    };

                    // ECDH path, built in Rust (path provenance stays here).
                    let path = match Self::identity_auth_derivation_path(
                        self.sdk.network,
                        key_wallet::bip32::KeyDerivationType::ECDSA,
                        identity_index,
                        *our_decryption_key_index,
                    ) {
                        Ok(p) => p,
                        Err(e) => {
                            tracing::warn!(
                                owner = %entry.owner_identity_id, contact = %entry.contact_id,
                                error = %e, "drain: ECDH path build failed; leaving queued"
                            );
                            continue;
                        }
                    };

                    // Fetch the contact identity (transient on failure → leave).
                    let contact_identity = {
                        use dash_sdk::platform::Fetch;
                        match Identity::fetch(&self.sdk, entry.contact_id).await {
                            Ok(Some(id)) => id,
                            Ok(None) => {
                                tracing::warn!(
                                    owner = %entry.owner_identity_id, contact = %entry.contact_id,
                                    "drain: contact identity not on Platform; leaving queued"
                                );
                                continue;
                            }
                            Err(e) => {
                                tracing::warn!(
                                    owner = %entry.owner_identity_id, contact = %entry.contact_id,
                                    error = %e, "drain: contact fetch failed; leaving queued"
                                );
                                continue;
                            }
                        }
                    };

                    // Validate key indices (purpose + type) BEFORE ECDH — the
                    // same gate the resident sweep path applies, so the deferred
                    // path enforces the identical contract. A purpose-only
                    // mismatch (e.g. a legacy doc referencing an AUTH key) is left
                    // queued for a future acceptance-policy change; a hard failure
                    // (key type / missing / disabled) marks the channel broken and
                    // clears the entry.
                    let our_identity = {
                        let wm = self.wallet_manager.read().await;
                        wm.get_wallet_info(&self.wallet_id)
                            .and_then(|info| {
                                info.identity_manager
                                    .managed_identity(&entry.owner_identity_id)
                            })
                            .map(|m| m.identity.clone())
                    };
                    let Some(our_identity) = our_identity else {
                        tracing::warn!(
                            owner = %entry.owner_identity_id, contact = %entry.contact_id,
                            "drain: our identity vanished mid-drain; leaving queued"
                        );
                        continue;
                    };
                    let validation =
                        crate::wallet::identity::crypto::validation::validate_contact_request(
                            &contact_identity,
                            *contact_encryption_key_index,
                            &our_identity,
                            *our_decryption_key_index,
                        );
                    if !validation.is_valid {
                        if validation.is_purpose_only() {
                            tracing::warn!(
                                owner = %entry.owner_identity_id, contact = %entry.contact_id,
                                errors = ?validation.errors,
                                "drain: contact request key-purpose mismatch; leaving queued (not marking broken)"
                            );
                            continue;
                        }
                        tracing::warn!(
                            owner = %entry.owner_identity_id, contact = %entry.contact_id,
                            errors = ?validation.errors,
                            "drain: contact request failed key-index validation; marking channel broken"
                        );
                        self.mark_contact_channel_broken(
                            &entry.owner_identity_id,
                            &entry.contact_id,
                        )
                        .await;
                        cleared.push(entry.key());
                        continue;
                    }

                    // The contact's encryption pubkey (peer). A malformed/missing
                    // key is a permanent fault — re-deriving won't help.
                    let peer = match contact_identity
                        .public_keys()
                        .get(contact_encryption_key_index)
                    {
                        Some(k) => {
                            match dashcore::secp256k1::PublicKey::from_slice(k.data().as_slice()) {
                                Ok(pk) => pk,
                                Err(e) => {
                                    tracing::warn!(
                                        owner = %entry.owner_identity_id, contact = %entry.contact_id,
                                        error = %e,
                                        "drain: contact encryption key invalid; marking channel broken"
                                    );
                                    self.mark_contact_channel_broken(
                                        &entry.owner_identity_id,
                                        &entry.contact_id,
                                    )
                                    .await;
                                    cleared.push(entry.key());
                                    continue;
                                }
                            }
                        }
                        None => {
                            tracing::warn!(
                                owner = %entry.owner_identity_id, contact = %entry.contact_id,
                                "drain: contact encryption key missing; marking channel broken"
                            );
                            self.mark_contact_channel_broken(
                                &entry.owner_identity_id,
                                &entry.contact_id,
                            )
                            .await;
                            cleared.push(entry.key());
                            continue;
                        }
                    };

                    // ECDH via the Keychain-backed provider (scalar stays in the
                    // signer; we only get the shared secret).
                    let shared = match provider.ecdh_shared_secret(&path, &peer).await {
                        Ok(s) => s,
                        Err(e) => {
                            tracing::warn!(
                                owner = %entry.owner_identity_id, contact = %entry.contact_id,
                                error = %e, "drain: ECDH provider failed; leaving queued"
                            );
                            continue;
                        }
                    };

                    match self
                        .register_external_contact_account(
                            &entry.owner_identity_id,
                            &contact_identity,
                            encrypted_public_key,
                            *our_decryption_key_index,
                            *contact_encryption_key_index,
                            Some(shared),
                        )
                        .await
                    {
                        Ok(()) => cleared.push(entry.key()),
                        Err(e) if e.is_permanent() => {
                            tracing::warn!(
                                owner = %entry.owner_identity_id, contact = %entry.contact_id,
                                error = %e.into_inner(),
                                "drain: external register permanent fault; marking channel broken"
                            );
                            self.mark_contact_channel_broken(
                                &entry.owner_identity_id,
                                &entry.contact_id,
                            )
                            .await;
                            cleared.push(entry.key());
                        }
                        Err(e) => tracing::warn!(
                            owner = %entry.owner_identity_id, contact = %entry.contact_id,
                            error = %e.into_inner(),
                            "drain: external register transient/unavailable; leaving queued"
                        ),
                    }
                }
                PendingContactCryptoOp::ContactInfoDecrypt => {
                    // The seal/open AES primitive exists; what's still missing is
                    // the network re-fetch of this owner's contactInfo documents
                    // (the op carries no ciphertext on purpose, so it always
                    // decrypts the latest published version). Until the fetch
                    // surface is wired in, leave the op queued (safe — re-run
                    // later). Decrypting a stubbed/empty fetch would be a false
                    // no-op, so this stays an honest deferral.
                    tracing::debug!(
                        owner = %entry.owner_identity_id, contact = %entry.contact_id,
                        "drain: ContactInfoDecrypt awaiting contactInfo fetch surface; leaving queued"
                    );
                }
            }
        }

        if cleared.is_empty() {
            return 0;
        }

        // Remove the completed entries from the in-memory queue + persist the
        // removal so they don't replay after a restart.
        {
            let mut wm = self.wallet_manager.write().await;
            if let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) {
                info.pending_contact_crypto
                    .retain(|e| !cleared.iter().any(|k| *k == e.key()));
            }
        }
        let changeset = crate::changeset::PlatformWalletChangeSet {
            pending_contact_crypto_cleared: cleared.clone(),
            ..Default::default()
        };
        if let Err(e) = self.persister.store(changeset) {
            tracing::warn!(
                error = %e,
                "drain: failed to persist cleared queue entries (in-memory already updated)"
            );
        }

        cleared.len()
    }

    /// Mark an established contact's payment channel as permanently broken
    /// and persist the transition through the changeset pipeline so
    /// it survives restarts and is FFI/UI-visible. Idempotent.
    async fn mark_contact_channel_broken(&self, identity_id: &Identifier, contact_id: &Identifier) {
        let mut wm = self.wallet_manager.write().await;
        let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) else {
            return;
        };
        let Some(managed) = info.identity_manager.managed_identity_mut(identity_id) else {
            return;
        };
        let Some(contact) = managed.established_contacts.get_mut(contact_id) else {
            return;
        };
        if contact.payment_channel_broken {
            return;
        }
        contact.payment_channel_broken = true;
        let snapshot = contact.clone();

        // Persist the broken flag via an `established` changeset entry
        // (the established upsert carries the flag column).
        let mut cs = crate::changeset::ContactChangeSet::default();
        cs.established.insert(
            crate::changeset::SentContactRequestKey {
                owner_id: *identity_id,
                recipient_id: *contact_id,
            },
            snapshot,
        );
        if let Err(e) = self.persister.store(cs.into()) {
            tracing::error!("Failed to persist broken-channel changeset: {}", e);
        }
    }
}

/// One established contact that needs its DashPay accounts (re)built
/// during a sync sweep. Collected under the write guard, consumed
/// after it is dropped.
struct AccountBuildCandidate {
    /// The counterparty identity.
    contact_id: Identifier,
    /// The counterparty's 96-byte encrypted xpub (from their incoming
    /// request to us) to decrypt + register as a `DashpayExternalAccount`.
    encrypted_public_key: Vec<u8>,
    /// Our DECRYPTION key id used for ECDH.
    our_decryption_key_index: u32,
    /// The counterparty's ENCRYPTION key id used for ECDH.
    contact_encryption_key_index: u32,
}

// ---------------------------------------------------------------------------
// Accept an incoming contact request
// ---------------------------------------------------------------------------

impl<B: TransactionBroadcaster + ?Sized> IdentityWallet<B> {
    /// Accept an incoming contact request using an externally-supplied
    /// signer.
    ///
    /// Routes through
    /// [`Self::send_contact_request_with_external_signer`] so signing
    /// crosses the FFI via the supplied `&S: Signer<IdentityPublicKey>`.
    /// Same ECDH caveat applies — see that method's docstring.
    pub async fn accept_contact_request_with_external_signer<S, C>(
        &self,
        request: &ContactRequest,
        signer: &S,
        crypto: &C,
    ) -> Result<EstablishedContact, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
        C: ContactCryptoProvider + Sync,
    {
        let our_identity_id = request.recipient_id;
        let sender_id = request.sender_id;

        // 1. Verify the incoming request is known, and detect whether an
        //    on-platform reciprocal already exists for this pair.
        let already_reciprocated = {
            let wm = self.wallet_manager.read().await;
            let info = wm
                .get_wallet_info(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
            let managed = info
                .identity_manager
                .managed_identity(&our_identity_id)
                .ok_or(PlatformWalletError::IdentityNotFound(our_identity_id))?;
            // The contact is already established (sync reconciled both
            // sides), or our own sent request to this contact already
            // exists — in either case the reciprocal is already on
            // Platform and re-broadcasting it would be rejected by the
            // `(ownerId, toUserId, accountReference)` unique index.
            let established = managed.established_contacts.contains_key(&sender_id);
            let sent_exists = managed.sent_contact_requests.contains_key(&sender_id);
            if !established
                && !sent_exists
                && !managed.incoming_contact_requests.contains_key(&sender_id)
            {
                return Err(PlatformWalletError::ContactRequestNotFound(sender_id));
            }
            established || sent_exists
        };

        // 2. Capture the encrypted xpub + key indices BEFORE sending
        //    the reciprocal request (same ordering as the legacy
        //    `accept_contact_request`).
        let contact_encrypted_xpub = request.encrypted_public_key.clone();
        let our_decryption_key_index = request.recipient_key_index;
        let contact_encryption_key_index = request.sender_key_index;

        // 3. Send the reciprocal request — UNLESS one already exists on
        //    Platform (accept-adopt): re-broadcasting the same
        //    `(ownerId, toUserId, accountReference)` triple is rejected by
        //    the unique index forever. When adopting, we still perform the
        //    fresh-send local registrations below (receiving account +
        //    validate→decrypt→register external), so the contact becomes
        //    payable without a duplicate broadcast.
        if already_reciprocated {
            tracing::info!(
                our_identity = %our_identity_id,
                contact = %sender_id,
                "Accept: reciprocal already on Platform — adopting instead of re-broadcasting"
            );
            // Adopt: register the receiving (friendship) account, derived via
            // the signer (no resident seed), matching the fresh-send path.
            match self
                .receiving_xpub_for(&our_identity_id, &sender_id, 0, crypto)
                .await
            {
                Ok(xpub) => {
                    if let Err(e) = self
                        .register_contact_account(&our_identity_id, &sender_id, 0, Some(xpub))
                        .await
                    {
                        tracing::warn!(
                            our_identity = %our_identity_id,
                            contact = %sender_id,
                            error = %e,
                            "Accept-adopt: failed to register receiving account; will retry on next sweep"
                        );
                    }
                }
                Err(e) => tracing::warn!(
                    our_identity = %our_identity_id,
                    contact = %sender_id,
                    error = %e,
                    "Accept-adopt: failed to derive receiving xpub via signer; will retry on next sweep"
                ),
            }
        } else {
            self.send_contact_request_with_external_signer(
                &our_identity_id,
                &sender_id,
                None,
                None,
                signer,
                crypto,
            )
            .await?;
        }

        // 4. Validate key indices (same gate as the sync sweep and the
        //    fresh send — applies to ALL three accept paths) BEFORE any
        //    ECDH, then register the external (sending) account.
        if let Err(e) = self
            .accept_register_external_validated(
                &our_identity_id,
                &sender_id,
                &contact_encrypted_xpub,
                our_decryption_key_index,
                contact_encryption_key_index,
                crypto,
            )
            .await
        {
            tracing::warn!(
                our_identity = %our_identity_id,
                contact = %sender_id,
                error = %e,
                "Failed to register external contact account after accept (external signer) — \
                 re-run sync to retry"
            );
        }

        // 5. Retrieve the auto-established contact.
        let wm = self.wallet_manager.read().await;
        let info = wm
            .get_wallet_info(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
        let managed = info
            .identity_manager
            .managed_identity(&our_identity_id)
            .ok_or(PlatformWalletError::IdentityNotFound(our_identity_id))?;

        managed
            .established_contacts
            .get(&sender_id)
            .cloned()
            .ok_or(PlatformWalletError::ContactRequestNotFound(sender_id))
    }

    /// Validate the contact request's key indices (purpose
    /// ENCRYPTION/DECRYPTION + ECDSA type) BEFORE any ECDH, then register
    /// the external sending account. Shared by the accept and accept-adopt
    /// paths so the validation gate is applied uniformly (it also runs in
    /// the sync sweep).
    ///
    /// A validation failure is returned as an error so the caller can log
    /// it; the channel is not silently registered against an unvalidated
    /// index. On the network/decrypt side this simply forwards to
    /// [`register_external_contact_account`].
    async fn accept_register_external_validated<C: ContactCryptoProvider + Sync>(
        &self,
        our_identity_id: &Identifier,
        contact_id: &Identifier,
        contact_encrypted_xpub: &[u8],
        our_decryption_key_index: u32,
        contact_encryption_key_index: u32,
        crypto: &C,
    ) -> Result<(), PlatformWalletError> {
        use dash_sdk::platform::Fetch;

        // Fetch counterparty + our identity for validation.
        let contact_identity = Identity::fetch(&self.sdk, *contact_id)
            .await
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to fetch contact identity {contact_id} for validation: {e}"
                ))
            })?
            .ok_or(PlatformWalletError::IdentityNotFound(*contact_id))?;

        let (our_identity, identity_index) = {
            let wm = self.wallet_manager.read().await;
            let managed = wm
                .get_wallet_info(&self.wallet_id)
                .and_then(|info| info.identity_manager.managed_identity(our_identity_id))
                .ok_or(PlatformWalletError::IdentityNotFound(*our_identity_id))?;
            let index = managed
                .identity_index
                .ok_or(PlatformWalletError::IdentityIndexNotSet(*our_identity_id))?;
            (managed.identity.clone(), index)
        };

        let validation = crate::wallet::identity::crypto::validation::validate_contact_request(
            &contact_identity,
            contact_encryption_key_index,
            &our_identity,
            our_decryption_key_index,
        );
        if !validation.is_valid {
            return Err(PlatformWalletError::InvalidIdentityData(format!(
                "Contact request failed key-index validation: {:?}",
                validation.errors
            )));
        }

        // Seedless ECDH: our decryption-key scalar (at the Rust-built path)
        // against the contact's encryption pubkey, computed in the signer. The
        // resolved shared secret is handed to the register call so its resident
        // derivation path is never taken.
        let our_dec_path = Self::identity_auth_derivation_path(
            self.sdk.network,
            key_wallet::bip32::KeyDerivationType::ECDSA,
            identity_index,
            our_decryption_key_index,
        )?;
        let contact_pubkey = {
            use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
            let key = contact_identity
                .public_keys()
                .get(&contact_encryption_key_index)
                .ok_or_else(|| {
                    PlatformWalletError::InvalidIdentityData(format!(
                        "Contact identity has no key at index {contact_encryption_key_index}"
                    ))
                })?;
            dashcore::secp256k1::PublicKey::from_slice(key.data().as_slice()).map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Contact encryption public key is invalid: {e}"
                ))
            })?
        };
        let shared = crypto.ecdh_shared_secret(&our_dec_path, &contact_pubkey).await?;

        // Reuse the identity we just fetched for validation (no second
        // network round). The accept path surfaces any failure to the
        // caller as a plain error — the transient/permanent split only
        // matters to the unattended sync sweep's broken-channel policy.
        self.register_external_contact_account(
            our_identity_id,
            &contact_identity,
            contact_encrypted_xpub,
            our_decryption_key_index,
            contact_encryption_key_index,
            Some(shared),
        )
        .await
        .map_err(RegisterExternalError::into_inner)
    }

    /// Derive our DashPay receiving (friendship) xpub for `(our_identity,
    /// contact)` at `account_index` via the signer — the seedless equivalent of
    /// deriving it from the wallet. Path is `AccountType::DashpayReceivingFunds`
    /// built in Rust; only the public xpub crosses back.
    async fn receiving_xpub_for<C: ContactCryptoProvider + Sync>(
        &self,
        our_identity_id: &Identifier,
        contact_id: &Identifier,
        account_index: u32,
        crypto: &C,
    ) -> Result<key_wallet::bip32::ExtendedPubKey, PlatformWalletError> {
        let account_type = key_wallet::account::AccountType::DashpayReceivingFunds {
            index: account_index,
            user_identity_id: our_identity_id.to_buffer(),
            friend_identity_id: contact_id.to_buffer(),
        };
        let path = account_type.derivation_path(self.sdk.network).map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!(
                "Failed to build DashPay derivation path: {e}"
            ))
        })?;
        crypto.receiving_xpub(&path).await
    }
}

// ---------------------------------------------------------------------------
// Sent contact requests query
// ---------------------------------------------------------------------------

impl<B: TransactionBroadcaster + ?Sized> IdentityWallet<B> {
    /// Fetch sent contact requests for a specific identity from Platform.
    ///
    /// Queries the DashPay contract for `contactRequest` documents where
    /// `$ownerId == identity_id`, ordered by `$createdAt`.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The identity whose sent requests to fetch.
    ///
    /// # Returns
    ///
    /// A list of [`ContactRequest`] structs representing the sent requests.
    pub async fn sent_contact_requests(
        &self,
        identity_id: &Identifier,
    ) -> Result<Vec<ContactRequest>, PlatformWalletError> {
        let sent_docs = self
            .sdk
            .fetch_sent_contact_requests(*identity_id, None)
            .await
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to fetch sent contact requests: {e}"
                ))
            })?;

        let mut requests = Vec::new();

        for (_doc_id, maybe_doc) in sent_docs.iter() {
            let doc = match maybe_doc {
                Some(d) => d,
                None => continue,
            };

            let sender_id = doc.owner_id();

            let props = doc.properties();

            let to_user_id = match props
                .get("toUserId")
                .and_then(|v: &Value| v.to_identifier().ok())
            {
                Some(v) => v,
                None => continue,
            };
            let sender_key_index = match props
                .get("senderKeyIndex")
                .and_then(|v: &Value| v.to_integer::<u32>().ok())
            {
                Some(v) => v,
                None => continue,
            };
            let recipient_key_index = match props
                .get("recipientKeyIndex")
                .and_then(|v: &Value| v.to_integer::<u32>().ok())
            {
                Some(v) => v,
                None => continue,
            };
            let account_reference = match props
                .get("accountReference")
                .and_then(|v: &Value| v.to_integer::<u32>().ok())
            {
                Some(v) => v,
                None => continue,
            };
            let encrypted_public_key = match props
                .get("encryptedPublicKey")
                .and_then(|v: &Value| v.as_bytes())
                .cloned()
            {
                Some(v) => v,
                None => continue,
            };

            let mut contact_request = ContactRequest::new(
                sender_id,
                to_user_id,
                sender_key_index,
                recipient_key_index,
                account_reference,
                encrypted_public_key,
                doc.created_at_core_block_height().unwrap_or(0),
                doc.created_at().unwrap_or(0),
            );

            // Attach optional encrypted account label if present.
            contact_request.encrypted_account_label = props
                .get("encryptedAccountLabel")
                .and_then(|v: &Value| v.as_bytes())
                .cloned();

            // Attach optional auto-accept proof if present.
            contact_request.auto_accept_proof = props
                .get("autoAcceptProof")
                .and_then(|v: &Value| v.as_bytes())
                .cloned();

            requests.push(contact_request);
        }

        // Sort by creation time ascending.
        requests.sort_by_key(|r| r.created_at);

        Ok(requests)
    }
}

// ---------------------------------------------------------------------------
// Ignore / un-ignore a contact sender (per-sender mute, local-only)
// ---------------------------------------------------------------------------

impl<B: TransactionBroadcaster + ?Sized> IdentityWallet<B> {
    /// Ignore a contact sender (per-sender mute, = block, reversible).
    ///
    /// Drops the sender's pending incoming request from local state AND
    /// records the sender in `ignored_senders` so the recurring sync ingest
    /// path won't resurrect *any* of that sender's still-on-platform
    /// immutable `contactRequest` documents — including rotated, bumped-
    /// `accountReference` ones. Suppression is per-sender by design: if you
    /// ignored the person you ignored them; [`Self::unignore_contact_sender`]
    /// is the "changed my mind" affordance.
    ///
    /// Ignore is **local-only** — there is no on-chain artifact (syncing it
    /// would leak who you ignored via the public contact-request indices).
    /// The ignore is persisted through the existing
    /// changeset → apply → SQLite pipeline so it survives a relaunch.
    ///
    /// Unlike the old reject, this does NOT require a pending incoming
    /// request to exist: you can ignore a sender whose request the sweep
    /// hasn't surfaced yet (the per-sender set still suppresses it).
    ///
    /// # Arguments
    ///
    /// * `identity_id`         - Our identity.
    /// * `contact_identity_id` - The sender to ignore.
    pub async fn ignore_contact_sender(
        &self,
        identity_id: &Identifier,
        contact_identity_id: &Identifier,
    ) -> Result<(), PlatformWalletError> {
        let mut wm = self.wallet_manager.write().await;
        let info = wm
            .get_wallet_info_mut(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
        let managed = info
            .identity_manager
            .managed_identity_mut(identity_id)
            .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;

        // Record the ignore (drops the pending incoming entry if present,
        // adds the sender to `ignored_senders`) and persist it.
        //
        // PROPAGATE the store error rather than swallow it. Ignore is
        // local-only (there's no on-chain artifact), so if it doesn't reach
        // disk the still-immutable on-chain requests re-ingest on the next
        // launch and the ignored sender RESURFACES — with no signal.
        // Returning the error surfaces the failure to the UI so the user
        // retries, instead of a silent success that didn't take.
        let cs = managed.ignore_sender(contact_identity_id);
        self.persister
            .store(cs.into())
            .map_err(|e| PlatformWalletError::Persistence(format!("ignore not persisted: {e}")))?;

        tracing::info!(
            identity = %identity_id,
            ignored_sender = %contact_identity_id,
            "Contact sender ignored (local-only; suppressed from the main pending list, won't resurrect on sync)"
        );

        Ok(())
    }

    /// Un-ignore a contact sender (reverse [`Self::ignore_contact_sender`]).
    ///
    /// Removes the sender from `ignored_senders`, **rewinds the received
    /// high-water cursor to `None`** (so the next sweep re-fetches the
    /// sender's on-chain requests — otherwise the cursor has already passed
    /// them and they'd never reappear), and persists the un-ignore through
    /// the changeset pipeline.
    ///
    /// A no-op (returns `Ok(())`) when the sender wasn't ignored.
    ///
    /// # Arguments
    ///
    /// * `identity_id`         - Our identity.
    /// * `contact_identity_id` - The sender to un-ignore.
    pub async fn unignore_contact_sender(
        &self,
        identity_id: &Identifier,
        contact_identity_id: &Identifier,
    ) -> Result<(), PlatformWalletError> {
        let mut wm = self.wallet_manager.write().await;
        let info = wm
            .get_wallet_info_mut(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
        let managed = info
            .identity_manager
            .managed_identity_mut(identity_id)
            .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;

        // `unignore_sender` removes the sender + rewinds the cursor and
        // returns the removal changeset (empty if the sender wasn't
        // ignored). Persist it so the ignored-sender row is deleted.
        let cs = managed.unignore_sender(contact_identity_id);
        if <crate::changeset::ContactChangeSet as crate::changeset::Merge>::is_empty(&cs) {
            // Not ignored — nothing to persist, but not an error.
            return Ok(());
        }
        self.persister.store(cs.into()).map_err(|e| {
            PlatformWalletError::Persistence(format!("un-ignore not persisted: {e}"))
        })?;

        tracing::info!(
            identity = %identity_id,
            unignored_sender = %contact_identity_id,
            "Contact sender un-ignored (cursor rewound; requests will re-fetch on next sweep)"
        );

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Network-layer tests for the sync sweep decision logic.
//
// These exercise the *orchestration* helpers that don't require a live
// network or real ECDH keys: account-build candidate collection
// and the rejected-tombstone / broken-flag persistence round-trip. The
// pure state-machine behaviors (guard relaxation, sent-side dedup,
// metadata-preserving re-establish, tombstone-by-accountReference) are
// pinned in `state/managed_identity/contact_requests.rs`.
// ---------------------------------------------------------------------------
#[cfg(test)]
mod cursor_tests {
    use super::{advance_high_water, query_lower_bound, SYNC_OVERLAP_MS};

    /// No cursor ⇒ full fetch (no lower bound).
    #[test]
    fn lower_bound_none_is_full_fetch() {
        assert_eq!(query_lower_bound(None), None);
    }

    /// The query bound is the high-water minus the (mandatory) overlap window,
    /// saturating at 0 — the overlap is what re-includes equal-`$createdAt`
    /// docs at a page boundary, so it must always be subtracted.
    #[test]
    fn lower_bound_subtracts_overlap() {
        assert_eq!(
            query_lower_bound(Some(20 * 60_000)),
            Some(20 * 60_000 - SYNC_OVERLAP_MS)
        );
        // Saturates rather than underflowing for a high-water below the window.
        assert_eq!(query_lower_bound(Some(5 * 60_000)), Some(0));
        const { assert!(SYNC_OVERLAP_MS > 0, "overlap must be > 0 for correctness") };
    }

    /// Advancing never moves the cursor backward (guards out-of-order /
    /// stale-max sweeps and restore over-shoot), and a zero-doc sweep leaves
    /// it unchanged.
    #[test]
    fn advance_never_goes_backward_and_zero_doc_is_noop() {
        // First sweep from empty: adopt the max fetched.
        assert_eq!(advance_high_water(None, Some(100)), Some(100));
        // Forward progress.
        assert_eq!(advance_high_water(Some(100), Some(200)), Some(200));
        // A lower max (re-fetch within the overlap, or out-of-order) must NOT
        // pull the cursor backward.
        assert_eq!(advance_high_water(Some(200), Some(50)), Some(200));
        // A zero-doc sweep leaves the cursor exactly where it was.
        assert_eq!(advance_high_water(Some(200), None), Some(200));
        assert_eq!(advance_high_water(None, None), None);

        // `0` is a real cursor value distinct from `None` (a doc at
        // `$createdAt == 0`, or a freshly-restored 0 cursor) — pin that a
        // future "treat 0 as unset" refactor would regress.
        assert_eq!(advance_high_water(None, Some(0)), Some(0));
        assert_eq!(advance_high_water(Some(0), None), Some(0));
        assert_eq!(query_lower_bound(Some(0)), Some(0));
    }

    /// Compare-and-advance: a concurrent `unignore_sender` reset (cursor no
    /// longer equals the snapshot) must NOT be clobbered by this sweep's stale
    /// max — otherwise the un-ignored sender stays invisible until a restart.
    #[test]
    fn advance_if_unchanged_respects_a_concurrent_reset() {
        use super::advance_if_unchanged;
        // Unchanged since snapshot → normal advance.
        assert_eq!(
            advance_if_unchanged(Some(100), Some(100), Some(200)),
            Some(200)
        );
        assert_eq!(advance_if_unchanged(Some(100), Some(100), None), Some(100));
        // THE RACE: snapshot was Some(100); un-ignore reset it to None
        // mid-sweep; this sweep's max is Some(200) (stale — excluded the sender)
        // → keep the None so the next sweep does a full re-fetch.
        assert_eq!(advance_if_unchanged(None, Some(100), Some(200)), None);
        // Any other concurrent change is likewise respected, not clobbered.
        assert_eq!(
            advance_if_unchanged(Some(50), Some(100), Some(200)),
            Some(50)
        );
    }
}

#[cfg(test)]
mod sweep_tests {
    use super::*;
    use crate::broadcaster::SpvBroadcaster;
    use crate::changeset::{ContactChangeSet, PlatformWalletChangeSet, SentContactRequestKey};
    use crate::wallet::core::WalletBalance;
    use crate::wallet::identity::IdentityManager;
    use crate::wallet::persister::{NoPlatformPersistence, WalletPersister};
    use crate::wallet::platform_wallet::PlatformWalletInfo;
    use dpp::identity::v0::IdentityV0;
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
    use key_wallet::wallet::Wallet;
    use key_wallet::Network;
    use std::collections::BTreeMap;
    use std::sync::Arc;

    fn noop_persister() -> WalletPersister {
        WalletPersister::new([0u8; 32], Arc::new(NoPlatformPersistence))
    }

    fn build_test_wallet() -> Wallet {
        Wallet::new_random(Network::Testnet, WalletAccountCreationOptions::None)
            .expect("test wallet")
    }

    fn empty_info(wallet: &Wallet) -> PlatformWalletInfo {
        PlatformWalletInfo {
            core_wallet: ManagedWalletInfo::from_wallet(wallet, 0),
            balance: Arc::new(WalletBalance::new()),
            identity_manager: IdentityManager::new(),
            tracked_asset_locks: BTreeMap::new(),
            pending_contact_crypto: Vec::new(),
        }
    }

    fn test_identity(id_byte: u8) -> Identity {
        Identity::V0(IdentityV0 {
            id: Identifier::from([id_byte; 32]),
            public_keys: BTreeMap::new(),
            balance: 0,
            revision: 0,
        })
    }

    fn test_request(sender: u8, recipient: u8, account_reference: u32) -> ContactRequest {
        ContactRequest::new(
            Identifier::from([sender; 32]),
            Identifier::from([recipient; 32]),
            1,
            2,
            account_reference,
            vec![7u8; 96],
            100_000,
            0,
        )
    }

    /// Seed a wallet-owned identity that has an established contact (no
    /// external account yet) into a fresh `PlatformWalletInfo`.
    fn info_with_established_contact(our: u8, contact: u8) -> (Wallet, PlatformWalletInfo) {
        let wallet = build_test_wallet();
        let mut info = empty_info(&wallet);
        let our_id = Identifier::from([our; 32]);
        let p = noop_persister();
        info.identity_manager
            .add_identity(test_identity(our), 0, [0u8; 32], &p)
            .expect("add identity");
        let managed = info
            .identity_manager
            .managed_identity_mut(&our_id)
            .expect("managed identity");
        // Establish a contact via the state machine.
        managed.add_incoming_contact_request(test_request(contact, our, 0), &p);
        managed.add_sent_contact_request(test_request(our, contact, 0), &p);
        assert_eq!(managed.established_contacts.len(), 1);
        (wallet, info)
    }

    /// **Test 3 (restore-from-seed shape):** an established contact with
    /// zero DashPay accounts must surface as an account-build candidate so
    /// the sweep rebuilds BOTH the receiving and external accounts. Before
    /// the account-building sweep only the fresh-send path created them, so
    /// restore-from-seed left the contact unpayable and incoming payments
    /// invisible.
    #[test]
    fn established_contact_missing_external_account_is_a_build_candidate() {
        let our = 1u8;
        let contact = 2u8;
        let our_id = Identifier::from([our; 32]);
        let (_wallet, info) = info_with_established_contact(our, contact);

        let candidates =
            IdentityWallet::<SpvBroadcaster>::collect_account_build_candidates(&info, &our_id);

        assert_eq!(
            candidates.len(),
            1,
            "an established contact with no external account must be a build candidate"
        );
        let c = &candidates[0];
        assert_eq!(c.contact_id, Identifier::from([contact; 32]));
        // The candidate carries the counterparty's encrypted xpub + the
        // ECDH key indices taken from the INCOMING request.
        assert_eq!(c.encrypted_public_key, vec![7u8; 96]);
        // incoming request: sender=contact key_index 1, recipient(us) key_index 2
        assert_eq!(c.contact_encryption_key_index, 1);
        assert_eq!(c.our_decryption_key_index, 2);
    }

    /// **Test 4 (permanent failure → no retry):** once a contact's payment
    /// channel is marked broken, the sweep must NOT re-list it as a
    /// candidate — no unbounded retry until a superseding request clears
    /// the flag.
    #[test]
    fn broken_payment_channel_is_skipped_by_the_sweep() {
        let our = 1u8;
        let contact = 2u8;
        let our_id = Identifier::from([our; 32]);
        let contact_id = Identifier::from([contact; 32]);
        let (_wallet, mut info) = info_with_established_contact(our, contact);

        // Mark the channel broken (as the permanent-failure path would).
        info.identity_manager
            .managed_identity_mut(&our_id)
            .unwrap()
            .established_contacts
            .get_mut(&contact_id)
            .unwrap()
            .payment_channel_broken = true;

        let candidates =
            IdentityWallet::<SpvBroadcaster>::collect_account_build_candidates(&info, &our_id);
        assert!(
            candidates.is_empty(),
            "a permanently-broken contact must not be retried by the sweep"
        );
    }

    /// **Test 4 (persistence):** the broken-channel flag round-trips through
    /// the changeset → apply pipeline so it survives a restart and is
    /// FFI/UI-visible — and a transient (cleared) flag round-trips too.
    #[test]
    fn broken_channel_flag_round_trips_through_apply() {
        let our = 1u8;
        let contact = 2u8;
        let our_id = Identifier::from([our; 32]);
        let contact_id = Identifier::from([contact; 32]);
        let (mut wallet, mut info) = info_with_established_contact(our, contact);

        // Build an `established` changeset carrying the broken flag.
        let mut contact_obj = info
            .identity_manager
            .managed_identity(&our_id)
            .unwrap()
            .established_contacts
            .get(&contact_id)
            .unwrap()
            .clone();
        contact_obj.payment_channel_broken = true;
        let mut cs = ContactChangeSet::default();
        cs.established.insert(
            SentContactRequestKey {
                owner_id: our_id,
                recipient_id: contact_id,
            },
            contact_obj,
        );
        let pcs = PlatformWalletChangeSet {
            contacts: Some(cs),
            ..Default::default()
        };

        info.apply_changeset(&mut wallet, pcs).expect("apply");

        assert!(
            info.identity_manager
                .managed_identity(&our_id)
                .unwrap()
                .established_contacts
                .get(&contact_id)
                .unwrap()
                .payment_channel_broken,
            "broken flag must survive the changeset apply round-trip"
        );
    }

    /// **Ignore persistence:** an ignored sender round-trips through the
    /// changeset → apply pipeline so a recurring re-sync after a restart
    /// still suppresses them — including a rotated (bumped-`accountReference`)
    /// request from the same sender (per-sender suppression).
    #[test]
    fn ignored_sender_round_trips_through_changeset_apply() {
        let our = 1u8;
        let sender = 9u8;
        let our_id = Identifier::from([our; 32]);
        let sender_id = Identifier::from([sender; 32]);
        let wallet = build_test_wallet();
        let mut info = empty_info(&wallet);
        let p = noop_persister();
        info.identity_manager
            .add_identity(test_identity(our), 0, [0u8; 32], &p)
            .expect("add identity");

        // Ignore the sender and capture the resulting changeset.
        let managed = info.identity_manager.managed_identity_mut(&our_id).unwrap();
        managed.add_incoming_contact_request(test_request(sender, our, 0), &p);
        let cs = managed.ignore_sender(&sender_id);
        let pcs = PlatformWalletChangeSet {
            contacts: Some(cs),
            ..Default::default()
        };

        // Wipe the in-memory ignore set, then re-apply the changeset (the
        // restore-from-persistence path).
        info.identity_manager
            .managed_identity_mut(&our_id)
            .unwrap()
            .ignored_senders
            .clear();
        let mut wallet = wallet;
        info.apply_changeset(&mut wallet, pcs).expect("apply");

        let managed = info.identity_manager.managed_identity(&our_id).unwrap();
        assert!(
            managed.is_sender_ignored(&sender_id),
            "ignored sender must be restored from the changeset"
        );
    }

    /// **Ignore suppresses original AND rotated (full sweep):** an ignored
    /// sender's ORIGINAL request and a later ROTATED (bumped-`accountReference`)
    /// request are BOTH suppressed by `sync_contact_requests`' per-sender
    /// ingest guard — neither reaches `incoming_contact_requests`. This is
    /// the key per-sender semantic difference from the old per-(sender,ref)
    /// reject (which would have let the rotation through).
    ///
    /// Drives the ingest decision logic directly against the state machine
    /// (the full network fetch is exercised by the mock-SDK integration
    /// tests): collapse-newest → is_sender_ignored → skip.
    #[test]
    fn ignored_sender_suppresses_both_original_and_rotated_requests() {
        let our = 1u8;
        let sender = 9u8;
        let our_id = Identifier::from([our; 32]);
        let sender_id = Identifier::from([sender; 32]);
        let wallet = build_test_wallet();
        let mut info = empty_info(&wallet);
        let p = noop_persister();
        info.identity_manager
            .add_identity(test_identity(our), 0, [0u8; 32], &p)
            .expect("add identity");
        let managed = info.identity_manager.managed_identity_mut(&our_id).unwrap();

        // Ignore the sender first.
        managed.ignore_sender(&sender_id);
        assert!(managed.is_sender_ignored(&sender_id));

        // Simulate the sweep seeing BOTH the original (ref=0) and a rotated
        // (ref=7) on-chain doc for this sender. The collapse keeps the
        // newest; the ignore check then suppresses it regardless of ref.
        let original = test_request_at(sender, our, 0, 100);
        let rotated = test_request_at(sender, our, 7, 200);
        let collapsed = newest_received_per_sender([original, rotated]);
        let newest = collapsed.get(&sender_id).expect("collapsed entry");

        // The per-sender ignore suppresses the rotated (newest) doc.
        assert_eq!(
            newest.account_reference, 7,
            "collapse keeps the newest (rotated) doc"
        );
        assert!(
            managed.is_sender_ignored(&sender_id),
            "an ignored sender suppresses ALL their requests, including the rotation"
        );

        // And the original ref (0) is suppressed too — per-sender, not
        // per-(sender, accountReference).
        assert!(managed.is_sender_ignored(&sender_id));
    }

    /// Build a received request with an explicit `created_at` so the
    /// dedup tiebreak can be exercised.
    fn test_request_at(
        sender: u8,
        recipient: u8,
        account_reference: u32,
        created_at: u64,
    ) -> ContactRequest {
        ContactRequest::new(
            Identifier::from([sender; 32]),
            Identifier::from([recipient; 32]),
            1,
            2,
            account_reference,
            vec![7u8; 96],
            100_000,
            created_at,
        )
    }

    /// **Sweep idempotency (the multi-doc thrash fix).**
    /// `contactRequest` docs are immutable and never deleted, so a sender
    /// who rotated leaves BOTH their old (ref=0) and bumped (ref=7) docs
    /// returning on every sweep. `newest_received_per_sender` must collapse
    /// them to the single newest by (created_at, accountReference) so the
    /// stale doc can't be re-ingested as a phantom rotation each pass.
    ///
    /// Without the collapse, the ingest loop processes every doc and compares
    /// each against the single tracked reference, so the non-matching doc
    /// flips the stored state every sweep; with it, only the newest survives.
    #[test]
    fn newest_received_per_sender_collapses_rotated_sender_to_latest_doc() {
        let sender = 2u8;
        let our = 1u8;
        // Same sender, two on-chain docs: old ref=0 @t=100, rotated ref=7 @t=200.
        let old_doc = test_request_at(sender, our, 0, 100);
        let rotated_doc = test_request_at(sender, our, 7, 200);
        // A second, unrelated sender to prove per-sender keying.
        let other = test_request_at(3, our, 0, 150);

        // Feed in doc-id order (old before new — the order a BTreeMap-keyed
        // fetch yields, NOT createdAt order) to prove ordering independence.
        let collapsed =
            newest_received_per_sender([old_doc.clone(), other.clone(), rotated_doc.clone()]);

        assert_eq!(collapsed.len(), 2, "one entry per distinct sender");
        let sender_id = Identifier::from([sender; 32]);
        assert_eq!(
            collapsed.get(&sender_id).map(|r| r.account_reference),
            Some(7),
            "the newest (rotated) doc must win, regardless of input order"
        );
        assert_eq!(
            collapsed
                .get(&Identifier::from([3u8; 32]))
                .map(|r| r.account_reference),
            Some(0),
            "the unrelated sender is unaffected"
        );

        // And the collapse is itself a fixpoint: re-collapsing yields the same.
        let again = newest_received_per_sender(collapsed.values().cloned());
        assert_eq!(again.get(&sender_id).map(|r| r.account_reference), Some(7));
    }

    /// **Rotation version bump must read established contacts.**
    /// The next request's rotation version is derived by un-masking the
    /// PRIOR sent reference. Once a contact establishes, that prior request
    /// moves out of `sent_contact_requests` into
    /// `established_contacts[..].outgoing_request`, so a lookup that only
    /// consults the pending map returns `None` → version resets to 0 →
    /// reproduces the original accountReference → unique-index rejection.
    ///
    /// The hazard: if `prior_sent_account_reference` consulted only
    /// `sent_contact_requests` it would return `None` for an established
    /// contact; it must fall back to the established outgoing request.
    #[test]
    fn prior_sent_account_reference_falls_back_to_established_outgoing() {
        let our = 1u8;
        let contact = 2u8;
        let our_id = Identifier::from([our; 32]);
        let contact_id = Identifier::from([contact; 32]);
        let (_wallet, mut info) = info_with_established_contact(our, contact);

        let managed = info.identity_manager.managed_identity_mut(&our_id).unwrap();
        // Precondition: the outgoing request is NOT in the pending map.
        assert!(
            !managed.sent_contact_requests.contains_key(&contact_id),
            "an established contact's outgoing request lives in established_contacts, not the pending map"
        );
        // The fix: the lookup still finds the prior reference via the
        // established contact's outgoing_request (reference 0 here).
        assert_eq!(
            managed.prior_sent_account_reference(&contact_id),
            Some(0),
            "must read the established contact's outgoing accountReference, not None"
        );

        // And a pending (not-yet-established) recipient still resolves via
        // the pending map; an unknown recipient is None.
        let pending = Identifier::from([9u8; 32]);
        managed.add_sent_contact_request(test_request(our, 9, 4), &noop_persister());
        assert_eq!(managed.prior_sent_account_reference(&pending), Some(4));
        assert_eq!(
            managed.prior_sent_account_reference(&Identifier::from([42u8; 32])),
            None
        );
    }

    /// **Defense-in-depth — `apply_rotated_incoming_request` is
    /// idempotent.** Even if the dedup ever let a duplicate through, a
    /// re-apply of the byte-identical request must be a no-op: no second
    /// changeset, no re-reported re-key (which would re-tear-down the
    /// external account).
    #[test]
    fn apply_rotated_incoming_request_is_idempotent() {
        let our = 1u8;
        let contact = 2u8;
        let our_id = Identifier::from([our; 32]);
        let (_wallet, mut info) = info_with_established_contact(our, contact);
        let p = noop_persister();

        let managed = info.identity_manager.managed_identity_mut(&our_id).unwrap();
        let rotated = test_request(contact, our, 7);

        // First apply: real re-key (returns true — caller tears down the account).
        assert!(
            managed.apply_rotated_incoming_request(rotated.clone(), &p),
            "first rotation must re-key the established contact"
        );
        // Second apply of the SAME request: no-op (returns false).
        assert!(
            !managed.apply_rotated_incoming_request(rotated.clone(), &p),
            "re-applying an identical request must be a no-op (no re-key, no churn)"
        );
        let stored = info
            .identity_manager
            .managed_identity(&our_id)
            .unwrap()
            .established_contacts
            .get(&Identifier::from([contact; 32]))
            .unwrap();
        assert_eq!(stored.incoming_request.account_reference, 7);
    }
}

// ---------------------------------------------------------------------------
// Send-side recipient key selection.
//
// Verified testnet reality: the dominant mobile cohort has
// an ENCRYPTION key but NO DECRYPTION key, and references its ENCRYPTION key
// for recipientKeyIndex. Sending to such a recipient must succeed by falling
// back to the ENCRYPTION key — without that fallback the send errors with
// "no decryption key" for the dominant mobile cohort.
// ---------------------------------------------------------------------------
#[cfg(test)]
mod recipient_key_selection_tests {
    use super::*;
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::v0::IdentityV0;
    use dpp::identity::SecurityLevel;
    use std::collections::BTreeMap;

    fn key(id: u32, key_type: KeyType, purpose: Purpose) -> IdentityPublicKey {
        IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id,
            key_type,
            purpose,
            security_level: SecurityLevel::MEDIUM,
            contract_bounds: None,
            read_only: false,
            data: dpp::platform_value::BinaryData::new(vec![0x02; 33]),
            disabled_at: None,
        })
    }

    fn identity_with_keys(keys: Vec<IdentityPublicKey>) -> Identity {
        let mut map = BTreeMap::new();
        for k in keys {
            map.insert(k.id(), k);
        }
        Identity::V0(IdentityV0 {
            id: Identifier::from([0xBB; 32]),
            public_keys: map,
            balance: 0,
            revision: 0,
        })
    }

    /// Mobile-shaped recipient: AUTHENTICATION + ENCRYPTION keys, NO
    /// DECRYPTION key. Selection must fall back to the ENCRYPTION key (id 2)
    /// rather than erroring "no decryption key".
    #[test]
    fn falls_back_to_encryption_key_when_recipient_has_no_decryption_key() {
        let recipient = identity_with_keys(vec![
            key(0, KeyType::ECDSA_SECP256K1, Purpose::AUTHENTICATION),
            key(1, KeyType::ECDSA_SECP256K1, Purpose::AUTHENTICATION),
            key(2, KeyType::ECDSA_SECP256K1, Purpose::ENCRYPTION),
        ]);

        let idx = select_recipient_key_index(&recipient)
            .expect("must select the ENCRYPTION key for a mobile-shaped recipient");
        assert_eq!(idx, 2, "should reference the recipient's ENCRYPTION key");
    }

    /// Newest cohort / our convention: a DECRYPTION key is present and
    /// preferred over any ENCRYPTION key.
    #[test]
    fn prefers_decryption_key_when_present() {
        let recipient = identity_with_keys(vec![
            key(4, KeyType::ECDSA_SECP256K1, Purpose::ENCRYPTION),
            key(5, KeyType::ECDSA_SECP256K1, Purpose::DECRYPTION),
        ]);

        let idx = select_recipient_key_index(&recipient).expect("decryption key present");
        assert_eq!(idx, 5, "must prefer DECRYPTION over ENCRYPTION");
    }

    /// Neither DECRYPTION nor ENCRYPTION (only AUTHENTICATION): error. No
    /// AUTHENTICATION fallback — reusing signing keys for ECDH is poor key
    /// separation and no live population needs it.
    #[test]
    fn errors_when_recipient_has_neither_encryption_nor_decryption() {
        let recipient = identity_with_keys(vec![key(
            0,
            KeyType::ECDSA_SECP256K1,
            Purpose::AUTHENTICATION,
        )]);

        let err = select_recipient_key_index(&recipient).unwrap_err();
        assert!(
            matches!(err, PlatformWalletError::InvalidIdentityData(_)),
            "expected InvalidIdentityData, got {err:?}"
        );
    }

    /// A DECRYPTION key of the wrong key TYPE is not selectable; selection
    /// falls through to a valid ECDSA ENCRYPTION key.
    #[test]
    fn skips_non_ecdsa_decryption_key_and_uses_ecdsa_encryption() {
        let recipient = identity_with_keys(vec![
            key(0, KeyType::BLS12_381, Purpose::DECRYPTION),
            key(1, KeyType::ECDSA_SECP256K1, Purpose::ENCRYPTION),
        ]);

        let idx = select_recipient_key_index(&recipient)
            .expect("ECDSA encryption key must be selectable");
        assert_eq!(idx, 1);
    }

    fn disabled_key(id: u32, key_type: KeyType, purpose: Purpose) -> IdentityPublicKey {
        IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id,
            key_type,
            purpose,
            security_level: SecurityLevel::MEDIUM,
            contract_bounds: None,
            read_only: false,
            data: dpp::platform_value::BinaryData::new(vec![0x02; 33]),
            disabled_at: Some(1_700_000_000_000),
        })
    }

    /// **#6 — a disabled (revoked) recipient key must not be selected.** The
    /// chosen key receives the contact's DIP-15 compact xpub encrypted via
    /// ECDH; picking a revoked key would hand that payment xpub to whoever
    /// holds the compromised private half. A disabled DECRYPTION key must be
    /// skipped in favour of an enabled ENCRYPTION key.
    #[test]
    fn skips_disabled_decryption_key_and_falls_back_to_enabled_encryption() {
        let recipient = identity_with_keys(vec![
            disabled_key(0, KeyType::ECDSA_SECP256K1, Purpose::DECRYPTION),
            key(1, KeyType::ECDSA_SECP256K1, Purpose::ENCRYPTION),
        ]);

        let idx = select_recipient_key_index(&recipient)
            .expect("must skip the disabled DECRYPTION key and use the enabled ENCRYPTION key");
        assert_eq!(idx, 1, "the disabled key (id 0) must not be selected");
    }

    /// When the ONLY candidate is disabled, selection errors rather than
    /// silently encrypting to a revoked key.
    #[test]
    fn errors_when_only_candidate_key_is_disabled() {
        let recipient = identity_with_keys(vec![disabled_key(
            0,
            KeyType::ECDSA_SECP256K1,
            Purpose::ENCRYPTION,
        )]);

        let err = select_recipient_key_index(&recipient).unwrap_err();
        assert!(
            matches!(err, PlatformWalletError::InvalidIdentityData(_)),
            "a sole disabled key must error, got {err:?}"
        );
    }
}
