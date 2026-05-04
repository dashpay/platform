//! Established contacts + DIP-14/15 contact key derivation + external account registration.

use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::Identity;
use dpp::prelude::Identifier;
use key_wallet::account::AccountType;

use super::*;
use crate::broadcaster::TransactionBroadcaster;
use crate::error::PlatformWalletError;
use crate::wallet::identity::types::dashpay::established_contact::EstablishedContact;
use crate::wallet::identity::types::dashpay::payment::DashpayAddressMatch;
use crate::wallet::platform_wallet::PlatformWalletInfo;

// ---------------------------------------------------------------------------
// Established contacts accessor
// ---------------------------------------------------------------------------

impl<B: TransactionBroadcaster + ?Sized> IdentityWallet<B> {
    // TODO: We don't want to clone all contacts on get - it's terrible.
    /// Get all established contacts across every identity managed by this wallet.
    ///
    /// Returns a flat list; each element includes the contact's identity ID.
    pub async fn established_contacts(&self) -> Vec<EstablishedContact> {
        let wm = self.wallet_manager.read().await;
        let Some(info) = wm.get_wallet_info(&self.wallet_id) else {
            return Vec::new();
        };
        // Flatten contacts across both buckets — observed identities
        // can hold contact requests too (received from a stranger we
        // haven't onboarded as wallet-owned yet). Touching the bucket
        // boundary explicitly keeps the iteration honest about what
        // it's reading.
        let mut out: Vec<EstablishedContact> = info
            .identity_manager
            .out_of_wallet_identities
            .values()
            .flat_map(|managed| managed.established_contacts.values().cloned())
            .collect();
        for inner in info.identity_manager.wallet_identities.values() {
            for managed in inner.values() {
                out.extend(managed.established_contacts.values().cloned());
            }
        }
        out
    }
}

// ---------------------------------------------------------------------------
// Contact xpub and payment address derivation (DIP-14 / DIP-15)
// ---------------------------------------------------------------------------

impl<B: TransactionBroadcaster + ?Sized> IdentityWallet<B> {
    /// Get the contact xpub data for a specific contact relationship.
    ///
    /// Derives the extended public key along path:
    /// `m/9'/coin'/15'/account'/(sender_id)/(recipient_id)`
    ///
    /// The last two segments use DIP-14 256-bit non-hardened derivation.
    ///
    /// # Arguments
    ///
    /// * `account_index` - Account index (hardened) in the derivation path.
    /// * `sender_id`     - Our identity identifier.
    /// * `recipient_id`  - The contact's identity identifier.
    pub async fn contact_xpub(
        &self,
        account_index: u32,
        sender_id: &Identifier,
        recipient_id: &Identifier,
    ) -> Result<crate::wallet::identity::crypto::dip14::ContactXpubData, PlatformWalletError> {
        let wm = self.wallet_manager.read().await;
        let wallet = wm
            .get_wallet(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
        crate::wallet::identity::crypto::dip14::derive_contact_xpub(
            wallet,
            self.sdk.network,
            account_index,
            sender_id,
            recipient_id,
        )
    }

    // TODO: Isn't this something what should be done internally?
    /// Derive payment addresses for a contact (for receiving payments from them).
    ///
    /// Returns `count` addresses starting from `start_index`, derived via
    /// standard BIP32 from the contact xpub.
    ///
    /// Register a DashPay contact account in the wallet's `ManagedWalletInfo`.
    ///
    /// Creates a `DashpayReceivingFunds` managed account with address pools
    /// so the SPV adapter monitors incoming payments from this contact.
    /// Call this when a contact is established (mutual requests exist).
    ///
    /// No-op if the account already exists for this contact relationship.
    pub async fn register_contact_account(
        &self,
        our_identity_id: &Identifier,
        contact_identity_id: &Identifier,
        account_index: u32,
    ) -> Result<(), PlatformWalletError> {
        let account_type = AccountType::DashpayReceivingFunds {
            index: account_index,
            user_identity_id: our_identity_id.to_buffer(),
            friend_identity_id: contact_identity_id.to_buffer(),
        };

        // Derive the account xpub and add to both Wallet and ManagedWalletInfo
        let mut wm = self.wallet_manager.write().await;
        let wallet = wm
            .get_wallet(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
        let path = account_type
            .derivation_path(self.sdk.network)
            .map_err(|err| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to derive DashPay contact account path: {err}"
                ))
            })?;
        let account_xpub = wallet.derive_extended_public_key(&path).map_err(|err| {
            PlatformWalletError::InvalidIdentityData(format!(
                "Failed to derive DashPay contact xpub: {err}"
            ))
        })?;

