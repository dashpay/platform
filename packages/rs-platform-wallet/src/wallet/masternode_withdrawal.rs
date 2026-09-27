//! Claim (withdraw) Platform credits from a masternode's owner identity.
//!
//! A masternode's Platform rewards accrue on the identity whose id is the
//! masternode's proTxHash. Two wallet-derived keys can move them to L1 via
//! an Identity Credit Withdrawal, and Platform treats them differently
//! (`rs-drive-abci` `signature_purpose_matches_requirements`):
//!
//! - the **owner key** (`ProviderOwnerKeys`, registered in the ProRegTx as
//!   a hash160 → identity key with purpose `OWNER`) may withdraw, but
//!   only to the node's **registered payout address** — a withdrawal
//!   signed with it must not carry an output script;
//! - the **transfer key** — the pubkey behind the registered P2PKH
//!   payout script (identity key with purpose `TRANSFER`) — may withdraw
//!   to **any** destination.
//!
//! Everything that decides *which* key, *which* derivation path and
//! *which* output script lives here, so the FFI and the apps only marshal:
//!
//! 1. [`PlatformWallet::masternode_withdrawal_keys`] (sync, no network)
//!    answers which of the two keys this wallet holds — seedless, from
//!    the account xpubs / address pools — and the payout address.
//! 2. [`PlatformWallet::masternode_withdraw`] (async) fetches the
//!    identity, picks the matching identity key, refuses to broadcast if
//!    the identity doesn't carry it, and signs through a
//!    [`key_wallet::signer::Signer`] (on iOS the mnemonic-resolver-backed
//!    signer — the seed never becomes resident).

use std::fmt;

use async_trait::async_trait;
use dashcore::hashes::{hash160, sha256d, Hash};
use dashcore::secp256k1::ecdsa::{RecoverableSignature, RecoveryId};
use dashcore::secp256k1::{Message, Secp256k1};
use dashcore::signer::CompactSignature;
use dashcore::{Address as DashAddress, AddressType, Network, ScriptBuf};
use dpp::address_funds::AddressWitness;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::signer::Signer as IdentitySigner;
use dpp::identity::{Identity, IdentityPublicKey, KeyType};
use dpp::platform_value::BinaryData;
use dpp::prelude::Identifier;
use dpp::ProtocolError;
use key_wallet::account::AccountType;
use key_wallet::bip32::{ChildNumber, DerivationPath};
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::signer::{Signer as CoreSigner, SignerMethod};

use dash_sdk::platform::transition::broadcast::BroadcastStateTransition;
use dash_sdk::platform::Fetch;
use dpp::identity::core_script::CoreScript;
use dpp::state_transition::identity_credit_withdrawal_transition::methods::{
    IdentityCreditWithdrawalTransitionMethodsV0, PreferredKeyPurposeForSigningWithdrawal,
};
use dpp::state_transition::identity_credit_withdrawal_transition::{
    IdentityCreditWithdrawalTransition, MIN_CORE_FEE_PER_BYTE,
};
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::withdrawal::Pooling;

use crate::broadcast_outcome::{broadcast_definitely_failed, carries_consensus_rejection};
use crate::error::{PlatformWalletError, SIGNER_KEY_UNAVAILABLE_PREFIX};
use crate::wallet::core::is_signable_funding_account;
use crate::wallet::identity::network::{
    select_owner_withdrawal_key, select_transfer_withdrawal_key,
};
use crate::wallet::platform_wallet::PlatformWallet;
use crate::wallet::provider_key_at_index::ProviderKeyKind;

/// Which wallet-held key signs a masternode withdrawal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MasternodeWithdrawalKey {
    /// The `ProviderOwnerKeys` key. Platform routes the payout to the
    /// registered payout address; no destination may be chosen.
    Owner,
    /// The payout-script key (identity purpose `TRANSFER`). Any
    /// destination is allowed.
    Transfer,
}

