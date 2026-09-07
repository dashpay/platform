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

use super::contacts::{ExternalAccountRegistration, RegisterExternalError};
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
    /// Extended public key at `path` (a generic derive-at-path): the DashPay
    /// receiving (friendship) xpub, and also the seed-binding self-check's
    /// BIP44 account-0 xpub ([`crate::PlatformWallet::verify_seed_binds`]).
    async fn receiving_xpub(
        &self,
        path: &key_wallet::bip32::DerivationPath,
    ) -> Result<key_wallet::bip32::ExtendedPubKey, PlatformWalletError>;

    /// ECDH shared secret between our key at `path` and the contact's `peer`
    /// pubkey. Returned in [`zeroize::Zeroizing`] so the DIP-15 friendship AES
    /// key is scrubbed on drop — it stays wrapped through the send / accept /
    /// drain flows and is only dereferenced at the final crypto boundary.
    async fn ecdh_shared_secret(
        &self,
        path: &key_wallet::bip32::DerivationPath,
        peer: &dashcore::secp256k1::PublicKey,
    ) -> Result<zeroize::Zeroizing<[u8; 32]>, PlatformWalletError>;

    /// Export the raw **auto-accept private key** at `path` (DIP-15 QR
    /// auto-accept) — the **one deliberate exception** to "the signer never
    /// returns a raw scalar." The auto-accept key is a shareable, expiry-bounded
    /// bearer credential the owner embeds in a QR (`dapk`), so it must leave the
    /// signer. `path` MUST be an auto-accept path (`m/9'/coin'/16'/expiry'`); the
    /// only caller is [`IdentityWallet::build_auto_accept_qr`], which builds it
    /// via `auto_accept_derivation_path`. The key authorizes only contact
    /// auto-acceptance — never payments or identity control.
    async fn export_auto_accept_private_key(
        &self,
        path: &key_wallet::bip32::DerivationPath,
    ) -> Result<dashcore::secp256k1::SecretKey, PlatformWalletError>;

    /// Export the raw **invitation-funding private key** at `path` (DIP-13
    /// sub-feature `3'`) — the second deliberate raw-key export (alongside
    /// [`Self::export_auto_accept_private_key`]). The invitation hands this
    /// one-time voucher key to the invitee so they can register their own
    /// identity from the funded asset lock, so it must leave the signer. `path`
    /// MUST be an invitation path (`m/9'/coin'/5'/3'/funding_index'`); the signer
    /// gates on the full shape (feature `5'` is shared with the user's own
    /// identity keys — see `export_invitation_private_key` on the resolver
    /// signer). The only caller is [`IdentityWallet::create_invitation`].
    ///
    /// Defaulted to an "unsupported" error so adding this method is not a
    /// source-breaking change for existing provider implementations:
    /// providers that never create invitations need no override, and a
    /// create attempted through one fails loudly (the invitation cannot be
    /// packaged without the exported key) rather than at compile time.
    /// Invitation-capable providers override with the path-gated export.
    async fn export_invitation_private_key(
        &self,
        _path: &key_wallet::bip32::DerivationPath,
    ) -> Result<dashcore::secp256k1::SecretKey, PlatformWalletError> {
        Err(PlatformWalletError::InvalidIdentityData(
            "invitation private-key export is not supported by this crypto provider".to_string(),
        ))
    }

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

    /// DIP-15 contactInfo **seal** — encrypt the contact id (`encToUserId`,
    /// AES-256-ECB) and the private-data plaintext (`privateData`, AES-256-CBC
    /// with `private_data_iv`) under the two hardened-child keys derived from
    /// `root_path` (the identity-auth path; the signer extends it internally with
    /// the DIP-15 `65536'`/`65537'` feature children + `derivation_index'`). The
    /// AES keys + scalars stay in the signer; only ciphertext returns.
    ///
    /// `root_path` MUST be built in Rust via `identity_auth_derivation_path_for_type`
    /// (never assembled by the host) — a wrong root silently produces contactInfo
    /// no client can decrypt.
    async fn contact_info_seal(
        &self,
        root_path: &key_wallet::bip32::DerivationPath,
        derivation_index: u32,
        contact_id: &[u8; 32],
        private_data_plaintext: &[u8],
        private_data_iv: &[u8; 16],
    ) -> Result<ContactInfoSealed, PlatformWalletError>;

    /// Inverse of [`Self::contact_info_seal`] — recover the contact id +
    /// private-data plaintext from the on-chain ciphertexts at `root_path` /
    /// `derivation_index`.
    async fn contact_info_open(
        &self,
        root_path: &key_wallet::bip32::DerivationPath,
        derivation_index: u32,
        enc_to_user_id: &[u8; 32],
        private_data_blob: &[u8],
    ) -> Result<ContactInfoOpened, PlatformWalletError>;
}

/// Result of [`ContactCryptoProvider::contact_info_seal`] — the two DIP-15
/// contactInfo ciphertexts to publish on chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContactInfoSealed {
    /// `encToUserId` ciphertext (AES-256-ECB of the 32-byte contact id).
    pub enc_to_user_id: [u8; 32],
    /// `privateData` ciphertext (`iv ‖ AES-256-CBC`).
    pub private_data: Vec<u8>,
}

/// Result of [`ContactCryptoProvider::contact_info_open`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContactInfoOpened {
    /// The recovered 32-byte contact id (the `toUserId` this doc is about).
    pub contact_id: [u8; 32],
    /// The recovered `privateData` plaintext (DIP-15 codec applied by the caller).
    pub private_data: Vec<u8>,
}

/// Test [`ContactCryptoProvider`] that derives from a resident test seed via
/// `key_wallet` — the seedless-test stand-in for the production Keychain signer.
/// It derives at exactly the Rust-built paths the production glue feeds, so a
/// test wired through this provider exercises the same key material the resident
/// seed would have produced, with no resident seed on the wallet under test —
/// faithful, unlike the canned/stub providers used by the queue-mechanics
/// drain tests.
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
    ) -> Result<zeroize::Zeroizing<[u8; 32]>, PlatformWalletError> {
        let xprv = self.wallet.derive_extended_private_key(path).map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!("test ecdh derive: {e}"))
        })?;
        Ok(zeroize::Zeroizing::new(
            platform_encryption::derive_shared_key_ecdh(&xprv.private_key, peer),
        ))
    }

    async fn export_auto_accept_private_key(
        &self,
        path: &key_wallet::bip32::DerivationPath,
    ) -> Result<dashcore::secp256k1::SecretKey, PlatformWalletError> {
        let xprv = self.wallet.derive_extended_private_key(path).map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!("test export auto-accept: {e}"))
        })?;
        Ok(xprv.private_key)
    }

    async fn export_invitation_private_key(
        &self,
        path: &key_wallet::bip32::DerivationPath,
    ) -> Result<dashcore::secp256k1::SecretKey, PlatformWalletError> {
        let xprv = self.wallet.derive_extended_private_key(path).map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!("test export invitation: {e}"))
        })?;
        Ok(xprv.private_key)
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

    async fn contact_info_seal(
        &self,
        root_path: &key_wallet::bip32::DerivationPath,
        derivation_index: u32,
        contact_id: &[u8; 32],
        private_data_plaintext: &[u8],
        private_data_iv: &[u8; 16],
    ) -> Result<ContactInfoSealed, PlatformWalletError> {
        let enc_key = self.contact_info_child(root_path, derivation_index, true)?;
        let priv_key = self.contact_info_child(root_path, derivation_index, false)?;
        Ok(ContactInfoSealed {
            enc_to_user_id: platform_encryption::encrypt_enc_to_user_id(&enc_key, contact_id),
            private_data: platform_encryption::encrypt_private_data(
                &priv_key,
                private_data_iv,
                private_data_plaintext,
            ),
        })
    }

    async fn contact_info_open(
        &self,
        root_path: &key_wallet::bip32::DerivationPath,
        derivation_index: u32,
        enc_to_user_id: &[u8; 32],
        private_data_blob: &[u8],
    ) -> Result<ContactInfoOpened, PlatformWalletError> {
        let enc_key = self.contact_info_child(root_path, derivation_index, true)?;
        let priv_key = self.contact_info_child(root_path, derivation_index, false)?;
        let private_data = platform_encryption::decrypt_private_data(&priv_key, private_data_blob)
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!("test contactInfo decrypt: {e}"))
            })?;
        Ok(ContactInfoOpened {
            contact_id: platform_encryption::decrypt_enc_to_user_id(&enc_key, enc_to_user_id),
            private_data,
        })
    }
}

#[cfg(test)]
impl SeedCryptoProvider {
    /// Derive one DIP-15 contactInfo hardened-child AES key (`encToUserId` =
    /// `enc=true`, `privateData` = `enc=false`) at
    /// `root_path / feature' / derivation_index'` — mirrors the resident
    /// `derive_contact_info_keys` so the test provider produces byte-identical
    /// keys to the production wallet derivation.
    fn contact_info_child(
        &self,
        root_path: &key_wallet::bip32::DerivationPath,
        derivation_index: u32,
        enc: bool,
    ) -> Result<[u8; 32], PlatformWalletError> {
        use crate::wallet::identity::crypto::contact_info::{
            ENC_TO_USER_ID_CHILD, PRIVATE_DATA_CHILD,
        };
        use key_wallet::bip32::ChildNumber;
        let feature = if enc {
            ENC_TO_USER_ID_CHILD
        } else {
            PRIVATE_DATA_CHILD
        };
        let path = root_path.clone().extend([
            ChildNumber::from_hardened_idx(feature).map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!("test contactInfo feature: {e}"))
            })?,
            ChildNumber::from_hardened_idx(derivation_index).map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!("test contactInfo index: {e}"))
            })?,
        ]);
        let xprv = self
            .wallet
            .derive_extended_private_key(&path)
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!("test contactInfo derive: {e}"))
            })?;
        Ok(xprv.private_key.secret_bytes())
    }
}

// ---------------------------------------------------------------------------
// Send contact request
// ---------------------------------------------------------------------------

/// How the optional DIP-15 `autoAcceptProof` is supplied to a contact-request
/// send. The proof signs over `(sender, recipient, accountReference)`, and
/// `accountReference` is computed **inside** the send — so the QR-scanner
/// variant carries the handed key and is signed there, once the reference is
/// known (binding the proof to the exact reference the document carries).
pub enum AutoAcceptProofSource {
    /// No proof (the normal manual flow).
    None,
    /// A pre-built proof blob.
    Provided(Vec<u8>),
    /// Sign the proof in-send with the auto-accept key decoded from a scanned
    /// QR (`dapk`), binding it to the accountReference this send computes.
    SignWithKey {
        /// The auto-accept private key handed out in the QR.
        secret_key: dashcore::secp256k1::SecretKey,
        /// The proof's expiry (the key blob's timestamp / derivation index).
        expiry: u32,
    },
}

impl AutoAcceptProofSource {
    /// Map a pre-built optional proof (the FFI / legacy shape) into a source.
    pub fn from_option(proof: Option<Vec<u8>>) -> Self {
        match proof {
            Some(p) => Self::Provided(p),
            None => Self::None,
        }
    }
}

