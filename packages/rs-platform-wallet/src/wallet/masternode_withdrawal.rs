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
//! 2. [`PlatformWalletManager::masternode_withdraw_with`] (async) fetches
//!    the identity, picks its enabled identity key the chosen key source can
//!    sign with, refuses to broadcast if there is none, and signs with the
//!    wallet's key (resident, or derived from a [`MasterKeyProvider`]'s
//!    master key) or a host-held signer. [`PlatformWallet::masternode_withdraw`]
//!    does the same through a [`key_wallet::signer::Signer`] (on iOS the
//!    mnemonic-resolver-backed signer — the seed never becomes resident).

use std::fmt;

use async_trait::async_trait;
use dashcore::address::Payload;
use dashcore::crypto::ecdsa::{EcdsaPublicKey, EcdsaRecSignature, EcdsaSecretKey, EcdsaSignature};
use dashcore::hashes::{hash160, sha256d};
use dashcore::{Address as DashAddress, AddressType, Network, PubkeyHash, ScriptBuf};
use dpp::address_funds::AddressWitness;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::signer::Signer as IdentitySigner;
use dpp::identity::{Identity, IdentityPublicKey, KeyType, Purpose};
use dpp::platform_value::BinaryData;
use dpp::prelude::Identifier;
use dpp::ProtocolError;
use key_wallet::account::AccountType;
use key_wallet::bip32::{ChildNumber, DerivationPath};
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::signer::{Signer as CoreSigner, SignerMethod};

use dash_sdk::dapi_grpc::platform::v0::ResponseMetadata;
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
use dpp::state_transition::StateTransition;
use dpp::withdrawal::Pooling;
use zeroize::Zeroize;

use crate::broadcast_outcome::{broadcast_tracking_delivery, BroadcastDelivery};
use crate::changeset::PlatformWalletPersistence;
use crate::error::{PlatformWalletError, SIGNER_KEY_UNAVAILABLE_PREFIX};
use crate::manager::PlatformWalletManager;
use crate::masternode::key_reference::is_enabled_hash160_key;
use crate::wallet::core::is_signable_funding_account;
use crate::wallet::core_address_key::{CoreAddressKeyError, MasterKeyProvider};
use crate::wallet::identity::network::SignerRef;
use crate::wallet::identity::WithdrawalOutcome;
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
    /// `payout_path_hint` is where the caller knows the P2PKH payout key
    /// sits, for a payout address the funding pools never generated (such as
    /// a shallow restored pool). It is used only when the pool lookup misses
    /// and is reported as `transfer_key` UNVERIFIED: a withdrawal refuses it,
    /// before anything is signed, unless the key derived there hashes to the
    /// payout key hash.
    ///
    /// Blocking (takes the wallet-manager read lock) — call from a plain
    /// thread, not from inside the async runtime.
    pub fn masternode_withdrawal_keys(
        &self,
        owner_key_hash: Option<&[u8; 20]>,
        payout_script: Option<&[u8]>,
        owner_key_index_hint: Option<u32>,
        payout_path_hint: Option<&DerivationPath>,
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
        let transfer_key =
            transfer_key.or_else(|| payout_key_hash160.and(payout_path_hint.cloned()));

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
    /// remaining credit balance as proven by Platform, with the proving block.
    ///
    /// Guards (all fail WITHOUT broadcasting):
    /// - owner path with a destination, or a key the wallet doesn't hold;
    /// - the identity is missing, or doesn't carry the expected
    ///   `OWNER` / `TRANSFER` key for the wallet's hash160.
    ///
    /// Outcomes of the broadcast are kept distinct. A transition that never
    /// left this wallet (refused before anything was sent) is an ordinary
    /// error and may be retried. Once a dispatch may have reached a node,
    /// anything short of a proved result, an endpoint's refusal or a
    /// consensus verdict included, is
    /// [`PlatformWalletError::MasternodeWithdrawalUnconfirmed`] and MUST NOT
    /// be retried until the claimable balance is re-read: no proof backs an
    /// endpoint's answer, and the identity nonce was already taken for this
    /// attempt, so a blind retry would submit a second withdrawal with the
    /// next one.
    pub async fn masternode_withdraw<S: CoreSigner + ?Sized>(
        &self,
        request: MasternodeWithdrawalRequest,
        keys: &MasternodeWithdrawalKeys,
        signer: &S,
    ) -> Result<WithdrawalOutcome, PlatformWalletError> {
        let (path, key_hash160, destination) = self.withdrawal_key_choice(&request, keys)?;
        if !signer.supports(SignerMethod::Digest) {
            return Err(PlatformWalletError::InvalidParameter(format!(
                "signer backend cannot sign digests: it advertises {:?}, but an identity \
                 credit withdrawal requires {:?}",
                signer.supported_methods(),
                SignerMethod::Digest,
            )));
        }
        let identity_signer = DerivedKeyIdentitySigner {
            signer,
            path,
            expected_key_hash160: key_hash160,
        };
        let request = MasternodeWithdrawalRequest {
            destination,
            ..request
        };
        withdraw_with_identity_signer(self.sdk(), &request, &identity_signer).await
    }

    /// The wallet key that signs `request`: its path at `keys`, the hash160
    /// its public key must have, and the destination (a transfer-key
    /// withdrawal defaults to the registered payout address).
    fn withdrawal_key_choice(
        &self,
        request: &MasternodeWithdrawalRequest,
        keys: &MasternodeWithdrawalKeys,
    ) -> Result<(DerivationPath, [u8; 20], Option<DashAddress>), PlatformWalletError> {
        refuse_owner_destination(request)?;
        match request.signing_key {
            MasternodeWithdrawalKey::Owner => {
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
                Ok((path, owner_hash, None))
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
                let destination = match request.destination.clone() {
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
                Ok((path, payout_hash, Some(destination)))
            }
        }
    }
}

/// Where a masternode withdrawal's signing key comes from.
pub enum MasternodeKeySource<'a, K: MasterKeyProvider> {
    /// This wallet's key at the path `keys` resolved for the request's
    /// signing key, derived from the resident keys or from the master key
    /// `master_key` supplies. The derived key must hash to the key `keys`
    /// names, so a wrong payout path hint or another wallet's master key is
    /// refused before anything is signed.
    Wallet {
        wallet: &'a PlatformWallet,
        keys: &'a MasternodeWithdrawalKeys,
        master_key: &'a K,
    },
    /// A host-held key. It signs for the owner identity's enabled key of the
    /// request's purpose that it reports it can sign with; with none, the
    /// withdrawal is refused before anything is broadcast. Name `K` as
    /// [`std::convert::Infallible`].
    Host {
        signer: &'a dyn IdentitySigner<IdentityPublicKey>,
    },
}