/// Which masternode-withdrawal signing keys this wallet holds, plus the
/// node's registered payout address. Produced by
/// [`PlatformWallet::masternode_withdrawal_keys`]; consumed by the UI (to
/// decide whether the destination is editable) and by
/// [`PlatformWallet::masternode_withdraw`] (to derive-sign).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MasternodeWithdrawalKeys {
    /// `Some((index, path))` when the masternode's owner key is one of this
    /// wallet's `ProviderOwnerKeys` (`m/9'/coin'/3'/2'/index`).
    pub owner_key: Option<(u32, DerivationPath)>,
    /// `Some(path)` when the registered payout script is a P2PKH to one of
    /// this wallet's funding-account addresses — the identity's `TRANSFER`
    /// key is then wallet-derivable at that path.
    pub transfer_key: Option<DerivationPath>,
    /// The registered payout address (base58 for this wallet's network),
    /// or `None` when the node has no payout script on record or it isn't
    /// encodable as an address. Owner-key withdrawals always pay here.
    pub payout_address: Option<String>,
    /// hash160 of the registered payout script's pubkey when it is P2PKH
    /// (the `TRANSFER` identity key's data); `None` for P2SH / unknown.
    pub payout_key_hash160: Option<[u8; 20]>,
}

impl MasternodeWithdrawalKeys {
    /// True when at least one wallet-held key can sign a withdrawal.
    pub fn can_withdraw(&self) -> bool {
        self.owner_key.is_some() || self.transfer_key.is_some()
    }

    /// True when the destination may differ from the payout address —
    /// i.e. the transfer key is in this wallet.
    pub fn can_choose_destination(&self) -> bool {
        self.transfer_key.is_some()
    }
}

/// One masternode withdrawal to submit.
#[derive(Debug, Clone)]
pub struct MasternodeWithdrawalRequest {
    /// proTxHash in stored WIRE order (as the aggregation returns it); the
    /// owner identity id is the display-order (reversed) form.
    pub pro_tx_hash: [u8; 32],
    /// Owner key hash160 from the ProRegTx (None ⇒ owner path unavailable).
    pub owner_key_hash: Option<[u8; 20]>,
    /// Amount to withdraw, in credits. Must be ≥ Platform's minimum
    /// withdrawal and a whole number of duffs (multiple of 1000 credits).
    pub amount_credits: u64,
    /// Which wallet key signs.
    pub signing_key: MasternodeWithdrawalKey,
    /// Destination for a transfer-key withdrawal. `None` ⇒ the registered
    /// payout address. MUST be `None` for [`MasternodeWithdrawalKey::Owner`]
    /// — Platform rejects an output script signed with the owner key.
    pub destination: Option<DashAddress>,
}

/// Default provider-key scan window when the owner pool has no deeper
/// watermark — the same floor the operator derive-and-compare uses.
const PROVIDER_KEY_WINDOW: u32 = 20;

impl PlatformWallet {
    /// Which masternode-withdrawal keys this wallet holds for the node
    /// described by `owner_key_hash` / `payout_script`, plus its payout
    /// address. Seedless: the owner key is found by deriving the
    /// `ProviderOwnerKeys` public keys from the account xpub and comparing
    /// hash160s; the transfer key by looking the payout address up in the
    /// funding accounts' address pools.
    ///
    /// `owner_key_index_hint` is a durable index the caller already knows
    /// for this owner key (the persisted ownership row, or the host's own
    /// address join). It is VERIFIED, never trusted: the key at that index is
    /// derived and compared first, so a restored wallet whose in-memory pool
    /// has no watermark (and whose owner key sits above the default scan
    /// window) still resolves. Without a hint — or if the hint doesn't match —
    /// the pool depth (at least [`PROVIDER_KEY_WINDOW`]) is scanned.
    ///
    /// Blocking (takes the wallet-manager read lock) — call from a plain
    /// thread, not from inside the async runtime.
    pub fn masternode_withdrawal_keys(
        &self,
        owner_key_hash: Option<&[u8; 20]>,
        payout_script: Option<&[u8]>,
        owner_key_index_hint: Option<u32>,
    ) -> Result<MasternodeWithdrawalKeys, PlatformWalletError> {
        let network = self.network();

        // Payout script → address (+ the P2PKH pubkey hash that is the
        // identity's TRANSFER key data).
        let payout_address = payout_script.and_then(|script| {
            DashAddress::from_script(&ScriptBuf::from_bytes(script.to_vec()), network).ok()
        });
        let payout_key_hash160 = payout_address.as_ref().and_then(|address| {
            if address.address_type() != Some(AddressType::P2pkh) {
                return None;
            }
            let script = address.script_pubkey();
            // P2PKH: OP_DUP OP_HASH160 <20> OP_EQUALVERIFY OP_CHECKSIG
            let bytes = script.as_bytes();
            (bytes.len() == 25)
                .then(|| <[u8; 20]>::try_from(&bytes[3..23]).ok())
                .flatten()
        });

        // Under the wallet-manager read lock: the owner pool's depth and the
        // payout address's derivation path. Dropped before the derive loop
        // below (which takes the wallet's own state lock).
        let (owner_scan_max, transfer_key) = {
            let wm = self.wallet_manager().blocking_read();
            let info = wm.get_wallet_info(&self.wallet_id()).ok_or_else(|| {
                PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id()))
            })?;

