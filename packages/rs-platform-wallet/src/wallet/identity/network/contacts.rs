//! Established contacts + DIP-14/15 contact key derivation + external account registration.

use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::Identity;
use dpp::prelude::Identifier;
use key_wallet::account::AccountType;
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;

use super::*;
use crate::broadcaster::TransactionBroadcaster;
use crate::changeset::{AccountRegistrationEntry, PlatformWalletChangeSet};
use crate::error::PlatformWalletError;
use crate::wallet::identity::types::dashpay::established_contact::EstablishedContact;
use crate::wallet::identity::types::dashpay::payment::DashpayAddressMatch;
use crate::wallet::platform_wallet::PlatformWalletInfo;

/// Build the persistence round for a newly registered DashPay account
/// (`DashpayReceivingFunds` / `DashpayExternalAccount`): the
/// [`AccountRegistrationEntry`] plus the account's initial address-pool
/// snapshot — the same shape `wallet_lifecycle` emits at wallet
/// creation. Without this round the account exists only in memory and
/// vanishes on relaunch, so its persisted UTXOs are dropped at the next
/// load (`load: ... dropped_no_account`) and received-payment history
/// can't be rebuilt.
fn dashpay_account_registration_changeset(
    account_type: AccountType,
    account_xpub: key_wallet::bip32::ExtendedPubKey,
    managed: &key_wallet::managed_account::ManagedCoreFundsAccount,
) -> PlatformWalletChangeSet {
    PlatformWalletChangeSet {
        account_registrations: vec![AccountRegistrationEntry {
            account_type,
            account_xpub,
        }],
        account_address_pools: crate::changeset::account_address_pool_entries(
            account_type,
            managed.managed_account_type().address_pools(),
        ),
        ..Default::default()
    }
}

/// Why a [`register_external_contact_account`] attempt failed, classified
/// for the payment-channel policy.
///
/// The distinction is load-bearing:
/// - **Permanent** marks the contact's payment channel broken (no unbounded
///   retry on a poisoned channel).
/// - **Transient** leaves the channel intact so the next sync sweep retries.
///
/// (In the seedless model this method no longer derives the ECDH scalar — the
/// caller passes a signer-derived `shared_key` — so a "key material
/// unavailable" classification no longer arises here; that DEFER decision now
/// lives at the drain's provider call.)
///
/// [`register_external_contact_account`]: IdentityWallet::register_external_contact_account
#[derive(Debug)]
pub enum RegisterExternalError {
    /// The request itself is unusable and re-deriving won't help — a
    /// malformed encrypted xpub, a missing/non-secp recipient key. Mark the
    /// channel broken.
    Permanent(PlatformWalletError),
    /// A local persistence / in-memory-insert hiccup — the account simply
    /// wasn't built this pass. Leave the channel intact; the next sweep
    /// retries.
    Transient(PlatformWalletError),
}

/// Outcome of a successful [`register_external_contact_account`] call.
///
/// The distinction matters to the rotation self-heal: only a [`Built`]
/// outcome proves the account was (re)constructed from the payload the
/// caller holds, so only [`Built`] may stamp the contact's
/// `external_account_reference` marker. An [`AlreadyExisted`] row is keyed
/// on `(index, user, friend)` — independent of `account_reference` — and
/// may predate a rotation (stale xpub); stamping it as current would stop
/// `external_account_needs_rebuild` from ever tearing it down.
///
/// [`Built`]: ExternalAccountRegistration::Built
/// [`AlreadyExisted`]: ExternalAccountRegistration::AlreadyExisted
/// [`register_external_contact_account`]: IdentityWallet::register_external_contact_account
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalAccountRegistration {
    /// The account was decrypted and (re)built from the supplied encrypted
    /// xpub by this call.
    Built,
    /// An account row already existed for the `(index, user, friend)` key;
    /// nothing was decrypted or replaced.
    AlreadyExisted,
}

impl RegisterExternalError {
    /// Whether this failure should permanently break the payment channel.
    pub fn is_permanent(&self) -> bool {
        matches!(self, RegisterExternalError::Permanent(_))
    }

    /// Unwrap to the underlying error (all arms carry one) for callers
    /// that don't act on the classification.
    pub fn into_inner(self) -> PlatformWalletError {
        match self {
            RegisterExternalError::Permanent(e) | RegisterExternalError::Transient(e) => e,
        }
    }
}

// ---------------------------------------------------------------------------
// Established contacts accessor
// ---------------------------------------------------------------------------

impl<B: TransactionBroadcaster + ?Sized> DashPayView<'_, B> {
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
            .flat_map(|managed| managed.dashpay().established_contacts().values().cloned())
            .collect();
        for inner in info.identity_manager.wallet_identities.values() {
            for managed in inner.values() {
                out.extend(managed.dashpay().established_contacts().values().cloned());
            }
        }
        out
    }
}