/// Why [`PlatformWalletManager::masternode_withdraw_with`] failed.
#[derive(Debug, thiserror::Error)]
pub enum MasternodeWithdrawalError<E> {
    /// The withdrawal was refused or failed.
    #[error(transparent)]
    Wallet(#[from] PlatformWalletError),
    /// The master key provider failed to supply the master key.
    #[error("the key source failed: {0}")]
    KeySource(#[source] E),
}

impl<E> From<CoreAddressKeyError<E>> for MasternodeWithdrawalError<E> {
    fn from(error: CoreAddressKeyError<E>) -> Self {
        match error {
            CoreAddressKeyError::Wallet(error) => Self::Wallet(error),
            CoreAddressKeyError::KeySource(error) => Self::KeySource(error),
        }
    }
}

impl<P: PlatformWalletPersistence + 'static> PlatformWalletManager<P> {
    /// Submit `request` signed with the key `source` names, and return the
    /// identity's remaining balance as proven by Platform. Both sources go
    /// through the same checks; see [`PlatformWallet::masternode_withdraw`]
    /// for what is refused before broadcasting and when an outcome is
    /// unconfirmed.
    pub async fn masternode_withdraw_with<K: MasterKeyProvider>(
        &self,
        request: MasternodeWithdrawalRequest,
        source: MasternodeKeySource<'_, K>,
    ) -> Result<WithdrawalOutcome, MasternodeWithdrawalError<K::Error>> {
        match source {
            MasternodeKeySource::Host { signer } => {
                Ok(withdraw_with_identity_signer(self.sdk(), &request, signer).await?)
            }
            MasternodeKeySource::Wallet {
                wallet,
                keys,
                master_key,
            } => {
                let (path, key_hash160, destination) =
                    wallet.withdrawal_key_choice(&request, keys)?;
                let secret = wallet
                    .private_key_for_hash160(&path, &key_hash160, master_key)
                    .await?;
                let request = MasternodeWithdrawalRequest {
                    destination,
                    ..request
                };
                Ok(withdraw_with_secret(self.sdk(), &request, &secret, path).await?)
            }
        }
    }
}

/// An owner-key withdrawal pays the registered payout address; Platform
/// rejects one that carries an output script.
fn refuse_owner_destination(
    request: &MasternodeWithdrawalRequest,
) -> Result<(), PlatformWalletError> {
    if request.signing_key == MasternodeWithdrawalKey::Owner && request.destination.is_some() {
        return Err(PlatformWalletError::InvalidParameter(
            "an owner-key withdrawal pays the registered payout address; a destination cannot \
             be chosen"
                .to_string(),
        ));
    }
    Ok(())
}

/// The P2PKH address paying `hash`.
fn p2pkh_address(hash: &[u8; 20], network: Network) -> DashAddress {
    DashAddress::new(
        network,
        Payload::PubkeyHash(PubkeyHash::from_byte_array(*hash)),
    )
}

/// The owner identity's enabled key for `signing_key` (OWNER or TRANSFER
/// purpose) that `signer` can sign with. Refuses when there is none, so
/// nothing is signed or broadcast.
fn select_withdrawal_key<'a>(
    identity: &'a Identity,
    signing_key: MasternodeWithdrawalKey,
    signer: &dyn IdentitySigner<IdentityPublicKey>,
) -> Result<&'a IdentityPublicKey, PlatformWalletError> {
    let purpose = match signing_key {
        MasternodeWithdrawalKey::Owner => Purpose::OWNER,
        MasternodeWithdrawalKey::Transfer => Purpose::TRANSFER,
    };
    identity
        .public_keys()
        .values()
        .filter(|key| is_enabled_hash160_key(key, purpose))
        .find(|key| signer.can_sign_with(key))
        .ok_or_else(|| {
            PlatformWalletError::InvalidIdentityData(format!(
                "the masternode identity {} has no enabled {purpose:?} key this signer can sign \
                 with; not broadcasting",
                identity.id()
            ))
        })
}