            let owner_scan_max = info
                .core_wallet
                .accounts
                .provider_owner_keys
                .as_ref()
                .and_then(|acct| {
                    acct.managed_account_type()
                        .address_pools()
                        .iter()
                        .filter_map(|p| p.highest_generated)
                        .max()
                })
                .map(|h| h.saturating_add(1))
                .unwrap_or(PROVIDER_KEY_WINDOW)
                .max(PROVIDER_KEY_WINDOW);

            let transfer_key = match (&payout_address, payout_key_hash160) {
                (Some(address), Some(_)) => info
                    .core_wallet
                    .accounts
                    .all_funding_accounts()
                    .into_iter()
                    .filter(|acc| is_signable_funding_account(acc.managed_account_type()))
                    .find_map(|acc| acc.address_derivation_path(address)),
                _ => None,
            };

            (owner_scan_max, transfer_key)
        };

        // Owner key: verify the caller's hint first, then derive-and-compare
        // over the pool depth. Seedless — `derive_provider_key_at_index`
        // reads the account xpub.
        let owner_key = match owner_key_hash {
            Some(target) => {
                self.find_owner_key(target, owner_scan_max, owner_key_index_hint, network)?
            }
            None => None,
        };

        Ok(MasternodeWithdrawalKeys {
            owner_key,
            transfer_key,
            payout_address: payout_address.map(|a| a.to_string()),
            payout_key_hash160,
        })
    }

    fn find_owner_key(
        &self,
        owner_key_hash: &[u8; 20],
        scan_max: u32,
        index_hint: Option<u32>,
        network: Network,
    ) -> Result<Option<(u32, DerivationPath)>, PlatformWalletError> {
        if let Some(hint) = index_hint {
            match self.owner_key_matches_at(hint, owner_key_hash)? {
                // No provider-owner-keys account at all ⇒ nothing to match.
                None => return Ok(None),
                Some(true) => return Ok(Some((hint, provider_owner_key_path(network, hint)?))),
                Some(false) => {} // stale hint — fall back to the scan
            }
        }
        for index in 0..scan_max {
            match self.owner_key_matches_at(index, owner_key_hash)? {
                None => return Ok(None),
                Some(true) => return Ok(Some((index, provider_owner_key_path(network, index)?))),
                Some(false) => {}
            }
        }
        Ok(None)
    }

    /// `Some(matches)` for the owner key derived at `index`; `None` when the
    /// wallet has no provider-owner-keys account to derive from.
    fn owner_key_matches_at(
        &self,
        index: u32,
        owner_key_hash: &[u8; 20],
    ) -> Result<Option<bool>, PlatformWalletError> {
        let derived =
            match self.derive_provider_key_at_index(ProviderKeyKind::Owner, index, None, false) {
                Ok(derived) => derived,
                Err(PlatformWalletError::AddressNotFound(_)) => return Ok(None),
                Err(e) => return Err(e),
            };
        let hash: [u8; 20] = hash160::Hash::hash(&derived.public_key_bytes).to_byte_array();
        Ok(Some(&hash == owner_key_hash))
    }

    /// Submit an Identity Credit Withdrawal from the masternode's owner
    /// identity, signed with the wallet key chosen in `request.signing_key`
    /// and derived through `signer` at the path resolved by
    /// [`Self::masternode_withdrawal_keys`] (`keys`). Returns the identity's
    /// remaining credit balance as proven by Platform.
    ///
    /// Guards (all fail WITHOUT broadcasting):
    /// - owner path with a destination, or a key the wallet doesn't hold;
    /// - the identity is missing, or doesn't carry the expected
    ///   `OWNER` / `TRANSFER` key for the wallet's hash160.
    ///
    /// Outcomes after the transition leaves this wallet are kept distinct:
    /// a definitive rejection (consensus error, or a transport verdict that
    /// rules out delivery) is an ordinary error and may be retried; anything
    /// ambiguous — broadcast accepted (or its ACK lost) and the result wait
    /// failed — is [`PlatformWalletError::MasternodeWithdrawalUnconfirmed`]
    /// and MUST NOT be retried until the claimable balance is re-read: the
    /// identity nonce was already consumed for this attempt, so a blind retry
    /// would submit a second withdrawal.
    pub async fn masternode_withdraw<S: CoreSigner + ?Sized>(
        &self,
        request: MasternodeWithdrawalRequest,
        keys: &MasternodeWithdrawalKeys,
        signer: &S,
    ) -> Result<u64, PlatformWalletError> {
        // Resolve (path, expected hash160, destination) per signing key.
        let (path, expected_hash160, destination) = match request.signing_key {
            MasternodeWithdrawalKey::Owner => {
                if request.destination.is_some() {
                    return Err(PlatformWalletError::InvalidParameter(
                        "an owner-key withdrawal pays the registered payout address; a \
                         destination cannot be chosen"
                            .to_string(),
                    ));
                }
                let owner_hash = request.owner_key_hash.ok_or_else(|| {
                    PlatformWalletError::InvalidParameter(
                        "masternode has no owner key hash on record".to_string(),
                    )
                })?;
                let (_, path) = keys.owner_key.clone().ok_or_else(|| {
                    PlatformWalletError::InvalidParameter(
                        "this wallet does not hold the masternode's owner key".to_string(),
                    )
                })?;
                (path, owner_hash, None)
            }
            MasternodeWithdrawalKey::Transfer => {
                let path = keys.transfer_key.clone().ok_or_else(|| {
                    PlatformWalletError::InvalidParameter(
                        "this wallet does not hold the masternode's payout (transfer) key"
                            .to_string(),
                    )
                })?;
                let payout_hash = keys.payout_key_hash160.ok_or_else(|| {
                    PlatformWalletError::InvalidParameter(
                        "masternode payout script is not a P2PKH address".to_string(),
                    )
                })?;
                // Default destination = the registered payout address; an
                // explicit one must be for this wallet's network.
                let destination = match request.destination {
                    Some(address) => address,
                    None => {
                        let payout = keys.payout_address.as_deref().ok_or_else(|| {
                            PlatformWalletError::InvalidParameter(
                                "masternode has no payout address on record".to_string(),
                            )
                        })?;
                        payout
                            .parse::<DashAddress<dashcore::address::NetworkUnchecked>>()
                            .map_err(|e| {
                                PlatformWalletError::InvalidParameter(format!(
                                    "payout address is not a valid Dash address: {e}"
                                ))
                            })?
                            .require_network(self.network())
                            .map_err(|e| {
                                PlatformWalletError::InvalidParameter(format!(
                                    "payout address is for another network: {e}"
                                ))
                            })?
                    }
                };
                (path, payout_hash, Some(destination))
            }
        };

        execute_masternode_withdrawal(
            self.sdk(),
            request.pro_tx_hash,
            request.amount_credits,
            request.signing_key,
            expected_hash160,
            path,
            destination,
            signer,
        )
        .await
    }
}

