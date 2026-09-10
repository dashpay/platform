//! Withdraw credits from an identity.

use async_trait::async_trait;
use dashcore::Address as DashAddress;
use dpp::address_funds::AddressWitness;
use dpp::identity::accessors::IdentitySettersV0;
use dpp::identity::Identity;
use dpp::identity::IdentityPublicKey;
use dpp::identity::Purpose;
use dpp::platform_value::BinaryData;
use dpp::prelude::Identifier;
use dpp::ProtocolError;

use dpp::identity::signer::Signer;

use dash_sdk::platform::transition::put_settings::PutSettings;
use dash_sdk::platform::transition::withdraw_from_identity::WithdrawFromIdentity;

use crate::error::PlatformWalletError;

use super::*;

// Borrowed-signer adapter — see `dpns.rs`/`transfer.rs` for the same
// pattern. Lets a `&S: Signer<IdentityPublicKey>` satisfy APIs that
// take an owned signer by generic bound.
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

// ---------------------------------------------------------------------------
// Withdrawal
// ---------------------------------------------------------------------------

impl IdentityWallet {
    /// Withdraw credits using an externally-supplied signer.
    ///
    /// Signing is routed through the supplied `&S: Signer<IdentityPublicKey>`.
    /// Required for external-signable wallets (no seed Rust-side) and the
    /// architecturally correct path per `swift-sdk/CLAUDE.md`.
    ///
    /// The identity is still looked up from the in-process
    /// `IdentityManager` so the local balance bookkeeping in
    /// `ManagedIdentity` stays consistent.
    pub async fn withdraw_credits_with_external_signer<S>(
        &self,
        identity_id: &Identifier,
        amount: u64,
        to_address: &DashAddress,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<(), PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        let identity = {
            let wm = self.wallet_manager.read().await;
            let info = wm.get_wallet_info(&self.wallet_id).ok_or_else(|| {
                crate::error::PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?;
            let manager = &info.identity_manager;
            manager
                .identity(identity_id)
                .map(|m| m.identity.clone())
                .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?
        };

        let new_balance = identity
            .withdraw(
                &self.sdk,
                Some(to_address.clone()),
                amount,
                None, // core_fee_per_byte
                None, // signing_withdrawal_key_to_use
                SignerRef(signer),
                settings,
            )
            .await
            .map_err(|e| {
                // Preserve a structured key-unavailable signer failure so the
                // FFI boundary can still restore code 31; only genuine
                // operation failures get stringified into `InvalidIdentityData`.
                crate::error::preserve_signer_key_unavailable_or(e, |e| {
                    PlatformWalletError::InvalidIdentityData(format!(
                        "Failed to withdraw credits: {}",
                        e
                    ))
                })
            })?;

        {
            let mut wm = self.wallet_manager.write().await;
            let info_guard = wm.get_wallet_info_mut(&self.wallet_id).ok_or_else(|| {
                crate::error::PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?;
            if let Some(managed) = info_guard.identity_manager.identity_mut(identity_id) {
                managed.identity.set_balance(new_balance);
                if let Err(e) = self.persister.store(managed.snapshot_changeset().into()) {
                    tracing::error!(
                        identity = %identity_id,
                        error = %e,
                        "Failed to persist identity balance update after withdraw (external signer)"
                    );
                }
            }
        }

        Ok(())
    }

    /// Withdraw credits using an externally-provided identity and signer.
    ///
    /// Unlike [`Self::withdraw_credits_with_external_signer`], this method does
    /// **not** look up the identity in the internal `IdentityManager`. Instead,
    /// the caller supplies the `Identity` object and a `Signer` implementation
    /// directly. This is useful when the caller manages identities outside of
    /// the platform-wallet `IdentityManager` (e.g. evo-tool's
    /// `QualifiedIdentity`).
    ///
    /// Returns the remaining credit balance after the withdrawal.
    #[allow(clippy::too_many_arguments)]
    pub async fn withdraw_credits_with_signer<S: Signer<IdentityPublicKey> + Send>(
        &self,
        identity: &Identity,
        to_address: Option<DashAddress>,
        amount: u64,
        signing_withdrawal_key_to_use: Option<&IdentityPublicKey>,
        signer: S,
        settings: Option<PutSettings>,
    ) -> Result<u64, dash_sdk::Error> {
        identity
            .withdraw(
                &self.sdk,
                to_address,
                amount,
                Some(1), // core_fee_per_byte
                signing_withdrawal_key_to_use,
                signer,
                settings,
            )
            .await
    }
}

/// Select the OWNER-purpose `IdentityPublicKey` on a masternode identity
/// whose key material matches `owner_key_hash160` — the hash160 of the
/// wallet-derived provider owner key.
///
/// Match rule: purpose `OWNER`, key type `ECDSA_HASH160`, and 20-byte data
/// equal to `owner_key_hash160` (a masternode's owner key is registered as
/// the hash160 in the ProRegTx). Returns `None` when no such key exists —
/// the caller **must not broadcast** in that case: signing an
/// identity-credit-withdrawal with a key the identity doesn't recognise
/// produces an invalid transition (rejected, but still a wasted attempt),
/// so a distinct error is surfaced instead.
///
/// Pure — no derivation or network. Unit-tested below; the
/// derive-and-sign orchestration in
/// `PlatformWallet::masternode_withdraw` feeds it the wallet-derived owner
/// key's hash160.
pub fn select_owner_withdrawal_key<'a, I>(
    identity_keys: I,
    owner_key_hash160: &[u8; 20],
) -> Option<&'a IdentityPublicKey>
where
    I: IntoIterator<Item = &'a IdentityPublicKey>,
{
    select_hash160_key(identity_keys, Purpose::OWNER, owner_key_hash160)
}

/// Select the TRANSFER-purpose `IdentityPublicKey` on a masternode identity
/// whose key material matches `payout_key_hash160` — the pubkey hash of the
/// node's registered P2PKH payout script. Platform registers that script's
/// pubkey hash as the identity's transfer key, so this is the key that can
/// withdraw to a chosen destination.
///
/// Same contract as [`select_owner_withdrawal_key`]: pure, and `None` means
/// "do not broadcast".
pub fn select_transfer_withdrawal_key<'a, I>(
    identity_keys: I,
    payout_key_hash160: &[u8; 20],
) -> Option<&'a IdentityPublicKey>
where
    I: IntoIterator<Item = &'a IdentityPublicKey>,
{
    select_hash160_key(identity_keys, Purpose::TRANSFER, payout_key_hash160)
}

fn select_hash160_key<'a, I>(
    identity_keys: I,
    purpose: Purpose,
    key_hash160: &[u8; 20],
) -> Option<&'a IdentityPublicKey>
where
    I: IntoIterator<Item = &'a IdentityPublicKey>,
{
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::identity::KeyType;
    identity_keys.into_iter().find(|key| {
        key.purpose() == purpose
            && key.key_type() == KeyType::ECDSA_HASH160
            && key.data().as_slice() == key_hash160.as_slice()
    })
}

#[cfg(test)]
mod masternode_withdrawal_tests {
    use super::{select_owner_withdrawal_key, select_transfer_withdrawal_key};
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};