/// [`withdraw_with_identity_signer`] with one secp256k1 `secret`: it signs
/// only for the identity key its public key hashes to. `path` names the key
/// in errors.
pub(crate) async fn withdraw_with_secret(
    sdk: &dash_sdk::Sdk,
    request: &MasternodeWithdrawalRequest,
    secret: &[u8; 32],
    path: DerivationPath,
) -> Result<WithdrawalOutcome, PlatformWalletError> {
    let core_signer = RawSecretCoreSigner::from_bytes(secret)?;
    let signer = DerivedKeyIdentitySigner {
        signer: &core_signer,
        path,
        expected_key_hash160: core_signer.public_key_hash160(),
    };
    withdraw_with_identity_signer(sdk, request, &signer).await
}

/// The network half of every masternode withdrawal: fetch the owner identity
/// (id = display-order proTxHash), select its enabled OWNER / TRANSFER key
/// that `signer` can sign with ([`select_withdrawal_key`]), sign, broadcast,
/// and wait for the proved balance. A transfer-key withdrawal without a
/// destination pays the selected key's P2PKH address. Error semantics: a
/// failure before the broadcast, or a broadcast refused before anything was
/// sent, stays retryable; every other outcome short of a proved result is
/// [`PlatformWalletError::MasternodeWithdrawalUnconfirmed`]
/// ([`settle_masternode_withdrawal`]).
async fn withdraw_with_identity_signer(
    sdk: &dash_sdk::Sdk,
    request: &MasternodeWithdrawalRequest,
    signer: &dyn IdentitySigner<IdentityPublicKey>,
) -> Result<WithdrawalOutcome, PlatformWalletError> {
    if request.amount_credits == 0 {
        return Err(PlatformWalletError::InvalidParameter(
            "masternode withdrawal amount must be greater than zero".to_string(),
        ));
    }
    refuse_owner_destination(request)?;

    // Owner identity id = display-order proTxHash.
    let mut id_bytes = request.pro_tx_hash;
    id_bytes.reverse();
    let identity_id = Identifier::from(id_bytes);

    let identity = Identity::fetch(sdk, identity_id)
        .await?
        .ok_or(PlatformWalletError::IdentityNotFound(identity_id))?;
    let identity_key = select_withdrawal_key(&identity, request.signing_key, signer)?.clone();
    let destination = match (request.signing_key, &request.destination) {
        (MasternodeWithdrawalKey::Transfer, None) => {
            let hash = <[u8; 20]>::try_from(identity_key.data().as_slice()).map_err(|_| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "the TRANSFER key of masternode identity {identity_id} is not a key hash"
                ))
            })?;
            Some(p2pkh_address(&hash, sdk.network))
        }
        (_, destination) => destination.clone(),
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
        request.amount_credits,
        Pooling::Never,
        MIN_CORE_FEE_PER_BYTE,
        0,
        SignerRef(signer),
        Some(&identity_key),
        PreferredKeyPurposeForSigningWithdrawal::TransferPreferred,
        nonce,
        sdk.version(),
        None,
    )
    .await
    .map_err(|e| definitive(dash_sdk::Error::Protocol(e)))?;

    broadcast_masternode_withdrawal(sdk, &state_transition, identity_id, request.amount_credits)
        .await
}

/// Broadcast the signed masternode withdrawal `state_transition`, keeping
/// what every dispatch proved ([`broadcast_tracking_delivery`]), then wait
/// for its result: split so an outcome that is not known stays typed
/// ([`settle_masternode_withdrawal`]).
async fn broadcast_masternode_withdrawal(
    sdk: &dash_sdk::Sdk,
    state_transition: &StateTransition,
    identity_id: Identifier,
    amount_credits: u64,
) -> Result<WithdrawalOutcome, PlatformWalletError> {
    let delivery = broadcast_tracking_delivery(sdk, state_transition).await;
    settle_masternode_withdrawal(identity_id, amount_credits, delivery, || {
        state_transition
            .wait_for_affected_state_with_metadata::<StateTransitionProofResult>(sdk, None)
    })
    .await
}

