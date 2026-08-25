//! Classic Dash **signed message** production.
//!
//! [`CoreWallet::sign_message`] signs an arbitrary short string with the
//! private key behind one of the wallet's own P2PKH addresses and returns the
//! base64 signature. Same semantics as dashj's `ECKey.signMessage` and Dash
//! Core's `signmessage` RPC, so a signature produced here verifies with
//! `verifymessage` and with any dashj-based verifier.
//!
//! ## The format
//!
//! The digest is `SHA256d(DASH_SIGNED_MSG_PREFIX ‖ varint(len(msg)) ‖ msg)`,
//! where the prefix is the historical `"\x19DarkCoin Signed Message:\n"` —
//! **not** `"Dash"`. Dash inherited the string from its pre-rename days and it
//! is consensus-visible in every existing verifier, so it can never be updated.
//! [`dashcore::sign_message::signed_msg_hash`] owns that construction; nothing
//! here re-implements it.
//!
//! The signature is the 65-byte BIP-137-style recoverable form —
//! `header ‖ r ‖ s`, with `header = 27 + recovery_id + 4` (the `+ 4` marking a
//! compressed public key) — base64-encoded. Verification recovers the public
//! key from the digest and compares its `PubkeyHash` to the address, which is
//! why the recovery id has to be right and why only P2PKH addresses are
//! signable: no other payload has a defined recovery comparison.
//!
//! ## Why the recovery id is found by trial
//!
//! [`Signer`] is a *plain*-ECDSA interface: `sign_ecdsa` hands back a
//! non-recoverable [`ecdsa::Signature`] plus the compressed public key, which is
//! all a P2PKH `scriptSig` needs. The recovery id is not recoverable from that
//! pair analytically, so it is found by trying all four candidates and keeping
//! the one that recovers the signer's own public key — exactly what dashj's
//! `ECKey.findRecoveryId` does. Keeping the trial here means every signer
//! backend (Keystore, Keychain, hardware) gets signed-message support without
//! growing a recoverable-signing method it would have to implement.
//!
//! ## Consumer
//!
//! The Android wallet's CrowdNode integration signs short strings — withdrawal
//! amounts, the account email — which CrowdNode verifies server-side against
//! the address that funded the account. That is a *proof of address ownership*,
//! not a spend: nothing is selected, reserved, broadcast, or persisted, and the
//! wallet-manager lock is held only long enough to resolve a derivation path.

use std::str::FromStr;

use dashcore::address::Payload;
use dashcore::hashes::Hash;
use dashcore::secp256k1::ecdsa::{RecoverableSignature, RecoveryId};
use dashcore::secp256k1::{Message, Secp256k1};
use dashcore::sign_message::{signed_msg_hash, MessageSignature};
use dashcore::{Address as DashAddress, AddressType, PublicKey as DashPublicKey};
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::signer::{Signer, SignerMethod};
use key_wallet::ManagedAccountType;

use crate::broadcaster::TransactionBroadcaster;
use crate::error::{PlatformWalletError, SIGNER_KEY_UNAVAILABLE_PREFIX};
use crate::wallet::core::CoreWallet;

/// Every recovery id a compact ECDSA signature can carry. Ids 2 and 3 encode an
/// overflowing `r` (x-coordinate ≥ the curve order), astronomically improbable
/// but valid, so all four are tried rather than just 0 and 1.
const RECOVERY_IDS: [i32; 4] = [0, 1, 2, 3];

/// Whether a funds account can be *signed for* by the local mnemonic.
///
/// Only `DashpayExternalAccount`s are watch-only: they hold a **contact's**
/// receiving addresses (we keep the contact's xpub to build payments *to* them
/// and to watch that side), so no private key of ours derives their addresses
/// and signing for them would fail. Every other funds account
/// (BIP44 / BIP32 / CoinJoin / DashPay-receiving) comes from our own seed.
///
/// This is a **signability** filter and nothing more. A watch-only account must
/// be refused even when a caller names its address explicitly.
///
/// Deliberately a private local helper rather than a shared one: the
/// funding-privacy work introduces `wallet::funding_privacy::
/// is_signable_funding_account` with this exact body and semantics, but that
/// module does not exist on this branch. Keeping the predicate here — same
/// name, same body — means the two converge to a single call site by deleting
/// this function when that module lands, with no behavior change to review.
/// Funding accounts whose keys this wallet can sign for: everything except
/// the DashPay external (watch-only, contact-owned) accounts. Shared with the
/// masternode payout-key lookup.
pub(crate) fn is_signable_funding_account(managed_type: &ManagedAccountType) -> bool {
    !matches!(
        managed_type,
        ManagedAccountType::DashpayExternalAccount { .. }
    )
}