/// The network half of a masternode withdrawal, shared by the wallet path
/// ([`PlatformWallet::masternode_withdraw`], key resolved by derivation) and
/// the tracked path (`PlatformWalletManager::tracked_masternode_withdraw`,
/// key supplied by the host): fetch the owner identity (id = display-order
/// proTxHash), select the OWNER / TRANSFER identity key matching
/// `expected_hash160`, sign with `signer` at `path` through
/// [`DerivedKeyIdentitySigner`] (which re-checks the derived key against
/// `expected_hash160` before emitting a signature), broadcast, and wait for
/// the proved balance. Error semantics: definitive rejections stay
/// retryable, ambiguous outcomes are
/// [`PlatformWalletError::MasternodeWithdrawalUnconfirmed`].
#[allow(clippy::too_many_arguments)]
pub(crate) async fn execute_masternode_withdrawal<S: CoreSigner + ?Sized + Sync>(
    sdk: &dash_sdk::Sdk,
    pro_tx_hash: [u8; 32],
    amount_credits: u64,
    signing_key: MasternodeWithdrawalKey,
    expected_hash160: [u8; 20],
    path: DerivationPath,
    destination: Option<DashAddress>,
    signer: &S,
) -> Result<u64, PlatformWalletError> {
    if amount_credits == 0 {
        return Err(PlatformWalletError::InvalidParameter(
            "masternode withdrawal amount must be greater than zero".to_string(),
        ));
    }
    if !signer.supports(SignerMethod::Digest) {
        return Err(PlatformWalletError::InvalidParameter(format!(
            "signer backend cannot sign digests: it advertises {:?}, but an identity \
             credit withdrawal requires {:?}",
            signer.supported_methods(),
            SignerMethod::Digest,
        )));
    }

    // Owner identity id = display-order proTxHash.
    let mut id_bytes = pro_tx_hash;
    id_bytes.reverse();
    let identity_id = Identifier::from(id_bytes);

    let identity = Identity::fetch(sdk, identity_id)
        .await?
        .ok_or(PlatformWalletError::IdentityNotFound(identity_id))?;

    let identity_key = match signing_key {
        MasternodeWithdrawalKey::Owner => {
            select_owner_withdrawal_key(identity.public_keys().values(), &expected_hash160)
        }
        MasternodeWithdrawalKey::Transfer => {
            select_transfer_withdrawal_key(identity.public_keys().values(), &expected_hash160)
        }
    }
    .cloned()
    .ok_or_else(|| {
        PlatformWalletError::InvalidIdentityData(format!(
            "the masternode identity {identity_id} has no {} key matching this wallet's key \
             (hash160 {}); not broadcasting",
            match signing_key {
                MasternodeWithdrawalKey::Owner => "OWNER",
                MasternodeWithdrawalKey::Transfer => "TRANSFER",
            },
            hex::encode(expected_hash160),
        ))
    })?;

    let identity_signer = DerivedKeyIdentitySigner {
        signer,
        path,
        expected_key_hash160: expected_hash160,
    };

    let definitive = |e: dash_sdk::Error| {
        crate::error::preserve_signer_key_unavailable_or(e, |e| {
            PlatformWalletError::InvalidIdentityData(format!("masternode withdrawal failed: {e}"))
        })
    };

    // Build + sign locally. Nothing has left the wallet yet, so every
    // error here is definitive and retryable.
    let nonce = sdk
        .get_identity_nonce(identity_id, true, None)
        .await
        .map_err(definitive)?;
    let output_script = destination.map(|address| CoreScript::new(address.script_pubkey()));
    let state_transition = IdentityCreditWithdrawalTransition::try_from_identity(
        &identity,
        output_script,
        amount_credits,
        Pooling::Never,
        MIN_CORE_FEE_PER_BYTE,
        0,
        identity_signer,
        Some(&identity_key),
        PreferredKeyPurposeForSigningWithdrawal::TransferPreferred,
        nonce,
        sdk.version(),
        None,
    )
    .await
    .map_err(|e| definitive(dash_sdk::Error::Protocol(e)))?;

    // Broadcast, then wait — split so an ambiguous outcome stays typed.
    let unconfirmed = |reason: String| PlatformWalletError::MasternodeWithdrawalUnconfirmed {
        identity_id,
        amount_credits,
        reason,
    };
    match state_transition.broadcast(sdk, None).await {
        Ok(()) => {}
        Err(e) if broadcast_definitely_failed(&e) => return Err(definitive(e)),
        Err(e) => {
            tracing::warn!(
                identity = %identity_id,
                error = %e,
                "masternode withdrawal broadcast returned no verdict; the transition may \
                 have been admitted — falling through to the result wait"
            );
        }
    }

    match state_transition
        .wait_for_affected_state::<StateTransitionProofResult>(sdk, None)
        .await
    {
        Ok(StateTransitionProofResult::VerifiedPartialIdentity(partial)) => partial
            .balance
            .ok_or_else(|| unconfirmed("the result proof carried no identity balance".to_string())),
        // Proved, but not the shape a withdrawal produces — the transition
        // landed; only the balance read-back is missing.
        Ok(_) => Err(unconfirmed(
            "the result proof did not carry the identity's balance".to_string(),
        )),
        Err(e) if carries_consensus_rejection(&e) => Err(definitive(e)),
        Err(e) => Err(unconfirmed(e.to_string())),
    }
}