        let account = key_wallet::Account {
            parent_wallet_id: Some(wallet.wallet_id),
            account_type,
            network: self.sdk.network,
            account_xpub,
            is_watch_only: false,
        };

        // Add managed wrapper to ManagedWalletInfo (address pools, state tracking)
        let info = wm
            .get_wallet_info_mut(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
        // DashPay accounts are funds-bearing; use the typed
        // `insert_funds` API exposed by the post-split collection
        // rather than wrapping in `OwnedManagedCoreAccount`.
        let managed =
            key_wallet::managed_account::ManagedCoreFundsAccount::from_account(&account);
        info.core_wallet.accounts.insert_funds(managed).map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!(
                "Failed to register contact account: {e}"
            ))
        })?;

        Ok(())
    }

    /// Match an on-chain address against this wallet's registered
    /// DashPay contact receival accounts.
    ///
    /// Iterates every `DashpayReceivingFunds` account in this
    /// wallet's [`key_wallet::managed_account::ManagedAccountCollection`]
    /// and checks whether the address belongs to any of their
    /// address pools. Returns the first match as a
    /// [`DashpayAddressMatch`], or `None` if the address is not
    /// a DashPay contact address for this wallet.
    ///
    /// Used by the SPV / backend task layer to classify observed
    /// transaction outputs as DashPay incoming payments from a
    /// specific contact — replaces the redundant
    /// `dashpay_address_mappings` reverse-lookup table the UI
    /// layer used to maintain. The authoritative state is already
    /// tracked by `register_contact_account`, which inserts the
    /// account into the wallet's `ManagedAccountCollection` so
    /// key-wallet manages the address pool (derivation + gap limit
    /// + used tracking).
    ///
    /// Only the external pool of each receival account is
    /// searched: DashPay uses a single-pool account type so all
    /// contact payment addresses live on that one pool.
    pub async fn match_incoming_dashpay_address(
        &self,
        address: &dashcore::Address,
    ) -> Option<DashpayAddressMatch> {
        let wm = self.wallet_manager.read().await;
        let info = wm.get_wallet_info(&self.wallet_id)?;
        Self::match_in_collection(info, address)
    }

    /// Blocking variant of [`match_incoming_dashpay_address`] for
    /// sync callers (SPV transaction-processing frame loop). Uses
    /// `tokio::sync::RwLock::blocking_read` — must NOT be called
    /// from within a tokio async context.
    pub fn match_incoming_dashpay_address_blocking(
        &self,
        address: &dashcore::Address,
    ) -> Option<DashpayAddressMatch> {
        let wm = self.wallet_manager.blocking_read();
        let info = wm.get_wallet_info(&self.wallet_id)?;
        Self::match_in_collection(info, address)
    }

    /// Non-blocking variant of [`match_incoming_dashpay_address`].
    /// Returns `Err(())` if the wallet-manager lock is currently
    /// contended (e.g. SPV is processing a block). Returns `Ok(None)`
    /// if the address does not belong to any DashPay receiving
    /// account. Safe to call from any thread, including tokio runtime
    /// threads, where the blocking variant would panic.
    #[allow(clippy::result_unit_err)]
    pub fn try_match_incoming_dashpay_address(
        &self,
        address: &dashcore::Address,
    ) -> Result<Option<DashpayAddressMatch>, ()> {
        let wm = self.wallet_manager.try_read().map_err(|_| ())?;
        let Some(info) = wm.get_wallet_info(&self.wallet_id) else {
            return Ok(None);
        };
        Ok(Self::match_in_collection(info, address))
    }

    /// Shared implementation that iterates
    /// `info.core_wallet.accounts.dashpay_receival_accounts` and
    /// checks each account's address pool for a match.
    pub(super) fn match_in_collection(
        info: &PlatformWalletInfo,
        address: &dashcore::Address,
    ) -> Option<DashpayAddressMatch> {
        use key_wallet::managed_account::managed_account_type::ManagedAccountType;

        for (key, account) in &info.core_wallet.accounts.dashpay_receival_accounts {
            let ManagedAccountType::DashpayReceivingFunds {
                user_identity_id,
                friend_identity_id,
                ..
            } = &account.managed_account_type
            else {
                // Routing invariant: dashpay_receival_accounts must
                // only contain DashpayReceivingFunds. If this ever
                // trips, it's a key-wallet bug.
                debug_assert!(
                    false,
                    "non-DashpayReceivingFunds in dashpay_receival_accounts"
                );
                continue;
            };
            let Some(info) = account.get_address_info(address) else {
                continue;
            };
            // Sanity check — the collection key should match the
            // account type's own identity ids.
            debug_assert_eq!(&key.user_identity_id, user_identity_id);
            debug_assert_eq!(&key.friend_identity_id, friend_identity_id);
            return Some(DashpayAddressMatch {
                user_identity_id: Identifier::from(*user_identity_id),
                friend_identity_id: Identifier::from(*friend_identity_id),
                address_index: info.index,
            });
        }
        None
    }

    /// # Arguments
    ///
    /// * `account_index` - Account index (hardened) in the derivation path.
    /// * `sender_id`     - Our identity identifier.
    /// * `recipient_id`  - The contact's identity identifier.
    /// * `start_index`   - First payment address index.
    /// * `count`         - Number of addresses to derive.
    pub async fn contact_payment_addresses(
        &self,
        account_index: u32,
        sender_id: &Identifier,
        recipient_id: &Identifier,
        start_index: u32,
        count: u32,
    ) -> Result<Vec<dashcore::Address>, PlatformWalletError> {
        let wm = self.wallet_manager.read().await;
        let wallet = wm
            .get_wallet(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
        let data = crate::wallet::identity::crypto::dip14::derive_contact_xpub(
            wallet,
            self.sdk.network,
            account_index,
            sender_id,
            recipient_id,
        )?;
        crate::wallet::identity::crypto::dip14::derive_contact_payment_addresses(
            &data.xpub,
            start_index,
            count,
            self.sdk.network,
        )
    }
}