impl<B: TransactionBroadcaster + ?Sized> CoreWallet<B> {
    /// Sign `message` with the private key behind `address` and return the
    /// base64 signature — a classic Dash signed message, verifiable by Dash
    /// Core's `verifymessage` RPC, dashj's `ECKey.verifyMessage`, and
    /// CrowdNode's server-side check.
    ///
    /// `address` must be a P2PKH address of THIS wallet, on this wallet's
    /// network, belonging to a *signable* funds account (BIP44 / BIP32 /
    /// CoinJoin / DashPay-receiving). A watch-only DashPay **external** account
    /// holds a contact's addresses whose keys we never had, so it is refused
    /// like any other unowned address —
    /// [`MessageSigningKeyUnavailable`](PlatformWalletError::MessageSigningKeyUnavailable),
    /// which the FFI surfaces as `ErrorSigningKeyUnavailable`.
    ///
    /// Not a spend: no UTXO is selected, reserved, or spent, and the accounts
    /// are visited only to look ONE address up.
    ///
    /// # Arguments
    ///
    /// * `address` — the P2PKH address whose key signs. Must be one of this
    ///   wallet's own, already present in an account's address pool (i.e.
    ///   generated — a gap-limit address the wallet has never derived is not
    ///   findable and is refused).
    /// * `message` — the string to sign, verbatim. It is length-prefixed into
    ///   the digest, so trailing whitespace and newlines are significant; the
    ///   verifier must be handed the identical bytes.
    /// * `signer` — the ECDSA signer that holds the key material (the
    ///   Keystore/Keychain-backed `MnemonicResolverCoreSigner` in production).
    ///   No private key crosses the boundary. It must advertise
    ///   [`SignerMethod::Digest`]; a `Transaction`-only backend cannot sign a
    ///   message and is refused with
    ///   [`MessageSigningFailed`](PlatformWalletError::MessageSigningFailed)
    ///   before it is invoked. A signer that reports its key missing — the
    ///   reserved key-unavailable marker at position 0 of its error rendering,
    ///   as `MnemonicResolverCoreSigner::NotFound` does when the keychain holds
    ///   no mnemonic — surfaces as
    ///   [`MessageSigningKeyUnavailable`](PlatformWalletError::MessageSigningKeyUnavailable),
    ///   the same typed condition as an unowned address.
    ///
    /// # Determinism
    ///
    /// key-wallet's signers produce RFC6979 deterministic, low-s normalized
    /// signatures, so the same `(address, message)` pair on the same seed always
    /// yields the same base64 — which is what lets the roundtrip test pin a
    /// golden vector.
    pub async fn sign_message<S: Signer>(
        &self,
        address: &str,
        message: &str,
        signer: &S,
    ) -> Result<String, PlatformWalletError> {
        // Resolve the address to a leaf derivation path under a READ lock, then
        // release it: the signer round-trip below can be a Keystore/Keychain
        // call (or a hardware round-trip) and must not hold the wallet-manager
        // lock, which every balance read and send path also needs. Nothing is
        // mutated here, so there is no validate-then-mutate window to protect.
        let (target, path) = {
            let wm = self.wallet_manager.read().await;
            let (_, info) = wm
                .get_wallet_and_info(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;

            // The wallet's own network, not `self.network()` (the SDK's): the
            // address must be checked against the keys it could belong to.
            let network = info.core_wallet.network();

            let parsed = DashAddress::from_str(address).map_err(|e| {
                PlatformWalletError::MessageSigningAddressInvalid {
                    address: address.to_string(),
                    reason: format!("not a valid Dash address: {e}"),
                }
            })?;
            let target = parsed.require_network(network).map_err(|e| {
                PlatformWalletError::MessageSigningAddressInvalid {
                    address: address.to_string(),
                    reason: format!("encoded for another network than {network:?}: {e}"),
                }
            })?;
            if target.address_type() != Some(AddressType::P2pkh) {
                return Err(PlatformWalletError::MessageSigningAddressInvalid {
                    address: address.to_string(),
                    reason: format!(
                        "signed messages verify by recovering a public key and comparing its \
                         PubkeyHash, so only P2PKH addresses can sign; this is {:?}",
                        target.address_type()
                    ),
                });
            }

            // PRIVACY-DOMAIN-OK: this iterates funds accounts to LOOK ONE
            // ADDRESS UP and stops at the first pool that owns it. No UTXO is
            // selected, no value moves, and no transaction is built, so there is
            // no on-chain link to create and nothing is accumulated across
            // accounts. Watch-only accounts are skipped because their keys are a
            // contact's, not ours.
            let path = info
                .core_wallet
                .accounts
                .all_funding_accounts()
                .into_iter()
                .filter(|acc| is_signable_funding_account(acc.managed_account_type()))
                .find_map(|acc| acc.address_derivation_path(&target))
                .ok_or_else(|| PlatformWalletError::MessageSigningKeyUnavailable {
                    address: target.to_string(),
                })?;

            (target, path)
        };

        // `Signer` advertises the methods it can perform and assigns dispatch to
        // the CALLER: `sign_ecdsa` is documented as valid only when the backend
        // supports `SignerMethod::Digest`. A classic signed message is
        // inherently a host-computed-digest operation — there is no transaction
        // for a device to re-hash and display — so a `Transaction`-only backend
        // (a hardware wallet whose whole policy is to never blind-sign) cannot
        // serve this call, and asking it to would defeat that policy.
        //
        // Refused here rather than left to the backend because an unadvertised
        // method has no defined behavior: an implementation may reject it, panic
        // because it believed the method unreachable, or — worst — blind-sign
        // anyway. Mirrors the identical pre-check key-wallet performs before its
        // own `sign_ecdsa` dispatch in `TransactionSigner::sig_and_pubkey`.
        if !signer.supports(SignerMethod::Digest) {
            return Err(PlatformWalletError::MessageSigningFailed {
                address: target.to_string(),
                reason: format!(
                    "signer backend cannot sign message digests: it advertises {:?}, but a \
                     classic signed message requires {:?}",
                    signer.supported_methods(),
                    SignerMethod::Digest,
                ),
            });
        }

        let hash = signed_msg_hash(message);
        // `sign_ecdsa`'s error is the associated type `S::Error`, bounded only
        // by `Display + Send + Sync + 'static` — there is no enum to match, so
        // `preserve_signer_key_unavailable_or` (which takes a `dash_sdk::Error`
        // and serves the state-transition signing paths) does not apply here.
        // The one typed signal a `Display`-only surface can carry is the
        // reserved machine marker a signer stamps at POSITION 0 of its own
        // rendering (`MnemonicResolverCoreSigner::NotFound` in production), so
        // key unavailability is recognized by that position-0 check — never a
        // substring sniff (#4183 review) — and it must happen BEFORE the
        // "signer rejected the digest at {path}: " context is prepended, which
        // would push the marker mid-string where no permitted check can see it.
        let (signature, public_key) = signer
            .sign_ecdsa(&path, hash.to_byte_array())
            .await
            .map_err(|e| {
                let rendered = e.to_string();
                if rendered.starts_with(SIGNER_KEY_UNAVAILABLE_PREFIX) {
                    PlatformWalletError::MessageSigningKeyUnavailable {
                        address: target.to_string(),
                    }
                } else {
                    PlatformWalletError::MessageSigningFailed {
                        address: target.to_string(),
                        reason: format!("signer rejected the digest at {path}: {rendered}"),
                    }
                }
            })?;

        // Signers return compressed public keys, so the address the signed
        // message will verify against is `P2PKH(compressed(pubkey))`.
        let dash_public_key = DashPublicKey {
            inner: public_key,
            compressed: true,
        };

        // The address a verifier will derive must be the address the caller
        // asked for. If path resolution ever hands back the wrong leaf, this is
        // the difference between a clear failure and silently vouching for an
        // address with someone else's key. Compared on the payload so the check
        // is network-independent.
        if *target.payload() != Payload::p2pkh(&dash_public_key) {
            return Err(PlatformWalletError::MessageSigningFailed {
                address: target.to_string(),
                reason: format!(
                    "the key at {path} does not own this address — its public key hashes to a \
                     different P2PKH payload"
                ),
            });
        }

        // `recover_ecdsa` needs only a `Verification` context — the signing
        // tables a full `Secp256k1::new()` would also allocate are dead weight
        // here, since the signature itself came from the signer.
        let secp = Secp256k1::verification_only();
        let digest = Message::from_digest(hash.to_byte_array());
        let compact = signature.serialize_compact();

        let recoverable = RECOVERY_IDS
            .into_iter()
            .filter_map(|id| RecoveryId::try_from(id).ok())
            .filter_map(|recid| RecoverableSignature::from_compact(&compact, recid).ok())
            .find(|candidate| {
                secp.recover_ecdsa(&digest, candidate)
                    .is_ok_and(|recovered| recovered == public_key)
            })
            .ok_or_else(|| PlatformWalletError::MessageSigningFailed {
                address: target.to_string(),
                reason: "no recovery id in 0..=3 recovers the signing public key".to_string(),
            })?;

        Ok(MessageSignature::new(recoverable, true).to_base64())
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use std::sync::Arc;

    use dashcore::secp256k1::{ecdsa, PublicKey, Secp256k1};
    use dashcore::sign_message::{signed_msg_hash, MessageSignature};
    use dashcore::{Address as DashAddress, Network};
    use key_wallet::signer::{Signer, SignerMethod, TransactionCategory};
    use key_wallet::DerivationPath;

    use crate::test_support::{
        mnemonic_wallet_manager, AlwaysRejectedBroadcaster, MESSAGE_SIGNING_TEST_MNEMONIC,
    };
    use crate::wallet::core::CoreWallet;
    use crate::wallet::core::WalletGeneration;
    use crate::wallet::platform_wallet::WalletId;
    use crate::PlatformWalletError;

    /// The message every fixture signs. Short and ASCII, like the withdrawal
    /// amounts and emails CrowdNode signs in production, and the same message
    /// dashj's own `ECKeyTest.verifyMessage` vector uses.
    const MESSAGE: &str = "hello";

    /// A `CoreWallet` over a manager fixture. Message signing never broadcasts,
    /// so the broadcaster is irrelevant.
    fn core_wallet(
        wallet_manager: Arc<
            tokio::sync::RwLock<
                key_wallet_manager::WalletManager<
                    crate::wallet::platform_wallet::PlatformWalletInfo,
                >,
            >,
        >,
        wallet_id: WalletId,
    ) -> CoreWallet<AlwaysRejectedBroadcaster> {
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        CoreWallet::new(
            sdk,
            wallet_manager,
            wallet_id,
            Arc::new(AlwaysRejectedBroadcaster),
            Arc::new(WalletGeneration::new()),
        )
    }

    /// Verify `signature_base64` against `address` the way an external verifier
    /// would: decode, re-derive the digest from the message alone, recover.
    fn verifies_for(signature_base64: &str, address: &DashAddress) -> bool {
        MessageSignature::from_base64(signature_base64)
            .expect("signature is valid base64 of a 65-byte recoverable signature")
            .is_signed_by_address(&Secp256k1::new(), address, signed_msg_hash(MESSAGE))
            .expect("P2PKH address is a supported verification target")
    }

    /// **Cross-implementation parity.** A signature produced by dashj
    /// (`ECKeyTest.verifyMessage`) must verify under this crate's
    /// `signed_msg_hash` + `MessageSignature`, proving the two agree on the
    /// `"\x19DarkCoin Signed Message:\n"` prefix, the varint length prefix, the
    /// double-SHA256, the 65-byte header encoding, and the compressed-key flag.
    ///
    /// This is the anchor the golden vector in
    /// [`roundtrip_signature_is_a_stable_golden`] is meaningful against: without
    /// it, a self-consistent but format-divergent implementation would pass
    /// every other test here and still be rejected by CrowdNode.
    #[test]
    fn dashj_vector_verifies() {
        let address = DashAddress::from_str("Xt5QmmzX2LaMgt81dcXGHhdf8pAhaVJKUW")
            .expect("valid mainnet P2PKH address")
            .require_network(Network::Mainnet)
            .expect("mainnet address on mainnet");
        assert!(
            verifies_for(
                "HPygR8+G/HJ0kSp0azMeW6bvzd1tGg0Nx1mCJ/ls5Yh2Z1WgA10Nc/yPbVYU4HbF8Z98vvXFC8iqGTGsdDgqfe4=",
                &address,
            ),
            "dashj's ECKeyTest.verifyMessage signature must verify here — a failure means the \
             signed-message format has diverged and CrowdNode will reject our signatures"
        );
    }

    /// **The empty message is signable.** Nothing about the format excludes it —
    /// the digest length-prefixes the message, so `""` prefixes as `varint(0)`
    /// and hashes to a perfectly ordinary digest.
    ///
    /// This is a contract the boundary layers depend on, not a curiosity: an
    /// empty Swift `String` marshals to a **nil** base address with length 0, so
    /// the FFI must accept a null `message_ptr` at `message_len == 0` rather
    /// than rejecting it as a null-pointer error. Signing `""` end-to-end here
    /// pins the semantic half of that contract; `empty_message_is_not_a_null_
    /// pointer_error` in the FFI crate pins the boundary half.
    #[tokio::test]
    async fn empty_message_is_signable() {
        let (wm, wallet_id, signer, address) =
            mnemonic_wallet_manager(MESSAGE_SIGNING_TEST_MNEMONIC).await;
        let core = core_wallet(wm, wallet_id);

        let signature = core
            .sign_message(&address.to_string(), "", &signer)
            .await
            .expect("the empty message is signable");

        assert!(
            MessageSignature::from_base64(&signature)
                .expect("valid base64 of a 65-byte recoverable signature")
                .is_signed_by_address(&Secp256k1::new(), &address, signed_msg_hash(""))
                .expect("P2PKH address is a supported verification target"),
            "a signature over the empty message must verify against signed_msg_hash(\"\")"
        );
    }

    /// A signature over one of the wallet's own receive addresses verifies
    /// against that address — the end-to-end contract, checked through the
    /// public verification path rather than by re-deriving the key.
    #[tokio::test]
    async fn signature_verifies_for_the_signing_address() {
        let (wm, wallet_id, signer, address) =
            mnemonic_wallet_manager(MESSAGE_SIGNING_TEST_MNEMONIC).await;
        let core = core_wallet(wm, wallet_id);

        let signature = core
            .sign_message(&address.to_string(), MESSAGE, &signer)
            .await
            .expect("a derived receive address of this wallet must be signable");

        assert!(
            verifies_for(&signature, &address),
            "signature {signature} must verify for {address}"
        );
    }

    /// RFC6979-deterministic golden. The signer signs deterministically and the
    /// mnemonic is fixed, so this exact base64 is stable across runs, platforms,
    /// and rebuilds — a change here means the digest construction, the
    /// derivation path, or the signature encoding moved.
    ///
    /// Cross-check against dashj by importing
    /// [`MESSAGE_SIGNING_TEST_MNEMONIC`], deriving `m/44'/1'/0'/0/0` (testnet
    /// coin type 1), and calling `ECKey.signMessage("hello")`; the strings must
    /// be identical. Verified 2026-07-31 against dashj (`HDKeyDerivation` +
    /// `ECKey.signMessage`, dashj-core test run): dashj produced this exact
    /// base64 for this address, byte-for-byte.
    #[tokio::test]
    async fn roundtrip_signature_is_a_stable_golden() {
        let (wm, wallet_id, signer, address) =
            mnemonic_wallet_manager(MESSAGE_SIGNING_TEST_MNEMONIC).await;
        let core = core_wallet(wm, wallet_id);

        // The first BIP44 external address of the fixed mnemonic on testnet,
        // pinned alongside the signature so a derivation change fails loudly
        // here rather than silently producing a valid signature for a
        // different address.
        assert_eq!(
            address.to_string(),
            "yRd4FhXfVGHXpsuZXPNkMrfD9GVj46pnjt",
            "fixture address drifted — the golden signature below is for the old address"
        );

        let signature = core
            .sign_message(&address.to_string(), MESSAGE, &signer)
            .await
            .expect("fixture address is signable");

        assert_eq!(
            signature,
            "HzW1qnS7pYhopcbz1ZbiQY+axMMDQ2cMXBjey37IQS15OOluF6l/rfEre7Bl8AzUOwYdANkLYRelpKJmIH4kfZM=",
            "RFC6979-deterministic golden vector"
        );
    }

    /// An address the wallet does not own is refused as
    /// `MessageSigningKeyUnavailable` (FFI code 31), not as a generic failure —
    /// the host routes that code to key repair / address correction.
    #[tokio::test]
    async fn unknown_address_is_key_unavailable() {
        let (wm, wallet_id, signer, _) =
            mnemonic_wallet_manager(MESSAGE_SIGNING_TEST_MNEMONIC).await;
        let core = core_wallet(wm, wallet_id);

        // A well-formed testnet P2PKH address from a different seed.
        let (_, _, _, foreign) = mnemonic_wallet_manager(
            "legal winner thank year wave sausage worth useful legal winner thank yellow",
        )
        .await;

        let result = core
            .sign_message(&foreign.to_string(), MESSAGE, &signer)
            .await;

        match result {
            Err(PlatformWalletError::MessageSigningKeyUnavailable { address }) => {
                assert_eq!(address, foreign.to_string());
            }
            other => panic!("expected MessageSigningKeyUnavailable, got {other:?}"),
        }
    }

    /// A mainnet address against a testnet wallet is rejected at the network
    /// check, before any account lookup — so a mainnet address that happens to
    /// share a payload with a testnet address the wallet owns can never be
    /// signed for.
    #[tokio::test]
    async fn wrong_network_address_is_rejected() {
        let (wm, wallet_id, signer, _) =
            mnemonic_wallet_manager(MESSAGE_SIGNING_TEST_MNEMONIC).await;
        let core = core_wallet(wm, wallet_id);

        let result = core
            .sign_message("Xt5QmmzX2LaMgt81dcXGHhdf8pAhaVJKUW", MESSAGE, &signer)
            .await;

        match result {
            Err(PlatformWalletError::MessageSigningAddressInvalid { reason, .. }) => {
                assert!(
                    reason.contains("another network"),
                    "expected a network-mismatch reason, got {reason:?}"
                );
            }
            other => panic!("expected MessageSigningAddressInvalid, got {other:?}"),
        }
    }

    /// A hardware-style backend that can only sign whole transactions it
    /// re-hashes and displays, and never a host-computed digest. `sign_ecdsa`
    /// panics rather than returning an error: the `Signer` contract makes an
    /// unadvertised method undefined to call, so reaching it at all is the bug
    /// this fixture exists to catch. A backend that instead blind-signed would
    /// silently defeat the policy it advertises, which no `Result` could
    /// express.
    struct TransactionOnlySigner;

    #[async_trait::async_trait]
    impl Signer for TransactionOnlySigner {
        type Error = String;

        fn supported_methods(&self) -> &[SignerMethod] {
            &[SignerMethod::Transaction(TransactionCategory::Classical)]
        }

        async fn sign_ecdsa(
            &self,
            _path: &DerivationPath,
            _sighash: [u8; 32],
        ) -> Result<(ecdsa::Signature, PublicKey), Self::Error> {
            panic!(
                "sign_ecdsa must never be reached on a signer that does not advertise \
                 SignerMethod::Digest — sign_message is required to refuse first"
            );
        }

        async fn public_key(&self, _path: &DerivationPath) -> Result<PublicKey, Self::Error> {
            panic!("public_key is not part of the signed-message path");
        }
    }

    /// A digest-capable backend that fails every signature the way a Keystore
    /// or Keychain reports a missing key: its `Display` begins with the
    /// reserved key-unavailable marker at position 0, exactly as
    /// `MnemonicResolverCoreSigner::NotFound` renders in production.
    struct KeyUnavailableSigner;

    #[async_trait::async_trait]
    impl Signer for KeyUnavailableSigner {
        type Error = String;

        fn supported_methods(&self) -> &[SignerMethod] {
            &[SignerMethod::Digest]
        }

        async fn sign_ecdsa(
            &self,
            _path: &DerivationPath,
            _sighash: [u8; 32],
        ) -> Result<(ecdsa::Signature, PublicKey), Self::Error> {
            Err(format!(
                "{}no private key stored for this derivation path",
                crate::error::SIGNER_KEY_UNAVAILABLE_PREFIX
            ))
        }

        async fn public_key(&self, _path: &DerivationPath) -> Result<PublicKey, Self::Error> {
            panic!("public_key is not part of the signed-message path");
        }
    }

    /// **The signer-reported missing key is a typed condition.** A signer whose
    /// failure rendering starts with the reserved key-unavailable marker maps
    /// to `MessageSigningKeyUnavailable` — FFI code 31, the host's key-repair
    /// route — never to `MessageSigningFailed`, whose context prefix would bury
    /// the marker mid-string and flatten the condition to `ErrorUnknown`.
    ///
    /// The promotion is `sign_message`'s own position-0 check on the signer's
    /// rendering BEFORE any context is added; the FFI boundary maps the typed
    /// variant and parses nothing.
    #[tokio::test]
    async fn signer_key_unavailable_is_typed_during_message_signing() {
        let (wm, wallet_id, _, address) =
            mnemonic_wallet_manager(MESSAGE_SIGNING_TEST_MNEMONIC).await;
        let core = core_wallet(wm, wallet_id);

        let result = core
            .sign_message(&address.to_string(), MESSAGE, &KeyUnavailableSigner)
            .await;

        match result {
            Err(PlatformWalletError::MessageSigningKeyUnavailable { address: reported }) => {
                assert_eq!(reported, address.to_string());
            }
            other => panic!("expected MessageSigningKeyUnavailable, got {other:?}"),
        }
    }

    /// A digest-capable backend whose failure merely MENTIONS the reserved
    /// marker after position 0 — the shape a foreign signer produces when its
    /// human-readable text quotes another error.
    struct MidStringMarkerSigner;

    #[async_trait::async_trait]
    impl Signer for MidStringMarkerSigner {
        type Error = String;

        fn supported_methods(&self) -> &[SignerMethod] {
            &[SignerMethod::Digest]
        }

        async fn sign_ecdsa(
            &self,
            _path: &DerivationPath,
            _sighash: [u8; 32],
        ) -> Result<(ecdsa::Signature, PublicKey), Self::Error> {
            Err(format!(
                "remote signer reported: {}oops",
                crate::error::SIGNER_KEY_UNAVAILABLE_PREFIX
            ))
        }

        async fn public_key(&self, _path: &DerivationPath) -> Result<PublicKey, Self::Error> {
            panic!("public_key is not part of the signed-message path");
        }
    }

    /// The marker only counts at position 0 of the signer's rendering: a
    /// mid-string mention stays `MessageSigningFailed`, never key-unavailable —
    /// promoting it would be the substring sniff #4183's review rejected, and
    /// would misroute a generic failure into the host's key repair.
    #[tokio::test]
    async fn mid_string_marker_is_not_promoted_during_message_signing() {
        let (wm, wallet_id, _, address) =
            mnemonic_wallet_manager(MESSAGE_SIGNING_TEST_MNEMONIC).await;
        let core = core_wallet(wm, wallet_id);

        let result = core
            .sign_message(&address.to_string(), MESSAGE, &MidStringMarkerSigner)
            .await;

        match result {
            Err(PlatformWalletError::MessageSigningFailed { reason, .. }) => {
                assert!(
                    reason.contains(crate::error::SIGNER_KEY_UNAVAILABLE_PREFIX),
                    "the signer's own text is preserved in the reason: {reason:?}"
                );
            }
            other => panic!("expected MessageSigningFailed, got {other:?}"),
        }
    }

    /// **Capability dispatch.** `Signer` assigns method dispatch to the caller,
    /// and documents `sign_ecdsa` as valid only when the backend advertises
    /// `SignerMethod::Digest`. A transaction-only backend must therefore be
    /// refused with a typed error *before* it is invoked — the fixture's
    /// `sign_ecdsa` panics, so a regression that drops the guard fails loudly
    /// here rather than blind-signing on a device whose policy forbids it.
    ///
    /// The address is one the wallet genuinely owns, so the refusal cannot be
    /// mistaken for the key-unavailable path: this pins the capability check
    /// specifically.
    #[tokio::test]
    async fn transaction_only_signer_is_refused_before_signing() {
        let (wm, wallet_id, _, address) =
            mnemonic_wallet_manager(MESSAGE_SIGNING_TEST_MNEMONIC).await;
        let core = core_wallet(wm, wallet_id);

        let result = core
            .sign_message(&address.to_string(), MESSAGE, &TransactionOnlySigner)
            .await;

        match result {
            Err(PlatformWalletError::MessageSigningFailed {
                address: reported,
                reason,
            }) => {
                assert_eq!(reported, address.to_string());
                assert!(
                    reason.contains("cannot sign message digests"),
                    "the reason must name the capability refusal, got {reason:?}"
                );
            }
            other => panic!("expected MessageSigningFailed, got {other:?}"),
        }
    }
}
