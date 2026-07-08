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
use dpp::prelude::Identifier;
use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;

use dash_sdk::platform::transition::put_identity::PutIdentity;
use dash_sdk::platform::transition::put_settings::PutSettings;

use crate::error::PlatformWalletError;
use crate::wallet::asset_lock::orchestration::submit_with_cl_height_retry;
use crate::wallet::identity::crypto::{encode_invitation_uri, validate_claimable};
use crate::wallet::identity::crypto::{InviterInfo, ParsedInvitation};
use crate::wallet::identity::network::contact_requests::ContactCryptoProvider;

use super::*;

/// Hard cap on the amount an invitation can lock (0.01 DASH). The voucher is a
/// bearer credential, so the blast radius of a leaked link is bounded here in
/// Rust — not just in the UI (spec §8 Finding 4). Generous enough for identity
/// registration plus a small starting balance; tune if onboarding needs more.
pub const MAX_INVITATION_DUFFS: u64 = 1_000_000;

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
        if amount_duffs == 0 {
            return Err(PlatformWalletError::InvalidIdentityData(
                "invitation amount must be greater than zero".to_string(),
            ));
        }
        if amount_duffs > MAX_INVITATION_DUFFS {
            return Err(PlatformWalletError::InvalidIdentityData(format!(
                "invitation amount {amount_duffs} exceeds the cap {MAX_INVITATION_DUFFS} duffs"
            )));
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

        // Export the one-time voucher private key at the funding path. This is
        // the one deliberate raw-key export (the whole point of an invitation);
        // it is path-gated to the invitation sub-feature inside the provider.
        let voucher_key = crypto_provider.export_invitation_private_key(&path).await?;

        let uri = encode_invitation_uri(&voucher_key, &proof, expiry_unix, inviter.as_ref())?;

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
        // Fail fast on a stale / wrong-type / mismatched link before any network.
        validate_claimable(&invitation, now_unix)?;
        preflight_keys_map(&keys_map)?;

        // The voucher key signs the asset lock's outer ST signature (ECDSA over
        // the credit-output pubkey hash). Convert to the SDK's `PrivateKey`.
        let network = self.sdk.network;
        let voucher_priv = PrivateKey::new(invitation.voucher_key, network);

        let placeholder = Identity::V0(IdentityV0 {
            id: Identifier::default(),
            public_keys: keys_map,
            balance: 0,
            revision: 0,
        });

        // Submit with the CL-height-too-low retry layer. The direct raw-key SDK
        // call doesn't inherit `register_identity_with_funding`'s retry layers,
        // so wrap it here (a transient 10506 would otherwise hard-fail the claim).
        let identity = submit_with_cl_height_retry(settings, |s| {
            placeholder.put_to_platform_and_wait_for_response_with_private_key(
                &self.sdk,
                invitation.asset_lock.clone(),
                &voucher_priv,
                identity_signer,
                s,
            )
        })
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