// ---------------------------------------------------------------------------
// External contact account registration (sending)
// ---------------------------------------------------------------------------

impl<B: TransactionBroadcaster + ?Sized> IdentityWallet<B> {
    /// Register a watch-only `DashpayExternalAccount` for sending payments
    /// to a contact. Uses the contact's decrypted xpub from their
    /// `contactRequest.encrypted_public_key`.
    ///
    /// Called during contact establishment — once both parties have exchanged
    /// requests and we can decrypt the contact's xpub. The account is
    /// watch-only: we hold the contact's public key and derive their payment
    /// addresses from it. We never hold a private key for this account.
    ///
    /// No-op (returns `Ok(())`) if the external account already exists.
    ///
    /// # Arguments
    ///
    /// * `our_identity_id`            - Our identity that shares the contact relationship.
    /// * `contact_identity_id`        - The contact's identity.
    /// * `contact_encrypted_xpub`     - 96-byte encrypted xpub from the contact's
    ///                                  `contactRequest` document (16-byte IV + 80-byte
    ///                                  AES-256-CBC ciphertext).
    /// * `our_decryption_key_index`   - Key ID of our ENCRYPTION key used for ECDH.
    /// * `contact_encryption_key_index` - Key ID of the contact's ENCRYPTION key used for ECDH.
    pub async fn register_external_contact_account(
        &self,
        our_identity_id: &Identifier,
        contact_identity_id: &Identifier,
        contact_encrypted_xpub: &[u8],
        our_decryption_key_index: u32,
        contact_encryption_key_index: u32,
    ) -> Result<(), PlatformWalletError> {
        let account_index: u32 = 0;

        // --- 1. Early-exit if the external account already exists. ---
        {
            let wm = self.wallet_manager.read().await;
            let info = wm
                .get_wallet_info(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
            use key_wallet::account::account_collection::DashpayAccountKey;
            let key = DashpayAccountKey {
                index: account_index,
                user_identity_id: our_identity_id.to_buffer(),
                friend_identity_id: contact_identity_id.to_buffer(),
            };
            if info
                .core_wallet
                .accounts
                .dashpay_external_accounts
                .contains_key(&key)
            {
                return Ok(());
            }
        }

        // --- 2. Derive our ECDH private key under a read lock. ---
        let our_private_key = {
            let wm = self.wallet_manager.read().await;
            let info = wm
                .get_wallet_info(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
            let managed = info
                .identity_manager
                .managed_identity(our_identity_id)
                .ok_or(PlatformWalletError::IdentityNotFound(*our_identity_id))?;
            // ECDH key derivation needs the wallet HD slot — only valid
            // for wallet-owned identities. Reject the out-of-wallet case
            // explicitly rather than letting derivation produce a
            // misleading error downstream.
            let identity_index = managed
                .identity_index
                .ok_or(PlatformWalletError::IdentityIndexNotSet(*our_identity_id))?;

            // Find our decryption key by its key ID.
            let our_encryption_key = managed
                .identity
                .public_keys()
                .get(&our_decryption_key_index)
                .cloned()
                .ok_or_else(|| {
                    PlatformWalletError::InvalidIdentityData(format!(
                        "Our encryption key {} not found on identity {}",
                        our_decryption_key_index, our_identity_id
                    ))
                })?;

            let wallet = wm
                .get_wallet(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;

            Self::derive_encryption_private_key(
                wallet,
                self.sdk.network,
                identity_index,
                &our_encryption_key,
            )?
        };

        // --- 3. Fetch the contact's identity from Platform and extract their encryption pubkey. ---
        let contact_public_key: dashcore::secp256k1::PublicKey = {
            use dash_sdk::platform::Fetch;
            let contact_identity = Identity::fetch(&self.sdk, *contact_identity_id)
                .await
                .map_err(|e| {
                    PlatformWalletError::InvalidIdentityData(format!(
                        "Failed to fetch contact identity {}: {}",
                        contact_identity_id, e
                    ))
                })?
                .ok_or_else(|| PlatformWalletError::IdentityNotFound(*contact_identity_id))?;

            let contact_key = contact_identity
                .public_keys()
                .get(&contact_encryption_key_index)
                .cloned()
                .ok_or_else(|| {
                    PlatformWalletError::InvalidIdentityData(format!(
                        "Contact encryption key {} not found on identity {}",
                        contact_encryption_key_index, contact_identity_id
                    ))
                })?;

            // Deserialize the compressed public key bytes from the identity key data.
            dashcore::secp256k1::PublicKey::from_slice(contact_key.data().as_slice()).map_err(
                |e| {
                    PlatformWalletError::InvalidIdentityData(format!(
                        "Contact encryption key is not a valid secp256k1 public key: {}",
                        e
                    ))
                },
            )?
        };

        // --- 4. Derive the ECDH shared key. ---
        let shared_key: [u8; 32] =
            platform_encryption::derive_shared_key_ecdh(&our_private_key, &contact_public_key);

        // --- 5. Decrypt the contact's xpub. ---
        let decrypted_xpub_bytes =
            platform_encryption::decrypt_extended_public_key(&shared_key, contact_encrypted_xpub)
                .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to decrypt contact xpub: {}",
                    e
                ))
            })?;

