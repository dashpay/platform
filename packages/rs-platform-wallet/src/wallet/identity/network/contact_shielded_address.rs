//! The shielded payment address a contact request carries.

use super::contact_requests::ContactCryptoProvider;
use super::identity_handle::IdentityWallet;
#[cfg(feature = "shielded")]
use super::identity_handle::{identity_auth_derivation_path_for_type, MASTER_KEY_INDEX};
use crate::broadcaster::TransactionBroadcaster;
use crate::error::PlatformWalletError;
#[cfg(feature = "shielded")]
use crate::wallet::identity::ManagedIdentity;
#[cfg(feature = "shielded")]
use crate::wallet::platform_wallet::PlatformWalletInfo;
#[cfg(feature = "shielded")]
use crate::wallet::shielded::shielded_identity_account_index;
#[cfg(feature = "shielded")]
use crate::wallet::shielded::tips::carries_master_key;
#[cfg(feature = "shielded")]
use dash_sdk::platform::dashpay::dashpay_contact_requests_carry_shielded_address;
#[cfg(feature = "shielded")]
use dash_sdk::platform::Fetch;
#[cfg(feature = "shielded")]
use dpp::data_contracts::SystemDataContract;
#[cfg(feature = "shielded")]
use dpp::prelude::DataContract;
use dpp::prelude::Identifier;
#[cfg(feature = "shielded")]
use dpp::util::hash::ripemd160_sha256;
#[cfg(feature = "shielded")]
use key_wallet::bip32::KeyDerivationType;
#[cfg(feature = "shielded")]
use key_wallet_manager::WalletManager;

/// Log why `owner_id`'s contact request goes without a shielded address. The
/// request is still sent, so this is the only trace of the reason.
#[cfg(feature = "shielded")]
fn without_address(owner_id: &Identifier, reason: &str) {
    tracing::debug!(
        identity = %owner_id,
        reason,
        "contact request sent without a shielded address"
    );
}

/// The slot (identity index) an identity is recorded at and the shielded account that
/// slot selects (`shielded_identity_account_index`), named so the two are not swapped.
#[cfg(feature = "shielded")]
#[derive(Clone, Copy)]
struct IdentityShieldedAccount {
    slot: u32,
    account: u32,
}

