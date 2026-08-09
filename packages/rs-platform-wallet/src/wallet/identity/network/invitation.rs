//! DashPay invitation create + claim flows (DIP-13 sub-feature 3').
//!
//! - [`create_invitation`](IdentityWallet::create_invitation) (inviter): fund a
//!   one-time asset-lock voucher at the invitation derivation path, export the
//!   voucher key, and package a `dashpay://invite` link.
//! - [`claim_invitation`](IdentityWallet::claim_invitation) (invitee): register
//!   a new identity funded by the imported voucher — ordinary identity
//!   registration whose asset-lock signature uses the imported raw voucher key
//!   instead of a wallet-derived one.
//!
//! The contact-bootstrap ("and now we're contacts") is intentionally NOT done
//! here: after a successful claim the UI asks the invitee whether to establish
//! contact with the sender and, if so, calls the existing contact-request path
//! ([`send_contact_request_with_external_signer`](IdentityWallet::send_contact_request_with_external_signer)).
//! See `docs/dashpay/DIP15_INVITATIONS_SPEC.md`.

use std::collections::BTreeMap;

use dpp::dashcore::consensus::Decodable;
use dpp::dashcore::{InstantLock, OutPoint, PrivateKey, Transaction};
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
use dpp::identity::state_transition::asset_lock_proof::InstantAssetLockProof;
use dpp::identity::v0::IdentityV0;
use dpp::identity::{Identity, IdentityPublicKey, KeyID, Purpose, SecurityLevel};
use dpp::prelude::{AssetLockProof, Identifier};
use key_wallet::bip32::{ChildNumber, DerivationPath};
use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;

use crate::changeset::{
    InvitationChangeSet, InvitationEntry, InvitationStatus, PersistenceCapabilities,
};

use dash_sdk::platform::transition::put_identity::PutIdentity;
use dash_sdk::platform::transition::put_settings::PutSettings;

use crate::error::PlatformWalletError;
use crate::wallet::identity::crypto::{
    encode_invitation_uri, voucher_output_index, wif_network_matches,
};
use crate::wallet::identity::crypto::{InviterInfo, ParsedInvitation};
use crate::wallet::identity::network::contact_requests::ContactCryptoProvider;

use super::*;

/// Hard cap on the amount an invitation can lock (0.26 DASH). The voucher is a
/// bearer credential, so the blast radius of a leaked link is bounded here in
/// Rust — not just in the UI. Sized for onboarding at BOTH username tiers: the
/// invitee spends the voucher on identity creation **plus** a DPNS name — a
/// normal name (~0.03 DASH, the legacy `DASH_PAY_FEE`) or a contested/premium
/// name (~0.25 DASH, `DASH_PAY_FEE_CONTESTED`), with the 0.01 margin covering
/// the create/claim fees. The earlier 0.05 value deferred the contested tier
/// "until contested-name-via-invite claim exists" — the claim path is
/// amount-agnostic and contested-invite claims are verified working, and the
/// Android wallet has funded contested invitations at 0.25 since 2024, so the
/// deferral is over. (The pre-merge 0.01 iteration was below even a usable
/// non-contested invitation and rejected its own onboarding default.)
pub const MAX_INVITATION_DUFFS: u64 = 26_000_000;

/// Floor on the amount an invitation can lock (0.003 DASH). A voucher funds a
/// Platform identity operation, and creating an identity — which is what the
/// invitee's claim does, and what a register-target reclaim does — requires the
/// asset lock to carry at least the identity-registration minimum (~0.00228 DASH
/// in credits on the current network; the state transition is rejected with
/// `IdentityAssetLockTransactionOutPointNotEnoughBalanceError` below it). A
/// voucher under this floor produces an invitation that can be neither claimed
/// nor reclaimed, so reject it at creation. Set above the bare floor to leave the
/// new identity a small usable starting balance; tune if the floor changes.
pub const MIN_INVITATION_DUFFS: u64 = 300_000;

/// Default TTL (24h) for an invitation's advisory expiry. The FFI sets
/// `expiry_unix = now + MAX_INVITATION_TTL_SECS`. The expiry is **advisory** — a
/// leaked-link finder holds the voucher key and ignores it — so it bounds only
/// the honest UI (don't submit an about-to-go-stale IS proof) and the reclaim
/// signal, NOT a leaked-link window. The real leak bound is `MAX_INVITATION_DUFFS`.
pub const MAX_INVITATION_TTL_SECS: u32 = 24 * 60 * 60;

/// Claim-by-fetch bound: how many times to look up the funding tx (each attempt
/// tries both byte orders) before giving up. InstantSend/ChainLock finality does
/// not guarantee the invitee's DAPI node has indexed the tx yet, so a freshly
/// shared invitation can miss purely from propagation lag; retrying bounds that.
const CLAIM_FETCH_MAX_ATTEMPTS: u32 = 5;

/// Fixed backoff between claim-by-fetch attempts (matches the identity-create
/// fetch-retry cadence elsewhere in the wallet). 5 attempts × 3s ≈ 12s of
/// tolerance for propagation delay before surfacing a "not found" error.
const CLAIM_FETCH_RETRY_DELAY: std::time::Duration = std::time::Duration::from_secs(3);

/// RAII scrub for the voucher `PrivateKey` copy used on the claim path.
/// `dashcore::PrivateKey` wraps a `secp256k1::SecretKey` that has no
/// Drop-zeroize, so the imported bearer scalar would otherwise linger in memory
/// after `claim_invitation` returns on every exit path (success or error).
/// Wiping it here mirrors the create path's explicit scrub of the exported
/// scalar — the voucher key is treated as bearer money end to end.
struct WipingPrivateKey(PrivateKey);

impl Drop for WipingPrivateKey {
    fn drop(&mut self) {
        self.0.inner.non_secure_erase();
    }
}

/// A freshly-created invitation: the shareable link plus the bookkeeping the
/// inviter tracks to reclaim an unclaimed voucher.
pub struct Invitation {
    /// The `dashpay://invite?…` link (legacy query form). **Contains the voucher
    /// key** (WIF) — treat as a secret (never log or persist it).
    pub uri: String,
    /// The funding asset lock's outpoint (the tracked lock's identity).
    pub out_point: dashcore::OutPoint,
    /// Amount locked (duffs).
    pub amount_duffs: u64,
    /// Advisory expiry (unix seconds).
    pub expiry_unix: u32,
}