        // --- 6. Reconstruct the ExtendedPubKey from the raw encoded bytes. ---
        let contact_xpub = key_wallet::bip32::ExtendedPubKey::decode(&decrypted_xpub_bytes)
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to decode contact xpub: {}",
                    e
                ))
            })?;

        // --- 7. Build the watch-only Account and register it. ---
        //
        // Two insertions are needed:
        //   a) `wallet.accounts` (immutable AccountCollection) — stores the Account with
        //      the contact's xpub so `send_payment` can retrieve it later for address
        //      derivation without carrying the xpub in a separate structure.
        //   b) `info.core_wallet.accounts` (ManagedAccountCollection) — stores the
        //      ManagedCoreAccount with pre-generated address pools so SPV can watch
        //      outbound addresses we have already derived for the contact.
        let account_type = AccountType::DashpayExternalAccount {
            index: account_index,
            user_identity_id: our_identity_id.to_buffer(),
            friend_identity_id: contact_identity_id.to_buffer(),
        };

        let account = key_wallet::Account {
            parent_wallet_id: Some(self.wallet_id),
            account_type,
            network: self.sdk.network,
            account_xpub: contact_xpub,
            is_watch_only: true,
        };

        // DashpayExternalAccount is funds-bearing; insert via the
        // typed `insert_funds` API after the upstream split.
        let managed =
            key_wallet::managed_account::ManagedCoreFundsAccount::from_account(&account);

        let mut wm = self.wallet_manager.write().await;
        let (wallet, info) = wm
            .get_wallet_mut_and_info_mut(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;

        // (a) Insert Account into the immutable wallet account collection so the
        //     xpub is accessible by `send_payment`.
        wallet
            .add_account(account_type, Some(contact_xpub))
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to add external contact account to wallet: {}",
                    e
                ))
            })?;

        // (b) Insert ManagedCoreFundsAccount for address-pool tracking.
        info.core_wallet.accounts.insert_funds(managed).map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!(
                "Failed to register external contact account: {}",
                e
            ))
        })?;

        tracing::info!(
            our_identity = %our_identity_id,
            contact = %contact_identity_id,
            "Registered DashpayExternalAccount for sending payments to contact"
        );

        Ok(())
    }
}