impl<B: TransactionBroadcaster + ?Sized> IdentityWallet<B> {
    /// The shielded payment address `owner_id` gives `to_user_id` in the contact request
    /// it sends under `account_reference`, or `None` when the request goes without one.
    ///
    /// The address is the one the identity's shielded account derives for this request
    /// alone (`AccountViewingKeys::contact_address`), so every contact gets a different
    /// one and a payment that arrives on it is attributed to that contact. It is `None`
    /// unless all of these hold:
    ///
    /// - the wallet holds the identity and records the slot (identity index) it was
    ///   created at, which selects its shielded account
    ///   (`shielded_identity_account_index`);
    /// - that account's viewing keys are bound;
    /// - the connected network's contact requests have the property that carries the
    ///   address. A bundled schema does not show the network has activated it;
    /// - the identity carries the ECDSA MASTER authentication key that `crypto` derives
    ///   from the wallet's seed at that slot. A recorded slot alone is not proof of the
    ///   account: hosts restore an identity at a placeholder slot, and discovery can
    ///   move one. The key proves the slot, so only the account of a proved slot derives
    ///   an address that is given away. A disabled key still proves it: rotating the
    ///   master key does not change the slot the identity was created at.
    ///
    /// The send proves the slot itself, through `crypto`; nothing has to be prepared
    /// beforehand. When `crypto` cannot derive the key, the request goes without an
    /// address rather than failing. The network is asked only once the slot is proved.
    ///
    /// The send is refused when the address would be one this identity has given
    /// another contact. That takes a collision in 87 bits of a keyed hash. The check
    /// runs only once the network is found to carry the address, so a send that goes
    /// without one is never refused for it.
    ///
    /// The address is derived from the account's keys as they are bound when it is
    /// derived, never from keys read before the provider was asked. A coordinator Clear
    /// leaves those keys in memory, so an address is still given from them after one,
    /// until the wallet is bound again.
    ///
    /// The key is derived for the slot the identity holds when this is called, and the
    /// slot is checked again with the key as the address is derived: an identity
    /// recorded at another slot by then gets no address, never the address of another
    /// account.
    ///
    /// The identity the address is derived for is the one that sends the request. The
    /// public send calls this with the pair claimed (`DashPayView::claim_public_pair`)
    /// and keeps the claim until the request is recorded, and an identity with a claim
    /// held is not removed (`IdentityManager::remove_identity`), so it is not replaced
    /// under its id while the request is signed and broadcast. A refresh from Platform
    /// adds keys or disables them, and a disabled key still proves the slot.
    pub(crate) async fn contact_shielded_address<C>(
        &self,
        owner_id: &Identifier,
        to_user_id: &Identifier,
        account_reference: u32,
        crypto: &C,
    ) -> Result<Option<[u8; 43]>, PlatformWalletError>
    where
        C: ContactCryptoProvider + Sync,
    {
        #[cfg(test)]
        if let Some(address) = test_hooks::given() {
            return Ok(Some(address));
        }
        #[cfg(not(feature = "shielded"))]
        {
            let _ = (owner_id, to_user_id, account_reference, crypto);
            Ok(None)
        }
        #[cfg(feature = "shielded")]
        {
            // The slot the identity holds now, and the account it selects.
            let held = {
                let wm = self.wallet_manager.read().await;
                let Some((held, _)) = self.held_identity_account(&wm, owner_id) else {
                    return Ok(None);
                };
                held
            };
            // Only whether the account is bound, so an unbound wallet asks the provider
            // for nothing. The keys are read again where the address is derived.
            if self
                .contact_shielded_addresses
                .account_keys(held.account)
                .await
                .is_none()
            {
                without_address(owner_id, "the identity's shielded account is not bound");
                return Ok(None);
            }
            let Some(master_key_hash) =
                self.seed_master_key_hash(owner_id, held.slot, crypto).await
            else {
                return Ok(None);
            };
            // Checked before the network is asked, so a send whose identity does not
            // prove its slot reads nothing from it.
            {
                let wm = self.wallet_manager.read().await;
                if self
                    .proved_identity(&wm, owner_id, held.account, &master_key_hash)
                    .is_none()
                {
                    return Ok(None);
                }
            }
            if !self.contact_requests_carry_shielded_address().await? {
                without_address(
                    owner_id,
                    "the network's contact requests do not carry a shielded address yet",
                );
                return Ok(None);
            }

            // The waits above released both locks, and meanwhile the identity can have
            // been removed and added back, at another slot or with other keys, and the
            // wallet can have been bound again with other keys for the account. The
            // keys are read again here and held until the address is derived: no bind
            // replaces them in between, as a bind takes the key map for writing. The
            // identity is proved again under the wallet-manager lock, with no wait
            // between it and the derivation. Holding the key guard while that lock is
            // taken cannot deadlock: no path takes the key map while it holds the
            // wallet manager. What this does not cover is a change after the address
            // is derived; the paragraph on the pair claim above says what keeps the
            // identity in place from here, and keys bound again after this return are
            // the concern of the bind.
            let Some(keys) = self
                .contact_shielded_addresses
                .account_keys(held.account)
                .await
            else {
                without_address(
                    owner_id,
                    "the identity's shielded account is no longer bound",
                );
                return Ok(None);
            };
            let wm = self.wallet_manager.read().await;
            let Some(managed) = self.proved_identity(&wm, owner_id, held.account, &master_key_hash)
            else {
                return Ok(None);
            };
            // Compared with the requests sent or held so far. A send to another contact
            // in flight at this moment is not among them: for the two to share an
            // address takes the same 87-bit collision between two requests signed at
            // once, and a payment to it would still reach this identity's account.
            let others = managed.other_sent_account_references(to_user_id);

            let owner = owner_id.as_bytes();
            let reference = u64::from(account_reference);
            let index = keys.contact_index(owner, to_user_id.as_bytes(), reference);
            let taken = others.iter().any(|(other, other_reference)| {
                keys.contact_index(owner, other.as_bytes(), u64::from(*other_reference)) == index
            });
            if taken {
                return Err(PlatformWalletError::InvalidIdentityData(
                    "The shielded address of this contact request is one already given to \
                     another contact; send the request under another account reference"
                        .to_string(),
                ));
            }
            Ok(Some(
                keys.contact_address(owner, to_user_id.as_bytes(), reference)
                    .to_raw_address_bytes(),
            ))
        }
    }