/// A [`CoreSigner`] over ONE raw secp256k1 secret, path-agnostic — the
/// tracked-masternode withdrawal signer, where the key was supplied by the
/// host (keychain / Keystore) instead of derived from a wallet seed. Every
/// `sign_ecdsa` call answers with this key regardless of the requested
/// path; [`DerivedKeyIdentitySigner`] then re-checks the produced public
/// key against the masternode identity key's hash160, so a wrong key still
/// cannot produce a broadcastable signature.
pub struct RawSecretCoreSigner {
    secret: dashcore::secp256k1::SecretKey,
}

impl RawSecretCoreSigner {
    /// `secret` must be a valid secp256k1 scalar (32 bytes).
    pub fn from_bytes(secret: &[u8; 32]) -> Result<Self, PlatformWalletError> {
        let secret = dashcore::secp256k1::SecretKey::from_slice(secret).map_err(|_| {
            PlatformWalletError::InvalidParameter("not a valid secp256k1 private key".to_string())
        })?;
        Ok(Self { secret })
    }

    /// hash160 of this key's compressed public key.
    pub fn public_key_hash160(&self) -> [u8; 20] {
        let secp = Secp256k1::signing_only();
        let public = dashcore::secp256k1::PublicKey::from_secret_key(&secp, &self.secret);
        hash160::Hash::hash(&public.serialize()).to_byte_array()
    }
}