impl std::fmt::Debug for Invitation {
    /// Redacts the URI — it embeds the bearer voucher key.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Invitation")
            .field("uri", &"<redacted>")
            .field("out_point", &self.out_point)
            .field("amount_duffs", &self.amount_duffs)
            .field("expiry_unix", &self.expiry_unix)
            .finish()
    }
}

/// Pre-flight the caller-supplied identity keys map: id=0 must be a MASTER-level
/// AUTHENTICATION key (it signs the IdentityCreate transition). Mirrors
/// `register_identity_with_funding`.
fn preflight_keys_map(
    keys_map: &BTreeMap<u32, IdentityPublicKey>,
) -> Result<(), PlatformWalletError> {
    if keys_map.is_empty() {
        return Err(PlatformWalletError::InvalidIdentityData(
            "keys_map must contain at least one identity public key".to_string(),
        ));
    }
    match keys_map.get(&0) {
        Some(k)
            if k.security_level() == SecurityLevel::MASTER
                && k.purpose() == Purpose::AUTHENTICATION => {}
        Some(_) => {
            return Err(PlatformWalletError::InvalidIdentityData(
                "keys_map[0] must be a MASTER-level AUTHENTICATION key \
                 (required to sign the IdentityCreate transition)"
                    .to_string(),
            ))
        }
        None => {
            return Err(PlatformWalletError::InvalidIdentityData(
                "keys_map must include key id=0 with MASTER security level".to_string(),
            ))
        }
    }
    Ok(())
}

/// Extract the u32 funding index from an invitation funding path's tail.
///
/// The invitation funding address is drawn from the account's address pool, so
/// the path's tail is a NORMAL (non-hardened) 32-bit index; the gate must accept
/// it — a hardened-only requirement would drop every real invitation record,
/// since real funding tails are never hardened. A hardened tail maps to the same
/// index field and is accepted too. Returns `None` for a 256-bit index variant
/// (never a u32 funding index, and never produced for an address-pool-derived
/// path) or for an empty path; the caller then skips the local invitation record
/// rather than failing the create — the link is already valid, and reclaim
/// resumes by outpoint, so this index is display metadata, not the key source.
fn funding_index_from_path(path: &DerivationPath) -> Option<u32> {
    match path.as_ref().last().copied() {
        Some(ChildNumber::Hardened { index }) | Some(ChildNumber::Normal { index }) => Some(index),
        _ => None,
    }
}

/// Byte-reverse a txid hex string. Old iOS invitation links carry the funding
/// txid in little-endian internal order; DAPI keys `getTransaction` by the
/// big-endian display id, so the claim retries with the id reversed on a miss
/// (mirrors Android's `Sha256Hash.wrap(id).reversedBytes` retry).
fn reverse_txid_hex(txid_hex: &str) -> Result<String, PlatformWalletError> {
    let mut bytes = hex::decode(txid_hex).map_err(|e| {
        PlatformWalletError::InvalidIdentityData(format!(
            "invitation funding txid is not valid hex: {e}"
        ))
    })?;
    bytes.reverse();
    Ok(hex::encode(bytes))
}

