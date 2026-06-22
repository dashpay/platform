//! Established contacts + DIP-14/15 contact key derivation + external account registration.

use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::Identity;
use dpp::prelude::Identifier;
use key_wallet::account::AccountType;
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;

use super::*;
use crate::broadcaster::TransactionBroadcaster;
use crate::changeset::{
    AccountAddressPoolEntry, AccountRegistrationEntry, PlatformWalletChangeSet,
};
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
    let mut cs = PlatformWalletChangeSet {
        account_registrations: vec![AccountRegistrationEntry {
            account_type,
            account_xpub,
        }],
        ..Default::default()
    };
    for pool in managed.managed_account_type().address_pools() {
        let addresses: Vec<key_wallet::AddressInfo> = pool.addresses.values().cloned().collect();
        if addresses.is_empty() {
            continue;
        }
        cs.account_address_pools.push(AccountAddressPoolEntry {
            account_type,
            pool_type: pool.pool_type,
            addresses,
        });
    }
    cs
}

/// Why a [`register_external_contact_account`] attempt failed, classified
/// for the payment-channel policy.
///
/// The three-way distinction is load-bearing:
/// - **Permanent** marks the contact's payment channel broken (no unbounded
///   retry on a poisoned channel).
/// - **Transient** leaves the channel intact so the next sync sweep retries.
/// - **Unavailable** means the key material to derive the ECDH scalar isn't
///   present *right now* (watch-only wallet / signer not unlocked); the build
///   is DEFERRED until a signer is available — neither broken nor churn-retried.
///
/// Misclassifying an `Unavailable` blip (e.g. a locked Keychain) as
/// `Permanent` silently and irreversibly kills payments to a contact over a
/// momentary, recoverable condition; misclassifying it as `Transient` churns a
/// doomed derivation every sweep. Both are wrong — hence the separate arm.
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
    /// The key material needed to derive the ECDH scalar isn't available right
    /// now — a watch-only wallet with no resident seed, or (in the seedless
    /// model) a Keychain signer that isn't unlocked. DEFER: leave the channel
    /// intact and do not churn-retry; the build runs once a signer is
    /// available. This is neither a malformed request nor a momentary infra
    /// hiccup.
    Unavailable(PlatformWalletError),
}

impl RegisterExternalError {
    /// Whether this failure should permanently break the payment channel.
    /// True only for a genuinely malformed request — never for `Unavailable`.
    pub fn is_permanent(&self) -> bool {
        matches!(self, RegisterExternalError::Permanent(_))
    }

    /// Whether the failure is "key material not available right now". The
    /// caller must DEFER (leave the channel intact, retry when a signer is
    /// available) — not break the channel and not churn-retry immediately.
    pub fn is_unavailable(&self) -> bool {
        matches!(self, RegisterExternalError::Unavailable(_))
    }