impl fmt::Debug for RawSecretCoreSigner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RawSecretCoreSigner")
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl CoreSigner for RawSecretCoreSigner {
    type Error = String;

    fn supported_methods(&self) -> &[SignerMethod] {
        &[SignerMethod::Digest]
    }

    async fn sign_ecdsa(
        &self,
        _path: &DerivationPath,
        sighash: [u8; 32],
    ) -> Result<
        (
            dashcore::secp256k1::ecdsa::Signature,
            dashcore::secp256k1::PublicKey,
        ),
        Self::Error,
    > {
        let secp = Secp256k1::new();
        let msg = Message::from_digest(sighash);
        Ok((
            secp.sign_ecdsa(&msg, &self.secret),
            dashcore::secp256k1::PublicKey::from_secret_key(&secp, &self.secret),
        ))
    }

    async fn public_key(
        &self,
        _path: &DerivationPath,
    ) -> Result<dashcore::secp256k1::PublicKey, Self::Error> {
        Ok(dashcore::secp256k1::PublicKey::from_secret_key(
            &Secp256k1::new(),
            &self.secret,
        ))
    }
}

/// `m/9'/coin'/3'/2'/index` — the `ProviderOwnerKeys` account base path plus
/// the (non-hardened) key index, exactly as the account's address pool
/// derives it.
pub(crate) fn provider_owner_key_path(
    network: Network,
    index: u32,
) -> Result<DerivationPath, PlatformWalletError> {
    let base = AccountType::ProviderOwnerKeys
        .derivation_path(network)
        .map_err(|e| PlatformWalletError::KeyDerivation(format!("owner key base path: {e}")))?;
    let child = ChildNumber::from_normal_idx(index)
        .map_err(|e| PlatformWalletError::KeyDerivation(format!("owner key index {index}: {e}")))?;
    Ok(base.child(child))
}

// ---------------------------------------------------------------------------
// Signer adapter: key-wallet `Signer` at a fixed path ⇒ dpp identity signer
// ---------------------------------------------------------------------------

/// Signs identity state transitions with ONE wallet-derived secp256k1 key:
/// the key at `path`, whose compressed pubkey must hash160 to
/// `expected_key_hash160` (the masternode identity key's data). Any other
/// key is refused, so a mismatched identity/wallet pairing can never
/// produce a signature.
///
/// Signature encoding follows `dashcore::signer::sign`: double-SHA256 the
/// signable bytes, ECDSA-sign, and emit the 65-byte compact *recoverable*
/// form (`27 + 4 + recid` prefix for a compressed key). The key-wallet
/// signer returns a plain signature + pubkey, so the recovery id is found
/// by trial — the same trick `CoreWallet::sign_message` uses.
struct DerivedKeyIdentitySigner<'a, S: ?Sized> {
    signer: &'a S,
    path: DerivationPath,
    expected_key_hash160: [u8; 20],
}

impl<'a, S: ?Sized> fmt::Debug for DerivedKeyIdentitySigner<'a, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DerivedKeyIdentitySigner")
            .field("path", &self.path.to_string())
            .field(
                "expected_key_hash160",
                &hex::encode(self.expected_key_hash160),
            )
            .finish()
    }
}

impl<'a, S: ?Sized> DerivedKeyIdentitySigner<'a, S> {
    fn matches(&self, key: &IdentityPublicKey) -> bool {
        match key.key_type() {
            KeyType::ECDSA_HASH160 => key.data().as_slice() == self.expected_key_hash160.as_slice(),
            KeyType::ECDSA_SECP256K1 => {
                hash160::Hash::hash(key.data().as_slice()).to_byte_array()
                    == self.expected_key_hash160
            }
            _ => false,
        }
    }
}