/// What a signed masternode withdrawal's broadcast (`delivery`) and result
/// wait (`wait`, run only when a dispatch may have reached a node) came to:
/// the identity's proved remaining balance with the block that proved it, or
/// the error the withdrawal reports.
///
/// Only a transition that never left the wallet
/// ([`BroadcastDelivery::NeverDelivered`]: refused before anything was sent)
/// is an ordinary, retryable error: nothing holds its bytes but the wallet,
/// so it can never execute. Once a dispatch may have reached a node, every
/// outcome short of the proved balance is
/// [`PlatformWalletError::MasternodeWithdrawalUnconfirmed`]: a wait that
/// fails, an endpoint's refusal of the broadcast and a consensus verdict on
/// either call included. The endpoint that sends a verdict is
/// unauthenticated, no proof or quorum signature backs it, and one that kept
/// or forwarded the signed transition can answer with any verdict, while a
/// retry signs a second withdrawal with the next identity nonce.
///
/// The withdrawal keeps no record of its signed transition: what settles an
/// unconfirmed one is the identity's nonce (once the nonce it signed with
/// reads as used, it executed or can no longer execute) and the claimable
/// balance, which the host re-reads before it submits again.
async fn settle_masternode_withdrawal<W, Fut>(
    identity_id: Identifier,
    amount_credits: u64,
    delivery: BroadcastDelivery,
    wait: W,
) -> Result<WithdrawalOutcome, PlatformWalletError>
where
    W: FnOnce() -> Fut,
    Fut: std::future::Future<
        Output = Result<(StateTransitionProofResult, ResponseMetadata), dash_sdk::Error>,
    >,
{
    // What the broadcast was answered with, kept for the reason an
    // unconfirmed withdrawal reports.
    let broadcast_answer = match delivery {
        BroadcastDelivery::Admitted => None,
        BroadcastDelivery::NeverDelivered(e) => {
            return Err(PlatformWalletError::InvalidIdentityData(format!(
                "masternode withdrawal failed: {e}"
            )));
        }
        BroadcastDelivery::MaybeDelivered(e) => {
            tracing::warn!(
                identity = %identity_id,
                error = %e,
                "masternode withdrawal broadcast may have been delivered; falling through \
                 to the result wait"
            );
            Some(e.to_string())
        }
    };
    let unconfirmed = |waited: String| PlatformWalletError::MasternodeWithdrawalUnconfirmed {
        identity_id,
        amount_credits,
        reason: match &broadcast_answer {
            Some(answer) => format!("{waited}; its broadcast answered: {answer}"),
            None => waited,
        },
    };

    match wait().await {
        Ok((StateTransitionProofResult::VerifiedPartialIdentity(partial), metadata)) => partial
            .balance
            .map(|balance| WithdrawalOutcome::proved(balance, metadata))
            .ok_or_else(|| unconfirmed("the result proof carried no identity balance".to_string())),
        // Verified, but not the identity balance a withdrawal's result
        // carries: the remaining balance is not proved.
        Ok(_) => Err(unconfirmed(
            "the result proof did not carry the identity's balance".to_string(),
        )),
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
    secret: EcdsaSecretKey,
}

impl RawSecretCoreSigner {
    /// `secret` must be a valid secp256k1 scalar (32 bytes).
    pub fn from_bytes(secret: &[u8; 32]) -> Result<Self, PlatformWalletError> {
        let secret = EcdsaSecretKey::from_bytes(secret).map_err(|_| {
            PlatformWalletError::InvalidParameter("not a valid secp256k1 private key".to_string())
        })?;
        Ok(Self { secret })
    }

    /// hash160 of this key's compressed public key.
    pub fn public_key_hash160(&self) -> [u8; 20] {
        let public = self.secret.public_key();
        hash160::Hash::hash(&public.to_compressed()).to_byte_array()
    }
}

impl Drop for RawSecretCoreSigner {
    fn drop(&mut self) {
        self.secret.zeroize();
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
    ) -> Result<(EcdsaSignature, EcdsaPublicKey), Self::Error> {
        // Recoverable signing preserves RFC6979 bytes without low-R grinding.
        Ok((
            self.secret
                .sign_recoverable(&sighash)
                .signature()
                .map_err(|e| e.to_string())?,
            self.secret.public_key(),
        ))
    }

    async fn public_key(&self, _path: &DerivationPath) -> Result<EcdsaPublicKey, Self::Error> {
        Ok(self.secret.public_key())
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
        let derived_hash: [u8; 20] =
            hash160::Hash::hash(&public_key.to_compressed()).to_byte_array();
        if derived_hash != self.expected_key_hash160 {
            return Err(ProtocolError::Generic(format!(
                "the key derived at {} does not match the masternode identity key (hash160 {} \
                 vs {}); refusing to sign",
                self.path,
                hex::encode(derived_hash),
                hex::encode(self.expected_key_hash160)
            )));
        }

        let recoverable = (0..4i32)
            .filter_map(|id| EcdsaRecSignature::from_parts(signature, id as u8, true).ok())
            .find(|candidate| {
                EcdsaPublicKey::recover(&digest, candidate)
                    .is_ok_and(|recovered| recovered == public_key)
            })
            .ok_or_else(|| {
                ProtocolError::Generic(
                    "no recovery id in 0..=3 recovers the withdrawal signing key".to_string(),
                )
            })?;

        Ok(recoverable.as_bytes().to_vec().into())
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
    use crate::broadcast_outcome::test_sdks::{sdk_refusing_connections, sdk_without_addresses};
    use crate::masternode::key_reference::test_support::{identity_with, key};
    use crate::test_support::{
        mnemonic_wallet_manager, platform_wallet_in, test_platform_wallet_manager,
        TEST_PLATFORM_WALLET_MNEMONIC,
    };
    use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};
    use crate::BlockTime;
    use dash_sdk::dapi_client::transport::TransportError;
    use dash_sdk::dapi_client::DapiClientError;
    use dash_sdk::dapi_grpc::tonic::{Code, Status};
    use dash_sdk::error::StateTransitionBroadcastError;
    use dashcore::address::Payload;
    use dashcore::crypto::ecdsa::{EcdsaPublicKey as PublicKey, EcdsaSecretKey as SecretKey};
    use dashcore::PubkeyHash;
    use dpp::consensus::basic::{BasicError, UnsupportedFeatureError};
    use dpp::consensus::ConsensusError;
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::{PartialIdentity, Purpose, SecurityLevel};
    use dpp::state_transition::identity_credit_withdrawal_transition::v1::IdentityCreditWithdrawalTransitionV1;
    use key_wallet::bip32::ExtendedPrivKey;
    use key_wallet::Mnemonic;
    use key_wallet_manager::WalletManager;
    use std::cell::Cell;
    use std::convert::Infallible;
    use std::str::FromStr;
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
    use std::sync::Arc;
    use tokio::runtime::Runtime;

    /// The identity the withdrawal tests withdraw from, and the amount.
    const WITHDRAWN_FROM: [u8; 32] = [0x31; 32];
    const WITHDRAWN: u64 = 1_000_000;

    /// A node's refusal of a dispatch, as an offline or overloaded endpoint
    /// answers one.
    fn unavailable() -> dash_sdk::Error {
        dash_sdk::Error::DapiClientError(DapiClientError::Transport(TransportError::Grpc(
            Status::new(Code::Unavailable, "test"),
        )))
    }

    fn verdict() -> ConsensusError {
        ConsensusError::BasicError(BasicError::UnsupportedFeatureError(
            UnsupportedFeatureError::new("test".to_string(), 1),
        ))
    }

    /// A consensus verdict as a broadcast returns one.
    fn broadcast_verdict() -> dash_sdk::Error {
        dash_sdk::Error::Protocol(ProtocolError::ConsensusError(Box::new(verdict())))
    }

    /// A consensus verdict as the result wait returns one.
    fn wait_verdict() -> dash_sdk::Error {
        dash_sdk::Error::StateTransitionBroadcastError(StateTransitionBroadcastError {
            code: 1,
            message: "rejected".to_string(),
            cause: Some(verdict()),
        })
    }

    fn wait_timeout() -> dash_sdk::Error {
        dash_sdk::Error::TimeoutReached(std::time::Duration::from_secs(80), "waiting".to_string())
    }

    /// The block a result proof is answered at.
    fn proving_block() -> ResponseMetadata {
        ResponseMetadata {
            height: 1234,
            core_chain_locked_height: 42,
            time_ms: 1_700_000_000_000,
            ..Default::default()
        }
    }

    fn proved_balance(balance: Option<u64>) -> (StateTransitionProofResult, ResponseMetadata) {
        let result = StateTransitionProofResult::VerifiedPartialIdentity(PartialIdentity {
            id: Identifier::from(WITHDRAWN_FROM),
            loaded_public_keys: Default::default(),
            balance,
            revision: None,
            not_found_public_keys: Default::default(),
        });
        (result, proving_block())
    }

    fn is_unconfirmed(settled: &Result<WithdrawalOutcome, PlatformWalletError>) -> bool {
        matches!(
            settled,
            Err(PlatformWalletError::MasternodeWithdrawalUnconfirmed {
                identity_id,
                amount_credits: WITHDRAWN,
                ..
            }) if *identity_id == Identifier::from(WITHDRAWN_FROM)
        )
    }

    /// Once a dispatch may have reached a node, a withdrawal whose result is
    /// not proved is unconfirmed, never failed: a node's refusal of the
    /// broadcast and a consensus verdict on either call included, as no proof
    /// backs an endpoint's answer and a retry signs a second withdrawal. The
    /// reason keeps what the wait and the broadcast were answered with.
    #[tokio::test]
    async fn should_report_a_masternode_withdrawal_unconfirmed_once_a_dispatch_may_have_reached_a_node(
    ) {
        let cases = [
            (
                "a node refused the broadcast, the wait timed out",
                BroadcastDelivery::MaybeDelivered(unavailable()),
                wait_timeout(),
            ),
            (
                "the broadcast was answered with a consensus verdict",
                BroadcastDelivery::MaybeDelivered(broadcast_verdict()),
                wait_timeout(),
            ),
            (
                "admitted, the wait was answered with a consensus verdict",
                BroadcastDelivery::Admitted,
                wait_verdict(),
            ),
            (
                "admitted, the wait timed out",
                BroadcastDelivery::Admitted,
                wait_timeout(),
            ),
        ];
        for (case, delivery, wait_error) in cases {
            let broadcast_answer = match &delivery {
                BroadcastDelivery::MaybeDelivered(e) => Some(e.to_string()),
                _ => None,
            };
            let waited = wait_error.to_string();
            let settled = settle_masternode_withdrawal(
                Identifier::from(WITHDRAWN_FROM),
                WITHDRAWN,
                delivery,
                || std::future::ready(Err(wait_error)),
            )
            .await;
            let Err(PlatformWalletError::MasternodeWithdrawalUnconfirmed { reason, .. }) = settled
            else {
                panic!("{case}: {settled:?}");
            };
            assert!(reason.contains(&waited), "{case}: {reason}");
            if let Some(answer) = broadcast_answer {
                assert!(reason.contains(&answer), "{case}: {reason}");
            }
        }
    }

    /// Only a withdrawal whose transition never left the wallet is an
    /// ordinary, retryable failure, and nothing waits for its result.
    #[tokio::test]
    async fn should_report_a_masternode_withdrawal_failed_only_when_nothing_was_sent() {
        let waited = Cell::new(false);
        let settled = settle_masternode_withdrawal(
            Identifier::from(WITHDRAWN_FROM),
            WITHDRAWN,
            BroadcastDelivery::NeverDelivered(dash_sdk::Error::DapiClientError(
                DapiClientError::NoAvailableAddresses,
            )),
            || {
                waited.set(true);
                std::future::ready(Err(wait_timeout()))
            },
        )
        .await;
        assert!(
            matches!(settled, Err(PlatformWalletError::InvalidIdentityData(_))),
            "{settled:?}"
        );
        assert!(!waited.get(), "nothing was sent, so nothing is waited for");
    }

    /// A proved result returns the identity's remaining balance and the block
    /// that proved it, whatever the broadcast answered; a proof that carries
    /// no balance leaves the withdrawal unconfirmed.
    #[tokio::test]
    async fn should_return_a_masternode_withdrawal_s_proved_balance() {
        for delivery in [
            BroadcastDelivery::Admitted,
            BroadcastDelivery::MaybeDelivered(unavailable()),
        ] {
            let settled = settle_masternode_withdrawal(
                Identifier::from(WITHDRAWN_FROM),
                WITHDRAWN,
                delivery,
                || std::future::ready(Ok(proved_balance(Some(7)))),
            )
            .await;
            assert_eq!(
                settled.ok(),
                Some(WithdrawalOutcome {
                    remaining_credits: 7,
                    proved_at: BlockTime::new(1234, 42, 1_700_000_000_000),
                })
            );
        }
        let settled = settle_masternode_withdrawal(
            Identifier::from(WITHDRAWN_FROM),
            WITHDRAWN,
            BroadcastDelivery::Admitted,
            || std::future::ready(Ok(proved_balance(None))),
        )
        .await;
        assert!(is_unconfirmed(&settled), "{settled:?}");
    }

    /// The broadcast of a signed withdrawal through a network SDK: a node that
    /// refuses the connection leaves it unconfirmed, as a transport failure
    /// cannot show that nothing was sent, and an SDK with no address to send
    /// to leaves it failed.
    #[tokio::test]
    async fn should_report_a_masternode_withdrawal_unconfirmed_when_its_node_refuses_the_connection(
    ) {
        let transition: StateTransition = IdentityCreditWithdrawalTransitionV1 {
            identity_id: Identifier::from(WITHDRAWN_FROM),
            amount: WITHDRAWN,
            nonce: 4,
            ..Default::default()
        }
        .into();

        let (refusing, _port) = sdk_refusing_connections();
        let refused = broadcast_masternode_withdrawal(
            &refusing,
            &transition,
            Identifier::from(WITHDRAWN_FROM),
            WITHDRAWN,
        )
        .await;
        assert!(is_unconfirmed(&refused), "{refused:?}");

        let never_sent = broadcast_masternode_withdrawal(
            &sdk_without_addresses(),
            &transition,
            Identifier::from(WITHDRAWN_FROM),
            WITHDRAWN,
        )
        .await;
        assert!(
            matches!(never_sent, Err(PlatformWalletError::InvalidIdentityData(_))),
            "{never_sent:?}"
        );
    }

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
        ) -> Result<(EcdsaSignature, PublicKey), Self::Error> {
            Ok((
                self.secret
                    .sign_recoverable(&sighash)
                    .signature()
                    .map_err(|e| e.to_string())?,
                self.secret.public_key(),
            ))
        }

        async fn public_key(&self, _path: &DerivationPath) -> Result<PublicKey, Self::Error> {
            Ok(self.secret.public_key())
        }
    }

    fn fixture() -> (FixedKeySigner, [u8; 20], IdentityPublicKey) {
        let secret = SecretKey::from_bytes(&[0x42u8; 32]).expect("valid scalar");
        let pubkey = secret.public_key();
        let hash: [u8; 20] = hash160::Hash::hash(&pubkey.to_compressed()).to_byte_array();
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

    // --- key sources -------------------------------------------------------

    #[derive(Debug, thiserror::Error)]
    #[error("the keychain is locked")]
    struct KeychainLocked;

    /// Supplies `master` (or fails without one), counts its calls, and on each
    /// call records whether `manager` could be write-locked, i.e. whether the
    /// caller held no wallet-manager guard across the call.
    struct TestMasterKey {
        master: Option<ExtendedPrivKey>,
        manager: Option<Arc<tokio::sync::RwLock<WalletManager<PlatformWalletInfo>>>>,
        calls: AtomicU32,
        manager_was_free: AtomicBool,
    }

    impl TestMasterKey {
        fn new(master: Option<ExtendedPrivKey>) -> Self {
            Self {
                master,
                manager: None,
                calls: AtomicU32::new(0),
                manager_was_free: AtomicBool::new(false),
            }
        }

        fn calls(&self) -> u32 {
            self.calls.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl MasterKeyProvider for TestMasterKey {
        type Error = KeychainLocked;

        async fn master_key(
            &self,
            _wallet_id: &WalletId,
        ) -> Result<ExtendedPrivKey, KeychainLocked> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if let Some(manager) = &self.manager {
                self.manager_was_free
                    .store(manager.try_write().is_ok(), Ordering::SeqCst);
            }
            self.master.clone().ok_or(KeychainLocked)
        }
    }

    fn test_master() -> ExtendedPrivKey {
        let seed = Mnemonic::from_phrase(TEST_PLATFORM_WALLET_MNEMONIC)
            .unwrap()
            .to_seed("");
        ExtendedPrivKey::new_master(Network::Testnet, seed.as_ref()).unwrap()
    }

    /// Receive key `index` of the test wallet's BIP-44 account 0: its path,
    /// its P2PKH script and the pubkey hash in it.
    fn receive_key(index: u32) -> (DerivationPath, Vec<u8>, [u8; 20]) {
        let path = DerivationPath::from_str(&format!("m/44'/1'/0'/0/{index}")).unwrap();
        let public = test_master()
            .derive_priv(&path)
            .unwrap()
            .private_key
            .public_key();
        let hash: [u8; 20] = hash160::Hash::hash(&public.to_compressed()).to_byte_array();
        (path, p2pkh(&hash).script_pubkey().to_bytes(), hash)
    }

    fn p2pkh(hash: &[u8; 20]) -> DashAddress {
        DashAddress::new(
            Network::Testnet,
            Payload::PubkeyHash(PubkeyHash::from_byte_array(*hash)),
        )
    }

    fn hash_of(secret: &[u8; 32]) -> [u8; 20] {
        let public = SecretKey::from_bytes(secret).unwrap().public_key();
        hash160::Hash::hash(&public.to_compressed()).to_byte_array()
    }

    fn transfer_request() -> MasternodeWithdrawalRequest {
        MasternodeWithdrawalRequest {
            pro_tx_hash: WITHDRAWN_FROM,
            owner_key_hash: None,
            amount_credits: WITHDRAWN,
            signing_key: MasternodeWithdrawalKey::Transfer,
            destination: None,
        }
    }

    /// A wallet with resident keys signs with them; the provider is not asked.
    #[test]
    fn should_derive_a_resident_wallet_key_without_the_master_key_provider() {
        let runtime = Runtime::new().unwrap();
        let (wallet_manager, wallet_id, _, _) =
            runtime.block_on(mnemonic_wallet_manager(TEST_PLATFORM_WALLET_MNEMONIC));
        let wallet = runtime.block_on(platform_wallet_in(wallet_manager, wallet_id));
        let (path, script, hash) = receive_key(0);
        let keys = wallet
            .masternode_withdrawal_keys(None, Some(&script), None, None)
            .unwrap();
        assert_eq!(keys.transfer_key, Some(path.clone()));
        let provider = TestMasterKey::new(None);

        let secret = runtime
            .block_on(wallet.private_key_for_hash160(&path, &hash, &provider))
            .unwrap();

        assert_eq!(hash_of(&secret), hash);
        assert_eq!(provider.calls(), 0);
    }

    /// A wallet without resident keys asks the provider once, and holds no
    /// wallet-manager guard while it runs: a provider may block on a prompt.
    #[test]
    fn should_ask_the_provider_once_with_no_wallet_manager_guard_held() {
        let runtime = Runtime::new().unwrap();
        let (manager, wallet_id) = runtime.block_on(test_platform_wallet_manager());
        let wallet = runtime.block_on(manager.get_wallet(&wallet_id)).unwrap();
        let (path, _, hash) = receive_key(0);
        let provider = TestMasterKey {
            manager: Some(Arc::clone(wallet.wallet_manager())),
            ..TestMasterKey::new(Some(test_master()))
        };

        let secret = runtime
            .block_on(wallet.private_key_for_hash160(&path, &hash, &provider))
            .unwrap();

        assert_eq!(hash_of(&secret), hash);
        assert_eq!(provider.calls(), 1);
        assert!(provider.manager_was_free.load(Ordering::SeqCst));
    }

    /// The provider's failure is the key source's, and stops the withdrawal
    /// before any network call.
    #[test]
    fn should_report_a_failed_master_key_provider_as_the_key_source() {
        let runtime = Runtime::new().unwrap();
        let (manager, wallet_id) = runtime.block_on(test_platform_wallet_manager());
        let wallet = runtime.block_on(manager.get_wallet(&wallet_id)).unwrap();
        let (_, script, _) = receive_key(0);
        let keys = wallet
            .masternode_withdrawal_keys(None, Some(&script), None, None)
            .unwrap();
        let provider = TestMasterKey::new(None);

        let result = runtime.block_on(manager.masternode_withdraw_with(
            transfer_request(),
            MasternodeKeySource::Wallet {
                wallet: &wallet,
                keys: &keys,
                master_key: &provider,
            },
        ));

        assert!(
            matches!(
                result,
                Err(MasternodeWithdrawalError::KeySource(KeychainLocked))
            ),
            "{result:?}"
        );
    }

    /// Another wallet's master key derives a valid key at the same path, but
    /// not this wallet's: refused before any network call.
    #[test]
    fn should_refuse_the_master_key_of_another_wallet() {
        let runtime = Runtime::new().unwrap();
        let (manager, wallet_id) = runtime.block_on(test_platform_wallet_manager());
        let wallet = runtime.block_on(manager.get_wallet(&wallet_id)).unwrap();
        let (_, script, _) = receive_key(0);
        let keys = wallet
            .masternode_withdrawal_keys(None, Some(&script), None, None)
            .unwrap();
        let other = ExtendedPrivKey::new_master(Network::Testnet, &[7u8; 64]).unwrap();
        let provider = TestMasterKey::new(Some(other));

        let result = runtime.block_on(manager.masternode_withdraw_with(
            transfer_request(),
            MasternodeKeySource::Wallet {
                wallet: &wallet,
                keys: &keys,
                master_key: &provider,
            },
        ));

        assert!(
            matches!(
                result,
                Err(MasternodeWithdrawalError::Wallet(
                    PlatformWalletError::KeyDerivation(_)
                ))
            ),
            "{result:?}"
        );
    }

    /// A payout key the funding pools never generated resolves from the
    /// caller's path hint, and signs once the derived key proves it.
    #[test]
    fn should_resolve_a_payout_key_outside_the_pools_from_its_path_hint() {
        let runtime = Runtime::new().unwrap();
        let (manager, wallet_id) = runtime.block_on(test_platform_wallet_manager());
        let wallet = runtime.block_on(manager.get_wallet(&wallet_id)).unwrap();
        let (path, script, hash) = receive_key(400);

        let unhinted = wallet
            .masternode_withdrawal_keys(None, Some(&script), None, None)
            .unwrap();
        let hinted = wallet
            .masternode_withdrawal_keys(None, Some(&script), None, Some(&path))
            .unwrap();
        assert_eq!(unhinted.transfer_key, None);
        assert_eq!(hinted.transfer_key, Some(path.clone()));
        assert_eq!(hinted.payout_key_hash160, Some(hash));

        let provider = TestMasterKey::new(Some(test_master()));
        let secret = runtime
            .block_on(wallet.private_key_for_hash160(&path, &hash, &provider))
            .unwrap();
        assert_eq!(hash_of(&secret), hash);
    }

    /// A hint naming another key is reported as the transfer key, but the key
    /// derived there does not prove the payout hash: refused before any
    /// network call.
    #[test]
    fn should_refuse_a_payout_path_hint_that_names_another_key() {
        let runtime = Runtime::new().unwrap();
        let (manager, wallet_id) = runtime.block_on(test_platform_wallet_manager());
        let wallet = runtime.block_on(manager.get_wallet(&wallet_id)).unwrap();
        let (_, script, _) = receive_key(400);
        let (wrong_path, _, _) = receive_key(401);
        let keys = wallet
            .masternode_withdrawal_keys(None, Some(&script), None, Some(&wrong_path))
            .unwrap();
        let provider = TestMasterKey::new(Some(test_master()));

        let result = runtime.block_on(manager.masternode_withdraw_with(
            transfer_request(),
            MasternodeKeySource::Wallet {
                wallet: &wallet,
                keys: &keys,
                master_key: &provider,
            },
        ));

        assert!(
            matches!(
                result,
                Err(MasternodeWithdrawalError::Wallet(
                    PlatformWalletError::KeyDerivation(_)
                ))
            ),
            "{result:?}"
        );
    }

    /// The owner key pays the registered payout address only, whatever signs.
    #[test]
    fn should_refuse_an_owner_withdrawal_with_a_destination_from_a_host_signer() {
        let runtime = Runtime::new().unwrap();
        let (manager, _) = runtime.block_on(test_platform_wallet_manager());
        let (signer, hash, _) = fixture();
        let adapter = DerivedKeyIdentitySigner {
            signer: &signer,
            path: DerivationPath::from_str("m/9'/1'/3'/2'/0").unwrap(),
            expected_key_hash160: hash,
        };
        let request = MasternodeWithdrawalRequest {
            owner_key_hash: Some(hash),
            signing_key: MasternodeWithdrawalKey::Owner,
            destination: Some(p2pkh(&hash)),
            ..transfer_request()
        };

        let result = runtime.block_on(manager.masternode_withdraw_with(
            request,
            MasternodeKeySource::<Infallible>::Host { signer: &adapter },
        ));

        assert!(
            matches!(
                result,
                Err(MasternodeWithdrawalError::Wallet(
                    PlatformWalletError::InvalidParameter(_)
                ))
            ),
            "{result:?}"
        );
    }

    /// Only an enabled key of the role's purpose that the signer can sign
    /// with is selected; otherwise nothing is signed.
    #[test]
    fn should_select_only_an_enabled_key_the_signer_can_sign_with() {
        let (signer, hash, _) = fixture();
        let adapter = DerivedKeyIdentitySigner {
            signer: &signer,
            path: DerivationPath::from_str("m/9'/1'/3'/2'/0").unwrap(),
            expected_key_hash160: hash,
        };
        let id = Identifier::from(WITHDRAWN_FROM);
        let selected = |keys, signing_key| {
            select_withdrawal_key(&identity_with(id, keys), signing_key, &adapter)
                .map(|key| key.id())
        };
        let transfer = |key_id, data, disabled| {
            key(
                key_id,
                Purpose::TRANSFER,
                KeyType::ECDSA_HASH160,
                data,
                disabled,
            )
        };

        // The signer's key, disabled, and an enabled one it cannot sign.
        assert!(matches!(
            selected(
                vec![transfer(0, hash, true), transfer(1, [0xEE; 20], false)],
                MasternodeWithdrawalKey::Transfer
            ),
            Err(PlatformWalletError::InvalidIdentityData(_))
        ));
        // The signer's key, enabled, after a disabled copy.
        assert_eq!(
            selected(
                vec![transfer(0, hash, true), transfer(1, hash, false)],
                MasternodeWithdrawalKey::Transfer
            )
            .ok(),
            Some(1)
        );
        // An owner withdrawal needs the OWNER key, not a TRANSFER one.
        assert!(matches!(
            selected(
                vec![transfer(0, hash, false)],
                MasternodeWithdrawalKey::Owner
            ),
            Err(PlatformWalletError::InvalidIdentityData(_))
        ));
        assert_eq!(
            selected(
                vec![
                    transfer(0, [0xEE; 20], false),
                    key(1, Purpose::OWNER, KeyType::ECDSA_HASH160, hash, false),
                ],
                MasternodeWithdrawalKey::Owner
            )
            .ok(),
            Some(1)
        );
    }
}