    /// Unwrap to the underlying error (all arms carry one) for callers
    /// that don't act on the classification.
    pub fn into_inner(self) -> PlatformWalletError {
        match self {
            RegisterExternalError::Permanent(e)
            | RegisterExternalError::Transient(e)
            | RegisterExternalError::Unavailable(e) => e,
        }
    }
}

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
    /// * `contact_identity`           - The contact's **already-fetched** identity. The
    ///                                  caller fetches it once (for the key-index validation
    ///                                  that must precede ECDH) and passes it in, so this
    ///                                  method performs **no network I/O** — every failure it
    ///                                  returns is therefore a permanent crypto/data fault,
    ///                                  not a transient DAPI blip.
    /// * `contact_encrypted_xpub`     - 96-byte encrypted xpub from the contact's
    ///                                  `contactRequest` document (16-byte IV + 80-byte
    ///                                  AES-256-CBC ciphertext).
    /// * `our_decryption_key_index`   - Key ID of our ENCRYPTION key used for ECDH.
    /// * `contact_encryption_key_index` - Key ID of the contact's ENCRYPTION key used for ECDH.
    ///
    /// Returns [`RegisterExternalError`] so the caller can apply the
    /// transient/permanent payment-channel policy: a `Permanent` failure
    /// (malformed encrypted xpub, missing/non-secp key) breaks the channel;
    /// a `Transient` one (persistence/insert hiccup) leaves it for retry.
    pub async fn register_external_contact_account(
        &self,
        our_identity_id: &Identifier,
        contact_identity: &Identity,
        contact_encrypted_xpub: &[u8],
        our_decryption_key_index: u32,
        contact_encryption_key_index: u32,
        // Seedless drain supplies the ECDH shared secret already computed by the
        // Keychain signer (the scalar never enters this crate). `None` = the
        // resident-seed path, which derives the scalar locally (steps 2–4).
        precomputed_shared_key: Option<[u8; 32]>,
    ) -> Result<(), RegisterExternalError> {
        use RegisterExternalError::{Permanent, Transient, Unavailable};
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
                return Ok(());
            }
        }

        // Obtain the ECDH shared secret: the seedless drain supplies it from the
        // Keychain signer (the scalar never enters this crate); otherwise derive
        // it from the resident seed (steps 2–4).
        let shared_key: [u8; 32] = if let Some(precomputed) = precomputed_shared_key {
            precomputed
        } else {
            // --- 2. Derive our ECDH private key under a read lock. ---
            let our_private_key = {
                let wm = self.wallet_manager.read().await;
                let info = wm.get_wallet_info(&self.wallet_id).ok_or_else(|| {
                    Transient(PlatformWalletError::WalletNotFound(hex::encode(
                        self.wallet_id,
                    )))
                })?;
                let managed = info
                    .identity_manager
                    .managed_identity(our_identity_id)
                    .ok_or_else(|| {
                        Transient(PlatformWalletError::IdentityNotFound(*our_identity_id))
                    })?;
                // ECDH key derivation needs the wallet HD slot — only valid
                // for wallet-owned identities. Reject the out-of-wallet case
                // explicitly rather than letting derivation produce a
                // misleading error downstream.
                let identity_index = managed.identity_index.ok_or_else(|| {
                    Transient(PlatformWalletError::IdentityIndexNotSet(*our_identity_id))
                })?;

                let wallet = wm.get_wallet(&self.wallet_id).ok_or_else(|| {
                    Transient(PlatformWalletError::WalletNotFound(hex::encode(
                        self.wallet_id,
                    )))
                })?;

                // The ECDH scalar can only be derived when the wallet has resident
                // key material. A watch-only / external-signable wallet (Keychain
                // signer not yet unlocked) can't derive *now* — classify
                // `Unavailable` so the build is DEFERRED, never broken: a locked
                // Keychain is recoverable, and breaking the channel over it would
                // irreversibly kill payments. Checked before the key-presence test
                // below so a seedless wallet defers rather than being judged on a
                // request it currently can't act on (request validity is already
                // enforced upstream in `build_contact_accounts`). Currently "can
                // derive" == `has_seed()`; the seedless model extends this to an
                // available resolver-backed signer.
                if !wallet.has_seed() {
                    return Err(Unavailable(PlatformWalletError::InvalidIdentityData(
                        format!(
                            "Cannot derive ECDH key for identity {}: wallet has no \
                         resident key material (watch-only / signer unavailable)",
                            our_identity_id
                        ),
                    )));
                }

                // Find our decryption key by its key ID. A missing key at the
                // validated index is a malformed-request fault, not transient.
                let our_encryption_key = managed
                    .identity
                    .public_keys()
                    .get(&our_decryption_key_index)
                    .cloned()
                    .ok_or_else(|| {
                        Permanent(PlatformWalletError::InvalidIdentityData(format!(
                            "Our encryption key {} not found on identity {}",
                            our_decryption_key_index, our_identity_id
                        )))
                    })?;

                Self::derive_encryption_private_key(
                    wallet,
                    self.sdk.network,
                    identity_index,
                    &our_encryption_key,
                )
                .map_err(Permanent)?
            };

            // --- 3. Extract the contact's encryption pubkey from the
            //        already-fetched identity (NO network I/O here — the caller
            //        fetched it for validation; re-fetching would turn a
            //        transient DAPI blip into a permanent broken channel). ---
            let contact_public_key: dashcore::secp256k1::PublicKey = {
                let contact_key = contact_identity
                    .public_keys()
                    .get(&contact_encryption_key_index)
                    .cloned()
                    .ok_or_else(|| {
                        Permanent(PlatformWalletError::InvalidIdentityData(format!(
                            "Contact encryption key {} not found on identity {}",
                            contact_encryption_key_index, contact_identity_id
                        )))
                    })?;

                // Deserialize the compressed public key bytes from the identity key data.
                dashcore::secp256k1::PublicKey::from_slice(contact_key.data().as_slice()).map_err(
                    |e| {
                        Permanent(PlatformWalletError::InvalidIdentityData(format!(
                            "Contact encryption key is not a valid secp256k1 public key: {}",
                            e
                        )))
                    },
                )?
            };

            // --- 4. Derive the ECDH shared key (resident path). ---
            platform_encryption::derive_shared_key_ecdh(&our_private_key, &contact_public_key)
        };

        // --- 5. Decrypt the contact's xpub. ---
        let decrypted_xpub_bytes =
            platform_encryption::decrypt_extended_public_key(&shared_key, contact_encrypted_xpub)
                .map_err(|e| {
                Permanent(PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to decrypt contact xpub: {}",
                    e
                )))
            })?;

        // --- 6. Reconstruct the ExtendedPubKey from the decrypted plaintext. ---
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

        Ok(())
    }
}