#[async_trait]
impl<'a, S> IdentitySigner<IdentityPublicKey> for DerivedKeyIdentitySigner<'a, S>
where
    S: CoreSigner + ?Sized + Sync,
{
    async fn sign(
        &self,
        identity_public_key: &IdentityPublicKey,
        data: &[u8],
    ) -> Result<BinaryData, ProtocolError> {
        if !self.matches(identity_public_key) {
            return Err(ProtocolError::Generic(format!(
                "masternode withdrawal signer holds only the key with hash160 {}, not {:?}",
                hex::encode(self.expected_key_hash160),
                identity_public_key.id()
            )));
        }

        let digest: [u8; 32] = sha256d::Hash::hash(data).to_byte_array();
        let (signature, public_key) =
            self.signer
                .sign_ecdsa(&self.path, digest)
                .await
                .map_err(|e| {
                    let rendered = e.to_string();
                    // Keep the typed key-unavailable marker at position 0 so
                    // the FFI still maps it to `ErrorSigningKeyUnavailable`.
                    if rendered.starts_with(SIGNER_KEY_UNAVAILABLE_PREFIX) {
                        ProtocolError::Generic(rendered)
                    } else {
                        ProtocolError::Generic(format!(
                            "signer rejected the withdrawal digest at {}: {rendered}",
                            self.path
                        ))
                    }
                })?;

        // Bind the derived key to the identity key BEFORE emitting anything:
        // a wrong index / wrong seed yields a different pubkey.
        let derived_hash: [u8; 20] = hash160::Hash::hash(&public_key.serialize()).to_byte_array();
        if derived_hash != self.expected_key_hash160 {
            return Err(ProtocolError::Generic(format!(
                "the key derived at {} does not match the masternode identity key (hash160 {} \
                 vs {}); refusing to sign",
                self.path,
                hex::encode(derived_hash),
                hex::encode(self.expected_key_hash160)
            )));
        }

        let secp = Secp256k1::verification_only();
        let msg = Message::from_digest(digest);
        let compact = signature.serialize_compact();
        let recoverable = (0..4i32)
            .filter_map(|id| RecoveryId::try_from(id).ok())
            .filter_map(|recid| RecoverableSignature::from_compact(&compact, recid).ok())
            .find(|candidate| {
                secp.recover_ecdsa(&msg, candidate)
                    .is_ok_and(|recovered| recovered == public_key)
            })
            .ok_or_else(|| {
                ProtocolError::Generic(
                    "no recovery id in 0..=3 recovers the withdrawal signing key".to_string(),
                )
            })?;

        Ok(recoverable.to_compact_signature(true).to_vec().into())
    }

    async fn sign_create_witness(
        &self,
        _key: &IdentityPublicKey,
        _data: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        Err(ProtocolError::Generic(
            "masternode withdrawal signer does not produce address witnesses".to_string(),
        ))
    }

    fn can_sign_with(&self, identity_public_key: &IdentityPublicKey) -> bool {
        self.matches(identity_public_key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashcore::secp256k1::{PublicKey, SecretKey};
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::{Purpose, SecurityLevel};
    use std::str::FromStr;

    /// Test signer: one fixed secp256k1 key regardless of path.
    struct FixedKeySigner {
        secret: SecretKey,
    }

    #[async_trait]
    impl CoreSigner for FixedKeySigner {
        type Error = String;

        fn supported_methods(&self) -> &[SignerMethod] {
            &[SignerMethod::Digest]
        }

        async fn sign_ecdsa(
            &self,
            _path: &DerivationPath,
            sighash: [u8; 32],
        ) -> Result<(dashcore::secp256k1::ecdsa::Signature, PublicKey), Self::Error> {
            let secp = Secp256k1::new();
            let msg = Message::from_digest(sighash);
            Ok((
                secp.sign_ecdsa(&msg, &self.secret),
                PublicKey::from_secret_key(&secp, &self.secret),
            ))
        }

        async fn public_key(&self, _path: &DerivationPath) -> Result<PublicKey, Self::Error> {
            Ok(PublicKey::from_secret_key(&Secp256k1::new(), &self.secret))
        }
    }

    fn fixture() -> (FixedKeySigner, [u8; 20], IdentityPublicKey) {
        let secret = SecretKey::from_slice(&[0x42u8; 32]).expect("valid scalar");
        let pubkey = PublicKey::from_secret_key(&Secp256k1::new(), &secret);
        let hash: [u8; 20] = hash160::Hash::hash(&pubkey.serialize()).to_byte_array();
        let key = IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: 0,
            purpose: Purpose::OWNER,
            security_level: SecurityLevel::CRITICAL,
            contract_bounds: None,
            key_type: KeyType::ECDSA_HASH160,
            read_only: false,
            data: BinaryData::new(hash.to_vec()),
            disabled_at: None,
        });
        (FixedKeySigner { secret }, hash, key)
    }

    #[tokio::test]
    async fn signature_verifies_against_the_identity_key_hash() {
        let (signer, hash, key) = fixture();
        let adapter = DerivedKeyIdentitySigner {
            signer: &signer,
            path: DerivationPath::from_str("m/9'/1'/3'/2'/0").unwrap(),
            expected_key_hash160: hash,
        };
        assert!(adapter.can_sign_with(&key));

        let data = b"identity credit withdrawal signable bytes";
        let signature = adapter.sign(&key, data).await.expect("signs");
        assert_eq!(signature.len(), 65);
        // Same contract as `dashcore::signer::verify_hash_signature`: recover
        // from double-SHA256(data) and compare hash160s.
        let digest = sha256d::Hash::hash(data).to_byte_array();
        dashcore::signer::verify_hash_signature(&digest, signature.as_slice(), &hash)
            .expect("recoverable signature matches the identity key hash");
    }

    #[tokio::test]
    async fn refuses_keys_it_does_not_hold() {
        let (signer, hash, _) = fixture();
        let adapter = DerivedKeyIdentitySigner {
            signer: &signer,
            path: DerivationPath::from_str("m/9'/1'/3'/2'/0").unwrap(),
            expected_key_hash160: hash,
        };
        let other = IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: 7,
            purpose: Purpose::TRANSFER,
            security_level: SecurityLevel::CRITICAL,
            contract_bounds: None,
            key_type: KeyType::ECDSA_HASH160,
            read_only: false,
            data: BinaryData::new(vec![0x11u8; 20]),
            disabled_at: None,
        });
        assert!(!adapter.can_sign_with(&other));
        assert!(adapter.sign(&other, b"x").await.is_err());
    }

    #[tokio::test]
    async fn refuses_to_sign_when_the_derived_key_differs() {
        let (signer, _, key) = fixture();
        // Expect a different hash than the signer's key actually derives.
        let adapter = DerivedKeyIdentitySigner {
            signer: &signer,
            path: DerivationPath::from_str("m/9'/1'/3'/2'/0").unwrap(),
            expected_key_hash160: [0x99u8; 20],
        };
        // `key` carries the signer's real hash, so `matches` is false and
        // nothing is signed; rebuild a key that claims the wrong hash to
        // reach the post-derivation binding check.
        let claimed = IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: 0,
            purpose: Purpose::OWNER,
            security_level: SecurityLevel::CRITICAL,
            contract_bounds: None,
            key_type: KeyType::ECDSA_HASH160,
            read_only: false,
            data: BinaryData::new(vec![0x99u8; 20]),
            disabled_at: None,
        });
        let err = adapter.sign(&claimed, b"x").await.unwrap_err().to_string();
        assert!(err.contains("does not match"), "{err}");
        let _ = key;
    }

    #[test]
    fn owner_key_path_is_dip3_owner_base_plus_normal_index() {
        let mainnet = provider_owner_key_path(Network::Mainnet, 4).unwrap();
        assert_eq!(mainnet.to_string(), "m/9'/5'/3'/2'/4");
        let testnet = provider_owner_key_path(Network::Testnet, 0).unwrap();
        assert_eq!(testnet.to_string(), "m/9'/1'/3'/2'/0");
    }

    #[test]
    fn keys_flags() {
        let none = MasternodeWithdrawalKeys {
            owner_key: None,
            transfer_key: None,
            payout_address: None,
            payout_key_hash160: None,
        };
        assert!(!none.can_withdraw());
        assert!(!none.can_choose_destination());

        let owner_only = MasternodeWithdrawalKeys {
            owner_key: Some((0, DerivationPath::from_str("m/9'/1'/3'/2'/0").unwrap())),
            ..none.clone()
        };
        assert!(owner_only.can_withdraw());
        assert!(!owner_only.can_choose_destination());

        let transfer = MasternodeWithdrawalKeys {
            transfer_key: Some(DerivationPath::from_str("m/44'/1'/0'/0/3").unwrap()),
            ..none
        };
        assert!(transfer.can_withdraw());
        assert!(transfer.can_choose_destination());
    }
}