    fn make_key(id: u32, purpose: Purpose, key_type: KeyType, data: Vec<u8>) -> IdentityPublicKey {
        IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id,
            purpose,
            security_level: SecurityLevel::CRITICAL,
            contract_bounds: None,
            key_type,
            read_only: false,
            data: dpp::platform_value::BinaryData::new(data),
            disabled_at: None,
        })
    }

    #[test]
    fn selects_the_matching_owner_hash160_key_over_decoys() {
        let owner_hash = [0x11u8; 20];
        let other_hash = [0x22u8; 20];
        let keys = [
            // Right hash, wrong purpose.
            make_key(
                0,
                Purpose::TRANSFER,
                KeyType::ECDSA_HASH160,
                owner_hash.to_vec(),
            ),
            // OWNER but wrong key type.
            make_key(
                1,
                Purpose::OWNER,
                KeyType::ECDSA_SECP256K1,
                owner_hash.to_vec(),
            ),
            // OWNER hash160 but a different hash.
            make_key(
                2,
                Purpose::OWNER,
                KeyType::ECDSA_HASH160,
                other_hash.to_vec(),
            ),
            // The one and only match.
            make_key(
                3,
                Purpose::OWNER,
                KeyType::ECDSA_HASH160,
                owner_hash.to_vec(),
            ),
        ];

        let selected = select_owner_withdrawal_key(keys.iter(), &owner_hash);
        assert!(selected.is_some(), "must find the matching OWNER key");
        assert_eq!(selected.unwrap().id(), 3);
    }

    #[test]
    fn returns_none_when_no_owner_key_matches() {
        let owner_hash = [0x11u8; 20];
        let keys = [
            make_key(
                0,
                Purpose::TRANSFER,
                KeyType::ECDSA_HASH160,
                owner_hash.to_vec(),
            ),
            make_key(
                1,
                Purpose::OWNER,
                KeyType::ECDSA_HASH160,
                [0x22u8; 20].to_vec(),
            ),
        ];
        assert!(
            select_owner_withdrawal_key(keys.iter(), &owner_hash).is_none(),
            "no OWNER key with this hash ⇒ None (caller must not broadcast)"
        );
    }

    #[test]
    fn transfer_selector_only_matches_transfer_hash160_keys() {
        let payout_hash = [0x33u8; 20];
        let keys = [
            // OWNER key with the payout hash — wrong purpose.
            make_key(
                0,
                Purpose::OWNER,
                KeyType::ECDSA_HASH160,
                payout_hash.to_vec(),
            ),
            // TRANSFER but a full pubkey, not a hash160.
            make_key(
                1,
                Purpose::TRANSFER,
                KeyType::ECDSA_SECP256K1,
                payout_hash.to_vec(),
            ),
            // The match.
            make_key(
                2,
                Purpose::TRANSFER,
                KeyType::ECDSA_HASH160,
                payout_hash.to_vec(),
            ),
        ];
        let selected = select_transfer_withdrawal_key(keys.iter(), &payout_hash);
        assert_eq!(selected.map(|k| k.id()), Some(2));
        assert!(select_transfer_withdrawal_key(keys.iter(), &[0x44u8; 20]).is_none());
        // The owner selector never returns the transfer key and vice versa.
        assert_eq!(
            select_owner_withdrawal_key(keys.iter(), &payout_hash).map(|k| k.id()),
            Some(0)
        );
    }
}