impl IdentityWallet {
    /// Create a DashPay invitation: fund a one-time asset-lock voucher at the
    /// DIP-13 invitation path and return a shareable `dashpay://invite` link.
    ///
    /// `asset_lock_signer` funds + signs the asset lock (the funding-input P2PKH
    /// signatures and the credit-output pubkey); `crypto_provider` exports the
    /// one-time voucher **private** key at the funding path (path-gated to the
    /// invitation sub-feature — see
    /// [`ContactCryptoProvider::export_invitation_private_key`]). In the FFI both
    /// are the same Keychain-resolver-backed signer.
    ///
    /// `expiry_unix` is an advisory bound; the caller (FFI) is responsible for
    /// clamping it to `now + MAX_INVITATION_TTL`. `inviter` is `Some` only when
    /// the inviter opted in to the contact-bootstrap ("send a request back").
    ///
    /// The proof is kept as an **InstantSend** proof (owner decision) — fast, and
    /// the embedded tx + islock make the link self-contained; staleness is
    /// bounded by the short advisory expiry, not a CL upgrade.
    ///
    /// Requires every bit in [`PersistenceCapabilities::INVITATION_CREATION`]:
    /// atomic changesets, invitation writes, account-address-pool writes, and
    /// wallet restore. The exported voucher key is derived from the persisted
    /// funding index, so a backend missing any one of those contracts could
    /// re-export the same bearer key after a restart. An incomplete backend is
    /// refused before any funds move.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_invitation<AS, CP>(
        &self,
        amount_duffs: u64,
        funding_account_index: u32,
        inviter: Option<InviterInfo>,
        expiry_unix: u32,
        created_at_secs: u32,
        asset_lock_signer: &AS,
        crypto_provider: &CP,
    ) -> Result<Invitation, PlatformWalletError>
    where
        AS: ::key_wallet::signer::ExtendedPubKeySigner + Send + Sync,
        CP: ContactCryptoProvider + Send + Sync,
    {
        if amount_duffs < MIN_INVITATION_DUFFS {
            return Err(PlatformWalletError::InvalidIdentityData(format!(
                "invitation amount {amount_duffs} is below the minimum {MIN_INVITATION_DUFFS} \
                 duffs; a smaller voucher cannot fund identity registration, so the invitation \
                 could be neither claimed nor reclaimed"
            )));
        }
        if amount_duffs > MAX_INVITATION_DUFFS {
            return Err(PlatformWalletError::InvalidIdentityData(format!(
                "invitation amount {amount_duffs} exceeds the cap {MAX_INVITATION_DUFFS} duffs"
            )));
        }
        if expiry_unix == 0 {
            return Err(PlatformWalletError::InvalidIdentityData(
                "invitation expiry_unix must be set (non-zero)".to_string(),
            ));
        }

        // Require the complete invitation persistence contract. A generic
        // atomic/durable assertion is insufficient: the backend might commit
        // other changesets while silently dropping invitation rows or funding
        // address-pool indices, or it might be unable to restore those indices.
        // Checked before any funds move.
        let capabilities = self.persister.persistence_capabilities();
        let required = PersistenceCapabilities::INVITATION_CREATION;
        if !capabilities.contains(required) {
            let missing = capabilities.missing(required);
            return Err(PlatformWalletError::Persistence(format!(
                "invitation creation requires persistence capabilities {:?} \
                     (missing mask 0x{:x}): an incomplete backend could re-export \
                     the same bearer voucher key after a restart",
                missing.names(),
                missing.bits(),
            )));
        }

        // Build + broadcast the voucher asset lock at the invitation funding
        // account (the builder auto-selects the next unused funding index and
        // returns its derivation path). `identity_index` is unused for the
        // `IdentityInvitation` funding type. Only the broadcast half runs here —
        // the proof wait is deferred until AFTER the invitation record below is
        // durably persisted, so an interruption during the (potentially long)
        // proof wait can no longer orphan the funded lock from the reclaim UI.
        let (path, out_point) = self
            .asset_locks
            .broadcast_funded_asset_lock(
                amount_duffs,
                funding_account_index,
                AssetLockFundingType::IdentityInvitation,
                0,
                asset_lock_signer,
            )
            .await?;

        // Persist the inviter-side invitation record NOW — immediately after the
        // broadcast succeeds, BEFORE every later fallible or slow step (the proof
        // wait, the InstantSend check, the voucher-key export, the URI encode).
        // The broadcast has already spent the DASH into the OP_RETURN, so any
        // voucher that fails a later step is still a *funded, reclaimable* lock;
        // recording it first is what keeps it visible in the "Sent
        // invitations"/reclaim UI. A ChainLock-confirmed voucher is rejected
        // further below as a usable *link* (the invitee needs an InstantSend
        // proof), but it remains a funded, reclaimable lock and is already
        // recorded here rather than orphaned. The store + flush are REQUIRED,
        // not best-effort: a funded voucher we cannot durably record is a hard
        // failure to surface, not a silent success. No secret is stored — only
        // the funding index (display metadata; reclaim resumes by outpoint). If
        // the funding path carries no u32 index tail (a structural can't-happen
        // for the invitation account), warn-skip: an untrackable row can't be
        // reclaimed either way.
        if let Some(funding_index) = funding_index_from_path(&path) {
            let mut inv_cs = InvitationChangeSet::default();
            inv_cs.invitations.insert(
                out_point,
                InvitationEntry {
                    out_point,
                    funding_index,
                    amount_duffs,
                    expiry_unix,
                    created_at_secs,
                    has_inviter: inviter.is_some(),
                    status: InvitationStatus::Created,
                },
            );
            self.persister
                .store(crate::changeset::PlatformWalletChangeSet {
                    invitations: Some(inv_cs),
                    ..Default::default()
                })
                .and_then(|_| self.persister.flush())
                .map_err(|e| {
                    PlatformWalletError::AssetLockTransaction(format!(
                        "the invitation voucher is funded but its record could not be \
                         persisted (it would be missing from Sent invitations and \
                         unreclaimable): {e}"
                    ))
                })?;
        } else {
            tracing::warn!(
                "invitation funding path has no u32 index tail; \
                 skipping the local invitation record"
            );
        }

        // Wait for the funding proof. The row above is already durable, so a
        // termination or failure inside this wait leaves a reclaimable, visible
        // invitation rather than an orphaned lock.
        let proof = self
            .asset_locks
            .wait_for_funded_asset_lock_proof(&out_point, funding_account_index)
            .await?;

        // The invitee's `validate_claimable` accepts only an InstantSend proof.
        // The proof wait falls back to a ChainLock proof if the IS lock doesn't
        // propagate within its 300s preference window — reject emitting a link
        // the invitee would silently reject (a dead voucher: funds locked, no
        // signal). The funding lock was recorded before the wait, so it stays
        // tracked/reclaimable; the inviter retries.
        if !matches!(proof, AssetLockProof::Instant(_)) {
            return Err(PlatformWalletError::AssetLockTransaction(
                "InstantSend lock did not confirm in time (a ChainLock proof was produced); \
                 the funding lock is reclaimable — please retry the invitation"
                    .to_string(),
            ));
        }

        // Export the one-time voucher private key at the funding path. This is
        // the one deliberate raw-key export (the whole point of an invitation);
        // it is path-gated to the invitation sub-feature inside the provider.
        let mut voucher_key = crypto_provider.export_invitation_private_key(&path).await?;

        // Build the (secret) URI, then scrub the exported scalar on BOTH the
        // success and the encode-error path. `secp256k1::SecretKey` has no
        // Drop-zeroize, so wipe it explicitly; scrubbing before propagating an
        // encode failure matters most there — on that path the key never
        // legitimately left the device, so a lingering copy is the worst kind.
        // The link carries the funding txid + islock (legacy query form), not
        // the embedded proof; the invitee refetches the tx at claim.
        let network = self.sdk.network;
        let uri_result = encode_invitation_uri(&voucher_key, network, &proof, inviter.as_ref());
        voucher_key.non_secure_erase();
        let uri = uri_result?;

        Ok(Invitation {
            uri,
            out_point,
            amount_duffs,
            expiry_unix,
        })
    }

    /// The identity id this invitation WOULD create, without claiming it.
    ///
    /// Platform derives a created identity's id from the asset-lock outpoint,
    /// so the id is knowable before the claim. The claim is otherwise the only
    /// way to learn a voucher is spent, which is why a used invitation surfaces
    /// as a raw "asset lock … already completely used" after the invitee has
    /// picked a username and entered their PIN.
    ///
    /// # Detects claims, not consumption — the signal is ONE-WAY
    ///
    /// An identity existing under the returned id means the voucher was
    /// **definitely** claimed. Its absence does **not** mean the voucher is
    /// usable.
    ///
    /// The lock can also be consumed by `IdentityTopUp` — the reclaim path
    /// behind `platform_wallet_topup_identity_with_existing_asset_lock_signer`
    /// with `consume_invitation_voucher: true` — which credits an EXISTING
    /// identity rather than creating the derived one. Afterwards no identity
    /// exists at this id, yet a claim still fails deterministically because the
    /// asset-lock output is already spent.
    ///
    /// Platform exposes no client query for spent asset locks (drive tracks
    /// them under `SpentAssetLockTransactions`, but no DAPI endpoint surfaces
    /// it), so consumption cannot be checked from here. Callers must therefore
    /// treat this as a fast-fail for the common case only: reject the
    /// invitation when an identity exists, and otherwise proceed WITHOUT
    /// concluding the voucher is usable. It narrows when the late failure
    /// happens; it does not remove it.
    ///
    /// Costs one funding-tx fetch: the outpoint is not in the link (the credit
    /// output is selected by pk↔script match, not by index), so the tx has to
    /// be refetched exactly as the claim does. Same wrong-network fail-fast as
    /// [`Self::claim_invitation`], so a testnet link on mainnet reports the
    /// network mismatch rather than a confusing fetch miss.
    pub async fn invitation_prospective_identity_id(
        &self,
        invitation: &ParsedInvitation,
    ) -> Result<Identifier, PlatformWalletError> {
        if !wif_network_matches(invitation.voucher_key_network, self.sdk.network) {
            return Err(PlatformWalletError::InvalidIdentityData(format!(
                "invitation is for the {:?} network but this wallet is on {:?}",
                invitation.voucher_key_network, self.sdk.network
            )));
        }
        let proof = self.reconstruct_asset_lock_proof(invitation).await?;
        proof.create_identifier().map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!(
                "invitation asset lock proof yielded no identity id: {e}"
            ))
        })
    }

    /// Claim a DashPay invitation: register a NEW identity for the invitee,
    /// funded by the imported voucher.
    ///
    /// The link carries only the voucher key + funding txid (+ optional islock),
    /// not the funding proof — so this **refetches** the funding transaction
    /// from Core and reconstructs the asset-lock proof, mirroring the legacy
    /// Android claim (`TopUpRepository.obtainAssetLockTransaction`):
    /// 1. Fetch the tx by `funding_txid`; retry byte-reversed on a miss (old iOS
    ///    links are little-endian).
    /// 2. Fail-fast that the fetched tx is really the funding tx, and (if an
    ///    islock is present) that the islock locks it.
    /// 3. Select the funded credit output by pk↔script match (not index 0).
    /// 4. Build an `InstantAssetLockProof` when an islock is present, else a
    ///    `ChainAssetLockProof` once the funding tx is chain-locked.
    ///
    /// The invitee's own identity keys (`keys_map`, derived from the invitee's
    /// seed) are signed by `identity_signer`; the asset-lock's outer
    /// state-transition signature is produced from the **imported voucher key**
    /// (`invitation.voucher_key`) via the SDK's in-process raw-key path. The
    /// invitee owns neither the lock's inputs nor its tracking, so this bypasses
    /// the wallet's `AssetLockFunding` machinery entirely.
    ///
    /// The contact-bootstrap is a separate step: on success the UI asks the
    /// invitee whether to establish contact with the sender and, if so, calls
    /// the existing contact-request path.
    pub async fn claim_invitation<S>(
        &self,
        invitation: ParsedInvitation,
        identity_index: u32,
        keys_map: BTreeMap<u32, IdentityPublicKey>,
        identity_signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<Identity, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        preflight_keys_map(&keys_map)?;

        // Reject a wrong-network link before any network work: a testnet WIF is a
        // valid key on the wrong chain, so it would otherwise surface as a
        // confusing funding-tx fetch miss rather than a clear "wrong network".
        if !wif_network_matches(invitation.voucher_key_network, self.sdk.network) {
            return Err(PlatformWalletError::InvalidIdentityData(format!(
                "invitation is for the {:?} network but this wallet is on {:?}",
                invitation.voucher_key_network, self.sdk.network
            )));
        }

        // Reconstruct the funding asset-lock proof by refetching the tx. Consensus
        // enforces pk↔output, islock↔tx, and identity_id↔outpoint, so the local
        // guards below are for fast-fail + correct-index selection, not theft
        // prevention (a crafted link at worst yields a failed claim).
        let asset_lock = self.reconstruct_asset_lock_proof(&invitation).await?;

        // The voucher key signs the asset lock's outer ST signature (ECDSA over
        // the credit-output pubkey hash). Convert to the SDK's `PrivateKey`,
        // scrubbed on every exit path by the `WipingPrivateKey` guard.
        let network = self.sdk.network;
        let voucher_priv = WipingPrivateKey(PrivateKey::new(invitation.voucher_key, network));

        let placeholder = Identity::V0(IdentityV0 {
            id: Identifier::default(),
            public_keys: keys_map,
            balance: 0,
            revision: 0,
        });

        // Submit directly. An InstantSend or ChainLock proof both prove finality;
        // a proof that no longer applies (e.g. the invite was already claimed) is
        // rejected by consensus and surfaced to the caller.
        let identity = placeholder
            .put_to_platform_and_wait_for_response_with_private_key(
                &self.sdk,
                asset_lock,
                &voucher_priv.0,
                identity_signer,
                settings,
            )
            .await
            .map_err(PlatformWalletError::Sdk)?;

        // Best-effort local bookkeeping — Platform has already accepted the
        // registration, so a local failure must NOT propagate (mirrors
        // `register_identity_with_funding` Step 4). The identity self-heals into
        // the IdentityManager on the next re-sync if this is skipped.
        {
            let identity_id = identity.id();
            let mut wm = self.wallet_manager.write().await;
            match wm.get_wallet_info_mut(&self.wallet_id) {
                Some(info) => {
                    match info.identity_manager.add_identity(
                        identity.clone(),
                        identity_index,
                        self.wallet_id,
                        &self.persister,
                    ) {
                        Ok(()) => {
                            let wallet_id = self.wallet_id;
                            let public_keys: Vec<(KeyID, IdentityPublicKey)> = identity
                                .public_keys()
                                .iter()
                                .map(|(k, v)| (*k, v.clone()))
                                .collect();
                            if let Some(managed) =
                                info.identity_manager.managed_identity_mut(&identity_id)
                            {
                                managed.wallet_id = Some(wallet_id);
                                for (key_id, pub_key) in public_keys {
                                    if let Err(e) = managed.add_key(
                                        pub_key,
                                        Some((wallet_id, identity_index, key_id)),
                                        &self.persister,
                                    ) {
                                        tracing::warn!(
                                            error = %e,
                                            %identity_id,
                                            "claim_invitation: identity key breadcrumb not persisted"
                                        );
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!(
                                error = %e,
                                %identity_id,
                                "claim_invitation: identity registered on Platform but local \
                                 add_identity failed; it will self-heal on the next re-sync"
                            );
                        }
                    }
                }
                None => {
                    tracing::warn!(
                        %identity_id,
                        "claim_invitation: identity registered on Platform but wallet info \
                         was not found locally; skipping local persistence"
                    );
                }
            }
        }

        Ok(identity)
    }

    /// Refetch the funding transaction and rebuild its asset-lock proof.
    ///
    /// Mirrors Android `TopUpRepository.obtainAssetLockTransaction`: fetch by
    /// txid (retry byte-reversed on a miss), verify the fetched tx is the funding
    /// tx, select the voucher's credit output, and assemble an InstantSend proof
    /// (when the link carried an islock) or a ChainLock proof (islock absent /
    /// `"null"` — a chainlock-confirmed invite).
    async fn reconstruct_asset_lock_proof(
        &self,
        invitation: &ParsedInvitation,
    ) -> Result<AssetLockProof, PlatformWalletError> {
        let sdk = &self.sdk;
        let fetched = fetch_funding_tx_with_retry(
            &invitation.funding_txid,
            |txid| async move {
                sdk.get_transaction(&txid)
                    .await
                    .map_err(PlatformWalletError::Sdk)
            },
            || tokio::time::sleep(CLAIM_FETCH_RETRY_DELAY),
        )
        .await?
        .ok_or_else(|| {
            PlatformWalletError::InvalidIdentityData(
                "invitation funding transaction not found (tried both byte orders across \
                 repeated attempts); it may not have propagated to the queried DAPI node yet — \
                 retry shortly"
                    .to_string(),
            )
        })?;
        assemble_asset_lock_proof(
            fetched.transaction,
            fetched.is_chain_locked,
            fetched.height,
            invitation,
        )
    }
}

/// Fetch the claim's funding transaction with the bounded propagation retry —
/// the injectable orchestration seam of `reconstruct_asset_lock_proof` (tests
/// script `fetch`/`delay`; production passes `Sdk::get_transaction` and a
/// `tokio::time::sleep`).
///
/// Two independent concerns are layered here:
///   1. Byte order — old iOS links carry the txid little-endian, so a
///      canonical miss is retried byte-reversed **within the same attempt**
///      (a compatibility fallback, not a temporal retry). A transport error is
///      NOT masked as a miss — it propagates immediately (it is not a "not
///      indexed yet" signal, so retrying it would only hide it).
///   2. Propagation lag — InstantSend/ChainLock finality does not guarantee
///      the queried DAPI node has indexed the tx yet, so a fresh invitation
///      can miss on both byte orders purely from propagation delay. Retry up
///      to [`CLAIM_FETCH_MAX_ATTEMPTS`] attempts with one `delay` between
///      attempts — and none after the last (the caller surfaces the miss
///      immediately).
///
/// Returns `Ok(None)` only after every attempt missed on both byte orders.
async fn fetch_funding_tx_with_retry<T, F, FFut, D, DFut>(
    funding_txid: &str,
    mut fetch: F,
    mut delay: D,
) -> Result<Option<T>, PlatformWalletError>
where
    F: FnMut(String) -> FFut,
    FFut: std::future::Future<Output = Result<Option<T>, PlatformWalletError>>,
    D: FnMut() -> DFut,
    DFut: std::future::Future<Output = ()>,
{
    for attempt in 0..CLAIM_FETCH_MAX_ATTEMPTS {
        if let Some(tx) = fetch(funding_txid.to_string()).await? {
            return Ok(Some(tx));
        }
        // Reversed lookup is computed lazily on a canonical miss, so a hit
        // never depends on the txid being reversible hex.
        let reversed = reverse_txid_hex(funding_txid)?;
        if let Some(tx) = fetch(reversed).await? {
            return Ok(Some(tx));
        }
        if attempt + 1 < CLAIM_FETCH_MAX_ATTEMPTS {
            delay().await;
        }
    }
    Ok(None)
}

/// Assemble the asset-lock proof from an already-fetched funding transaction — the
/// pure, testable core of the claim reconstruction (the fetch/retry lives in
/// `reconstruct_asset_lock_proof`). Validates the tx is the funding tx (either byte
/// order), selects the voucher's credit output, and builds an InstantSend proof
/// (link carried an islock) or a ChainLock proof (islock absent), requiring
/// chain-lock finality for the latter.
fn assemble_asset_lock_proof(
    transaction: Transaction,
    is_chain_locked: bool,
    height: u32,
    invitation: &ParsedInvitation,
) -> Result<AssetLockProof, PlatformWalletError> {
    // Fail-fast: the fetched tx must actually be the funding tx (either byte
    // order). DAPI returns whatever tx matches the id we asked for, so this
    // guards a backend that answers with an unrelated tx.
    let fetched_txid = transaction.txid().to_string();
    let reversed_txid = reverse_txid_hex(&invitation.funding_txid).ok();
    if fetched_txid != invitation.funding_txid
        && reversed_txid.as_deref() != Some(fetched_txid.as_str())
    {
        return Err(PlatformWalletError::InvalidIdentityData(
            "fetched transaction id does not match the invitation funding txid".to_string(),
        ));
    }

    // Select the funded credit output the voucher key controls (not index 0
    // — a legacy invite's credit output need not be first).
    let output_index = voucher_output_index(&transaction, &invitation.voucher_key)?;

    match &invitation.islock_hex {
        Some(islock_hex) => {
            let islock_bytes = hex::decode(islock_hex).map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "invitation islock is not valid hex: {e}"
                ))
            })?;
            // The hex is not self-describing; the modern deterministic islock
            // (ISDLOCK) carries its version byte, which is exactly what the
            // consensus decoder reads first.
            let instant_lock = InstantLock::consensus_decode(&mut islock_bytes.as_slice())
                .map_err(|e| {
                    PlatformWalletError::InvalidIdentityData(format!(
                        "invitation islock could not be decoded: {e}"
                    ))
                })?;
            if instant_lock.txid != transaction.txid() {
                return Err(PlatformWalletError::InvalidIdentityData(
                    "invitation islock does not lock the funding transaction".to_string(),
                ));
            }
            Ok(AssetLockProof::Instant(InstantAssetLockProof::new(
                instant_lock,
                transaction,
                output_index,
            )))
        }
        None => {
            // ChainLock invite: the proof references the outpoint + a chain-locked
            // core height. Require the funding tx to be chain-locked so the proof
            // proves finality; the inviter/invitee retries once the block is
            // chain-locked otherwise.
            if !is_chain_locked {
                return Err(PlatformWalletError::InvalidIdentityData(
                    "chainlock invitation funding transaction is not yet chain-locked; \
                     retry once it confirms"
                        .to_string(),
                ));
            }
            let out_point = OutPoint::new(transaction.txid(), output_index);
            let out_point_bytes: [u8; 36] = out_point.into();
            Ok(AssetLockProof::Chain(ChainAssetLockProof::new(
                height,
                out_point_bytes,
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A real DIP-13 invitation funding path ends in a NORMAL (non-hardened)
    /// address-pool index. This is the record that a hardened-only gate silently
    /// dropped, leaving the "Sent invitations" list empty despite a valid link.
    #[test]
    fn funding_index_extracted_from_non_hardened_tail() {
        let path = DerivationPath::from(vec![
            ChildNumber::Hardened { index: 9 },
            ChildNumber::Normal { index: 1 }, // coin type — non-hardened in practice
            ChildNumber::Hardened { index: 5 },
            ChildNumber::Hardened { index: 3 },
            ChildNumber::Normal { index: 42 }, // funding index — non-hardened
        ]);
        assert_eq!(funding_index_from_path(&path), Some(42));
    }

    /// A hardened tail carries the index in the same field, so it is accepted too.
    #[test]
    fn funding_index_extracted_from_hardened_tail() {
        let path = DerivationPath::from(vec![
            ChildNumber::Hardened { index: 9 },
            ChildNumber::Hardened { index: 3 },
            ChildNumber::Hardened { index: 7 },
        ]);
        assert_eq!(funding_index_from_path(&path), Some(7));
    }

    /// A 256-bit index cannot be a u32 funding index — skip rather than truncate.
    #[test]
    fn funding_index_none_for_256bit_tail() {
        let path = DerivationPath::from(vec![
            ChildNumber::Normal { index: 3 },
            ChildNumber::Normal256 { index: [0u8; 32] },
        ]);
        assert_eq!(funding_index_from_path(&path), None);
    }

    /// An empty path has no tail to read an index from.
    #[test]
    fn funding_index_none_for_empty_path() {
        let path = DerivationPath::from(Vec::<ChildNumber>::new());
        assert_eq!(funding_index_from_path(&path), None);
    }

    // --- assemble_asset_lock_proof: the claim-time proof reconstruction guards ---

    use crate::wallet::identity::crypto::invitation::{voucher_credit_script, ParsedInvitation};
    use dpp::dashcore::consensus::Encodable;
    use dpp::dashcore::secp256k1::SecretKey;
    use dpp::dashcore::transaction::special_transaction::asset_lock::AssetLockPayload;
    use dpp::dashcore::transaction::special_transaction::TransactionPayload;
    use dpp::dashcore::{Network, TxOut};

    fn voucher_secret() -> SecretKey {
        SecretKey::from_slice(&[0x11u8; 32]).unwrap()
    }

    /// An asset-lock tx whose single credit output pays the voucher key.
    fn funding_tx(key: &SecretKey) -> Transaction {
        let payload = AssetLockPayload {
            version: 1,
            credit_outputs: vec![TxOut {
                value: 100_000,
                script_pubkey: voucher_credit_script(key),
            }],
        };
        Transaction {
            version: 3,
            lock_time: 0,
            input: vec![],
            output: vec![],
            special_transaction_payload: Some(TransactionPayload::AssetLockPayloadType(payload)),
        }
    }

    fn parsed(
        key: SecretKey,
        funding_txid: String,
        islock_hex: Option<String>,
    ) -> ParsedInvitation {
        ParsedInvitation {
            voucher_key: key,
            voucher_key_network: Network::Testnet,
            funding_txid,
            islock_hex,
            inviter: None,
        }
    }

    /// A backend answering with an unrelated tx (id matches neither byte order) is
    /// rejected before any proof is built.
    #[test]
    fn assemble_rejects_txid_mismatch() {
        let key = voucher_secret();
        let tx = funding_tx(&key);
        let inv = parsed(key, "00".repeat(32), None);
        let err = assemble_asset_lock_proof(tx, true, 100, &inv).unwrap_err();
        assert!(format!("{err}").contains("does not match"));
    }

    /// A ChainLock invite (no islock) whose funding tx is not yet chain-locked is
    /// rejected — the proof must prove finality.
    #[test]
    fn assemble_chainlock_requires_chain_lock() {
        let key = voucher_secret();
        let tx = funding_tx(&key);
        let txid = tx.txid().to_string();
        let inv = parsed(key, txid, None);
        let err = assemble_asset_lock_proof(tx, false, 100, &inv).unwrap_err();
        assert!(format!("{err}").contains("not yet chain-locked"));
    }

    /// A chain-locked ChainLock invite assembles a ChainLock proof at the tx's
    /// voucher output.
    #[test]
    fn assemble_chainlock_ok_when_locked() {
        let key = voucher_secret();
        let tx = funding_tx(&key);
        let txid = tx.txid().to_string();
        let inv = parsed(key, txid, None);
        let proof = assemble_asset_lock_proof(tx, true, 100, &inv).unwrap();
        assert!(matches!(proof, AssetLockProof::Chain(_)));
    }

    /// The prospective identity id is derived from the *selected* credit
    /// output's outpoint. Selection itself is pinned next to
    /// `voucher_output_index`; what matters here is that the id follows it — a
    /// voucher sitting behind a decoy must not yield the index-0 id, or the
    /// "has this invitation been used?" check answers about a stranger's
    /// identity and reports a perfectly good voucher as spent.
    #[test]
    fn prospective_id_follows_the_selected_credit_output() {
        let key = voucher_secret();
        let decoy = SecretKey::from_slice(&[0x22u8; 32]).unwrap();
        let payload = AssetLockPayload {
            version: 1,
            credit_outputs: vec![
                TxOut {
                    value: 100_000,
                    script_pubkey: voucher_credit_script(&decoy),
                },
                TxOut {
                    value: 100_000,
                    script_pubkey: voucher_credit_script(&key),
                },
            ],
        };
        let tx = Transaction {
            version: 3,
            lock_time: 0,
            input: vec![],
            output: vec![],
            special_transaction_payload: Some(TransactionPayload::AssetLockPayloadType(payload)),
        };
        let txid = tx.txid();
        let inv = parsed(key, txid.to_string(), None);

        let proof = assemble_asset_lock_proof(tx, true, 100, &inv).unwrap();
        let id = proof.create_identifier().unwrap();

        let from_index_0 =
            ChainAssetLockProof::new(100, OutPoint::new(txid, 0).into()).create_identifier();
        let from_index_1 =
            ChainAssetLockProof::new(100, OutPoint::new(txid, 1).into()).create_identifier();

        assert_ne!(id, from_index_0, "id must not come from credit output 0");
        assert_eq!(id, from_index_1);
    }

    /// The prospective id is a pure function of the asset-lock OUTPOINT, so it
    /// carries no information about whether that lock has been consumed.
    ///
    /// This is why the claimed-check is one-way. A voucher claimed normally
    /// creates the identity at this id, and the check sees it. A voucher
    /// RECLAIMED into an existing identity — `IdentityTopUp` via
    /// `platform_wallet_topup_identity_with_existing_asset_lock_signer` with
    /// `consume_invitation_voucher: true` — spends the very same outpoint but
    /// creates nothing here, so the check still finds no identity while a claim
    /// would fail deterministically.
    ///
    /// Pinned as an executable fact because the id derivation is what a reader
    /// would otherwise assume encodes "spent": it does not, and Platform
    /// exposes no client query for spent asset locks to fill the gap.
    #[test]
    fn prospective_id_is_outpoint_derived_and_says_nothing_about_consumption() {
        let key = voucher_secret();
        let payload = AssetLockPayload {
            version: 1,
            credit_outputs: vec![TxOut {
                value: 100_000,
                script_pubkey: voucher_credit_script(&key),
            }],
        };
        let tx = Transaction {
            version: 3,
            lock_time: 0,
            input: vec![],
            output: vec![],
            special_transaction_payload: Some(TransactionPayload::AssetLockPayloadType(payload)),
        };
        let txid = tx.txid();
        let inv = parsed(key, txid.to_string(), None);

        let id = assemble_asset_lock_proof(tx, true, 100, &inv)
            .unwrap()
            .create_identifier()
            .unwrap();

        // Nothing but (txid, vout) feeds it — the same value a reclaim would
        // leave behind untouched.
        let from_outpoint =
            ChainAssetLockProof::new(100, OutPoint::new(txid, 0).into()).create_identifier();
        assert_eq!(
            id, from_outpoint,
            "the id must be derivable from the outpoint alone, which is exactly \
             why its absence cannot prove the lock is unspent"
        );
    }

    /// An islock that locks a DIFFERENT tx than the funding tx is rejected (the
    /// txid-binding guard), so a link can't pair a valid islock with a foreign tx.
    #[test]
    fn assemble_rejects_islock_for_wrong_tx() {
        let key = voucher_secret();
        let tx = funding_tx(&key);
        let txid = tx.txid().to_string();
        // A default islock carries a zeroed txid, which won't equal the funding tx.
        let mut islock_bytes = Vec::new();
        InstantLock::default()
            .consensus_encode(&mut islock_bytes)
            .unwrap();
        let inv = parsed(key, txid, Some(hex::encode(islock_bytes)));
        let err = assemble_asset_lock_proof(tx, true, 100, &inv).unwrap_err();
        assert!(format!("{err}").contains("does not lock the funding transaction"));
    }

    // --- fetch_funding_tx_with_retry: the claim propagation-retry seam ---

    mod fetch_retry {
        use std::collections::VecDeque;
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::{Arc, Mutex};

        use super::super::{
            fetch_funding_tx_with_retry, reverse_txid_hex, CLAIM_FETCH_MAX_ATTEMPTS,
        };
        use crate::PlatformWalletError;

        /// A canonical 64-hex txid whose byte-reversed form differs from it.
        fn canonical_txid() -> String {
            format!("{}{}", "11".repeat(31), "22")
        }

        /// Scripted fetch: pops the next result per call and records the txid
        /// each call asked for; plus a counter-only delay.
        #[allow(clippy::type_complexity)]
        fn harness(
            script: Vec<Result<Option<u32>, PlatformWalletError>>,
        ) -> (
            Arc<Mutex<VecDeque<Result<Option<u32>, PlatformWalletError>>>>,
            Arc<Mutex<Vec<String>>>,
            Arc<AtomicU32>,
        ) {
            (
                Arc::new(Mutex::new(script.into_iter().collect())),
                Arc::new(Mutex::new(Vec::new())),
                Arc::new(AtomicU32::new(0)),
            )
        }

        async fn run(
            script: Vec<Result<Option<u32>, PlatformWalletError>>,
        ) -> (Result<Option<u32>, PlatformWalletError>, Vec<String>, u32) {
            let canonical = canonical_txid();
            let (queue, calls, delays) = harness(script);
            let (queue_f, calls_f, delays_f) =
                (Arc::clone(&queue), Arc::clone(&calls), Arc::clone(&delays));
            let result = fetch_funding_tx_with_retry(
                &canonical,
                move |txid| {
                    let queue = Arc::clone(&queue_f);
                    let calls = Arc::clone(&calls_f);
                    async move {
                        calls.lock().expect("calls").push(txid);
                        queue
                            .lock()
                            .expect("script")
                            .pop_front()
                            .expect("script exhausted — the loop fetched more than scripted")
                    }
                },
                move || {
                    let delays = Arc::clone(&delays_f);
                    async move {
                        delays.fetch_add(1, Ordering::SeqCst);
                    }
                },
            )
            .await;
            let recorded = calls.lock().expect("calls").clone();
            (result, recorded, delays.load(Ordering::SeqCst))
        }

        /// A first-call canonical hit returns immediately: one lookup, no
        /// reversed fallback, no delay.
        #[tokio::test]
        async fn canonical_hit_returns_immediately() {
            let (result, calls, delays) = run(vec![Ok(Some(7))]).await;
            assert_eq!(result.expect("no error"), Some(7));
            assert_eq!(calls, vec![canonical_txid()]);
            assert_eq!(delays, 0);
        }

        /// A canonical miss falls back to the byte-reversed lookup WITHIN the
        /// same attempt (the legacy-iOS little-endian compatibility path) — no
        /// delay is spent on the byte-order fallback.
        #[tokio::test]
        async fn reversed_hit_within_same_attempt_no_delay() {
            let (result, calls, delays) = run(vec![Ok(None), Ok(Some(9))]).await;
            assert_eq!(result.expect("no error"), Some(9));
            assert_eq!(
                calls,
                vec![
                    canonical_txid(),
                    reverse_txid_hex(&canonical_txid()).expect("reversible")
                ]
            );
            assert_eq!(delays, 0);
        }

        /// A hit on a later attempt (propagation lag) succeeds after exactly
        /// one delay per fully-missed attempt.
        #[tokio::test]
        async fn hit_after_missed_attempts_counts_one_delay_per_miss() {
            // Attempts 1 and 2 miss on both orders; attempt 3 hits canonically.
            let (result, calls, delays) =
                run(vec![Ok(None), Ok(None), Ok(None), Ok(None), Ok(Some(3))]).await;
            assert_eq!(result.expect("no error"), Some(3));
            assert_eq!(calls.len(), 5);
            assert_eq!(delays, 2);
        }

        /// Exhausting every attempt returns Ok(None) — a miss, not an error —
        /// after 2 lookups per attempt, with NO delay after the final attempt.
        #[tokio::test]
        async fn exhausts_attempts_without_trailing_delay() {
            let script = (0..CLAIM_FETCH_MAX_ATTEMPTS * 2)
                .map(|_| Ok(None))
                .collect();
            let (result, calls, delays) = run(script).await;
            assert_eq!(result.expect("misses are not errors"), None);
            assert_eq!(calls.len(), (CLAIM_FETCH_MAX_ATTEMPTS * 2) as usize);
            assert_eq!(delays, CLAIM_FETCH_MAX_ATTEMPTS - 1);
        }

        /// A transport error is NOT masked as a miss: it propagates
        /// immediately, with no further lookups and no extra delay.
        #[tokio::test]
        async fn transport_error_propagates_immediately() {
            // Attempt 1 misses both orders (1 delay); attempt 2's canonical
            // lookup errors.
            let (result, calls, delays) = run(vec![
                Ok(None),
                Ok(None),
                Err(PlatformWalletError::InvalidIdentityData(
                    "simulated transport failure".to_string(),
                )),
            ])
            .await;
            assert!(
                matches!(result, Err(PlatformWalletError::InvalidIdentityData(_))),
                "transport error must propagate, got {result:?}"
            );
            assert_eq!(calls.len(), 3, "no lookup may follow the error");
            assert_eq!(delays, 1);
        }
    }

    // --- create_invitation: the durable-persistence precondition ---

    mod durability_gate {
        use std::sync::Arc;

        use crate::events::{EventHandler, PlatformEventHandler};
        use crate::wallet::identity::network::contact_requests::SeedCryptoProvider;
        use crate::wallet::persister::NoPlatformPersistence;
        use crate::PlatformWalletError;
        use key_wallet::mnemonic::{Language, Mnemonic};
        use key_wallet::signer::{Signer, SignerMethod};
        use key_wallet::wallet::initialization::WalletAccountCreationOptions;
        use key_wallet::Network;

        const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon \
             abandon abandon abandon abandon abandon about";

        struct NoopEventHandler;
        impl EventHandler for NoopEventHandler {}
        impl PlatformEventHandler for NoopEventHandler {}

        /// Signer that must never be reached — the durability gate fires
        /// before any key material is touched or funds move.
        struct UnreachableSigner;

        #[async_trait::async_trait]
        impl Signer for UnreachableSigner {
            type Error = String;

            fn supported_methods(&self) -> &[SignerMethod] {
                &[SignerMethod::Digest]
            }

            async fn sign_ecdsa(
                &self,
                _path: &key_wallet::DerivationPath,
                _sighash: [u8; 32],
            ) -> Result<
                (
                    dashcore::secp256k1::ecdsa::Signature,
                    dashcore::secp256k1::PublicKey,
                ),
                Self::Error,
            > {
                unreachable!("the durability gate must fire before any signing")
            }

            async fn public_key(
                &self,
                _path: &key_wallet::DerivationPath,
            ) -> Result<key_wallet::bip32::PublicKey, Self::Error> {
                unreachable!("the durability gate must fire before any key derivation")
            }
        }

        #[async_trait::async_trait]
        impl ::key_wallet::signer::ExtendedPubKeySigner for UnreachableSigner {
            async fn extended_public_key(
                &self,
                _path: &key_wallet::bip32::DerivationPath,
            ) -> Result<key_wallet::bip32::ExtendedPubKey, Self::Error> {
                unreachable!("the durability gate must fire before any key derivation")
            }
        }

        /// A bearer voucher key must never be produced by a wallet whose
        /// backend cannot durably persist the funding index: on a restart the
        /// index resets and the SAME key would be re-exported to a later
        /// invitation. `create_invitation` refuses up-front — before any
        /// funds move or keys derive.
        #[tokio::test]
        async fn create_invitation_requires_durable_persistence() {
            let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
            let persister = Arc::new(NoPlatformPersistence);
            let handler: Arc<dyn PlatformEventHandler> = Arc::new(NoopEventHandler);
            let manager = Arc::new(crate::PlatformWalletManager::new(
                sdk,
                Arc::clone(&persister),
                handler,
            ));
            let mnemonic =
                Mnemonic::from_phrase(TEST_MNEMONIC, Language::English).expect("valid mnemonic");
            let seed = mnemonic.to_seed("");
            let wallet = manager
                .create_wallet_from_seed_bytes(
                    Network::Testnet,
                    &seed,
                    WalletAccountCreationOptions::None,
                    Some(0),
                )
                .await
                .expect("wallet creation");
            let iw = wallet.identity();

            let crypto_provider = SeedCryptoProvider::from_seed(seed, Network::Testnet);
            let err = iw
                .create_invitation(
                    1_000_000,
                    0,
                    None,
                    1,
                    1,
                    &UnreachableSigner,
                    &crypto_provider,
                )
                .await
                .expect_err("a non-durable backend must be refused");

            assert!(
                matches!(err, PlatformWalletError::Persistence(_)),
                "expected the durability refusal, got {err:?}"
            );
            assert!(
                format!("{err}").contains("persistence capabilities"),
                "the error must explain the missing capability contract, got: {err}"
            );
        }
    }
}
