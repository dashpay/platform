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

use dpp::dashcore::PrivateKey;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::v0::IdentityV0;
use dpp::identity::{Identity, IdentityPublicKey, KeyID, Purpose, SecurityLevel};
use dpp::prelude::{AssetLockProof, Identifier};
use key_wallet::bip32::{ChildNumber, DerivationPath};
use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;

use crate::changeset::{InvitationChangeSet, InvitationEntry, InvitationStatus};

use dash_sdk::platform::transition::put_identity::PutIdentity;
use dash_sdk::platform::transition::put_settings::PutSettings;

use crate::error::PlatformWalletError;
use crate::wallet::identity::crypto::{encode_invitation_uri, validate_claimable};
use crate::wallet::identity::crypto::{InviterInfo, ParsedInvitation};
use crate::wallet::identity::network::contact_requests::ContactCryptoProvider;

use super::*;

/// Hard cap on the amount an invitation can lock (0.01 DASH). The voucher is a
/// bearer credential, so the blast radius of a leaked link is bounded here in
/// Rust — not just in the UI. Generous enough for identity registration plus a
/// small starting balance; tune if onboarding needs more.
pub const MAX_INVITATION_DUFFS: u64 = 1_000_000;

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
    /// The `dashpay://invite?data=…` link. **Contains the voucher key** — treat
    /// as a secret (never log or persist it).
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
    pub async fn create_invitation<AS, CP>(
        &self,
        amount_duffs: u64,
        funding_account_index: u32,
        inviter: Option<InviterInfo>,
        expiry_unix: u32,
        asset_lock_signer: &AS,
        crypto_provider: &CP,
    ) -> Result<Invitation, PlatformWalletError>
    where
        AS: ::key_wallet::signer::Signer + Send + Sync,
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

        // Build + broadcast the voucher asset lock at the invitation funding
        // account (the builder auto-selects the next unused funding index and
        // returns its derivation path). `identity_index` is unused for the
        // `IdentityInvitation` funding type.
        let (proof, path, out_point) = self
            .asset_locks
            .create_funded_asset_lock_proof(
                amount_duffs,
                funding_account_index,
                AssetLockFundingType::IdentityInvitation,
                0,
                asset_lock_signer,
            )
            .await?;

        // The invitee's `validate_claimable` accepts only an InstantSend proof.
        // `create_funded_asset_lock_proof` falls back to a ChainLock proof if the
        // IS lock doesn't propagate within its 300s preference window — reject
        // that here rather than emit a link the invitee would silently reject (a
        // dead voucher: funds locked, no signal). The funding lock stays
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
        let uri_result = encode_invitation_uri(&voucher_key, &proof, expiry_unix, inviter.as_ref());
        voucher_key.non_secure_erase();
        let uri = uri_result?;

        // Persist an inviter-side invitation record for the "Sent invitations"
        // list + (future) reclaim. No secret is stored — only the funding index,
        // which is display metadata (reclaim resumes by outpoint, not by this
        // index). Best-effort: the link is already valid, so skipping the record
        // never fails the create.
        if let Some(funding_index) = funding_index_from_path(&path) {
            let mut inv_cs = InvitationChangeSet::default();
            inv_cs.invitations.insert(
                out_point,
                InvitationEntry {
                    out_point,
                    funding_index,
                    amount_duffs,
                    expiry_unix,
                    created_at_secs: expiry_unix.saturating_sub(MAX_INVITATION_TTL_SECS),
                    has_inviter: inviter.is_some(),
                    status: InvitationStatus::Created,
                },
            );
            if let Err(e) = self
                .persister
                .store(crate::changeset::PlatformWalletChangeSet {
                    invitations: Some(inv_cs),
                    ..Default::default()
                })
            {
                tracing::warn!(error = %e, "failed to persist invitation record; the link is still valid");
            }
        } else {
            tracing::warn!(
                "invitation funding path has no u32 index tail; \
                 skipping the local invitation record"
            );
        }

        Ok(Invitation {
            uri,
            out_point,
            amount_duffs,
            expiry_unix,
        })
    }

    /// Claim a DashPay invitation: register a NEW identity for the invitee,
    /// funded by the imported voucher.
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
        now_unix: u32,
        settings: Option<PutSettings>,
    ) -> Result<Identity, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        // Fail fast on a stale / wrong-type / mismatched link (and a zero clock
        // read) before any network.
        validate_claimable(&invitation, now_unix)?;
        preflight_keys_map(&keys_map)?;

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

        // Submit directly. The CL-height-too-low (10506) retry only helps
        // ChainLock proofs; the claim path only ever carries an InstantSend proof
        // (enforced at create, re-checked by `validate_claimable`), so a
        // CL-height retry would be dead code. A within-expiry IS proof either
        // lands or is rejected — the inviter re-creates on the rare rejection.
        let identity = placeholder
            .put_to_platform_and_wait_for_response_with_private_key(
                &self.sdk,
                invitation.asset_lock.clone(),
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
}