impl<B: TransactionBroadcaster + ?Sized> DashPayView<'_, B> {
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
        auto_accept_proof: AutoAcceptProofSource,
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
        //    required for ECDH. Shared selector — same enabled-only policy
        //    as the contactInfo publish path, so both DashPay surfaces
        //    agree on the ECDH root and a disabled (rotated-away) first
        //    key can't hard-fail the pre-send validator below while an
        //    enabled replacement exists.
        let sender_encryption_key = select_own_encryption_key(&sender_identity)?.clone();
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
            .receiving_xpub_for(
                sender_identity_id,
                recipient_identity_id,
                account_index,
                crypto,
            )
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
        let sender_enc_path = IdentityWallet::<B>::identity_auth_derivation_path(
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

        // 4c. Resolve the auto-accept proof now that `account_reference` is
        //     known. The QR-scanner variant signs `(sender, recipient,
        //     account_reference)` here with the handed key, binding the proof to
        //     the exact reference this document carries.
        let auto_accept_proof: Option<Vec<u8>> = match auto_accept_proof {
            AutoAcceptProofSource::None => None,
            AutoAcceptProofSource::Provided(p) => Some(p),
            AutoAcceptProofSource::SignWithKey { secret_key, expiry } => Some(
                crate::wallet::identity::crypto::auto_accept::sign_auto_accept_proof(
                    &secret_key,
                    sender_identity_id,
                    recipient_identity_id,
                    account_reference,
                    expiry,
                ),
            ),
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
        //    encryption key, so the SDK write helper receives the finished secret
        //    (`EcdhProvider::ClientSide`) and no private key is ever materialized
        //    here. The recipient key is resolved exactly as the SDK would
        //    (`recipientKeyIndex` on the recipient identity); the helper re-checks
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

        // 7. Broadcast through the write helper. All inputs are resolved
        //    above; the helper assembles the SDK `EcdhProvider` + xpub
        //    closure and dispatches `Sdk::send_contact_request`, keeping
        //    the seven-generic SDK signature out of this call site — see
        //    `sdk_writer.rs`.
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
        let mut contact_request = ContactRequest::new(
            *sender_identity_id,
            result.recipient_id,
            sender_key_index,
            recipient_key_index,
            result.account_reference,
            encrypted_public_key,
            result.document.created_at_core_block_height().unwrap_or(0),
            result.document.created_at().unwrap_or(0),
        );
        // Mirror the broadcast doc's optional `encryptedAccountLabel` onto the
        // local outgoing row so it matches what landed on Platform (same reason
        // the 96-byte ciphertext is read off the doc above). Without this the
        // sender's own row never reflects the label it just sent.
        contact_request.encrypted_account_label = result
            .document
            .properties()
            .get("encryptedAccountLabel")
            .and_then(|v: &Value| v.to_binary_bytes().ok());

        {
            let mut wm = self.wallet_manager.write().await;
            let info = wm
                .get_wallet_info_mut(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
            let managed = info
                .identity_manager
                .managed_identity_mut(sender_identity_id)
                .ok_or(PlatformWalletError::IdentityNotFound(*sender_identity_id))?;
            managed
                .add_sent_contact_request(contact_request.clone(), &self.persister)
                .map_err(|e| {
                    PlatformWalletError::Persistence(format!(
                        "sent contact request not persisted: {e}"
                    ))
                })?;
        }

        // Register our receiving (friendship) account from the xpub the signer
        // already derived above — same friendship key, no second derivation and
        // no resident seed.
        self.register_contact_account(
            sender_identity_id,
            recipient_identity_id,
            account_index,
            contact_xpub_ext,
        )
        .await?;

        Ok(contact_request)
    }
}

/// QR-scan send lives in a non-generic `impl` block because it resolves a DPNS
/// name via [`IdentityWallet::resolve_name`] (reached through the view's
/// `Deref`), which is defined on the non-generic `impl IdentityWallet`.
impl DashPayView<'_> {
    /// Send a contact request from a scanned DIP-15 auto-accept QR
    /// (`dash:?du=<username>&dapk=<key_blob>`).
    ///
    /// Resolves the QR's `du` username to the owner's identity, decodes the
    /// handed auto-accept key from `dapk`, and sends a contact request carrying a
    /// proof signed (in-send) over this send's `accountReference` — so the
    /// owner's client can verify it and auto-accept without a manual tap.
    pub async fn send_contact_request_from_qr<S, C>(
        &self,
        sender_identity_id: &Identifier,
        uri: &str,
        signer: &S,
        crypto: &C,
    ) -> Result<ContactRequest, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
        C: ContactCryptoProvider + Sync,
    {
        use crate::wallet::identity::crypto::auto_accept::{
            decode_auto_accept_key_blob, parse_dashpay_contact_uri,
        };

        let (username, key_blob) = parse_dashpay_contact_uri(uri)?;
        let recipient_id = self.resolve_name(&username).await?.ok_or_else(|| {
            PlatformWalletError::InvalidIdentityData(format!(
                "auto-accept QR username '{username}' did not resolve to an identity"
            ))
        })?;
        let (secret_key, expiry) = decode_auto_accept_key_blob(&key_blob)?;

        self.send_contact_request_with_external_signer(
            sender_identity_id,
            &recipient_id,
            None,
            AutoAcceptProofSource::SignWithKey { secret_key, expiry },
            signer,
            crypto,
        )
        .await
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

/// Sent-side analog of [`newest_received_per_sender`], keyed by
/// **recipient**. Immutable `contactRequest` docs are never deleted
/// on-chain, so a rotation (re-key) re-send leaves MULTIPLE of our own sent
/// docs to the same recipient — the old reference plus the bumped one — and
/// the sweep re-fetches them all. `fetch_sent_contact_requests` orders them
/// `$createdAt`-ASC, so a restore-from-seed ingesting them raw would
/// auto-establish against the OLDEST doc (frozen at the first send) and drop
/// the newer ones on the same-reference / already-established guard. Collapse
/// to the single newest doc per recipient (newest by `$createdAt`, tiebreak
/// on `account_reference`) so the sent-side establishes / rotation-supersedes
/// with the freshest outgoing reference — otherwise the next rotation
/// collides on the contract's `($ownerId, toUserId, accountReference)` unique
/// index.
fn newest_sent_per_recipient(
    requests: impl IntoIterator<Item = ContactRequest>,
) -> std::collections::BTreeMap<Identifier, ContactRequest> {
    let mut newest: std::collections::BTreeMap<Identifier, ContactRequest> =
        std::collections::BTreeMap::new();
    for req in requests {
        let recipient = req.recipient_id;
        let replace = newest
            .get(&recipient)
            .map(|cur| {
                (req.created_at, req.account_reference) > (cur.created_at, cur.account_reference)
            })
            .unwrap_or(true);
        if replace {
            newest.insert(recipient, req);
        }
    }
    newest
}

/// Ingest one identity's collapsed **received** contact requests into local
/// state, returning whether every write reached disk.
///
/// A `false` return means a persist failed and the loop stopped there, so an
/// unknown number of the requests handed in were never ingested and their
/// account builds were never enqueued. Two things must follow from it, and
/// they are the caller's to do: leave the received high-water cursor
/// unadvanced (so the next sweep re-fetches this range) and mark the identity
/// in the [`ContactSyncReport`] (so the pass cannot report itself complete).
/// Stopping rather than continuing past the failure is deliberate — ingesting
/// later requests would let the cursor's max cover a request that never
/// persisted if the caller ever advanced it.
///
/// Split out of the sweep so the persist-failure branches are reachable in a
/// test without standing up a Platform that answers document queries.
fn ingest_received_requests(
    managed: &mut crate::wallet::identity::ManagedIdentity,
    persister: &crate::wallet::persister::WalletPersister,
    identity_id: Identifier,
    newest_by_sender: std::collections::BTreeMap<Identifier, ContactRequest>,
    rotated_contacts: &mut Vec<Identifier>,
    all_requests: &mut Vec<ContactRequest>,
) -> bool {
    for (sender_id, contact_request) in newest_by_sender {
        // Ignore (per-sender mute, local-only): an ignored sender's requests
        // are ALL suppressed from the main pending list — including rotated
        // (bumped accountReference) ones. Checked FIRST and per-sender, unlike
        // the old per-(sender, accountReference) reject: if you ignored the
        // person you ignored them. `unignore_sender` rewinds the cursor so
        // this skip stops firing on the next sweep.
        if managed.is_sender_ignored(&sender_id) {
            tracing::debug!(
                sender = %sender_id,
                recipient = %identity_id,
                account_reference = contact_request.account_reference,
                "Skipping ignored sender's contact request"
            );
            continue;
        }
        // Do NOT skip just because the sender is in `sent_contact_requests` —
        // that is the reciprocal we need to let through to auto-establish.
        // True dedup is (sender, accountReference): the SAME reference as the
        // tracked incoming/established state is a re-ingest of a known doc; a
        // DIFFERENT reference from a known sender is a rotation request
        // (receive side) and must get through.
        let tracked_reference = managed
            .dashpay()
            .incoming_contact_requests()
            .get(&sender_id)
            .map(|r| r.account_reference)
            .or_else(|| {
                managed
                    .dashpay()
                    .established_contacts()
                    .get(&sender_id)
                    .map(|c| c.incoming_request.account_reference)
            });
        if tracked_reference == Some(contact_request.account_reference) {
            continue;
        }

        if tracked_reference.is_some() {
            // Rotation: supersede the tracked request. When an established
            // contact was re-keyed, queue the stale external account for
            // teardown so the build sweep re-registers from the new xpub.
            match managed.apply_rotated_incoming_request(contact_request.clone(), persister) {
                Ok(true) => rotated_contacts.push(sender_id),
                Ok(false) => {}
                Err(e) => {
                    tracing::error!(
                        recipient = %identity_id, error = %e,
                        "received-request rotation persist failed; leaving received cursor for retry"
                    );
                    return false;
                }
            }
            all_requests.push(contact_request);
            continue;
        }

        if let Err(e) = managed.add_incoming_contact_request(contact_request.clone(), persister) {
            tracing::error!(
                recipient = %identity_id, error = %e,
                "received-request ingest persist failed; leaving received cursor for retry"
            );
            return false;
        }
        all_requests.push(contact_request);
    }
    true
}

/// Ingest one identity's collapsed **sent** contact requests into local state,
/// returning whether every write reached disk. The sent-side counterpart of
/// [`ingest_received_requests`], with the same contract on a `false` return:
/// the sent cursor stays unadvanced and the identity is marked in the report.
///
/// `add_sent_contact_request` carries its own duplicate / metadata-loss guard,
/// so re-ingesting the same range on the next sweep is safe.
fn ingest_sent_requests(
    managed: &mut crate::wallet::identity::ManagedIdentity,
    persister: &crate::wallet::persister::WalletPersister,
    identity_id: Identifier,
    newest_by_recipient: std::collections::BTreeMap<Identifier, ContactRequest>,
) -> bool {
    for (_recipient_id, contact_request) in newest_by_recipient {
        if let Err(e) = managed.add_sent_contact_request(contact_request, persister) {
            tracing::error!(
                owner = %identity_id, error = %e,
                "sent-request ingest persist failed; leaving sent cursor for retry"
            );
            return false;
        }
    }
    true
}

/// Snapshot-aware removal of drained queue entries. Removes from `queue`
/// only those entries still **value-equal** to a snapshot in `drained` —
/// the full entries snapshotted before the lock-free drain. An entry a
/// concurrent `upsert_pending_contact_crypto` refreshed mid-drain is LEFT
/// queued (its live value no longer equals the stale snapshot's), so a
/// payload changed under the drain survives to the next drain instead of
/// being clobbered by a key-only removal. Whole-value equality rather than
/// an `enqueued_at_ms` freshness token: the timestamp is wall-clock
/// milliseconds (documented as observability/ordering only), so a same-ms
/// upsert or a clock rollback could alias a token that a changed payload
/// can never alias. A refresh that reproduces the identical payload
/// compares equal and is removed — fine, since the drain just processed
/// exactly those bytes. Returns the keys actually removed, so the caller
/// emits exactly those to the persisted `pending_contact_crypto_cleared`
/// delta (never a key whose fresher entry is still queued).
fn retain_drained_by_snapshot(
    queue: &mut Vec<crate::changeset::PendingContactCrypto>,
    drained: &[crate::changeset::PendingContactCrypto],
) -> Vec<crate::changeset::PendingContactCryptoKey> {
    let mut removed = Vec::new();
    queue.retain(|e| {
        let stale_match = drained.contains(e);
        if stale_match {
            removed.push(e.key());
        }
        !stale_match
    });
    removed
}

/// Await `future`, giving up once `deadline` has passed. `None` deadline is
/// the unbounded behaviour; a `None` return means the budget is spent.
///
/// **Why the drains bound themselves rather than being wrapped in a
/// `tokio::time::timeout` by their caller.** Both drains commit per-entry side
/// effects as they go (`register_contact_account`, `mark_contact_channel_broken`,
/// the reciprocal send) while accumulating the dequeue list in a local `cleared`
/// vec applied once at the end. Dropping the drain future mid-loop — which is
/// exactly what an outer `timeout` does — discards that vec, so entries whose
/// work really landed stay queued and the returned count reports zero for work
/// that happened. The queue is at-least-once, so nothing is corrupted, but the
/// count is a lie precisely when a caller has a budget to report against.
///
/// Threading the deadline inside keeps the loop the only thing that ever ends
/// early: it stops **between** entries, and within an entry only the reads that
/// precede its first commit are bounded. Every commit still runs to completion.
async fn bounded<F: std::future::Future>(
    deadline: Option<std::time::Instant>,
    future: F,
) -> Option<F::Output> {
    let Some(deadline) = deadline else {
        return Some(future.await);
    };
    let remaining = deadline.saturating_duration_since(std::time::Instant::now());
    if remaining.is_zero() {
        return None;
    }
    tokio::time::timeout(remaining, future).await.ok()
}

/// Whether `deadline` has passed. Checked at the top of each drain iteration so
/// a spent budget ends the loop between entries, never inside one.
///
/// Shared with the discovery scan's DPNS enrichment tail, which needs the same
/// between-items stop for the same reason.
pub(crate) fn budget_spent(deadline: Option<std::time::Instant>) -> bool {
    deadline.is_some_and(|d| std::time::Instant::now() >= d)
}

/// Whether a registered outbound `DashpayExternalAccount` for `contact`
/// must be torn down + rebuilt because it was NOT built from the contact's
/// current `incoming_request.account_reference`.
///
/// Only a **registered** account (`has_external == true`) can be stale — a
/// missing account is handled by the ordinary build-candidate collection.
/// A permanently-broken channel is left alone (the sweep never rebuilds
/// broken contacts; they heal on a superseding request). The staleness test
/// is `external_account_reference != Some(incoming_request.account_reference)`:
/// a mismatch (or a `None` marker from a cold restart that did not carry it)
/// means the persisted, tombstone-less account row rebuilt the rotated-away
/// xpub while the contact already tracks the new reference — so `send_payment`
/// would derive addresses the contact no longer watches until it is rebuilt.
fn external_account_needs_rebuild(contact: &EstablishedContact, has_external: bool) -> bool {
    has_external
        && !contact.payment_channel_broken
        && contact.external_account_reference != Some(contact.incoming_request.account_reference)
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
/// No AUTHENTICATION or TRANSFER fallback: reusing a signing or
/// fund-authorizing key for ECDH is poor key separation, and nothing forces us
/// to when we are the one choosing. `ECDSA_SECP256K1` is required either way
/// (every observed key is that type, and ECDH needs the full key).
///
/// That mainnet's legacy Android/dashj population *does* reference
/// AUTHENTICATION/TRANSFER keys is a fact about immutable history, handled by
/// the wider receive-side policy
/// ([`dash_sdk::platform::dashpay::recipient_key_purpose_is_acceptable_on_receive`]).
/// It must not relax what we mint: this selector calls
/// [`dash_sdk::platform::dashpay::recipient_key_purpose_is_valid`] for
/// membership, so the cohort cannot drift from the SDK's request-creation
/// gate. Only the preference ORDER below (DECRYPTION first, ENCRYPTION second)
/// is local to the selector.
fn select_recipient_key_index(recipient_identity: &Identity) -> Result<u32, PlatformWalletError> {
    // Membership comes from the shared mint predicate, never from a purpose
    // list repeated here — a local copy is exactly how the SDK's
    // request-creation gate and this selector would drift apart on the next
    // policy change.
    //
    // Skip disabled (revoked) keys: encrypting the DIP-15 compact xpub to a
    // key whose private half may be compromised would hand the contact's
    // payment xpub to whoever holds the revoked key. `disabled_at().is_none()`
    // mirrors the validator's disabled-key gate.
    let mut eligible: Vec<(&u32, &dpp::identity::IdentityPublicKey)> = recipient_identity
        .public_keys()
        .iter()
        .filter(|(_, k)| {
            dash_sdk::platform::dashpay::recipient_key_purpose_is_valid(k.purpose())
                && k.key_type() == KeyType::ECDSA_SECP256K1
                && k.disabled_at().is_none()
        })
        .collect();
    // The only policy local to this selector: DECRYPTION before ENCRYPTION,
    // then lowest key id (which `public_keys()`'s BTreeMap order already
    // gives, and the stable sort preserves).
    eligible.sort_by_key(|(_, k)| k.purpose() != Purpose::DECRYPTION);
    eligible.first().map(|(id, _)| **id).ok_or_else(|| {
        PlatformWalletError::InvalidIdentityData(
            "Recipient identity has no enabled ECDSA_SECP256K1 DECRYPTION or ENCRYPTION key"
                .to_string(),
        )
    })
}

/// Select our OWN ECDH root key: the first **enabled** `ECDSA_SECP256K1`
/// ENCRYPTION key on the identity (`BTreeMap` order → lowest key id).
///
/// The single selection policy shared by the contact-request send path and
/// the contactInfo publish path, so the two DashPay surfaces always agree
/// on which key is the ECDH root. Skipping disabled keys mirrors
/// [`select_recipient_key_index`] and the validator's disabled-key gate:
/// after a disable-and-replace key rotation, selecting the disabled
/// lowest-id key would hard-fail pre-send validation on every new outgoing
/// request even though an enabled replacement exists.
pub(crate) fn select_own_encryption_key(
    identity: &Identity,
) -> Result<&IdentityPublicKey, PlatformWalletError> {
    identity
        .public_keys()
        .iter()
        .find(|(_, k)| {
            k.purpose() == Purpose::ENCRYPTION
                && k.key_type() == KeyType::ECDSA_SECP256K1
                && k.disabled_at().is_none()
        })
        .map(|(_, k)| k)
        .ok_or_else(|| {
            PlatformWalletError::InvalidIdentityData(
                "Identity has no enabled ECDSA_SECP256K1 encryption key".to_string(),
            )
        })
}

// ---------------------------------------------------------------------------
// Sync contact requests from platform
// ---------------------------------------------------------------------------

/// Max `AutoAccept` ops queued per owner. Bounds the work a flood of
/// junk-`autoAcceptProof` contact requests can create for the owner's next
/// signer-present drain; over the cap, requests stay manually acceptable.
const MAX_AUTO_ACCEPT_QUEUED_PER_OWNER: usize = 64;

/// Count the ops that represent a contact **waiting for an unlock to finish
/// setup** — the needs-unlock banner's source.
///
/// `RegisterReceiving` / `RegisterExternal` build a contact's payment account
/// and converge to 0 once drained (candidate selection skips contacts whose
/// external account already exists). `AutoAccept` is an inbound request with a
/// valid-looking proof awaiting auto-acceptance at the next signer-present drain
/// — also "waiting to finish setup," and it clears on accept/permanent-reject,
/// so it converges too.
///
/// `ContactInfoDecrypt` is intentionally excluded: it is re-enqueued on every
/// signerless sweep (there is no already-decrypted gate), so it is structurally
/// always present and would make the signal a permanent `> 0` — re-tripping the
/// banner shortly after every unlock on a healthy wallet.
fn count_account_build_ops(queue: &[crate::changeset::PendingContactCrypto]) -> usize {
    use crate::changeset::PendingContactCryptoOp;
    queue
        .iter()
        .filter(|e| {
            matches!(
                e.op,
                PendingContactCryptoOp::RegisterReceiving
                    | PendingContactCryptoOp::RegisterExternal { .. }
                    | PendingContactCryptoOp::AutoAccept
            )
        })
        .count()
}

/// What one contact-request pass actually reached, as opposed to what it
/// returned.
///
/// The sweep is deliberately log-and-continue per identity: one identity's
/// transient DAPI error must not stall DashPay sync for every other identity
/// on the wallet. That is right for a recurring background sweep and wrong for
/// anything that treats the pass as a precondition, because the two endings it
/// collapses are opposites — "Platform answered, and there is nothing new" and
/// "Platform answered nobody, so we do not know". Both arrive as
/// `Ok(vec![])`.
///
/// The distinction matters most at startup, where a completed pass is the
/// promise that a contact's DIP-15 addresses exist before the compact-filter
/// scan passes their funding height. An address the wallet is not watching by
/// then produces no transaction at all, so recording an unreachable pass as a
/// successful one does not merely mislabel a status — it starts Core SPV
/// against an address set that is silently short.
#[derive(Debug, Default, Clone)]
pub struct ContactSyncReport {
    /// Newly discovered incoming contact requests. Real whatever else failed:
    /// they were fetched, ingested and persisted.
    pub requests: Vec<ContactRequest>,
    /// Identities the pass tried to fetch for.
    pub identities_attempted: usize,
    /// Identities whose **received-side fetch** did not come back. Purely a
    /// statement about reaching Platform — a local fault is NOT recorded here
    /// (see [`Self::unpersisted_identities`]), because this list is what
    /// [`Self::is_fully_degraded`] reads to call an outage, and a local
    /// failure on a pass Platform answered in full is not an outage. Their
    /// high-water cursors are deliberately left unadvanced, so the next sweep
    /// re-fetches exactly the range this one missed.
    pub failed_identities: Vec<Identifier>,
    /// Identities whose received side ingested but whose **sent**-side fetch
    /// failed. Their incoming requests are real; what is missing is the
    /// reciprocal reconciliation that establishes contacts, and the sent
    /// cursor stays unadvanced so the next sweep retries it. Remote, like
    /// [`Self::failed_identities`].
    pub degraded_identities: Vec<Identifier>,
    /// Identities whose fetches were answered but whose **local ingest** did
    /// not land: a persister `store()` failure part-way through either
    /// direction, or the wallet / managed identity being gone by the time the
    /// write guard was taken.
    ///
    /// Kept apart from the two remote lists on purpose. The failure is real
    /// and definitionally leaves the pass incomplete — the ingest loop
    /// `break`s, abandoning every remaining fetched request of that direction,
    /// and holds that direction's cursor back for retry — so it must stop the
    /// pass being recorded as a completed sync. But it says nothing about
    /// Platform's reachability, so it must not be able to turn a pass that
    /// reached everybody into [`Self::is_fully_degraded`] and, through
    /// [`DashPayView::sync_contact_requests`], a
    /// [`PlatformWalletError::ContactSyncUnreachable`].
    pub unpersisted_identities: Vec<Identifier>,
}

impl ContactSyncReport {
    /// Every identity's fetches, both directions, were answered **and**
    /// everything they returned reached local state.
    ///
    /// The only state in which the pass may be recorded as a completed one. A
    /// wallet with no identities is complete by this rule — there was nothing
    /// to fetch, which is an answer rather than a degradation.
    pub fn is_complete(&self) -> bool {
        self.failed_identities.is_empty()
            && self.degraded_identities.is_empty()
            && self.unpersisted_identities.is_empty()
    }

    /// Not one identity's contact documents could be **read from Platform**.
    ///
    /// The signature of an unreachable Platform rather than of an empty
    /// wallet, and the ending that must never be mistaken for a clean pass. A
    /// wallet with no identities is NOT fully degraded: nothing was attempted,
    /// so nothing failed. Neither is a wallet whose fetches all succeeded and
    /// whose local writes all failed — that is a local fault, and reporting it
    /// as an outage would send a host retrying the network for a disk it
    /// cannot write.
    pub fn is_fully_degraded(&self) -> bool {
        self.identities_attempted > 0 && self.failed_identities.len() == self.identities_attempted
    }
}

impl<B: TransactionBroadcaster + ?Sized> DashPayView<'_, B> {
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
    ///
    /// # Errors
    ///
    /// [`PlatformWalletError::ContactSyncUnreachable`] when the pass had
    /// identities to fetch for and not one of them could be read. That ending
    /// is indistinguishable from a clean empty result in the return value
    /// alone, and reporting it as success is what let a startup sequence
    /// record an unreachable Platform as a completed contact pass. Callers
    /// that need to tell a partial pass from a complete one — rather than only
    /// a total failure from everything else — should call
    /// [`Self::sync_contact_requests_reporting`] instead.
    pub async fn sync_contact_requests(&self) -> Result<Vec<ContactRequest>, PlatformWalletError> {
        let report = self.sync_contact_requests_reporting().await?;
        if report.is_fully_degraded() {
            return Err(PlatformWalletError::ContactSyncUnreachable {
                identities: report.identities_attempted,
            });
        }
        Ok(report.requests)
    }

    /// [`Self::sync_contact_requests`], reporting what the pass reached.
    ///
    /// Same work, same side effects; the difference is only that the caller
    /// gets the failure set rather than a `Vec` that cannot express it. Use
    /// this wherever a *complete* pass is a precondition — a partial one is
    /// still `Ok`, and still leaves some contacts' account builds unenqueued.
    pub async fn sync_contact_requests_reporting(
        &self,
    ) -> Result<ContactSyncReport, PlatformWalletError> {
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
                        .map(|m| {
                            (
                                m.dashpay().high_water_received_ms(),
                                m.dashpay().high_water_sent_ms(),
                            )
                        })
                        .unwrap_or((None, None));
                    (id, hwr, hws)
                })
                .collect()
        };

        let mut report = ContactSyncReport {
            identities_attempted: identities.len(),
            ..Default::default()
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
                    // Nothing of this identity's is ingested this pass, and its
                    // cursors stay where they were. Recorded rather than only
                    // logged so a caller that treats the pass as a precondition
                    // can tell this from a clean empty result — see
                    // `ContactSyncReport`.
                    report.failed_identities.push(identity_id);
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
                    report.degraded_identities.push(identity_id);
                    Default::default()
                }
            };

            // Max `$createdAt` over docs FETCHED this sweep (not over docs that
            // survive ingest's collapse/dedup) — the cursor records how far this
            // sweep got. The fetch returned without error, so the received
            // cursor may advance to that max; the sent cursor advances only if
            // `sent_ok`. The fetch is `$createdAt`-ascending and may stop at a
            // per-sweep page budget, so this may be a partial — advancing to the
            // max fetched is exactly what lets the next sweep resume the rest.
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
                    // Fetched, but there is no longer anywhere to put it —
                    // nothing of this identity's is ingested. A LOCAL fault,
                    // not a remote one: Platform answered. Recorded so the
                    // pass cannot be called complete, and recorded in the
                    // local bucket so it cannot be mistaken for an outage.
                    report.unpersisted_identities.push(identity_id);
                    continue;
                };
                let managed = match info.identity_manager.managed_identity_mut(&identity_id) {
                    Some(m) => m,
                    None => {
                        report.unpersisted_identities.push(identity_id);
                        continue;
                    }
                };
                // Established contacts re-keyed by a rotation request in
                // this pass — their stale external accounts are torn down
                // below so the build sweep re-registers from the new xpub.
                let mut rotated_contacts: Vec<Identifier> = Vec::new();
                // Track whether every ingest reached disk. A swallowed persist
                // failure would let the cursor advance past a request that
                // never persisted, so the next `$createdAt >` sweep would skip
                // it. On failure we stop ingesting that direction and leave its
                // cursor unadvanced so the next sweep re-fetches and retries —
                // and, since the rest of that direction is then abandoned
                // un-ingested, the identity is marked below so the pass cannot
                // report itself complete.

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

                let received_persist_ok = ingest_received_requests(
                    managed,
                    &self.persister,
                    identity_id,
                    newest_by_sender,
                    &mut rotated_contacts,
                    &mut all_requests,
                );

                // (2) Ingest our own sent requests. `add_sent_contact_request`
                //     guards itself against duplicates / metadata loss.
                //     Collapse to the single newest doc per recipient FIRST
                //     (see `newest_sent_per_recipient`): a rotation re-send
                //     leaves the old + bumped docs on-chain and the fetch is
                //     `$createdAt`-ASC, so ingesting raw would establish
                //     against the stale OLDEST reference on a restore-from-seed
                //     and collide on the next rotation.
                let parsed_sent = sent_docs.iter().filter_map(|(_doc_id, maybe_doc)| {
                    let doc = maybe_doc.as_ref()?;
                    // For a sent request the recipient is `toUserId`.
                    let recipient_id = doc
                        .properties()
                        .get("toUserId")
                        .and_then(|v: &Value| v.to_identifier().ok())?;
                    Self::parse_sent_contact_request_doc(doc, identity_id, recipient_id)
                });
                let newest_by_recipient = newest_sent_per_recipient(parsed_sent);

                let sent_persist_ok = ingest_sent_requests(
                    managed,
                    &self.persister,
                    identity_id,
                    newest_by_recipient,
                );

                // (2a') Rotation self-heal across restart: an external account
                //       rebuilt from the persisted (tombstone-less) registration
                //       row after a restart can carry the STALE xpub while the
                //       established contact already tracks the new incoming
                //       reference (the deferred rebuild was lost with the
                //       in-memory queue at load). Such an account exists but was
                //       NOT built from the current reference, so the plain
                //       `has_external` gate would skip it forever. Detect a
                //       registered external account whose recorded
                //       `external_account_reference` does not match the contact's
                //       current `incoming_request.account_reference` (including a
                //       `None` marker from a cold restore that didn't carry it),
                //       and enqueue it for teardown + rebuild alongside the
                //       in-pass rotations. Idempotent: once rebuilt the marker is
                //       stamped, so the next sweep sees a match and skips it.
                //       Accesses the managed identity's established contacts and
                //       `info.core_wallet` as DISJOINT fields of `info` via a
                //       plain `for` loop (the same split the teardown loop below
                //       relies on) — a closure capturing whole `info` would
                //       collide with the mutable `managed` reborrow.
                {
                    use key_wallet::account::account_collection::DashpayAccountKey;
                    for (contact_id, contact) in managed.dashpay().established_contacts().iter() {
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
                        if external_account_needs_rebuild(contact, has_external)
                            && !rotated_contacts.contains(contact_id)
                        {
                            rotated_contacts.push(*contact_id);
                        }
                    }
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
                // fetched this sweep, never below the current value. Advance a
                // direction's cursor only when BOTH its fetch succeeded AND
                // every ingest reached disk — a mid-sweep fetch error or a
                // persist failure leaves that cursor intact so the next sweep
                // re-fetches and retries (no burying, no skip past an
                // unpersisted request). The compare-and-advance itself (a
                // concurrent `unignore_sender` rewind must not be clobbered by
                // this sweep's stale max) lives in the state layer.
                if received_persist_ok {
                    managed.advance_high_water_received(hw_received, max_received);
                }
                if sent_ok && sent_persist_ok {
                    managed.advance_high_water_sent(hw_sent, max_sent);
                }

                // A held-back cursor and a report that says "complete" cannot
                // both be right. Either `break` above abandoned the rest of
                // that direction's fetched requests un-ingested — their
                // account builds never enqueued — so the pass is incomplete by
                // the same rule the cursor logic already applies to itself.
                // Left unrecorded, `is_complete()` stayed true, startup called
                // `record_sync_ran()`, and the launch could reach `Ready`
                // promising DIP-15 addresses that were never registered: the
                // headline defect of this change, reached through the local
                // door instead of the fetch door.
                if !received_persist_ok || !sent_persist_ok {
                    report.unpersisted_identities.push(identity_id);
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

            // (4) Enqueue DIP-15 auto-accept for inbound requests carrying a
            //     proof. Signerless: verify + accept happen later in
            //     `drain_auto_accepts` at a signer-present moment.
            self.enqueue_pending_auto_accepts(&identity_id).await;
        }

        report.requests = all_requests;
        Ok(report)
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
        // `to_binary_bytes()` (not `as_bytes()`, which matches only
        // `Value::Bytes`) so the parse is robust to whatever byte-ish variant
        // the document query yields (`Bytes`/`Bytes32`/`Array<U8>`/base64
        // `Text`) — and stays symmetric with the send-side bookkeeping, which
        // also uses `to_binary_bytes()`. A variant mismatch here would silently
        // drop a field (the bug class this path already shipped once).
        let encrypted_public_key = props
            .get("encryptedPublicKey")
            .and_then(|v: &Value| v.to_binary_bytes().ok());
        // Optional DIP-15 auto-accept proof — read so the sweep can enqueue an
        // `AutoAccept` drain. Without this it would be dropped (the proof can't
        // be acted on if it never reaches the request), so the contact would only
        // ever be addable manually.
        let auto_accept_proof = props
            .get("autoAcceptProof")
            .and_then(|v: &Value| v.to_binary_bytes().ok());
        // Optional DIP-15 `encryptedAccountLabel` — the contact's label for the
        // account they shared. Read so the receive-side surfacing (decrypt +
        // "Their account" row) has something to decrypt; without this the sweep
        // silently drops the label and it never reaches the recipient.
        let encrypted_account_label = props
            .get("encryptedAccountLabel")
            .and_then(|v: &Value| v.to_binary_bytes().ok());

        match (
            sender_key_index,
            recipient_key_index,
            account_reference,
            encrypted_public_key,
        ) {
            (Some(ski), Some(rki), Some(ar), Some(epk)) => {
                let mut request = ContactRequest::new(
                    sender_id,
                    recipient_id,
                    ski,
                    rki,
                    ar,
                    epk,
                    doc.created_at_core_block_height().unwrap_or(0),
                    doc.created_at().unwrap_or(0),
                );
                request.auto_accept_proof = auto_accept_proof;
                request.encrypted_account_label = encrypted_account_label;
                Some(request)
            }
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

    /// Enqueue a DIP-15 `AutoAccept` op for each inbound contact request to
    /// `identity_id` that carries a structurally-valid `autoAcceptProof` and is
    /// not yet established — so the next signer-present auto-accept pass
    /// verifies + auto-accepts it. Signerless (the sweep has no signer): only a
    /// cheap structural pre-check (length + ECDSA key-type byte) runs here; the
    /// cryptographic verify happens in the drain.
    ///
    /// Bounded to [`MAX_AUTO_ACCEPT_QUEUED_PER_OWNER`] entries per owner so a
    /// flood of junk-proof requests can't grow the queue without limit — over
    /// the cap the request is simply left manually acceptable. Dedup by
    /// `(owner, sender, AutoAccept)` means re-runs are idempotent.
    async fn enqueue_pending_auto_accepts(&self, identity_id: &Identifier) {
        use crate::changeset::{
            upsert_pending_contact_crypto, PendingContactCrypto, PendingContactCryptoOp,
            PlatformWalletChangeSet,
        };

        // Collect candidate senders + the current AutoAccept count under a read
        // guard (no awaits held).
        let to_enqueue: Vec<Identifier> = {
            let wm = self.wallet_manager.read().await;
            let Some(info) = wm.get_wallet_info(&self.wallet_id) else {
                return;
            };
            let Some(managed) = info.identity_manager.managed_identity(identity_id) else {
                return;
            };
            let mut already = managed
                .dashpay()
                .pending_contact_crypto
                .iter()
                .filter(|e| matches!(e.op, PendingContactCryptoOp::AutoAccept))
                .count();
            let mut picked = Vec::new();
            for (sender, request) in managed.dashpay().incoming_contact_requests() {
                if already >= MAX_AUTO_ACCEPT_QUEUED_PER_OWNER {
                    tracing::warn!(
                        owner = %identity_id,
                        "auto-accept enqueue cap reached; leaving further requests manually acceptable"
                    );
                    break;
                }
                // Signerless pre-check only (established? structurally valid
                // proof? not a proof a prior drain already rejected this
                // launch?) — the real ECDSA verify is in the drain.
                if managed.should_enqueue_auto_accept(sender, request) {
                    picked.push(*sender);
                    already += 1;
                }
            }
            picked
        };
        if to_enqueue.is_empty() {
            return;
        }

        let enqueued_at_ms = crate::util::now_ms();
        let entries: Vec<PendingContactCrypto> = to_enqueue
            .into_iter()
            .map(|sender| PendingContactCrypto {
                owner_identity_id: *identity_id,
                contact_id: sender,
                op: PendingContactCryptoOp::AutoAccept,
                enqueued_at_ms,
            })
            .collect();

        {
            let mut wm = self.wallet_manager.write().await;
            let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) else {
                return;
            };
            let Some(managed) = info.identity_manager.managed_identity_mut(identity_id) else {
                tracing::warn!(
                    owner = %identity_id,
                    "auto-accept enqueue for a non-resident identity; dropping"
                );
                return;
            };
            for entry in &entries {
                upsert_pending_contact_crypto(
                    managed.dashpay_pending_contact_crypto_mut(),
                    entry.clone(),
                );
            }
        }
        let changeset = PlatformWalletChangeSet {
            pending_contact_crypto_added: entries,
            ..Default::default()
        };
        if let Err(e) = self.persister.store(changeset) {
            tracing::warn!(
                owner = %identity_id, error = %e,
                "failed to persist auto-accept enqueue; will re-enqueue next sweep"
            );
        }
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
        for (contact_id, contact) in managed.dashpay().established_contacts() {
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

        // The recurring sweep has NO signer (it runs unattended in the
        // background), so it can derive NO private-key material — neither the
        // receiving (friendship) xpub nor the ECDH shared secret. Every
        // account-build op is therefore DEFERRED: enqueue it for the
        // signer-backed drain to complete when a signer becomes available
        // (Keychain unlock / signer-present action). The drain fetches the
        // contact, validates the key indices, and performs the derivation.
        //
        // We only SKIP identities that aren't ours to build (unmanaged /
        // out-of-wallet — no HD slot); there is nothing to enqueue for those.
        let is_ours = {
            let wm = self.wallet_manager.read().await;
            wm.get_wallet_info(&self.wallet_id)
                .and_then(|info| info.identity_manager.managed_identity(identity_id))
                .map(|managed| managed.identity_index.is_some())
                .unwrap_or(false)
        };
        if !is_ours {
            tracing::info!(
                identity = %identity_id,
                contact = %contact_id,
                "Skipping DashPay account build for unmanaged/out-of-wallet identity"
            );
            return;
        }

        // Enqueue the deferred crypto ops (receiving xpub + external decrypt).
        // Idempotent per (owner, contact, kind), so re-enqueuing every sweep is
        // a no-op until the drain clears them.
        self.enqueue_deferred_contact_crypto(identity_id, &candidate)
            .await;
        tracing::info!(
            identity = %identity_id,
            contact = %contact_id,
            "Deferred DashPay account build: enqueued for the signer-backed drain"
        );
    }

    /// Enqueue the deferred contact-crypto ops for a contact discovered by the
    /// signerless sweep. The sweep never derives, so this is its only
    /// account-build action; the signer-backed drain completes the ops when a
    /// signer is available. Idempotent per `(owner, contact, kind)` —
    /// re-enqueuing each sweep updates the entry in place. Stores only the
    /// on-chain ciphertext + public key indices, never a secret.
    async fn enqueue_deferred_contact_crypto(
        &self,
        identity_id: &Identifier,
        candidate: &AccountBuildCandidate,
    ) {
        use crate::changeset::{
            upsert_pending_contact_crypto, PendingContactCrypto, PendingContactCryptoOp,
            PlatformWalletChangeSet,
        };
        let enqueued_at_ms = crate::util::now_ms();

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

        // In-memory upsert onto the owner identity's queue, under the write
        // lock (released before persisting). All entries share this owner.
        {
            let mut wm = self.wallet_manager.write().await;
            let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) else {
                return;
            };
            let Some(managed) = info.identity_manager.managed_identity_mut(identity_id) else {
                tracing::warn!(
                    identity = %identity_id, contact = %candidate.contact_id,
                    "deferred contact-crypto enqueue for a non-resident identity; dropping"
                );
                return;
            };
            for entry in &entries {
                upsert_pending_contact_crypto(
                    managed.dashpay_pending_contact_crypto_mut(),
                    entry.clone(),
                );
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

    /// Number of deferred **account-build** contact-crypto ops queued for this
    /// wallet (in-memory): the `RegisterReceiving` / `RegisterExternal` ops that
    /// build a contact's payment account and need a signer unlock to complete.
    ///
    /// A `> 0` count means some contacts are waiting for an unlock to finish
    /// setup. It is a wallet-scoped upper bound — it aggregates across the
    /// wallet's identities and includes ops that may resolve to channel-broken
    /// on the next drain — so a caller should phrase it as "waiting," not
    /// "will succeed." `ContactInfoDecrypt` is excluded (see
    /// [`count_account_build_ops`]). Signerless / public read; no persistence.
    pub async fn pending_contact_crypto_count(&self) -> usize {
        let wm = self.wallet_manager.read().await;
        wm.get_wallet_info(&self.wallet_id)
            .map(|info| {
                info.identity_manager
                    .managed_identities()
                    .map(|m| count_account_build_ops(&m.dashpay().pending_contact_crypto))
                    .sum::<usize>()
            })
            .unwrap_or(0)
    }

    /// Total number of queued contact-crypto entries for this wallet — every
    /// op kind, **including** the `ContactInfoDecrypt` refreshes that
    /// [`Self::pending_contact_crypto_count`] deliberately excludes. This is
    /// the "does a signer-present drain have any work at all" probe: unlike
    /// the banner count (which reports an actionable *backlog* to a user),
    /// a scheduler should run the drain whenever anything is queued, or
    /// ContactInfoDecrypt-only queues would never be applied. Signerless /
    /// public read; no persistence.
    pub async fn drainable_contact_crypto_count(&self) -> usize {
        let wm = self.wallet_manager.read().await;
        wm.get_wallet_info(&self.wallet_id)
            .map(|info| {
                info.identity_manager
                    .managed_identities()
                    .map(|m| m.dashpay().pending_contact_crypto.len())
                    .sum::<usize>()
            })
            .unwrap_or(0)
    }

    /// Drain the persisted deferred-crypto queue using `provider` for the
    /// Keychain-derived key material, stopping once `deadline` passes
    /// (`None` is unbounded). Returns the number of entries completed
    /// (removed from the queue).
    ///
    /// Per entry: run the op; on success remove it and persist the removal; on
    /// unavailable/transient failure leave it for the next drain. The
    /// `RegisterExternal` + `ContactInfoDecrypt` ops (which need a contact
    /// fetch + ECDH/contactInfo derivation) drain in a follow-up and are left
    /// queued here — so calling this is always safe, it just completes what it
    /// can.
    ///
    /// The drain ends between entries, so the count it returns and the queue
    /// removals it persists always describe work that actually completed —
    /// see [`bounded`] for why this cannot be an outer timeout. Entries it
    /// never reached stay queued for the next drain.
    ///
    /// # Crate-private
    ///
    /// Everything this derives comes from whatever seed `provider` resolves,
    /// and none of it is authenticated: a provider mapped to another wallet
    /// registers contact accounts under the wrong xpub, and
    /// `register_contact_account` keys existence on the contact tuple rather
    /// than on the xpub, so the wrong addresses are written once and every
    /// later correct-seed pass no-ops. The check that rules that out lives in
    /// [`Self::drain_pending_contact_crypto_verified`], so this primitive is
    /// reachable only from inside the crate — a caller outside it cannot name
    /// a drain that skipped the check.
    pub(crate) async fn drain_pending_contact_crypto_until<P: ContactCryptoProvider + Sync>(
        &self,
        provider: &P,
        deadline: Option<std::time::Instant>,
    ) -> usize {
        use crate::changeset::{PendingContactCryptoKey, PendingContactCryptoOp};

        // Snapshot every resident identity's queue into one flat owned Vec
        // (each entry self-identifies by `owner_identity_id`), then run the
        // async ops without holding the lock.
        let entries: Vec<crate::changeset::PendingContactCrypto> = {
            let wm = self.wallet_manager.read().await;
            wm.get_wallet_info(&self.wallet_id)
                .map(|info| {
                    info.identity_manager
                        .managed_identities()
                        .flat_map(|m| m.dashpay().pending_contact_crypto.iter().cloned())
                        .collect()
                })
                .unwrap_or_default()
        };
        if entries.is_empty() {
            return 0;
        }

        // Our identity's key inventory, once per drain that has external
        // builds queued. The whole legacy-cohort bug is a statement about this
        // layout — an identity minted before DashPay encryption keys existed
        // carries only AUTHENTICATION/TRANSFER slots, so inbound requests
        // reference those ids and nothing downstream makes sense without
        // knowing that. Reading it back off an exported log beats asking the
        // user to query Platform. On-chain public metadata only; no key data.
        // Gated on the level: the block allocates a set, a string per key and a
        // join, and takes the wallet-manager read lock. Purpose-rejected
        // entries stay queued and revisit this path every sweep, so leaving
        // that work unconditional would add recurring cost to the very path
        // this change exists to make cheap.
        if tracing::enabled!(tracing::Level::INFO) {
            use dpp::identity::accessors::IdentityGettersV0;
            use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
            let owners: std::collections::BTreeSet<Identifier> = entries
                .iter()
                .filter(|e| matches!(e.op, PendingContactCryptoOp::RegisterExternal { .. }))
                .map(|e| e.owner_identity_id)
                .collect();
            if !owners.is_empty() {
                let wm = self.wallet_manager.read().await;
                if let Some(info) = wm.get_wallet_info(&self.wallet_id) {
                    for owner in owners {
                        let Some(managed) = info.identity_manager.managed_identity(&owner) else {
                            continue;
                        };
                        let keys: Vec<String> = managed
                            .identity
                            .public_keys()
                            .iter()
                            .map(|(id, k)| {
                                format!(
                                    "{id}:{:?}/{:?}{}",
                                    k.purpose(),
                                    k.key_type(),
                                    if k.disabled_at().is_some() {
                                        "/DISABLED"
                                    } else {
                                        ""
                                    }
                                )
                            })
                            .collect();
                        tracing::info!(
                            owner = %owner,
                            identity_index = ?managed.identity_index,
                            keys = %keys.join(" "),
                            "drain: our identity key inventory (id:purpose/type)"
                        );
                    }
                }
            }
        }

        let mut cleared: Vec<PendingContactCryptoKey> = Vec::new();
        // Distinct key-purpose rejections seen this drain, and how many entries
        // each blocked — summarised once at the end instead of one WARN per
        // entry. A purpose-rejected entry stays queued by design (the policy,
        // not the immutable document, is what might change), so per-entry
        // WARNing repeats every sweep for the life of the wallet: mainnet logs
        // from one wallet show 396 such lines for 27 contacts in a single
        // session. Every reason still reaches the log, once, with its count.
        let mut policy_blocked: std::collections::BTreeMap<String, usize> =
            std::collections::BTreeMap::new();
        // How much of `cleared` is already dequeued + persisted, and the running
        // total actually removed. Bookkeeping lands per entry, so at most one
        // entry's worth can ever be in flight.
        let mut flushed: usize = 0;
        let mut drained_total: usize = 0;
        for entry in &entries {
            // Land the previous entry's dequeue before starting any new work.
            if cleared.len() > flushed {
                drained_total += self
                    .flush_drained_contact_crypto(&entries, &cleared[flushed..])
                    .await;
                flushed = cleared.len();
            }
            // Stop between entries, never inside one: an entry that has already
            // committed its side effect must reach the `cleared.push` that
            // records it.
            if budget_spent(deadline) {
                tracing::info!(
                    processed = cleared.len(),
                    total = entries.len(),
                    "drain: budget spent; leaving the rest queued"
                );
                break;
            }
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
                    // Bounded: the derive is the last step before this entry
                    // commits anything, so abandoning it changes no state.
                    let Some(xpub) = bounded(deadline, provider.receiving_xpub(&path)).await else {
                        tracing::info!(
                            owner = %entry.owner_identity_id, contact = %entry.contact_id,
                            "drain: budget spent deriving the receiving xpub; leaving queued"
                        );
                        continue;
                    };
                    match xpub {
                        Ok(xpub) => match self
                            .register_contact_account(
                                &entry.owner_identity_id,
                                &entry.contact_id,
                                0,
                                xpub,
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
                    let path = match IdentityWallet::<B>::identity_auth_derivation_path(
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

                    // Validate OUR key first — it needs nothing but the
                    // resident identity, so a request that can never be used is
                    // rejected before spending a Platform round trip on the
                    // contact. This is the dominant rejection in practice
                    // (legacy documents reference our AUTHENTICATION/TRANSFER
                    // key), and because such an entry stays queued the fetch
                    // below would otherwise repeat on every sweep, forever.
                    //
                    // Deciding here means a MIXED failure — our key
                    // purpose-rejected and the contact's key hard-faulted —
                    // leaves the entry queued where a composed validator would
                    // mark the channel broken. Deliberate: see
                    // `validate_recipient_key`. Marking broken is unappealable
                    // by the user, and the retry it avoids costs no fetch.
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
                    // Did only the widened receive-side policy let this request
                    // through — i.e. does either referenced key name a purpose
                    // we would never mint ourselves? That marks it as the
                    // legacy dashj cohort, whose ECDH/AES byte compatibility
                    // with our implementation has not been cross-validated
                    // against a dashj-produced payload. Used far below to keep
                    // a decrypt failure from being charged to the document.
                    //
                    // The recipient term is known here; the sender term needs
                    // the identity fetched below and is OR-ed in there.
                    let recipient_widened = our_identity
                        .get_public_key_by_id(*our_decryption_key_index)
                        .map(|k| {
                            !dash_sdk::platform::dashpay::recipient_key_purpose_is_valid(
                                k.purpose(),
                            )
                        })
                        .unwrap_or(false);
                    let recipient_validation =
                        crate::wallet::identity::crypto::validation::validate_recipient_key(
                            &our_identity,
                            *our_decryption_key_index,
                        );
                    if !recipient_validation.is_valid {
                        if self
                            .apply_drain_validation_failure(
                                entry,
                                recipient_validation,
                                &mut policy_blocked,
                            )
                            .await
                        {
                            cleared.push(entry.key());
                        }
                        continue;
                    }

                    // Fetch the contact identity (transient on failure → leave).
                    // Bounded: a Platform round trip, and nothing in this entry
                    // has committed yet.
                    let contact_identity = {
                        use dash_sdk::platform::Fetch;
                        let fetched =
                            bounded(deadline, Identity::fetch(&self.sdk, entry.contact_id)).await;
                        let Some(fetched) = fetched else {
                            tracing::info!(
                                owner = %entry.owner_identity_id, contact = %entry.contact_id,
                                "drain: budget spent fetching the contact identity; leaving queued"
                            );
                            continue;
                        };
                        match fetched {
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

                    // The sender side of the legacy classification — the
                    // widening moved the sender rule from ENCRYPTION-only to
                    // ENCRYPTION-or-AUTHENTICATION too, so an AUTHENTICATION
                    // sender against a mint-valid recipient is just as much an
                    // unverified legacy payload.
                    let accepted_by_legacy_widening = recipient_widened
                        || contact_identity
                            .get_public_key_by_id(*contact_encryption_key_index)
                            .map(|k| k.purpose() != Purpose::ENCRYPTION)
                            .unwrap_or(false);

                    // The sender half — the checks that need the identity we
                    // just fetched.
                    let validation =
                        crate::wallet::identity::crypto::validation::validate_sender_key(
                            &contact_identity,
                            *contact_encryption_key_index,
                        );
                    if !validation.is_valid {
                        if self
                            .apply_drain_validation_failure(entry, validation, &mut policy_blocked)
                            .await
                        {
                            cleared.push(entry.key());
                        }
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
                            // Left queued, not broken: the contact's identity
                            // can gain this key later, exactly as ours can, and
                            // the document that names it cleared consensus and
                            // cannot be re-minted. Breaking here would end the
                            // relationship over a gap that may close by itself.
                            tracing::warn!(
                                owner = %entry.owner_identity_id, contact = %entry.contact_id,
                                key_index = *contact_encryption_key_index,
                                "drain: contact has no key at the referenced index yet; \
                                 leaving queued (not marking broken)"
                            );
                            continue;
                        }
                    };

                    // Everything the external build is about to depend on, in
                    // one line, BEFORE it can fail. Recorded at INFO because
                    // the legacy cohort's viability is an open question that
                    // only real mainnet wallets can answer, and an exported log
                    // is the only channel we get: without this, a failure below
                    // says what broke but not what it was working from.
                    //
                    // Public metadata only — key ids, purposes, types and
                    // lengths. Never the shared secret, and never the
                    // decrypted xpub.
                    {
                        use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
                        let our_key = our_identity.get_public_key_by_id(*our_decryption_key_index);
                        let their_key =
                            contact_identity.get_public_key_by_id(*contact_encryption_key_index);
                        tracing::info!(
                            owner = %entry.owner_identity_id,
                            contact = %entry.contact_id,
                            identity_index,
                            our_key_id = *our_decryption_key_index,
                            our_key_purpose = ?our_key.map(|k| k.purpose()),
                            our_key_type = ?our_key.map(|k| k.key_type()),
                            their_key_id = *contact_encryption_key_index,
                            their_key_purpose = ?their_key.map(|k| k.purpose()),
                            their_key_type = ?their_key.map(|k| k.key_type()),
                            ciphertext_len = encrypted_public_key.len(),
                            legacy_widened = accepted_by_legacy_widening,
                            ecdh_path = %path,
                            "drain: building external account"
                        );
                    }

                    // ECDH via the Keychain-backed provider (scalar stays in the
                    // signer; we only get the shared secret).
                    // Bounded: the last step before the external-account
                    // registration commits.
                    let Some(shared) =
                        bounded(deadline, provider.ecdh_shared_secret(&path, &peer)).await
                    else {
                        tracing::info!(
                            owner = %entry.owner_identity_id, contact = %entry.contact_id,
                            "drain: budget spent on ECDH; leaving queued"
                        );
                        continue;
                    };
                    let shared = match shared {
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
                            shared.clone(),
                        )
                        .await
                    {
                        Ok(registration) => {
                            // Stamp the rotation self-heal marker and clear
                            // any stale broken-channel flag — only when the
                            // account was actually (re)built from this
                            // entry's payload. An `AlreadyExisted` no-op may
                            // have hit a pre-rotation row (the account key
                            // ignores `account_reference`); stamping it as
                            // current would suppress the sweep's teardown +
                            // rebuild forever (same reasoning as the accept
                            // path).
                            if registration == ExternalAccountRegistration::Built {
                                self.note_external_account_registered(
                                    &entry.owner_identity_id,
                                    &entry.contact_id,
                                    encrypted_public_key,
                                )
                                .await;
                            }
                            // Surface the contact's account label from the same
                            // ECDH shared key (best-effort, cosmetic — never
                            // fails the drain).
                            self.store_contact_account_label(
                                &entry.owner_identity_id,
                                &entry.contact_id,
                                &shared,
                            )
                            .await;
                            cleared.push(entry.key());
                        }
                        // A permanent fault on a legacy-cohort request is NOT
                        // charged to the document. Decrypt and compact-xpub
                        // parse are the only gates on the plaintext, so an
                        // ECDH/AES convention gap between us and dashj would
                        // surface here as a "permanent" fault and break every
                        // legacy channel at once — and a broken channel only
                        // heals when the CONTACT sends a fresh request, an
                        // appeal the user cannot file. Leaving it queued keeps
                        // a later convention fix able to recover it, and costs
                        // only a retry.
                        Err(e) if e.is_permanent() && accepted_by_legacy_widening => {
                            tracing::warn!(
                                owner = %entry.owner_identity_id, contact = %entry.contact_id,
                                error = %e.into_inner(),
                                "drain: legacy-cohort external register failed; leaving queued \
                                 (not marking broken — may be our own convention gap)"
                            );
                            continue;
                        }
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
                    // Re-fetch the owner's contactInfo docs + decrypt + apply via
                    // the signer (the op carries no payload, so the latest
                    // published version always wins). The owner-ownership /
                    // confused-deputy guard lives in `drain_contact_info_decrypt`.
                    // Bounded as a whole: its apply runs under a single write
                    // lock with no await between persist and the in-memory
                    // mutation, so abandoning it either leaves nothing applied
                    // or leaves it fully applied and still queued — and a
                    // re-run re-fetches the latest published version anyway.
                    let decrypted = bounded(
                        deadline,
                        self.drain_contact_info_decrypt(&entry.owner_identity_id, provider),
                    )
                    .await;
                    let Some(decrypted) = decrypted else {
                        tracing::info!(
                            owner = %entry.owner_identity_id,
                            "drain: budget spent decrypting contactInfo; leaving queued"
                        );
                        continue;
                    };
                    match decrypted {
                        Ok(applied) => {
                            tracing::debug!(
                                owner = %entry.owner_identity_id, applied,
                                "drain: contactInfo decrypted + applied"
                            );
                            cleared.push(entry.key());
                        }
                        // Provider/fetch failure (signer unavailable, network blip,
                        // or a non-owned entry) → leave queued for the next drain.
                        Err(e) => tracing::warn!(
                            owner = %entry.owner_identity_id, error = %e,
                            "drain: contactInfo decrypt failed; leaving queued"
                        ),
                    }
                }
                PendingContactCryptoOp::AutoAccept => {
                    // Verifying the proof and sending the reciprocal both need the
                    // identity signer, which this provider-only drain doesn't
                    // carry. Handled by `drain_auto_accepts` at a signer-present
                    // moment; skip here so the entry stays queued (the inbound
                    // request remains manually acceptable meanwhile).
                }
            }
        }

        // Land whatever the last entry completed.
        drained_total += self
            .flush_drained_contact_crypto(&entries, &cleared[flushed..])
            .await;

        // One line for every entry the key-purpose policy turned away, instead
        // of one per entry per sweep. Kept at WARN and carrying the distinct
        // reasons: this is the signal that a live on-chain cohort is failing
        // our acceptance policy, which is exactly how the legacy dashj cohort
        // was found — it must stay visible in an exported log, just not 27
        // times a pass.
        if !policy_blocked.is_empty() {
            let blocked: usize = policy_blocked.values().sum();
            tracing::warn!(
                entries = blocked,
                reasons = ?policy_blocked,
                "drain: contact requests left queued by the key-purpose policy \
                 (not marking broken; they retry when the policy changes)"
            );
        }
        // One-line verdict for the pass. "Did the legacy contacts build?" is
        // answerable from this alone, without counting per-entry lines across a
        // multi-megabyte export.
        tracing::info!(
            entries = entries.len(),
            drained = drained_total,
            still_queued = entries.len().saturating_sub(drained_total),
            "drain: pass complete"
        );

        drained_total
    }

    /// The drain's validation-failure policy, shared by the recipient-half and
    /// sender-half checks so both halves classify identically.
    ///
    /// - A failure that can still resolve — a purpose mismatch (our acceptance
    ///   policy might change) or an absent key id (identities gain keys) — is
    ///   counted into `policy_blocked` and left queued. The `contactRequest`
    ///   cleared consensus and is immutable; a channel marked broken here needs
    ///   a superseding request from the CONTACT to heal, an appeal the user
    ///   cannot file.
    /// - Only a fault that immutable facts make permanent — a key type that
    ///   cannot do ECDH, a key we disabled — breaks the channel, so the sweep
    ///   stops collecting it.
    ///
    /// Returns `true` when the caller should clear the entry from the queue.
    ///
    /// Takes `validation` by value: a purpose-rejected entry stays queued by
    /// design and comes back through here on every sweep, so cloning its
    /// reasons into the summary would allocate once per contact per pass for
    /// the life of the wallet. Moving them costs nothing — neither caller uses
    /// the result afterwards.
    async fn apply_drain_validation_failure(
        &self,
        entry: &crate::changeset::PendingContactCrypto,
        validation: crate::wallet::identity::crypto::validation::ContactRequestValidation,
        policy_blocked: &mut std::collections::BTreeMap<String, usize>,
    ) -> bool {
        if !validation.is_permanent() {
            tracing::debug!(
                owner = %entry.owner_identity_id, contact = %entry.contact_id,
                errors = ?validation.errors,
                "drain: contact request key-purpose mismatch; leaving queued (not marking broken)"
            );
            for reason in validation.errors {
                *policy_blocked.entry(reason).or_default() += 1;
            }
            return false;
        }
        tracing::warn!(
            owner = %entry.owner_identity_id, contact = %entry.contact_id,
            errors = ?validation.errors,
            "drain: contact request failed key-index validation; marking channel broken"
        );
        self.mark_contact_channel_broken(&entry.owner_identity_id, &entry.contact_id)
            .await;
        true
    }

    /// Apply the dequeue for entries a drain just completed: remove them from
    /// their owners' in-memory queues and persist the removal. Returns how many
    /// were actually removed.
    ///
    /// Called after **each** completed entry rather than once per drain. The
    /// per-entry side effects (`register_contact_account`, the reciprocal send)
    /// commit as the loop runs, so batching the bookkeeping to the end means a
    /// drain that stops — its budget, or a caller that drops the future —
    /// leaves work it really finished still queued and reported as zero. One
    /// entry's work and one entry's dequeue now land together.
    async fn flush_drained_contact_crypto(
        &self,
        entries: &[crate::changeset::PendingContactCrypto],
        cleared: &[crate::changeset::PendingContactCryptoKey],
    ) -> usize {
        if cleared.is_empty() {
            return 0;
        }

        // The drain ran over a lock-free SNAPSHOT: a concurrent rotation sweep
        // may have `upsert`ed a fresh payload for one of these keys mid-drain.
        // Removal must be value-aware — collect the full snapshot entries the
        // drain actually processed so a payload changed under us survives to
        // the next drain (the queue holds at most one entry per key, so the
        // lookup is unambiguous).
        let cleared_snapshots: Vec<crate::changeset::PendingContactCrypto> = cleared
            .iter()
            .filter_map(|k| entries.iter().find(|e| e.key() == *k).cloned())
            .collect();

        // Remove the completed entries from the in-memory queue + persist the
        // removal so they don't replay after a restart. Only remove a live
        // entry still value-equal to the snapshot's — a mid-drain upsert
        // (changed payload) is left queued for the next drain rather than
        // clobbered by this stale snapshot.
        let removed: Vec<crate::changeset::PendingContactCryptoKey> = {
            let mut wm = self.wallet_manager.write().await;
            match wm.get_wallet_info_mut(&self.wallet_id) {
                Some(info) => {
                    // Route each drained snapshot entry back to its owner's queue
                    // and apply the value-aware retain there. Owner is on every
                    // entry (and in the equality key), so this preserves the
                    // concurrent-upsert safety per identity. An identity removed
                    // between the snapshot and here is skipped: its queue died
                    // with it, and `apply` ignores the cleared delta anyway.
                    let mut removed = Vec::new();
                    for snap in &cleared_snapshots {
                        if let Some(managed) = info
                            .identity_manager
                            .managed_identity_mut(&snap.owner_identity_id)
                        {
                            removed.extend(retain_drained_by_snapshot(
                                managed.dashpay_pending_contact_crypto_mut(),
                                std::slice::from_ref(snap),
                            ));
                        }
                    }
                    removed
                }
                None => Vec::new(),
            }
        };
        if removed.is_empty() {
            return 0;
        }
        let removed_count = removed.len();
        let changeset = crate::changeset::PlatformWalletChangeSet {
            pending_contact_crypto_cleared: removed,
            ..Default::default()
        };
        if let Err(e) = self.persister.store(changeset) {
            tracing::warn!(
                error = %e,
                "drain: failed to persist cleared queue entries (in-memory already updated)"
            );
        }

        removed_count
    }

    /// Drain queued `AutoAccept` ops (DIP-15 QR auto-accept) — verify each
    /// inbound request's `autoAcceptProof` and, if valid + unexpired,
    /// auto-accept it (send the reciprocal), stopping once `deadline` passes
    /// (`None` is unbounded). Requires the identity `signer` (the reciprocal
    /// is a signed state transition) as well as the crypto `provider` (to
    /// derive our auto-accept public key); the provider-only
    /// `drain_pending_contact_crypto_until` deliberately skips these. Returns
    /// the number auto-accepted.
    ///
    /// Anti-DoS: the cheap local checks (proof present, expiry, ECDSA verify
    /// against our own re-derived key) run **before** any network/accept, so a
    /// flood of junk proofs is cleared without per-entry round-trips. Verdict
    /// mapping: invalid / expired / malformed / bad-index ⇒ permanent (clear);
    /// provider-unavailable / accept-send failure ⇒ transient (leave queued).
    ///
    /// Ends between entries, so a reciprocal that was sent is always recorded
    /// as accepted — see [`bounded`] for why an outer timeout would not hold
    /// that. Entries it never reached stay queued.
    ///
    /// # Crate-private
    ///
    /// A provider resolving another wallet's seed re-derives the wrong
    /// auto-accept key, so a valid proof fails to verify — and the verdict
    /// mapping calls a verify failure *permanent*, clearing the entry. The
    /// damage is a contact request silently dropped and never offered again,
    /// which is why this primitive is reachable only through
    /// [`Self::drain_auto_accepts_verified`] and only from inside the crate.
    pub(crate) async fn drain_auto_accepts_until<S, P>(
        &self,
        signer: &S,
        provider: &P,
        deadline: Option<std::time::Instant>,
    ) -> usize
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
        P: ContactCryptoProvider + Sync,
    {
        use crate::changeset::{PendingContactCryptoKey, PendingContactCryptoOp};
        use crate::wallet::identity::crypto::auto_accept::{
            auto_accept_derivation_path, auto_accept_proof_expiry,
            verify_auto_accept_proof_with_pubkey,
        };

        // Snapshot just the AutoAccept entries, across every resident identity
        // (both buckets — an AutoAccept op can legitimately sit on an
        // out-of-wallet identity, since the enqueue gate isn't wallet-scoped).
        let entries: Vec<crate::changeset::PendingContactCrypto> = {
            let wm = self.wallet_manager.read().await;
            wm.get_wallet_info(&self.wallet_id)
                .map(|info| {
                    info.identity_manager
                        .managed_identities()
                        .flat_map(|m| m.dashpay().pending_contact_crypto.iter())
                        .filter(|e| matches!(e.op, PendingContactCryptoOp::AutoAccept))
                        .cloned()
                        .collect()
                })
                .unwrap_or_default()
        };
        if entries.is_empty() {
            return 0;
        }

        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let mut cleared: Vec<PendingContactCryptoKey> = Vec::new();
        // Permanent verify failures to mark so the sync sweep's enqueue gate
        // does not re-queue the same bad proof every pass. `(owner, sender,
        // proof)` — transient failures (provider unavailable / reciprocal-send
        // failure) are NOT pushed here so they stay retryable.
        let mut verify_failed: Vec<(Identifier, Identifier, Vec<u8>)> = Vec::new();
        let mut accepted: usize = 0;
        // How much of each has already been applied — see the loop head.
        let mut flushed: usize = 0;
        let mut verify_flushed: usize = 0;

        for entry in &entries {
            // Land the previous entry's dequeue + verify-failure marks before
            // starting new work, so a stop can strand at most one entry's.
            if cleared.len() > flushed || verify_failed.len() > verify_flushed {
                self.flush_cleared_auto_accepts(
                    &cleared[flushed..],
                    &verify_failed[verify_flushed..],
                )
                .await;
                flushed = cleared.len();
                verify_flushed = verify_failed.len();
            }
            // Stop between entries: an accept whose reciprocal already went out
            // must reach its `cleared.push` / `accepted += 1`.
            if budget_spent(deadline) {
                tracing::info!(
                    processed = cleared.len(),
                    total = entries.len(),
                    "auto-accept: budget spent; leaving the rest queued"
                );
                break;
            }
            let owner = entry.owner_identity_id; // us (the QR owner / recipient)
            let sender = entry.contact_id; // the scanner (request $ownerId)

            // Re-load the inbound request (carrying the proof) from local state.
            let request = {
                let wm = self.wallet_manager.read().await;
                wm.get_wallet_info(&self.wallet_id)
                    .and_then(|info| info.identity_manager.managed_identity(&owner))
                    .and_then(|mi| {
                        mi.dashpay()
                            .incoming_contact_requests()
                            .get(&sender)
                            .cloned()
                    })
            };
            let Some(request) = request else {
                // Gone (already established / removed) — nothing to do.
                cleared.push(entry.key());
                continue;
            };
            let Some(proof) = request.auto_accept_proof.as_deref() else {
                cleared.push(entry.key()); // no proof (shouldn't happen) — permanent
                continue;
            };

            // Expiry is the proof's embedded index — the same value that keys
            // verification, so it can't be lied about independently of the sig.
            let Some(expiry) = auto_accept_proof_expiry(proof) else {
                cleared.push(entry.key()); // malformed — permanent
                verify_failed.push((owner, sender, proof.to_vec()));
                continue;
            };
            if now_secs > expiry as u64 {
                tracing::info!(
                    owner = %owner, sender = %sender, expiry,
                    "auto-accept: proof expired; clearing (request stays manually acceptable)"
                );
                cleared.push(entry.key()); // expired — permanent
                verify_failed.push((owner, sender, proof.to_vec()));
                continue;
            }

            // Derive OUR auto-accept public key at the proof's expiry, via the
            // provider (seedless — no resident wallet). Local ECDSA verify runs
            // before any network/accept (anti-DoS). Bind the sender to the
            // consensus-authenticated request `$ownerId` (request.sender_id).
            let path = match auto_accept_derivation_path(self.sdk.network, expiry) {
                Ok(p) => p,
                Err(e) => {
                    tracing::warn!(owner = %owner, sender = %sender, error = %e,
                        "auto-accept: bad expiry index; clearing");
                    cleared.push(entry.key()); // bad index — permanent
                    verify_failed.push((owner, sender, proof.to_vec()));
                    continue;
                }
            };
            // Bounded: the local verify and the accept both come after this,
            // so abandoning the derive commits nothing.
            let Some(derived) = bounded(deadline, provider.receiving_xpub(&path)).await else {
                tracing::info!(owner = %owner, sender = %sender,
                    "auto-accept: budget spent deriving our proof key; leaving queued");
                continue;
            };
            let pubkey = match derived {
                Ok(xpub) => xpub.public_key,
                Err(e) => {
                    tracing::warn!(owner = %owner, sender = %sender, error = %e,
                        "auto-accept: provider unavailable; leaving queued");
                    continue; // transient — leave queued
                }
            };
            let valid = verify_auto_accept_proof_with_pubkey(
                &pubkey,
                proof,
                &request.sender_id,
                &owner,
                request.account_reference,
            );
            if !valid {
                tracing::warn!(owner = %owner, sender = %sender,
                    "auto-accept: proof did not verify; clearing");
                cleared.push(entry.key()); // invalid — permanent
                verify_failed.push((owner, sender, proof.to_vec()));
                continue;
            }

            // Valid + unexpired → accept (send the reciprocal). Idempotent.
            match self
                .accept_contact_request_with_external_signer(&request, signer, provider)
                .await
            {
                Ok(_) => {
                    tracing::info!(owner = %owner, sender = %sender,
                        "auto-accept: proof verified; contact auto-accepted");
                    accepted += 1;
                    cleared.push(entry.key());
                }
                Err(e) => tracing::warn!(owner = %owner, sender = %sender, error = %e,
                    "auto-accept: reciprocal send failed; leaving queued"),
            }
        }

        // Land whatever the last entry resolved.
        self.flush_cleared_auto_accepts(&cleared[flushed..], &verify_failed[verify_flushed..])
            .await;

        accepted
    }

    /// Dequeue the `AutoAccept` entries a drain just resolved and record the
    /// permanent verify failures among them. Called after each resolved entry,
    /// for the same reason as [`Self::flush_drained_contact_crypto`]: a
    /// reciprocal that has already been broadcast must not be able to end up
    /// still queued because the drain stopped before its bookkeeping ran.
    async fn flush_cleared_auto_accepts(
        &self,
        cleared: &[crate::changeset::PendingContactCryptoKey],
        verify_failed: &[(Identifier, Identifier, Vec<u8>)],
    ) {
        // Each list is applied on its own. The caller advances both cursors
        // after this returns, so gating one on the other would discard the
        // ungated one for good. Not reachable today — every `verify_failed`
        // push is paired with a `cleared` push — but a future "leave queued,
        // remember the bad proof" case would lose its marks silently.
        if cleared.is_empty() && verify_failed.is_empty() {
            return;
        }

        {
            let mut wm = self.wallet_manager.write().await;
            if let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) {
                if !cleared.is_empty() {
                    // Remove the cleared entries from their owners' queues. Each
                    // cleared key names its owner, and only that owner's queue can
                    // hold it, so retain each affected owner's queue against the
                    // full cleared set. A plain key-`retain` is sufficient here
                    // (unlike the value-aware `retain_drained_by_snapshot` the main
                    // drain uses): `AutoAccept` is a payload-less unit variant, so a
                    // concurrent mid-drain upsert can't change an entry's value —
                    // key-equality is total.
                    let mut cleared_owners: Vec<Identifier> =
                        cleared.iter().map(|k| k.owner_identity_id).collect();
                    cleared_owners.sort();
                    cleared_owners.dedup();
                    for owner in &cleared_owners {
                        if let Some(managed) = info.identity_manager.managed_identity_mut(owner) {
                            managed
                                .dashpay_pending_contact_crypto_mut()
                                .retain(|e| !cleared.iter().any(|k| *k == e.key()));
                        }
                    }
                }
                // Record permanent verify failures so the next sweep's
                // enqueue gate skips the same bad proof (in-memory only —
                // retried once per launch; the request stays manually
                // acceptable).
                for (owner, sender, proof) in verify_failed {
                    if let Some(managed) = info.identity_manager.managed_identity_mut(owner) {
                        managed.mark_auto_accept_verify_failed(sender, proof);
                    }
                }
            }
        }

        // Only the dequeue is persisted; the verify-failure marks are in-memory
        // by design.
        if cleared.is_empty() {
            return;
        }
        let changeset = crate::changeset::PlatformWalletChangeSet {
            pending_contact_crypto_cleared: cleared.to_vec(),
            ..Default::default()
        };
        if let Err(e) = self.persister.store(changeset) {
            tracing::warn!(error = %e,
                "auto-accept: failed to persist cleared entries (in-memory already updated)");
        }
    }

    /// Build a DIP-15 auto-accept QR URI (`dash:?du=<username>&dapk=<key_blob>`),
    /// valid for [`AUTO_ACCEPT_TTL_SECS`](crate::wallet::identity::crypto::auto_accept::AUTO_ACCEPT_TTL_SECS).
    ///
    /// Derives the wallet's auto-accept key at `m/9'/coin'/16'/expiry'` via
    /// `provider` — the deliberate raw-key export (the key is a bearer credential
    /// the QR shares) — encodes the 38-byte `dapk` blob, and assembles the URI.
    ///
    /// The QR's `du` is `owner_identity_id`'s DPNS name (a scanner resolves it
    /// back to the owner's identity). `username` is the locally-cached name when
    /// available; pass an empty string to resolve it on-chain instead — imported
    /// or restored identities carry the name on-chain but not in the local cache,
    /// which is exactly when this matters. Errors if no name can be found.
    pub async fn build_auto_accept_qr<P: ContactCryptoProvider + Sync>(
        &self,
        owner_identity_id: &Identifier,
        username: &str,
        provider: &P,
    ) -> Result<String, PlatformWalletError> {
        use crate::wallet::identity::crypto::auto_accept::{
            auto_accept_derivation_path, encode_auto_accept_key_blob, encode_dashpay_contact_uri,
            AUTO_ACCEPT_TTL_SECS,
        };
        // Prefer the caller-supplied (cached) name; fall back to an on-chain
        // lookup when it is empty so the QR works regardless of local caching.
        let resolved_name;
        let username = if username.is_empty() {
            let names = self
                .sdk
                .get_dpns_usernames_by_identity(*owner_identity_id, None)
                .await
                .map_err(|e| {
                    PlatformWalletError::InvalidIdentityData(format!(
                        "auto-accept QR: failed to resolve a DPNS name on-chain for \
                         {owner_identity_id}: {e}"
                    ))
                })?;
            // Any of the identity's names resolves back to it, so the choice is
            // cosmetic; pick the lexicographically smallest for a deterministic
            // QR that stays stable across rebuilds. The app's "main name"
            // preference isn't visible on this on-chain path — the cached-name
            // branch above honors it whenever a local name is present.
            resolved_name = names.into_iter().map(|n| n.label).min().ok_or_else(|| {
                PlatformWalletError::InvalidIdentityData(
                    "auto-accept QR requires a DPNS username; none is registered for this identity"
                        .to_string(),
                )
            })?;
            resolved_name.as_str()
        } else {
            username
        };
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as u32)
            .unwrap_or(0);
        let expiry = now.saturating_add(AUTO_ACCEPT_TTL_SECS);
        let path = auto_accept_derivation_path(self.sdk.network, expiry)?;
        let secret_key = provider.export_auto_accept_private_key(&path).await?;
        let blob = encode_auto_accept_key_blob(&secret_key, expiry);
        Ok(encode_dashpay_contact_uri(username, &blob))
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
        let Some(contact) = managed.established_contact_mut(contact_id) else {
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

    /// Record that the contact's outbound `DashpayExternalAccount` is now
    /// built from the contact's current `incoming_request.account_reference`,
    /// and clear any stale `payment_channel_broken` flag — the two success
    /// side-effects of a completed [`register_external_contact_account`].
    ///
    /// - Stamping `external_account_reference` lets the sweep's rotation
    ///   self-heal skip a healthy account (marker matches the tracked
    ///   reference) and detect a stale one after a restart (F2).
    /// - Clearing `payment_channel_broken` heals a channel that a prior
    ///   permanent-fault marked broken but that a successful re-register (via
    ///   the drain or a user re-accept) proves is usable again — otherwise the
    ///   UI reports "broken" and blocks sending forever, since the sweep skips
    ///   broken contacts and never re-registers them (F12).
    ///
    /// Idempotent (skips the persist when nothing changed). Takes its own
    /// write guard; the caller must hold no wallet-manager guard.
    ///
    /// `built_from_ciphertext` is the `encryptedPublicKey` blob the account
    /// was actually registered from. Registration and this stamp run under
    /// SEPARATE guards (the drain awaits the ECDH provider lock-free in
    /// between), so a rotation sweep can advance `incoming_request` after
    /// the payload was snapshotted but before this stamp. Stamping the LIVE
    /// reference in that window would mark an account built from the
    /// rotated-away xpub as current — `external_account_needs_rebuild` then
    /// never fires and `send_payment` silently derives addresses the
    /// contact no longer watches. Comparing the live ciphertext against the
    /// one we built from detects the race; on mismatch the marker is left
    /// stale (or `None`) so the sweep's teardown + rebuild picks up the
    /// fresh request.
    pub(crate) async fn note_external_account_registered(
        &self,
        identity_id: &Identifier,
        contact_id: &Identifier,
        built_from_ciphertext: &[u8],
    ) {
        let mut wm = self.wallet_manager.write().await;
        let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) else {
            return;
        };
        let Some(managed) = info.identity_manager.managed_identity_mut(identity_id) else {
            return;
        };
        let Some(contact) = managed.established_contact_mut(contact_id) else {
            return;
        };
        if contact.incoming_request.encrypted_public_key != built_from_ciphertext {
            tracing::warn!(
                owner = %identity_id, contact = %contact_id,
                "external account registered from a superseded payload (rotation raced \
                 the registration); leaving the self-heal marker stale so the sweep rebuilds"
            );
            return;
        }
        let current_reference = contact.incoming_request.account_reference;
        let already_current = contact.external_account_reference == Some(current_reference);
        if already_current && !contact.payment_channel_broken {
            return;
        }
        // Persist BEFORE committing to memory: if the store fails, the
        // in-memory marker must stay stale so the `already_current` guard
        // above and the sweep's `external_account_needs_rebuild` predicate
        // keep retriggering until a persist succeeds — otherwise memory runs
        // ahead of disk and nothing in-process ever retries (only a restart
        // reloading the stale marker would heal it).
        let mut updated = contact.clone();
        updated.external_account_reference = Some(current_reference);
        updated.payment_channel_broken = false;

        let mut cs = crate::changeset::ContactChangeSet::default();
        cs.established.insert(
            crate::changeset::SentContactRequestKey {
                owner_id: *identity_id,
                recipient_id: *contact_id,
            },
            updated.clone(),
        );
        if let Err(e) = self.persister.store(cs.into()) {
            tracing::error!(
                "Failed to persist external-account-registered changeset: {}",
                e
            );
            return;
        }
        *contact = updated;
    }

    /// Decrypt the contact's incoming `encryptedAccountLabel` with the ECDH
    /// shared key and store the printable plaintext on the established
    /// contact's `contact_account_label`. Best-effort and cosmetic: a missing
    /// label, a decrypt/UTF-8 failure, or non-printable garbage leaves or sets
    /// `None` — it never breaks the payment channel or fails the caller.
    ///
    /// The label is derived strictly from the **incoming** request (the label
    /// the contact chose for the account they shared); the outgoing request
    /// carries a label *we* sent and is never a source. AES-CBC has no
    /// integrity, so a corrupt or non-conforming-sender ciphertext can unpad
    /// into valid-UTF-8 garbage — empty / whitespace-only / control-char
    /// results are coerced to `None` so the UI shows nothing rather than
    /// garbage.
    ///
    /// Written to the in-memory `established` contact and flushed through an
    /// `established` changeset entry. Unlike the broken-channel flag, the label
    /// is **not** durably persisted by the SQLite backend: it is intentionally
    /// re-derived from the incoming request on the next contact-info sweep so it
    /// never goes stale against rotated key material (the field resets to `None`
    /// on every (re-)establish/rotation) — a cold restart restores it empty and
    /// the next sweep repopulates it. Self-contained locking: takes its own
    /// write guard, and the decrypt is synchronous, so nothing awaits or
    /// re-locks under it. The caller must hold no wallet-manager guard.
    pub(crate) async fn store_contact_account_label(
        &self,
        identity_id: &Identifier,
        contact_id: &Identifier,
        shared_key: &[u8; 32],
    ) {
        let mut wm = self.wallet_manager.write().await;
        let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) else {
            return;
        };
        let Some(managed) = info.identity_manager.managed_identity_mut(identity_id) else {
            return;
        };
        let Some(contact) = managed.established_contact_mut(contact_id) else {
            return;
        };

        // The contact's label rides their incoming request only.
        let ciphertext = match &contact.incoming_request.encrypted_account_label {
            Some(ct) => ct.clone(),
            None => return,
        };

        let decrypted = match platform_encryption::decrypt_account_label(shared_key, &ciphertext) {
            Ok(s) => {
                let trimmed = s.trim();
                if trimmed.is_empty() || trimmed.chars().any(char::is_control) {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            }
            Err(e) => {
                tracing::debug!(
                    owner = %identity_id, contact = %contact_id, error = %e,
                    "Could not decrypt the contact's account label; leaving it unset"
                );
                return;
            }
        };

        // Idempotent — skip the changeset write when nothing changed.
        if contact.contact_account_label == decrypted {
            return;
        }
        contact.contact_account_label = decrypted;
        let snapshot = contact.clone();

        let mut cs = crate::changeset::ContactChangeSet::default();
        cs.established.insert(
            crate::changeset::SentContactRequestKey {
                owner_id: *identity_id,
                recipient_id: *contact_id,
            },
            snapshot,
        );
        if let Err(e) = self.persister.store(cs.into()) {
            tracing::error!("Failed to persist account-label changeset: {}", e);
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

impl<B: TransactionBroadcaster + ?Sized> DashPayView<'_, B> {
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
        let (already_established, already_reciprocated) = {
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
            let established = managed
                .dashpay()
                .established_contacts()
                .contains_key(&sender_id);
            let sent_exists = managed
                .dashpay()
                .sent_contact_requests()
                .contains_key(&sender_id);
            if !established
                && !sent_exists
                && !managed
                    .dashpay()
                    .incoming_contact_requests()
                    .contains_key(&sender_id)
            {
                return Err(PlatformWalletError::ContactRequestNotFound(sender_id));
            }
            (established, established || sent_exists)
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
            // Establish the contact locally from the accepted incoming request
            // when it is not yet established. In the `sent_exists && !established`
            // adopt path (our own reciprocal already sent, the counterparty's
            // request not yet swept into `incoming_contact_requests`) nothing
            // else inserts an `EstablishedContact` — `register_contact_account`
            // and `register_external_contact_account` only touch account
            // collections — so the step-5 `established_contacts.get` would return
            // `None` and the accept would spuriously fail with
            // `ContactRequestNotFound` despite the account registrations
            // succeeding. Ingesting the request here collapses the pending sent
            // entry + the incoming request into an established contact (the same
            // path the normal flow and the sync sweep use), so the final lookup
            // is guaranteed non-`None`. Idempotent: `add_incoming_contact_request`
            // preserves metadata on an already-established pair.
            if !already_established {
                let mut wm = self.wallet_manager.write().await;
                let managed = wm
                    .get_wallet_info_mut(&self.wallet_id)
                    .and_then(|info| info.identity_manager.managed_identity_mut(&our_identity_id))
                    .ok_or(PlatformWalletError::IdentityNotFound(our_identity_id))?;
                managed
                    .add_incoming_contact_request(request.clone(), &self.persister)
                    .map_err(|e| {
                        PlatformWalletError::Persistence(format!(
                            "accept-adopt: incoming request not persisted: {e}"
                        ))
                    })?;
            }
            // Adopt: register the receiving (friendship) account, derived via
            // the signer (no resident seed), matching the fresh-send path.
            match self
                .receiving_xpub_for(&our_identity_id, &sender_id, 0, crypto)
                .await
            {
                Ok(xpub) => {
                    if let Err(e) = self
                        .register_contact_account(&our_identity_id, &sender_id, 0, xpub)
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
                AutoAcceptProofSource::None,
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
            .dashpay()
            .established_contacts()
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
        let our_dec_path = IdentityWallet::<B>::identity_auth_derivation_path(
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
        let shared = crypto
            .ecdh_shared_secret(&our_dec_path, &contact_pubkey)
            .await?;

        // Reuse the identity we just fetched for validation (no second
        // network round). The accept path surfaces any failure to the
        // caller as a plain error — the transient/permanent split only
        // matters to the unattended sync sweep's broken-channel policy.
        let registration = self
            .register_external_contact_account(
                our_identity_id,
                &contact_identity,
                contact_encrypted_xpub,
                shared.clone(),
            )
            .await
            .map_err(RegisterExternalError::into_inner)?;

        // Stamp the rotation self-heal marker and clear any stale
        // broken-channel flag (a user re-accept of a healed channel must stop
        // reporting broken — the sweep never re-registers broken contacts).
        // ONLY when the account was actually (re)built from this request's
        // payload: an `AlreadyExisted` no-op may have hit a row built from a
        // PRE-rotation xpub (the account key ignores `account_reference`), and
        // stamping it as current would make `external_account_needs_rebuild`
        // skip the teardown + rebuild forever — `send_payment` would keep
        // deriving from an xpub the contact no longer watches. Left unstamped,
        // the sweep detects the stale marker and rebuilds; its build path
        // stamps + clears broken then.
        if registration == ExternalAccountRegistration::Built {
            self.note_external_account_registered(
                our_identity_id,
                contact_id,
                contact_encrypted_xpub,
            )
            .await;
        }

        // Surface the contact's account label from the same ECDH shared key
        // (best-effort, cosmetic — a label failure never fails the accept).
        self.store_contact_account_label(our_identity_id, contact_id, &shared)
            .await;

        Ok(())
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
        let path = account_type
            .derivation_path(self.sdk.network)
            .map_err(|e| {
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

impl<B: TransactionBroadcaster + ?Sized> DashPayView<'_, B> {
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

impl<B: TransactionBroadcaster + ?Sized> DashPayView<'_, B> {
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
    use super::{query_lower_bound, SYNC_OVERLAP_MS};

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

        // `0` is a real cursor value distinct from `None` — pin that a
        // future "treat 0 as unset" refactor would regress.
        assert_eq!(query_lower_bound(Some(0)), Some(0));
    }
}

#[cfg(test)]
mod contact_sync_report_tests {
    use super::ContactSyncReport;
    use dpp::prelude::Identifier;

    fn id(b: u8) -> Identifier {
        Identifier::from([b; 32])
    }

    /// The clean pass: every identity answered, nothing new to report. This
    /// must stay distinguishable from the unreachable case below, because it
    /// is the only one that entitles a caller to say the contact set is
    /// current.
    #[test]
    fn an_answered_pass_with_no_new_requests_is_complete() {
        let report = ContactSyncReport {
            identities_attempted: 2,
            ..Default::default()
        };

        assert!(report.is_complete());
        assert!(!report.is_fully_degraded());
    }

    /// A wallet with no identities had nothing to fetch. That is an answer,
    /// not a degradation — and specifically not a *total* one, or an empty
    /// wallet would report the same thing as a total outage.
    #[test]
    fn a_wallet_with_no_identities_is_complete_and_not_degraded() {
        let report = ContactSyncReport::default();

        assert!(report.is_complete());
        assert!(
            !report.is_fully_degraded(),
            "nothing was attempted, so nothing failed"
        );
    }

    /// Not one identity could be read: the DAPI-unreachable ending that used
    /// to arrive as `Ok(vec![])`.
    #[test]
    fn a_pass_that_read_no_identity_is_fully_degraded() {
        let report = ContactSyncReport {
            identities_attempted: 2,
            failed_identities: vec![id(1), id(2)],
            ..Default::default()
        };

        assert!(!report.is_complete());
        assert!(report.is_fully_degraded());
    }

    /// The partial pass. What it fetched is real, so it is not a total
    /// failure — and it is still not complete, because the identities it
    /// missed have contact requests nobody looked at and account builds
    /// nobody enqueued. Treating this as a completed sync is the same bug as
    /// the total case, one identity at a time.
    #[test]
    fn a_partial_pass_is_neither_complete_nor_fully_degraded() {
        let report = ContactSyncReport {
            identities_attempted: 3,
            failed_identities: vec![id(1)],
            ..Default::default()
        };

        assert!(!report.is_complete());
        assert!(!report.is_fully_degraded());
    }

    /// A sent-side failure ingests the received side, so nothing is lost —
    /// but the reciprocal reconciliation that establishes contacts did not
    /// happen, so the pass still may not be recorded as complete.
    #[test]
    fn a_sent_side_failure_alone_still_degrades_the_pass() {
        let report = ContactSyncReport {
            identities_attempted: 1,
            degraded_identities: vec![id(1)],
            ..Default::default()
        };

        assert!(!report.is_complete());
        assert!(
            !report.is_fully_degraded(),
            "the received side was read; this is not a total failure"
        );
    }

    /// Platform answered every identity; the disk did not take what came back.
    /// The ingest `break`s, abandoning the rest of that direction's fetched
    /// requests un-ingested and holding its cursor back for retry — so the
    /// pass is incomplete by the same rule the cursor logic applies to itself.
    /// Reported as complete, startup called `record_sync_ran()` and the launch
    /// could reach `Ready` promising DIP-15 addresses that were never
    /// registered.
    #[test]
    fn a_local_persist_failure_makes_the_pass_incomplete() {
        let report = ContactSyncReport {
            identities_attempted: 2,
            unpersisted_identities: vec![id(1)],
            ..Default::default()
        };

        assert!(
            !report.is_complete(),
            "a direction whose ingest did not reach disk is not a completed sync"
        );
    }

    /// The other side of that coin: a local fault is NOT an outage. Every
    /// fetch was answered, so nothing here says Platform is unreachable, and
    /// `sync_contact_requests` must not turn a disk problem into
    /// `ContactSyncUnreachable` — which reads to a host as "retry the
    /// network" for a condition retrying the network cannot fix.
    #[test]
    fn a_local_persist_failure_is_not_an_outage() {
        let report = ContactSyncReport {
            identities_attempted: 2,
            unpersisted_identities: vec![id(1), id(2)],
            ..Default::default()
        };

        assert!(
            !report.is_fully_degraded(),
            "every received fetch was answered; this is a local fault, not an outage"
        );
        assert!(!report.is_complete());
    }
}

#[cfg(test)]
mod sweep_tests {
    use super::*;
    use crate::broadcaster::SpvBroadcaster;
    use crate::changeset::{ContactChangeSet, PlatformWalletChangeSet, SentContactRequestKey};
    use crate::wallet::core::WalletGeneration;
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

    /// A host persister whose `store()` always fails — disk full, DB error,
    /// host bug. The condition the ingest `break`s on.
    struct FailingPersistence;

    impl crate::changeset::PlatformWalletPersistence for FailingPersistence {
        fn store(
            &self,
            _wallet_id: crate::wallet::platform_wallet::WalletId,
            _changeset: PlatformWalletChangeSet,
        ) -> Result<(), crate::changeset::PersistenceError> {
            Err(crate::changeset::PersistenceError::backend("disk full"))
        }

        fn flush(
            &self,
            _wallet_id: crate::wallet::platform_wallet::WalletId,
        ) -> Result<(), crate::changeset::PersistenceError> {
            Ok(())
        }

        fn load(
            &self,
        ) -> Result<crate::changeset::ClientStartState, crate::changeset::PersistenceError>
        {
            Ok(crate::changeset::ClientStartState::default())
        }
    }

    fn failing_persister() -> WalletPersister {
        WalletPersister::new([0u8; 32], Arc::new(FailingPersistence))
    }

    fn build_test_wallet() -> Wallet {
        Wallet::new_random(Network::Testnet, WalletAccountCreationOptions::None)
            .expect("test wallet")
    }

    fn empty_info(wallet: &Wallet) -> PlatformWalletInfo {
        PlatformWalletInfo {
            core_wallet: ManagedWalletInfo::from_wallet(wallet, 0),
            generation: Arc::new(WalletGeneration::new()),
            identity_manager: IdentityManager::new(),
            tracked_asset_locks: BTreeMap::new(),
            observed_input_conflicts: Default::default(),
            dpns_name_states: BTreeMap::new(),
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
        managed
            .add_incoming_contact_request(test_request(contact, our, 0), &p)
            .expect("setup persists");
        managed
            .add_sent_contact_request(test_request(our, contact, 0), &p)
            .expect("setup persists");
        assert_eq!(managed.dashpay().established_contacts().len(), 1);
        (wallet, info)
    }

    // -----------------------------------------------------------------------
    // The ingest persist-failure branches.
    //
    // A `false` return is the whole signal: it is what holds that direction's
    // high-water cursor back AND what marks the identity in the
    // `ContactSyncReport`, so the pass cannot report itself complete after
    // abandoning fetched requests un-ingested. Before this change the boolean
    // reached only the cursor, and the report stayed silent.
    // -----------------------------------------------------------------------

    /// A fresh identity with nothing tracked, ready for a first ingest.
    fn info_with_bare_identity(our: u8) -> PlatformWalletInfo {
        let wallet = build_test_wallet();
        let mut info = empty_info(&wallet);
        info.identity_manager
            .add_identity(test_identity(our), 0, [0u8; 32], &noop_persister())
            .expect("add identity");
        info
    }

    fn one_received(
        sender: u8,
        recipient: u8,
        reference: u32,
    ) -> BTreeMap<Identifier, ContactRequest> {
        newest_received_per_sender([test_request(sender, recipient, reference)])
    }

    /// The control: a persister that takes the write ingests the request and
    /// reports success. Without this the failure tests below would also pass
    /// against a function that always returned `false`.
    #[test]
    fn a_received_ingest_that_persists_reports_success() {
        let our = 1u8;
        let our_id = Identifier::from([our; 32]);
        let mut info = info_with_bare_identity(our);
        let managed = info
            .identity_manager
            .managed_identity_mut(&our_id)
            .expect("managed identity");

        let mut rotated = Vec::new();
        let mut all_requests = Vec::new();
        let ok = ingest_received_requests(
            managed,
            &noop_persister(),
            our_id,
            one_received(2, our, 0),
            &mut rotated,
            &mut all_requests,
        );

        assert!(
            ok,
            "a persister that succeeds must report a complete ingest"
        );
        assert_eq!(all_requests.len(), 1);
        assert_eq!(managed.dashpay().incoming_contact_requests().len(), 1);
    }

    /// The first-ingest persist failure (`add_incoming_contact_request`).
    /// The request is not tracked and must not be reported as newly
    /// discovered — a caller that took it as real would act on a request that
    /// no longer exists anywhere after a restart.
    #[test]
    fn a_received_ingest_persist_failure_reports_the_pass_incomplete() {
        let our = 1u8;
        let our_id = Identifier::from([our; 32]);
        let mut info = info_with_bare_identity(our);
        let managed = info
            .identity_manager
            .managed_identity_mut(&our_id)
            .expect("managed identity");

        let mut rotated = Vec::new();
        let mut all_requests = Vec::new();
        let ok = ingest_received_requests(
            managed,
            &failing_persister(),
            our_id,
            one_received(2, our, 0),
            &mut rotated,
            &mut all_requests,
        );

        assert!(
            !ok,
            "a persist failure must be reported so the cursor is held AND the pass is \
             marked incomplete"
        );
        assert!(
            all_requests.is_empty(),
            "a request that never persisted must not be reported as newly discovered"
        );
    }

    /// The rotation persist failure (`apply_rotated_incoming_request`) — the
    /// second of the three branches, reached only when the sender is already
    /// tracked under a different `accountReference`.
    #[test]
    fn a_received_rotation_persist_failure_reports_the_pass_incomplete() {
        let our = 1u8;
        let contact = 2u8;
        let our_id = Identifier::from([our; 32]);
        // Established at reference 0 by the fixture; the sweep now sees the
        // sender's rotated doc at reference 7.
        let (_wallet, mut info) = info_with_established_contact(our, contact);
        let managed = info
            .identity_manager
            .managed_identity_mut(&our_id)
            .expect("managed identity");

        let mut rotated = Vec::new();
        let mut all_requests = Vec::new();
        let ok = ingest_received_requests(
            managed,
            &failing_persister(),
            our_id,
            one_received(contact, our, 7),
            &mut rotated,
            &mut all_requests,
        );

        assert!(
            !ok,
            "a rotation whose persist failed leaves the pass incomplete"
        );
        assert!(
            rotated.is_empty(),
            "an unpersisted rotation must not tear down the external account"
        );
    }

    /// The sent-side persist failure (`add_sent_contact_request`) — the third
    /// branch. Its own direction's cursor is the one held back, and the
    /// identity is marked degraded rather than failed, but the pass is no more
    /// complete than in the received case.
    #[test]
    fn a_sent_ingest_persist_failure_reports_the_pass_incomplete() {
        let our = 1u8;
        let our_id = Identifier::from([our; 32]);
        let mut info = info_with_bare_identity(our);
        let managed = info
            .identity_manager
            .managed_identity_mut(&our_id)
            .expect("managed identity");

        let newest = newest_sent_per_recipient([test_request(our, 2, 0)]);
        assert!(!ingest_sent_requests(
            managed,
            &failing_persister(),
            our_id,
            newest
        ));

        // Control on the same fixture: the write succeeds, so the ingest
        // reports success.
        let newest = newest_sent_per_recipient([test_request(our, 3, 0)]);
        assert!(ingest_sent_requests(
            managed,
            &noop_persister(),
            our_id,
            newest
        ));
    }

    // -----------------------------------------------------------------------
    // The NEXT sweep has to actually retry the write.
    //
    // Holding the direction's high-water cursor makes the next sweep re-fetch
    // the same range, and reporting the pass incomplete stops the launch
    // claiming a sync it did not finish. Review found that neither of those
    // gets the write to disk on its own: the state methods committed the
    // mutation to memory BEFORE calling `persister.store`, so a failed store
    // left the request sitting in `incoming_contact_requests` /
    // `established_contacts` / `sent_contact_requests` anyway. The re-fetched
    // range then hit the same-reference dedup — `tracked_reference ==
    // Some(request.account_reference)` here, the no-op guards inside
    // `add_sent_contact_request`, the `already_applied` guard inside
    // `apply_rotated_incoming_request` — reported success and advanced the
    // cursor. The backend never received the write, a later startup called the
    // sync complete and reached `Ready`, and the contact was gone after a
    // restart.
    //
    // These three drive two sweeps over the same fetched range, the first
    // against a persister that fails and the second against one that takes the
    // write, and assert the second actually ingests. Each covers one of the
    // three branches.
    // -----------------------------------------------------------------------

    /// Counts the writes that reached the backend, so a retry that silently
    /// no-ops is distinguishable from one that re-stored.
    #[derive(Default)]
    struct CountingPersistence(std::sync::atomic::AtomicUsize);

    impl crate::changeset::PlatformWalletPersistence for CountingPersistence {
        fn store(
            &self,
            _wallet_id: crate::wallet::platform_wallet::WalletId,
            _changeset: crate::changeset::PlatformWalletChangeSet,
        ) -> Result<(), crate::changeset::PersistenceError> {
            self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }

        fn flush(
            &self,
            _wallet_id: crate::wallet::platform_wallet::WalletId,
        ) -> Result<(), crate::changeset::PersistenceError> {
            Ok(())
        }

        fn load(
            &self,
        ) -> Result<crate::changeset::ClientStartState, crate::changeset::PersistenceError>
        {
            Ok(crate::changeset::ClientStartState::default())
        }
    }

    fn counting_persister() -> (WalletPersister, Arc<CountingPersistence>) {
        let backend = Arc::new(CountingPersistence::default());
        (WalletPersister::new([0u8; 32], backend.clone()), backend)
    }

    fn store_count(backend: &Arc<CountingPersistence>) -> usize {
        backend.0.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Branch 1, the fresh received request (`add_incoming_contact_request`).
    /// The first sweep's persist fails; the second must re-ingest the same
    /// request and get it to disk, not skip it as already tracked.
    #[test]
    fn a_received_ingest_that_failed_to_persist_is_retried_by_the_next_sweep() {
        let our = 1u8;
        let our_id = Identifier::from([our; 32]);
        let mut info = info_with_bare_identity(our);
        let managed = info
            .identity_manager
            .managed_identity_mut(&our_id)
            .expect("managed identity");

        // Sweep 1: the write fails, so the cursor is held and the pass is
        // reported incomplete.
        let mut rotated = Vec::new();
        let mut all_requests = Vec::new();
        assert!(
            !ingest_received_requests(
                managed,
                &failing_persister(),
                our_id,
                one_received(2, our, 0),
                &mut rotated,
                &mut all_requests,
            ),
            "precondition: the failed write must report the pass incomplete"
        );
        assert!(
            managed.dashpay().incoming_contact_requests().is_empty(),
            "a request that never reached disk must not be tracked in memory, or the \
             retry below is skipped as already known"
        );

        // Sweep 2: the held-back cursor re-fetches the SAME range against a
        // working persister.
        let (persister, backend) = counting_persister();
        let mut rotated = Vec::new();
        let mut all_requests = Vec::new();
        assert!(
            ingest_received_requests(
                managed,
                &persister,
                our_id,
                one_received(2, our, 0),
                &mut rotated,
                &mut all_requests,
            ),
            "the retry must complete the pass"
        );

        assert_eq!(
            store_count(&backend),
            1,
            "the retry must actually re-store — a no-op that reports success \
             advances the cursor over a write the backend never received"
        );
        assert_eq!(
            all_requests.len(),
            1,
            "the retried request must surface as newly discovered"
        );
        assert_eq!(
            managed.dashpay().incoming_contact_requests().len(),
            1,
            "and land in memory once it is safely on disk"
        );
    }

    /// Branch 2, the rotation (`apply_rotated_incoming_request`). The retry has
    /// two guards to get past: the sweep's `tracked_reference` skip and the
    /// method's own `already_applied` idempotency guard. A rotation committed
    /// to memory on a failed store trips both.
    #[test]
    fn a_received_rotation_that_failed_to_persist_is_retried_by_the_next_sweep() {
        let our = 1u8;
        let contact = 2u8;
        let our_id = Identifier::from([our; 32]);
        let (_wallet, mut info) = info_with_established_contact(our, contact);
        let managed = info
            .identity_manager
            .managed_identity_mut(&our_id)
            .expect("managed identity");

        // Sweep 1: the sender's rotated doc at reference 7 fails to persist.
        let mut rotated = Vec::new();
        let mut all_requests = Vec::new();
        assert!(
            !ingest_received_requests(
                managed,
                &failing_persister(),
                our_id,
                one_received(contact, our, 7),
                &mut rotated,
                &mut all_requests,
            ),
            "precondition: the failed rotation must report the pass incomplete"
        );
        assert_eq!(
            managed.dashpay().established_contacts()[&Identifier::from([contact; 32])]
                .incoming_request
                .account_reference,
            0,
            "memory must stay on the OLD reference — on the new one, both the sweep's \
             same-reference skip and `already_applied` swallow the retry"
        );

        // Sweep 2: the same rotated doc, against a working persister.
        let (persister, backend) = counting_persister();
        let mut rotated = Vec::new();
        let mut all_requests = Vec::new();
        assert!(
            ingest_received_requests(
                managed,
                &persister,
                our_id,
                one_received(contact, our, 7),
                &mut rotated,
                &mut all_requests,
            ),
            "the retry must complete the pass"
        );

        assert_eq!(
            store_count(&backend),
            1,
            "the retried rotation must actually re-store"
        );
        assert_eq!(
            rotated,
            vec![Identifier::from([contact; 32])],
            "and re-key the contact, so the caller tears down the stale external account"
        );
        assert_eq!(
            managed.dashpay().established_contacts()[&Identifier::from([contact; 32])]
                .incoming_request
                .account_reference,
            7,
            "the new key material must be the tracked one once it is on disk"
        );
    }

    /// Branch 3, the fresh sent request (`add_sent_contact_request`). Its
    /// same-reference no-op guard returns `Ok(())`, so a memory-committed
    /// failed write makes the retry report success without storing anything.
    #[test]
    fn a_sent_ingest_that_failed_to_persist_is_retried_by_the_next_sweep() {
        let our = 1u8;
        let our_id = Identifier::from([our; 32]);
        let mut info = info_with_bare_identity(our);
        let managed = info
            .identity_manager
            .managed_identity_mut(&our_id)
            .expect("managed identity");

        // Sweep 1: the write fails.
        let newest = newest_sent_per_recipient([test_request(our, 2, 0)]);
        assert!(
            !ingest_sent_requests(managed, &failing_persister(), our_id, newest),
            "precondition: the failed write must report the pass incomplete"
        );
        assert!(
            managed.dashpay().sent_contact_requests().is_empty(),
            "an unpersisted sent request must not be tracked, or the retry hits the \
             same-reference no-op guard"
        );

        // Sweep 2: the same range, against a working persister.
        let (persister, backend) = counting_persister();
        let newest = newest_sent_per_recipient([test_request(our, 2, 0)]);
        assert!(
            ingest_sent_requests(managed, &persister, our_id, newest),
            "the retry must complete the pass"
        );

        assert_eq!(
            store_count(&backend),
            1,
            "the retried sent request must actually re-store"
        );
        assert_eq!(
            managed.dashpay().sent_contact_requests().len(),
            1,
            "and land in memory once it is safely on disk"
        );
    }

    /// The auto-establish shape, which loses the most on a failed store: it
    /// consumes the pending entry from the opposite direction's map. Committed
    /// before the store, a failure left the incoming request *removed* and the
    /// established contact tracked but unpersisted — so the retry could no
    /// longer reproduce the auto-establish, and a restart came back with
    /// neither the pending request nor the contact.
    #[test]
    fn a_failed_auto_establish_leaves_both_sides_intact_for_the_retry() {
        let our = 1u8;
        let contact = 2u8;
        let our_id = Identifier::from([our; 32]);
        let contact_id = Identifier::from([contact; 32]);
        let mut info = info_with_bare_identity(our);
        let managed = info
            .identity_manager
            .managed_identity_mut(&our_id)
            .expect("managed identity");

        // We have already sent to this contact; their reciprocal now arrives.
        managed
            .add_sent_contact_request(test_request(our, contact, 0), &noop_persister())
            .expect("the outgoing request persists");
        assert_eq!(managed.dashpay().sent_contact_requests().len(), 1);

        // Sweep 1: the auto-establish write fails.
        let mut rotated = Vec::new();
        let mut all_requests = Vec::new();
        assert!(
            !ingest_received_requests(
                managed,
                &failing_persister(),
                our_id,
                one_received(contact, our, 0),
                &mut rotated,
                &mut all_requests,
            ),
            "precondition: the failed write must report the pass incomplete"
        );
        assert_eq!(
            managed.dashpay().sent_contact_requests().len(),
            1,
            "the outgoing request must survive the failed store — without it the retry \
             cannot reproduce the auto-establish and silently downgrades the pair"
        );
        assert!(
            managed.dashpay().established_contacts().is_empty(),
            "and nothing may be tracked as established while it is not on disk"
        );

        // Sweep 2: the same reciprocal, against a working persister.
        let (persister, backend) = counting_persister();
        let mut rotated = Vec::new();
        let mut all_requests = Vec::new();
        assert!(
            ingest_received_requests(
                managed,
                &persister,
                our_id,
                one_received(contact, our, 0),
                &mut rotated,
                &mut all_requests,
            ),
            "the retry must complete the pass"
        );

        assert_eq!(
            store_count(&backend),
            1,
            "the retried auto-establish must actually re-store"
        );
        assert!(
            managed
                .dashpay()
                .established_contacts()
                .contains_key(&contact_id),
            "the contact must be established on the retry"
        );
        assert!(
            managed.dashpay().sent_contact_requests().is_empty(),
            "and the pending outgoing entry consumed, now that the establish is on disk"
        );
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
            DashPayView::<SpvBroadcaster>::collect_account_build_candidates(&info, &our_id);

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
            .established_contact_mut(&contact_id)
            .unwrap()
            .payment_channel_broken = true;

        let candidates =
            DashPayView::<SpvBroadcaster>::collect_account_build_candidates(&info, &our_id);
        assert!(
            candidates.is_empty(),
            "a permanently-broken contact must not be retried by the sweep"
        );
    }

    /// **F2 rotation self-heal predicate.** After a restart the persisted,
    /// tombstone-less account-registration row rebuilds the contact's OLD
    /// (rotated-away) external xpub while the established contact already
    /// tracks the NEW incoming reference. `external_account_needs_rebuild`
    /// must flag such a registered-but-stale account for teardown + rebuild —
    /// including the `None` marker case (a cold restore that did not carry the
    /// marker), which conservatively forces one rebuild. A registered account
    /// whose marker MATCHES the tracked reference is healthy (no churn), a
    /// missing account is not this predicate's job, and a broken channel is
    /// left alone.
    #[test]
    fn external_account_needs_rebuild_detects_stale_registered_account() {
        let contact_id = Identifier::from([2u8; 32]);
        let outgoing = test_request(1, 2, 0);
        let mut incoming = test_request(2, 1, 0);
        incoming.account_reference = 200; // the CURRENT (post-rotation) reference
        let mut contact = EstablishedContact::new(contact_id, outgoing, incoming);

        // Registered account still built from the OLD reference (100) — stale.
        contact.external_account_reference = Some(100);
        assert!(
            external_account_needs_rebuild(&contact, true),
            "a registered account whose marker != the tracked reference is stale"
        );

        // Cold restore that did not carry the marker (`None`) — force a rebuild.
        contact.external_account_reference = None;
        assert!(
            external_account_needs_rebuild(&contact, true),
            "a None marker after restart must force one rebuild"
        );

        // Healthy: marker matches the tracked reference — no rebuild (no churn).
        contact.external_account_reference = Some(200);
        assert!(
            !external_account_needs_rebuild(&contact, true),
            "a registered account built from the current reference is healthy"
        );

        // No registered account: not this predicate's job (ordinary candidate).
        contact.external_account_reference = None;
        assert!(
            !external_account_needs_rebuild(&contact, false),
            "a missing external account is not a stale-rebuild case"
        );

        // Broken channel is left alone even if the marker mismatches.
        contact.external_account_reference = Some(100);
        contact.payment_channel_broken = true;
        assert!(
            !external_account_needs_rebuild(&contact, true),
            "a broken channel is never re-registered by the sweep"
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
            .dashpay()
            .established_contacts()
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
                .dashpay()
                .established_contacts()
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
        managed
            .add_incoming_contact_request(test_request(sender, our, 0), &p)
            .expect("setup persists");
        let cs = managed.ignore_sender(&sender_id);
        let pcs = PlatformWalletChangeSet {
            contacts: Some(cs),
            ..Default::default()
        };

        // Wipe the in-memory ignore set, then re-apply the changeset (the
        // restore-from-persistence path).
        let managed = info.identity_manager.managed_identity_mut(&our_id).unwrap();
        let wiped: Vec<_> = managed
            .dashpay()
            .ignored_senders()
            .iter()
            .copied()
            .collect();
        for sender in &wiped {
            managed.apply_unignored_sender(sender);
        }
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

    /// **Sent-side restore-from-seed rotation (the frozen-outgoing bug).**
    /// A rotation re-send leaves our OWN old + bumped sent docs on-chain;
    /// `fetch_sent_contact_requests` returns them `$createdAt`-ASC. Ingesting
    /// raw would establish/track against the OLDEST (stale) outgoing
    /// reference, so the next rotation re-derives the same reference and
    /// collides on the unique index. `newest_sent_per_recipient` must
    /// collapse to the single newest doc per recipient so restore tracks the
    /// freshest outgoing reference.
    #[test]
    fn newest_sent_per_recipient_collapses_rotated_recipient_to_latest_doc() {
        let our = 1u8;
        let recipient = 2u8;
        // Our own sent docs to one recipient: old ref=100 @t=100, rotated
        // ref=101 @t=200. `test_request_at(sender, recipient, ..)` — here we
        // are the sender.
        let old_doc = test_request_at(our, recipient, 100, 100);
        let rotated_doc = test_request_at(our, recipient, 101, 200);
        // A second recipient to prove per-recipient keying.
        let other = test_request_at(our, 3, 100, 150);

        // Feed old-before-new (the $createdAt-ASC order the fetch yields).
        let collapsed =
            newest_sent_per_recipient([old_doc.clone(), other.clone(), rotated_doc.clone()]);

        assert_eq!(collapsed.len(), 2, "one entry per distinct recipient");
        let recipient_id = Identifier::from([recipient; 32]);
        assert_eq!(
            collapsed.get(&recipient_id).map(|r| r.account_reference),
            Some(101),
            "the newest (rotated) sent doc must win, not the stale oldest"
        );
        assert_eq!(
            collapsed
                .get(&Identifier::from([3u8; 32]))
                .map(|r| r.account_reference),
            Some(100),
            "the unrelated recipient is unaffected"
        );

        // Fixpoint: re-collapsing yields the same.
        let again = newest_sent_per_recipient(collapsed.values().cloned());
        assert_eq!(
            again.get(&recipient_id).map(|r| r.account_reference),
            Some(101)
        );
    }

    /// **Receive-side label ingest (the sweep-parser drop bug).**
    /// The recurring sweep parses received `contactRequest` docs via
    /// `parse_contact_request_doc`. It must carry the optional DIP-15
    /// `encryptedAccountLabel` onto the `ContactRequest` — otherwise the
    /// label the sender attached (and which lands on-chain) is silently
    /// dropped on ingest, so the receive-side surfacing has nothing to
    /// decrypt. Pins that the field survives the parse (red before the
    /// parser read it, green after).
    #[test]
    fn parse_contact_request_doc_carries_encrypted_account_label() {
        use dpp::document::{Document, DocumentV0};
        use dpp::platform_value::Value;
        use std::collections::BTreeMap;

        let sender = Identifier::from([2u8; 32]);
        let recipient = Identifier::from([1u8; 32]);

        let label_ct = vec![0x2au8; 48];
        let mut properties = BTreeMap::new();
        properties.insert("senderKeyIndex".to_string(), Value::U32(0));
        properties.insert("recipientKeyIndex".to_string(), Value::U32(0));
        properties.insert("accountReference".to_string(), Value::U32(5));
        properties.insert(
            "encryptedPublicKey".to_string(),
            Value::Bytes(vec![1u8; 96]),
        );
        properties.insert(
            "encryptedAccountLabel".to_string(),
            Value::Bytes(label_ct.clone()),
        );

        let doc = Document::V0(DocumentV0 {
            contract_version: None,
            id: Identifier::from([9u8; 32]),
            owner_id: sender,
            properties,
            revision: None,
            created_at: Some(123),
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: Some(456),
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            creator_id: None,
        });

        let parsed =
            DashPayView::<SpvBroadcaster>::parse_contact_request_doc(&doc, sender, recipient)
                .expect("a complete contactRequest doc must parse");
        assert_eq!(
            parsed.encrypted_account_label,
            Some(label_ct),
            "the sweep parser must carry encryptedAccountLabel onto the request, \
             else the receive-side label surfacing has nothing to decrypt"
        );
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
            !managed.dashpay().sent_contact_requests().contains_key(&contact_id),
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
        managed
            .add_sent_contact_request(test_request(our, 9, 4), &noop_persister())
            .expect("setup persists");
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
            managed
                .apply_rotated_incoming_request(rotated.clone(), &p)
                .expect("rotation persists in test"),
            "first rotation must re-key the established contact"
        );
        // Second apply of the SAME request: no-op (returns false).
        assert!(
            !managed
                .apply_rotated_incoming_request(rotated.clone(), &p)
                .expect("rotation persists in test"),
            "re-applying an identical request must be a no-op (no re-key, no churn)"
        );
        let stored = info
            .identity_manager
            .managed_identity(&our_id)
            .unwrap()
            .dashpay()
            .established_contacts()
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

    /// **Own-key selector: a disabled first ENCRYPTION key is skipped in
    /// favour of the enabled replacement.** After a disable-and-replace key
    /// rotation the lowest-id ENCRYPTION key is disabled; selecting it
    /// would hard-fail the pre-send validator on every new outgoing contact
    /// request ("Sender key N is disabled") even though an enabled
    /// replacement exists — while the contactInfo path (which already
    /// filtered disabled keys) would use the replacement, splitting the two
    /// surfaces' notion of the ECDH root. Was red against the send path's
    /// unfiltered inline selection.
    #[test]
    fn own_key_selector_skips_disabled_first_encryption_key() {
        let identity = identity_with_keys(vec![
            key(0, KeyType::ECDSA_SECP256K1, Purpose::AUTHENTICATION),
            disabled_key(1, KeyType::ECDSA_SECP256K1, Purpose::ENCRYPTION),
            key(2, KeyType::ECDSA_SECP256K1, Purpose::ENCRYPTION),
        ]);

        let selected = select_own_encryption_key(&identity)
            .expect("must skip the disabled key and select the enabled replacement");
        assert_eq!(
            selected.id(),
            2,
            "the disabled lowest-id ENCRYPTION key (id 1) must not be the ECDH root"
        );
    }

    /// Own-key selector: enabled-only, so an identity whose ONLY
    /// ENCRYPTION key is disabled errors instead of deriving ECDH from a
    /// revoked key.
    #[test]
    fn own_key_selector_errors_when_only_encryption_key_is_disabled() {
        let identity = identity_with_keys(vec![
            key(0, KeyType::ECDSA_SECP256K1, Purpose::AUTHENTICATION),
            disabled_key(1, KeyType::ECDSA_SECP256K1, Purpose::ENCRYPTION),
        ]);

        let err = select_own_encryption_key(&identity).unwrap_err();
        assert!(
            matches!(err, PlatformWalletError::InvalidIdentityData(_)),
            "a sole disabled ENCRYPTION key must error, got {err:?}"
        );
    }
}

#[cfg(test)]
mod contact_info_provider_tests {
    use super::*;
    use crate::wallet::identity::crypto::contact_info::derive_contact_info_keys;
    use crate::wallet::identity::network::identity_auth_derivation_path_for_type;
    use key_wallet::bip32::KeyDerivationType;
    use key_wallet::mnemonic::{Language, Mnemonic};
    use key_wallet::Network;

    // Canonical BIP-39 test mnemonic.
    const PHRASE: &str =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    /// Invariant: the contactInfo seal/open the signer produces
    /// must be byte-identical to the resident `derive_contact_info_keys` AT THE
    /// REAL identity-auth root path — not an arbitrary path. contactInfo is
    /// self-encrypted (no counterparty round-trip), so a wrong root silently
    /// writes data no client can decrypt, with no on-chain oracle. This pins the
    /// provider's seal to the production derivation at the real path and confirms
    /// open round-trips.
    #[tokio::test]
    async fn contact_info_seal_open_matches_resident_derivation_at_real_auth_path() {
        let seed = Mnemonic::from_phrase(PHRASE, Language::English)
            .expect("valid mnemonic")
            .to_seed("");
        let network = Network::Testnet;
        let identity_index = 0u32;
        let root_key_id = 2u32;
        let derivation_index = 0u32;
        let contact_id = [0x33u8; 32];
        let plaintext = b"hello private data".to_vec();
        let iv = [0x11u8; 16];

        // The REAL identity-auth root the production publish path builds.
        let root_path = identity_auth_derivation_path_for_type(
            network,
            KeyDerivationType::ECDSA,
            identity_index,
            root_key_id,
        )
        .expect("auth path");

        let provider = SeedCryptoProvider::from_seed(seed, network);
        let sealed = provider
            .contact_info_seal(&root_path, derivation_index, &contact_id, &plaintext, &iv)
            .await
            .expect("seal");

        // Resident twin at the same real path → byte-identical ciphertext.
        let wallet = key_wallet::wallet::Wallet::from_seed_bytes(
            seed,
            network,
            key_wallet::wallet::initialization::WalletAccountCreationOptions::None,
        )
        .expect("wallet");
        let keys = derive_contact_info_keys(
            &wallet,
            network,
            identity_index,
            root_key_id,
            derivation_index,
        )
        .expect("resident keys");
        let enc_k: [u8; 32] = *keys.enc_to_user_id_key;
        let priv_k: [u8; 32] = *keys.private_data_key;
        let expected_enc = platform_encryption::encrypt_enc_to_user_id(&enc_k, &contact_id);
        let expected_priv = platform_encryption::encrypt_private_data(&priv_k, &iv, &plaintext);
        assert_eq!(
            sealed.enc_to_user_id, expected_enc,
            "encToUserId must equal the resident derivation at the REAL auth path"
        );
        assert_eq!(
            sealed.private_data, expected_priv,
            "privateData must equal the resident derivation at the REAL auth path"
        );

        // open round-trips the inputs.
        let opened = provider
            .contact_info_open(
                &root_path,
                derivation_index,
                &sealed.enc_to_user_id,
                &sealed.private_data,
            )
            .await
            .expect("open");
        assert_eq!(
            opened.contact_id, contact_id,
            "open recovers the contact id"
        );
        assert_eq!(
            opened.private_data, plaintext,
            "open recovers the private data"
        );
    }

    /// The DIP-15 friendship ECDH key must stay inside `Zeroizing` all the way
    /// back to platform-wallet so it is scrubbed on drop rather than left in a
    /// bare `[u8; 32]`. Binding the result to `Zeroizing<[u8; 32]>` fails to
    /// compile against a plain-array trait return; the value must still match
    /// the resident `derive_shared_key_ecdh` at the same path.
    #[tokio::test]
    async fn ecdh_shared_secret_returns_zeroizing_matching_resident_derivation() {
        use dashcore::secp256k1::{PublicKey, Secp256k1, SecretKey};

        let seed = Mnemonic::from_phrase(PHRASE, Language::English)
            .expect("valid mnemonic")
            .to_seed("");
        let network = Network::Testnet;

        let path =
            identity_auth_derivation_path_for_type(network, KeyDerivationType::ECDSA, 0u32, 2u32)
                .expect("auth path");

        let peer = PublicKey::from_secret_key(
            &Secp256k1::new(),
            &SecretKey::from_slice(&[0x42u8; 32]).expect("peer secret"),
        );

        let provider = SeedCryptoProvider::from_seed(seed, network);
        let shared: zeroize::Zeroizing<[u8; 32]> = provider
            .ecdh_shared_secret(&path, &peer)
            .await
            .expect("ecdh shared secret");

        let wallet = key_wallet::wallet::Wallet::from_seed_bytes(
            seed,
            network,
            key_wallet::wallet::initialization::WalletAccountCreationOptions::None,
        )
        .expect("wallet");
        let xprv = wallet
            .derive_extended_private_key(&path)
            .expect("resident xprv");
        let expected = platform_encryption::derive_shared_key_ecdh(&xprv.private_key, &peer);

        assert_eq!(
            *shared, expected,
            "the Zeroizing ECDH key must equal the resident derivation at the same path"
        );
    }

    /// The "needs unlock" count must track only the account-build ops
    /// (`RegisterReceiving` / `RegisterExternal`) and exclude
    /// `ContactInfoDecrypt`. `ContactInfoDecrypt` is re-enqueued on every
    /// signerless sweep, so counting it would make the count a permanent
    /// `> 0` and re-trip the UI banner ~15s after every unlock on a healthy
    /// wallet. This fails against a naive `pending_contact_crypto.len()`.
    #[test]
    fn account_build_count_excludes_contact_info_decrypt() {
        use crate::changeset::{PendingContactCrypto, PendingContactCryptoOp};

        let owner = Identifier::from([1u8; 32]);
        let mk = |contact: u8, op| PendingContactCrypto {
            owner_identity_id: owner,
            contact_id: Identifier::from([contact; 32]),
            op,
            enqueued_at_ms: 0,
        };

        let queue = vec![
            mk(2, PendingContactCryptoOp::RegisterReceiving),
            mk(
                2,
                PendingContactCryptoOp::RegisterExternal {
                    encrypted_public_key: vec![0u8; 96],
                    our_decryption_key_index: 0,
                    contact_encryption_key_index: 0,
                },
            ),
            mk(3, PendingContactCryptoOp::ContactInfoDecrypt),
            mk(5, PendingContactCryptoOp::AutoAccept),
        ];
        // Two account-build ops + one auto-accept = 3 "waiting to finish setup";
        // the ContactInfoDecrypt is not counted.
        assert_eq!(count_account_build_ops(&queue), 3);

        // A queue of only ContactInfoDecrypt is zero actionable backlog.
        assert_eq!(
            count_account_build_ops(&[mk(4, PendingContactCryptoOp::ContactInfoDecrypt)]),
            0
        );
        // AutoAccept alone counts (a contact waiting to be auto-accepted).
        assert_eq!(
            count_account_build_ops(&[mk(6, PendingContactCryptoOp::AutoAccept)]),
            1
        );

        // Empty queue is zero.
        assert_eq!(count_account_build_ops(&[]), 0);
    }
}

#[cfg(test)]
mod stamp_race_tests {
    //! The rotation self-heal stamp must be payload-bound, not live-state
    //! bound: registration (drain / accept) and the stamp run under separate
    //! guards, so a rotation sweep can advance `incoming_request` in between.
    //! Stamping the live reference onto an account built from the superseded
    //! payload would silence `external_account_needs_rebuild` forever.

    use super::*;
    use crate::events::{EventHandler, PlatformEventHandler};
    use crate::wallet::identity::EstablishedContact;
    use crate::wallet::persister::{NoPlatformPersistence, WalletPersister};
    use dpp::identity::v0::IdentityV0;
    use dpp::identity::Identity;
    use key_wallet::mnemonic::{Language, Mnemonic};
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use key_wallet::Network;
    use std::collections::BTreeMap;
    use std::sync::Arc;

    const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon \
         abandon abandon abandon abandon abandon about";

    struct NoopEventHandler;
    impl EventHandler for NoopEventHandler {}
    impl PlatformEventHandler for NoopEventHandler {}

    fn bare_identity(id: [u8; 32]) -> Identity {
        Identity::V0(IdentityV0 {
            id: Identifier::from(id),
            public_keys: BTreeMap::new(),
            balance: 0,
            revision: 0,
        })
    }

    /// The stamp must be a no-op when the live incoming request's ciphertext
    /// no longer matches the payload the account was registered from (a
    /// rotation raced the registration), and must stamp normally when they
    /// match. Was red against the live-reference stamp.
    #[tokio::test]
    async fn stamp_skips_when_registration_raced_a_rotation() {
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
        let wallet_id = wallet.wallet_id();
        let iw = wallet.identity();
        let p = WalletPersister::new(wallet_id, Arc::clone(&persister) as _);

        let owner = Identifier::from([0xAA; 32]);
        let contact = Identifier::from([0xBB; 32]);
        // Live state: the contact ROTATED — incoming_request now carries the
        // fresh ciphertext (7s) under reference 7; no marker yet.
        let fresh_cipher = vec![7u8; 96];
        let stale_cipher = vec![9u8; 96];
        {
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            info.identity_manager
                .add_identity(bare_identity([0xAA; 32]), 0, wallet_id, &p)
                .expect("add owner");
            let outgoing = ContactRequest::new(owner, contact, 0, 0, 0, vec![0u8; 96], 100, 0);
            let incoming =
                ContactRequest::new(contact, owner, 0, 0, 7, fresh_cipher.clone(), 100, 0);
            info.identity_manager
                .managed_identity_mut(&owner)
                .expect("managed")
                .apply_established_contact(EstablishedContact::new(contact, outgoing, incoming));
        }

        let marker = |iw: &IdentityWallet<crate::broadcaster::SpvBroadcaster>| {
            let iw = iw.clone();
            async move {
                let wm = iw.wallet_manager.read().await;
                wm.get_wallet_info(&wallet_id)
                    .and_then(|info| info.identity_manager.managed_identity(&owner))
                    .and_then(|m| m.dashpay().established_contacts().get(&contact).cloned())
                    .expect("established contact")
                    .external_account_reference
            }
        };

        // Drain finished registering from the STALE (pre-rotation) payload:
        // the stamp must detect the mismatch and leave the marker unset so
        // the sweep's teardown + rebuild path picks up the fresh request.
        iw.dashpay()
            .note_external_account_registered(&owner, &contact, &stale_cipher)
            .await;
        assert_eq!(
            marker(iw).await,
            None,
            "a stamp from a superseded payload must NOT mark the account current"
        );

        // Registration from the LIVE payload stamps the tracked reference.
        iw.dashpay()
            .note_external_account_registered(&owner, &contact, &fresh_cipher)
            .await;
        assert_eq!(
            marker(iw).await,
            Some(7),
            "a stamp from the live payload records the tracked reference"
        );
    }
}

#[cfg(test)]
mod drain_budget_tests {
    use super::{bounded, budget_spent};
    use std::time::{Duration, Instant};

    #[tokio::test]
    async fn no_deadline_always_runs_the_future_to_completion() {
        // The shape every pre-existing caller relies on: an unbounded drain
        // must not acquire an early-exit path just because one caller wanted
        // a budget.
        assert!(!budget_spent(None));
        assert_eq!(bounded(None, async { 7 }).await, Some(7));
    }

    #[tokio::test]
    async fn a_spent_deadline_refuses_before_polling_the_future() {
        // Not merely "returns None": the future must never start, because in
        // the drains it is the step that precedes a commit.
        let past = Instant::now() - Duration::from_secs(1);
        assert!(budget_spent(Some(past)));

        let mut polled = false;
        let result = bounded(Some(past), async {
            polled = true;
            7
        })
        .await;
        assert_eq!(result, None);
        assert!(!polled, "a spent budget must not start the work");
    }

    #[tokio::test]
    async fn a_live_deadline_lets_a_finished_future_through() {
        let future = Instant::now() + Duration::from_secs(30);
        assert!(!budget_spent(Some(future)));
        assert_eq!(bounded(Some(future), async { 7 }).await, Some(7));
    }

    #[tokio::test(start_paused = true)]
    async fn a_deadline_that_passes_mid_await_abandons_the_step() {
        let deadline = Instant::now() + Duration::from_millis(50);
        let result = bounded(deadline.into(), async {
            tokio::time::sleep(Duration::from_secs(5)).await;
            7
        })
        .await;
        assert_eq!(result, None, "the step outlasted the budget");
    }
}