// ---------------------------------------------------------------------------
// Contact xpub and payment address derivation (DIP-14 / DIP-15)
// ---------------------------------------------------------------------------

impl<B: TransactionBroadcaster + ?Sized> DashPayView<'_, B> {
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
        // Our receiving (friendship) xpub, derived by the Keychain signer. There
        // is no resident-seed path — every caller supplies it.
        account_xpub: key_wallet::bip32::ExtendedPubKey,
    ) -> Result<(), PlatformWalletError> {
        let account_type = AccountType::DashpayReceivingFunds {
            index: account_index,
            user_identity_id: our_identity_id.to_buffer(),
            friend_identity_id: contact_identity_id.to_buffer(),
        };

        // Derive the account xpub and add to both Wallet and ManagedWalletInfo
        let mut wm = self.wallet_manager.write().await;

        // Early-exit if the account already exists — keeps the recurring
        // sweep's re-registration a true no-op (no duplicate persistence
        // round, no managed-state churn).
        {
            use key_wallet::account::account_collection::DashpayAccountKey;
            let info = wm
                .get_wallet_info(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
            let key = DashpayAccountKey {
                index: account_index,
                user_identity_id: our_identity_id.to_buffer(),
                friend_identity_id: contact_identity_id.to_buffer(),
            };
            if info
                .core_wallet
                .accounts
                .dashpay_receival_accounts
                .contains_key(&key)
            {
                return Ok(());
            }
        }

        let wallet = wm
            .get_wallet(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;

        let account = key_wallet::Account {
            parent_wallet_id: Some(wallet.wallet_id),
            account_type,
            network: self.sdk.network,
            account_xpub,
            is_watch_only: false,
        };

        // DashPay accounts are funds-bearing; use the typed
        // `insert_funds_bearing_account` API exposed by the post-split
        // collection rather than wrapping in `OwnedManagedCoreAccount`.
        let managed = key_wallet::managed_account::ManagedCoreFundsAccount::from_account(&account);

        // Persist the registration BEFORE the in-memory inserts: a store
        // failure aborts with nothing mutated, while an insert failure
        // after a successful store leaves only a benign extra row that
        // the next load restores into a valid (empty) account.
        self.persister
            .store(dashpay_account_registration_changeset(
                account_type,
                account_xpub,
                &managed,
            ))
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to persist contact account registration: {e}"
                ))
            })?;

        let (wallet, info) = wm
            .get_wallet_mut_and_info_mut(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;

        // Mirror the restored shape: the immutable `wallet.accounts`
        // collection holds the Account (like `build_wallet_start_state`
        // recreates it at load), the managed collection holds pools +
        // UTXO state.
        wallet
            .add_account(account_type, Some(account_xpub))
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to add contact account to wallet: {e}"
                ))
            })?;
        info.core_wallet
            .accounts
            .insert_funds_bearing_account(managed)
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to register contact account: {e}"
                ))
            })?;

        tracing::info!(
            our_identity = %our_identity_id,
            contact = %contact_identity_id,
            "Registered DashpayReceivingFunds account for receiving payments from contact"
        );

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
            } = account.managed_account_type()
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
}

// ---------------------------------------------------------------------------
// External contact account registration (sending)
// ---------------------------------------------------------------------------