    /// The slot `owner_id` holds and the shielded account it selects, with the identity,
    /// when `wm` holds the identity for this wallet with a slot recorded. `None`
    /// otherwise, with the reason logged. The slot is not proved here.
    #[cfg(feature = "shielded")]
    fn held_identity_account<'a>(
        &self,
        wm: &'a WalletManager<PlatformWalletInfo>,
        owner_id: &Identifier,
    ) -> Option<(IdentityShieldedAccount, &'a ManagedIdentity)> {
        let Some(managed) = wm
            .get_wallet_info(&self.wallet_id)
            .and_then(|info| info.identity_manager.managed_identity(owner_id))
            .filter(|identity| identity.wallet_id == Some(self.wallet_id))
        else {
            without_address(owner_id, "the wallet does not hold the identity");
            return None;
        };
        let Some(held) = managed.identity_index.and_then(|slot| {
            shielded_identity_account_index(slot)
                .ok()
                .map(|account| IdentityShieldedAccount { slot, account })
        }) else {
            without_address(owner_id, "the identity has no shielded account");
            return None;
        };
        Some((held, managed))
    }

    /// The identity `owner_id` as `wm` holds it, when its slot still selects `account`
    /// and it carries the MASTER key whose hash is `master_key_hash`
    /// (`carries_master_key`), the key the seed derives at that slot. `None` otherwise,
    /// with the reason logged.
    #[cfg(feature = "shielded")]
    fn proved_identity<'a>(
        &self,
        wm: &'a WalletManager<PlatformWalletInfo>,
        owner_id: &Identifier,
        account: u32,
        master_key_hash: &[u8; 20],
    ) -> Option<&'a ManagedIdentity> {
        let (held, managed) = self.held_identity_account(wm, owner_id)?;
        if held.account != account {
            without_address(
                owner_id,
                "the identity moved to another slot while the address was derived",
            );
            return None;
        }
        if !carries_master_key(&managed.identity, master_key_hash) {
            without_address(
                owner_id,
                "the identity does not carry the master key the wallet's seed derives \
                 at its slot",
            );
            return None;
        }
        Some(managed)
    }

    /// The hash of the ECDSA MASTER authentication key `crypto` derives at `slot`, the
    /// key an identity created at that slot carries (as
    /// `derive_identity_auth_key_hash_from_master` derives it from the seed). `None`
    /// when `crypto` does not derive it, with the reason logged: a contact request does
    /// not fail because its address cannot be proved.
    #[cfg(feature = "shielded")]
    pub(crate) async fn seed_master_key_hash<C>(
        &self,
        owner_id: &Identifier,
        slot: u32,
        crypto: &C,
    ) -> Option<[u8; 20]>
    where
        C: ContactCryptoProvider + Sync,
    {
        let derived = match identity_auth_derivation_path_for_type(
            self.sdk.network,
            KeyDerivationType::ECDSA,
            slot,
            MASTER_KEY_INDEX,
        ) {
            Ok(path) => crypto.receiving_xpub(&path).await,
            Err(error) => Err(error),
        };
        match derived {
            Ok(xpub) => Some(ripemd160_sha256(&xpub.public_key.to_compressed())),
            Err(error) => {
                without_address(
                    owner_id,
                    &format!(
                        "the crypto provider did not derive the identity's master key: {error}"
                    ),
                );
                None
            }
        }
    }

    /// Whether the connected network's DashPay contract gives contact requests the
    /// property that carries a shielded address, which it does from DashPay v3 at
    /// protocol version 15. The contract is read from the network: the bundled schema
    /// is the latest one, and an SDK seeded at the latest version resolves it before
    /// the network has activated it. What the property is, the SDK alone says
    /// (`dashpay_contact_requests_carry_shielded_address`).
    #[cfg(feature = "shielded")]
    async fn contact_requests_carry_shielded_address(&self) -> Result<bool, PlatformWalletError> {
        let contract = DataContract::fetch(&self.sdk, SystemDataContract::Dashpay.id())
            .await?
            .ok_or_else(|| {
                PlatformWalletError::InvalidIdentityData(
                    "DashPay contract is not available".to_string(),
                )
            })?;
        Ok(dashpay_contact_requests_carry_shielded_address(&contract))
    }
}

/// A test's own answer to [`IdentityWallet::contact_shielded_address`], for tests of how
/// an address a request carries is read, stored and forgotten. They need a request that
/// carries a known address and not the wallet state the derivation takes it from, which
/// `shielded/contact_address_tests.rs` covers.
#[cfg(test)]
pub(crate) mod test_hooks {
    use std::cell::Cell;

    thread_local! {
        static GIVEN: Cell<Option<[u8; 43]>> = const { Cell::new(None) };
    }

    /// While it lives, every contact request sent from this thread carries the address.
    /// Dropped, it puts back what the thread's requests carried before it.
    #[must_use = "the address is given only while the guard lives"]
    pub(crate) struct GivenAddress(Option<[u8; 43]>);

    /// Have every contact request this thread sends carry `address`.
    pub(crate) fn give_every_contact(address: [u8; 43]) -> GivenAddress {
        GivenAddress(GIVEN.replace(Some(address)))
    }

    impl Drop for GivenAddress {
        fn drop(&mut self) {
            GIVEN.set(self.0);
        }
    }

    pub(super) fn given() -> Option<[u8; 43]> {
        GIVEN.get()
    }
}
