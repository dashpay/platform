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

use std::collections::BTreeMap;

use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::{Identity, IdentityPublicKey, KeyID};
use dpp::prelude::{AssetLockProof, Identifier};
use key_wallet::bip32::{ChildNumber, DerivationPath};
use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;

use crate::changeset::{
    InvitationChangeSet, InvitationEntry, InvitationStatus, PersistenceCapabilities,
};

use dash_sdk::platform::transition::put_settings::PutSettings;

pub use dash_sdk::platform::dashpay::{
    InvitationClaimStatus, MAX_INVITATION_DUFFS, MAX_INVITATION_TTL_SECS, MIN_INVITATION_DUFFS,
};

use crate::error::PlatformWalletError;
use crate::wallet::identity::crypto::{encode_invitation_uri, InviterInfo, ParsedInvitation};
use crate::wallet::identity::network::contact_requests::ContactCryptoProvider;

use super::*;

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
        // returns its derivation path). The DASH backing it is POOLED across
        // `ASSET_LOCK_FUNDING_SOURCES` — the BIP44 and BIP32 accounts at
        // `funding_account_index` plus every DashPay contact-receiving account
        // — so an invitation can be funded from a balance spread across
        // accounts, with no sweep-then-lock hop.
        // `identity_index` is unused for the
        // `IdentityInvitation` funding type. Only the broadcast half runs here —
        // the proof wait is deferred until AFTER the invitation record below is
        // durably persisted, so an interruption during the (potentially long)
        // proof wait cannot orphan the funded lock from the reclaim UI.
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
    /// Forwards to [`dash_sdk::Sdk::invitation_prospective_identity_id`]; see
    /// there for the one-way caveat (an identity at the id proves a claim, its
    /// absence does not prove the voucher is unspent).
    pub async fn invitation_prospective_identity_id(
        &self,
        invitation: &ParsedInvitation,
    ) -> Result<Identifier, PlatformWalletError> {
        Ok(self
            .sdk
            .invitation_prospective_identity_id(invitation)
            .await?)
    }

    /// What an invitation is worth and whether it was already claimed.
    /// Forwards to [`dash_sdk::Sdk::invitation_claim_status`].
    pub async fn invitation_claim_status(
        &self,
        invitation: &ParsedInvitation,
    ) -> Result<InvitationClaimStatus, PlatformWalletError> {
        Ok(self.sdk.invitation_claim_status(invitation).await?)
    }

    /// Claim a DashPay invitation: register a NEW identity for the invitee,
    /// funded by the imported voucher, then record it in this wallet.
    ///
    /// The claim itself — refetching the funding transaction, rebuilding the
    /// asset-lock proof, the stale-islock → ChainLock fallback and adopting an
    /// earlier claim that already landed — is
    /// [`dash_sdk::Sdk::claim_invitation`]. This adds the identity and its
    /// keys to the wallet's identity manager at `identity_index`.
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
        let identity = self
            .sdk
            .claim_invitation(invitation, keys_map, identity_signer, settings)
            .await?;

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

    // --- create_invitation: the durable-persistence precondition ---

    mod durability_gate {
        use std::sync::Arc;

        use crate::events::{EventHandler, PlatformEventHandler};
        use crate::wallet::identity::network::contact_requests::SeedCryptoProvider;
        use crate::wallet::persister::NoPlatformPersistence;
        use crate::PlatformWalletError;
        use key_wallet::mnemonic::Mnemonic;
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
                    dashcore::crypto::ecdsa::EcdsaSignature,
                    dashcore::crypto::ecdsa::EcdsaPublicKey,
                ),
                Self::Error,
            > {
                unreachable!("the durability gate must fire before any signing")
            }

            async fn public_key(
                &self,
                _path: &key_wallet::DerivationPath,
            ) -> Result<dashcore::crypto::ecdsa::EcdsaPublicKey, Self::Error> {
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
            let mnemonic = Mnemonic::from_phrase(TEST_MNEMONIC).expect("valid mnemonic");
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