impl<B: TransactionBroadcaster + ?Sized> DashPayView<'_, B> {
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
    /// * `contact_identity`           - The contact's **already-fetched** identity. The
    ///                                  caller fetches it once (for the key-index validation
    ///                                  that must precede ECDH) and passes it in, so this
    ///                                  method performs **no network I/O** — every failure it
    ///                                  returns is therefore a permanent crypto/data fault,
    ///                                  not a transient DAPI blip.
    /// * `contact_encrypted_xpub`     - 96-byte encrypted xpub from the contact's
    ///                                  `contactRequest` document (16-byte IV + 80-byte
    ///                                  AES-256-CBC ciphertext).
    /// * `shared_key`                 - The ECDH shared secret, computed by the Keychain
    ///                                  signer (the raw scalar never enters this crate). The
    ///                                  caller derives it through the `ContactCryptoProvider`;
    ///                                  the key indices it used live with the caller.
    ///
    /// Returns [`RegisterExternalError`] so the caller can apply the
    /// transient/permanent payment-channel policy: a `Permanent` failure
    /// (malformed encrypted xpub, missing/non-secp key) breaks the channel;
    /// a `Transient` one (persistence/insert hiccup) leaves it for retry.
    /// On success returns [`ExternalAccountRegistration`] so the caller can
    /// tell a real (re)build from an already-existed no-op — only the former
    /// may stamp the rotation self-heal marker (see the enum docs).
    ///
    /// **Never wrap this call in a timeout.** Unlike its sibling
    /// [`Self::register_contact_account`], which persists inside the write lock
    /// it already holds, this one persists *before* acquiring the lock — so
    /// there is a genuine `.await` between the durable write and the in-memory
    /// inserts. A future dropped in that window leaves an account on disk that
    /// no in-memory collection knows about for the rest of the process's life;
    /// a real crash reloads it from disk, an in-process cancellation does not.
    /// (The next drain does re-register it idempotently, so this is a landmine
    /// rather than data loss — but the asymmetry with the sibling is not
    /// something to rediscover by analogy.)
    pub async fn register_external_contact_account(
        &self,
        our_identity_id: &Identifier,
        contact_identity: &Identity,
        contact_encrypted_xpub: &[u8],
        shared_key: zeroize::Zeroizing<[u8; 32]>,
    ) -> Result<ExternalAccountRegistration, RegisterExternalError> {
        use RegisterExternalError::{Permanent, Transient};
        let account_index: u32 = 0;
        let contact_identity_id = contact_identity.id();

        // --- 1. Early-exit if the external account already exists. ---
        {
            let wm = self.wallet_manager.read().await;
            let info = wm.get_wallet_info(&self.wallet_id).ok_or_else(|| {
                Transient(PlatformWalletError::WalletNotFound(hex::encode(
                    self.wallet_id,
                )))
            })?;
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
                return Ok(ExternalAccountRegistration::AlreadyExisted);
            }
        }

        // --- 2. Decrypt the contact's xpub with the signer-derived secret. ---
        let decrypted_xpub_bytes =
            platform_encryption::decrypt_extended_public_key(&shared_key, contact_encrypted_xpub)
                .map_err(|e| {
                Permanent(PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to decrypt contact xpub: {}",
                    e
                )))
            })?;

        // --- 3. Reconstruct the ExtendedPubKey from the decrypted plaintext. ---
        //
        // DIP-15 + both reference clients (iOS dash-shared-core, Android dashj)
        // use the 69-byte COMPACT form (fingerprint ‖ chaincode ‖ pubkey) —
        // the version/depth/child-number metadata is omitted on the wire and
        // reconstructed here from the known friendship-path context. Only
        // chain_code + public_key feed non-hardened ckd_pub, so reconstruction
        // yields identical payment addresses (pinned by
        // `reconstructed_xpub_derives_identical_addresses` in crypto::dip14).
        //
        // Backward-compat: a locally-stored legacy plaintext could be the old
        // 78/107-byte BIP32/DIP-14 serialization. Nothing nonconforming has
        // reached chain, but we keep one cheap fallback branch as insurance.
        let contact_xpub = match platform_encryption::parse_compact_xpub(&decrypted_xpub_bytes) {
            Ok(compact) => crate::wallet::identity::crypto::dip14::reconstruct_contact_xpub(
                compact,
                self.sdk.network,
            )
            .map_err(Permanent)?,
            Err(_) => {
                key_wallet::bip32::ExtendedPubKey::decode(&decrypted_xpub_bytes).map_err(|e| {
                    Permanent(PlatformWalletError::InvalidIdentityData(format!(
                        "Decrypted contact xpub is neither a 69-byte DIP-15 compact form \
                         nor a 78/107-byte BIP32/DIP-14 serialization: {e}"
                    )))
                })?
            }
        };

        // --- 4. Build the watch-only Account and register it. ---
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
        let managed = key_wallet::managed_account::ManagedCoreFundsAccount::from_account(&account);

        // Persist the registration BEFORE the in-memory inserts (same
        // rationale as `register_contact_account`): without this round
        // the account vanishes on relaunch and `send_payment` loses its
        // xpub + derived-address state until the next sweep rebuilds it.
        self.persister
            .store(dashpay_account_registration_changeset(
                account_type,
                contact_xpub,
                &managed,
            ))
            .map_err(|e| {
                Transient(PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to persist external contact account registration: {e}"
                )))
            })?;

        let mut wm = self.wallet_manager.write().await;
        let (wallet, info) = wm
            .get_wallet_mut_and_info_mut(&self.wallet_id)
            .ok_or_else(|| {
                Transient(PlatformWalletError::WalletNotFound(hex::encode(
                    self.wallet_id,
                )))
            })?;

        // (a) Insert Account into the immutable wallet account collection so the
        //     xpub is accessible by `send_payment`.
        wallet
            .add_account(account_type, Some(contact_xpub))
            .map_err(|e| {
                Transient(PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to add external contact account to wallet: {}",
                    e
                )))
            })?;

        // (b) Insert ManagedCoreFundsAccount for address-pool tracking.
        info.core_wallet
            .accounts
            .insert_funds_bearing_account(managed)
            .map_err(|e| {
                Transient(PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to register external contact account: {}",
                    e
                )))
            })?;

        tracing::info!(
            our_identity = %our_identity_id,
            contact = %contact_identity_id,
            "Registered DashpayExternalAccount for sending payments to contact"
        );

        Ok(ExternalAccountRegistration::Built)
    }
}
