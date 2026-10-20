//! ProUpServTx (provider update service) orchestration.
//!
//! Revives a PoSe-banned masternode or evonode by re-asserting its current
//! service values in a ProUpServTx signed with the operator BLS key. The
//! payload commits to the funding inputs (`inputs_hash`) and each funding
//! input's ECDSA sighash covers the finished payload, so the build order is
//! fixed: select and reserve inputs → write `inputs_hash` → BLS-sign the
//! payload → ECDSA-sign the inputs → broadcast. key-wallet's
//! `TransactionBuilder::set_payload_finalizer` is the seam that makes steps
//! two and three possible between selection and input signing.
//!
//! This is deliberately revive-only: every payload field except the payout
//! script re-asserts the live masternode-list entry, and the payout script
//! follows a hard rule (see [`resolve_operator_payout_script`]) so an unban
//! can never silently clear an operator payout on-chain. The payload version
//! follows Dash Core v24's rules (see [`prepare_update_service_placeholder`]).

use dashcore::block::Version as BlockVersion;
use dashcore::blockdata::script::ScriptBuf;
use dashcore::blockdata::transaction::special_transaction::provider_registration::ProviderMasternodeType;
use dashcore::blockdata::transaction::special_transaction::provider_update_service::{
    ProTxVersion, ProviderUpdateServicePayload,
};
use dashcore::blockdata::transaction::special_transaction::{
    SpecialTransactionBasePayloadEncodable, TransactionPayload,
};
use dashcore::bls_sig_utils::BLSSignature;
use dashcore::blsful::{Bls12381G2Impl, SecretKey as BlsSecretKey, SignatureSchemes};
use dashcore::hash_types::{InputsHash, SpecialTransactionPayloadHash};
use dashcore::hashes::Hash;
use dashcore::sml::masternode_list_entry::net_info::{
    Bip155Network, ExtNetInfo, NetInfoEntry, NetInfoPurpose,
};
use dashcore::sml::masternode_list_entry::{
    EntryMasternodeType, MasternodeListEntry, MasternodeNetInfo,
};
use dashcore::{Address as DashAddress, Block, BlockHash, Network, Transaction, Txid};
use key_wallet::wallet::managed_wallet_info::transaction_builder::{
    BuilderError, TransactionBuilder, TransactionSigner,
};
use std::fmt::Display;
use std::future::Future;
use std::net::{IpAddr, SocketAddr};
use std::ops::Range;
use zeroize::Zeroizing;

use super::locator::bls_public_keys;
use crate::broadcaster::TransactionBroadcaster;
use crate::error::PlatformWalletError;
use crate::spv::{SpvRuntime, SpvTip};
use crate::util::now_secs;
use crate::wallet::asset_lock::sync::locate::BLOCK_REQUEST_SETTINGS;
use crate::wallet::core::{CoreWallet, SignedCoreTransaction, SEND_FUNDING_SOURCES};
use crate::wallet::platform_wallet::PlatformWallet;

/// What an update-service (unban) request lets the caller choose. Everything
/// else — service address, masternode type, platform node id and HTTP port —
/// is taken from the live masternode-list entry.
#[derive(Debug, Clone)]
pub struct MasternodeUpdateServiceParams {
    /// ProRegTx hash of the masternode to update, in WIRE order (the same
    /// order `MasternodeListSummary::pro_tx_hash` uses).
    pub pro_tx_hash: [u8; 32],
    /// Platform P2P port for an evonode payload; `None` for a regular
    /// masternode. Pass it for every evonode: a pre-v3 list entry does not
    /// carry it, so it is required there, while a v3 entry's address map is
    /// re-asserted as is, so there it is optional and must match a Platform
    /// P2P port the entry lists. Once V24 is active, a pre-v3 evonode's
    /// update writes this port into its new v3 address map, after which
    /// this wallet re-asserts it and cannot change it; on mainnet that
    /// update takes only port 26656.
    pub platform_p2p_port: Option<u16>,
    /// Operator payout address. Consensus REPLACES the current operator
    /// payout script with this payload's, and an empty script clears it —
    /// see [`resolve_operator_payout_script`] for the rule that keeps that
    /// from happening silently.
    pub operator_payout_address: Option<String>,
}

/// Build, operator-BLS-sign, fund, input-sign, and broadcast a ProUpServTx
/// that re-asserts `pro_tx_hash`'s current service values — the transaction
/// Core produces for `protx update_service` — which revives the masternode
/// if it is PoSe-banned.
///
/// `operator_secret` is the operator's BLS12-381 secret scalar in big-endian
/// bytes (the same convention as [`bls_public_keys`]); it is verified against
/// the masternode-list entry's operator public key (both basic and legacy
/// serializations) before any network work. `signer` signs the wallet's
/// funding inputs and never sees the operator key.
///
/// The fee is paid from `wallet`'s core funds ([`SEND_FUNDING_SOURCES`]).
/// On a definitively rejected broadcast the reserved inputs are released;
/// on an ambiguous outcome the error is
/// [`PlatformWalletError::TransactionBroadcastUnconfirmed`] and the inputs
/// stay reserved for the wallet's normal reconciliation.
pub async fn execute_masternode_update_service<S: TransactionSigner + ?Sized + Sync>(
    wallet: &PlatformWallet,
    spv: &SpvRuntime,
    params: MasternodeUpdateServiceParams,
    operator_secret: Zeroizing<[u8; 32]>,
    signer: &S,
) -> Result<Txid, PlatformWalletError> {
    let signed =
        prepare_masternode_update_service(wallet, spv, params, operator_secret, signer).await?;
    wallet.core().broadcast_finalized_transaction(&signed).await
}

/// Everything [`execute_masternode_update_service`] does except the
/// broadcast: the same preflights, payload, funding, operator-BLS signature
/// and input signatures, stopping at a fully signed transaction whose inputs
/// stay reserved.
///
/// For hosts that show the ProUpServTx before sending it. The returned
/// transaction is exactly what a broadcast would put on the network, so a
/// preview built from it cannot drift from what is sent. The caller then
/// either broadcasts it (`CoreWallet::broadcast_finalized_transaction`) or
/// abandons it (`CoreWallet::abandon_transaction`) — dropping it without
/// either strands the reservation until the TTL backstop reclaims it.
///
/// Broadcast it promptly: its payload version is chosen for the next few
/// blocks (see [`V24_ACTIVATION_MARGIN_BLOCKS`]), and an evonode's version-2
/// payload held until Dash Core's V24 fork activates is refused by the
/// network.
pub async fn prepare_masternode_update_service<S: TransactionSigner + ?Sized + Sync>(
    wallet: &PlatformWallet,
    spv: &SpvRuntime,
    params: MasternodeUpdateServiceParams,
    operator_secret: Zeroizing<[u8; 32]>,
    signer: &S,
) -> Result<SignedCoreTransaction, PlatformWalletError> {
    let entries = spv
        .masternode_list_entries()
        .await
        .ok_or(PlatformWalletError::MasternodeListUnavailable)?;
    let (entry, list_shows_v24) = update_service_target(&entries, &params.pro_tx_hash)?;

    verify_operator_secret(entry.operator_public_key.as_ref(), &operator_secret)?;

    let operator_reward = fetch_operator_reward(wallet, &params.pro_tx_hash).await?;
    let script_payout = resolve_operator_payout_script(
        operator_reward,
        params.operator_payout_address.as_deref(),
        wallet.network(),
    )?;

    let placeholder = update_service_placeholder(
        list_shows_v24,
        |v24_active| {
            prepare_update_service_placeholder(
                entry,
                wallet.network(),
                v24_active,
                params.platform_p2p_port,
                script_payout.clone(),
            )
        },
        v24_active_at_spv_tip(wallet, spv),
    )
    .await?;

    build_sign_update_service(wallet.core(), placeholder, operator_secret, signer).await
}

/// The list entry for `pro_tx_hash` (wire order) and whether the list shows
/// that the V24 hard fork is active. Version-3 entries exist only once V24
/// is active, so any one of them proves it. Their absence proves nothing:
/// Core moves an entry to version 3 only when a version-3 provider
/// transaction for it is mined, so right after activation every entry is
/// still legacy (see [`update_service_placeholder`]).
pub(crate) fn update_service_target<'a>(
    entries: &'a [MasternodeListEntry],
    pro_tx_hash: &[u8; 32],
) -> Result<(&'a MasternodeListEntry, bool), PlatformWalletError> {
    let entry = entries
        .iter()
        .find(|entry| entry.pro_reg_tx_hash.as_byte_array() == pro_tx_hash)
        .ok_or_else(|| {
            PlatformWalletError::InvalidParameter(format!(
                "masternode {} is not in the masternode list",
                display_hex(pro_tx_hash)
            ))
        })?;
    let v24_active = entries
        .iter()
        .any(|entry| entry.version >= ProTxVersion::ExtAddr as u16);
    Ok((entry, v24_active))
}

/// The lowest coinbase payload version Dash Core accepts while
/// DEPLOYMENT_V24 is active; before it Core refuses this version
/// (`CheckCbTx`).
const V24_COINBASE_PAYLOAD_VERSION: u16 = 4;

/// How old the SPV header tip may be for its pre-V24 coinbase to stand for
/// the next block. A tip that old means the header chain has stalled, and V24
/// may have activated in the blocks it has not seen.
const MAX_PRE_V24_TIP_AGE_SECS: u64 = 60 * 60;

/// How far past the local clock a block time may lie: Dash Core's
/// `MAX_FUTURE_BLOCK_TIME`. A tip further ahead means the local clock is
/// behind, and the tip's age cannot be judged from it.
const MAX_FUTURE_BLOCK_TIME_SECS: u64 = 2 * 60 * 60;

/// The block-version bit that signals DEPLOYMENT_V24.
const V24_SIGNAL_BIT: u8 = 13;

/// How close to a block where V24 may activate the wallet stops building
/// payloads Core refuses after V24: 30 blocks, about 75 minutes at Dash's
/// 2.5-minute block spacing, for the blocks a recent tip may lag behind the
/// network and the time a prepared transaction waits before it is broadcast.
const V24_ACTIVATION_MARGIN_BLOCKS: u32 = 30;

/// How old a pre-V24 tip may be while V24 may activate at the next period
/// start. The margin above is counted from the tip, so the tip must be fresh
/// enough that few blocks can hide behind it: about 6 in 15 minutes, against
/// about 24 in the hour [`MAX_PRE_V24_TIP_AGE_SECS`] allows elsewhere.
const MAX_TIP_AGE_NEAR_V24_ACTIVATION_SECS: u64 = 15 * 60;

/// DEPLOYMENT_V24's version-bits parameters on one network.
struct V24Deployment {
    /// Blocks per period (`nWindowSize`). A deployment's state is that of
    /// the first block of its period, so V24 can only activate at a height
    /// that is a multiple of this.
    period: u32,
    /// The fewest signalling blocks in a period that can lock V24 in
    /// (`nThresholdMin`, the floor Core's decaying threshold stops at).
    min_threshold: usize,
}

/// DEPLOYMENT_V24's parameters in Dash Core v24.0.0-rc.4's
/// `src/chainparams.cpp`. Regtest can override them with `-vbparams`, and a
/// devnet keeps them but may change its block spacing (`-powtargetspacing`),
/// which changes how long the activation margin lasts, so off mainnet and
/// testnet the checks built on them are best-effort.
fn v24_deployment(network: Network) -> V24Deployment {
    let (period, min_threshold) = match network {
        Network::Mainnet => (4032, 2420),
        Network::Testnet => (100, 60),
        Network::Devnet => (120, 72),
        Network::Regtest => (250, 150),
    };
    V24Deployment {
        period,
        min_threshold,
    }
}

/// The first block after `tip_height` where V24 can activate, and the heights
/// of the period whose signalling decides whether it can: V24 is active from
/// a period start only if the period before it was locked in, which takes at
/// least [`V24Deployment::min_threshold`] signalling blocks in the period
/// before that. The heights are `None` when the chain is too short to have
/// them.
fn next_v24_activation_window(tip_height: u32, network: Network) -> (u64, Option<Range<u32>>) {
    let period = v24_deployment(network).period;
    let next_period_start = (u64::from(tip_height) / u64::from(period) + 1) * u64::from(period);
    let signal_heights = next_period_start
        .checked_sub(2 * u64::from(period))
        .and_then(|start| {
            Some(u32::try_from(start).ok()?..u32::try_from(start + u64::from(period)).ok()?)
        });
    (next_period_start, signal_heights)
}

/// Whether Core refuses `payload` once V24 is active: an evonode service
/// update below version 3 (`bad-protx-version-disallowed`).
fn refused_after_v24(payload: &ProviderUpdateServicePayload) -> bool {
    payload.mn_type == Some(ProviderMasternodeType::HighPerformance as u16)
        && payload.version < ProTxVersion::ExtAddr as u16
}

/// The placeholder for the V24 state Core will check it against.
///
/// Once V24 is active Core refuses an evonode service update below version
/// 3, while before it Core refuses version 3, so a wrong guess fails on one
/// side of the fork. Core rejects either with a consensus error and
/// discourages every peer that relays it, and peer-to-peer broadcast reports
/// no rejection, so the version must be right before anything is sent.
///
/// `build` makes the placeholder for a given V24 state. It runs first with
/// the list's view (`list_shows_v24`), which also refuses a malformed request
/// before the tip block is fetched. Only when the result is a payload Core
/// refuses after V24 is `tip_shows_v24` awaited, to learn whether V24 is
/// active for the next block; when it is, the placeholder is rebuilt for
/// V24. A regular masternode's or a version-3 entry's payload is the same on
/// both sides, so for those the future is dropped without running.
async fn update_service_placeholder(
    list_shows_v24: bool,
    build: impl Fn(bool) -> Result<ProviderUpdateServicePayload, PlatformWalletError>,
    tip_shows_v24: impl Future<Output = Result<bool, PlatformWalletError>>,
) -> Result<ProviderUpdateServicePayload, PlatformWalletError> {
    let placeholder = build(list_shows_v24)?;
    if refused_after_v24(&placeholder) && tip_shows_v24.await? {
        return build(true);
    }
    Ok(placeholder)
}

/// Whether V24 is active for the block after the SPV header tip: the tip's
/// block, fetched from DAPI, and the stored headers of the period whose
/// signalling decides the next possible activation, read by
/// [`v24_active_after_tip`].
async fn v24_active_at_spv_tip(
    wallet: &PlatformWallet,
    spv: &SpvRuntime,
) -> Result<bool, PlatformWalletError> {
    let tip = spv
        .tip()
        .await
        .ok_or(PlatformWalletError::MasternodeListUnavailable)?;
    let block = wallet
        .sdk()
        .get_block_by_hash(&tip.hash, BLOCK_REQUEST_SETTINGS)
        .await
        .map_err(|e| v24_unknown(format!("fetching tip block {} failed: {e}", tip.hash)))?
        .ok_or_else(|| v24_unknown(format!("tip block {} was not served", tip.hash)))?;
    let network = wallet.network();
    let signal_versions = match next_v24_activation_window(tip.height, network).1 {
        Some(heights) => spv.header_versions(heights).await,
        None => None,
    };
    v24_active_after_tip(
        &block,
        &tip,
        network,
        now_secs(),
        signal_versions.as_deref(),
    )
}

/// Whether V24 is active for the block after `tip`, read from `block`, the
/// tip's block as DAPI served it, and `signal_versions`, the versions of the
/// headers whose signalling decides the next possible activation (`None`
/// when the SPV store does not hold them all).
///
/// A version-4 coinbase proves V24, since an active deployment never
/// deactivates. A lower version proves only that V24 was not active at the
/// tip, so it stands for the transaction being built only when the tip is
/// recent by a local clock that agrees with it, and V24 cannot activate
/// within [`V24_ACTIVATION_MARGIN_BLOCKS`]: either too few headers signalled
/// for it to lock in, or the next period start is further away than that
/// and the tip is fresh enough that few blocks hide behind it.
/// The SPV client does not follow the mined signal that starts the
/// deployment, so it cannot tell a locked-in fork from one that merely had
/// enough signalling; it treats both as about to activate. Without the
/// signalling headers only the distance to the next period start decides.
fn v24_active_after_tip(
    block: &Block,
    tip: &SpvTip,
    network: Network,
    now_secs: u64,
    signal_versions: Option<&[BlockVersion]>,
) -> Result<bool, PlatformWalletError> {
    if v24_active_at_block(block, &tip.hash)? {
        return Ok(true);
    }
    let tip_time = u64::from(tip.time);
    if tip_time > now_secs.saturating_add(MAX_FUTURE_BLOCK_TIME_SECS) {
        return Err(v24_unknown(format!(
            "the SPV header tip's block time is {}s past the local clock",
            tip_time - now_secs
        )));
    }
    let tip_age_secs = now_secs.saturating_sub(tip_time);
    if tip_age_secs > MAX_PRE_V24_TIP_AGE_SECS {
        return Err(v24_unknown(format!(
            "the SPV header tip is {tip_age_secs}s old"
        )));
    }
    let deployment = v24_deployment(network);
    let (next_period_start, _) = next_v24_activation_window(tip.height, network);
    let may_lock_in = signal_versions
        .filter(|versions| versions.len() == deployment.period as usize)
        .is_none_or(|versions| {
            versions
                .iter()
                .filter(|version| version.is_signalling_soft_fork(V24_SIGNAL_BIT))
                .count()
                >= deployment.min_threshold
        });
    if !may_lock_in {
        return Ok(false);
    }
    if next_period_start - u64::from(tip.height) <= u64::from(V24_ACTIVATION_MARGIN_BLOCKS) {
        return Err(v24_unknown(format!(
            "V24 may activate at height {next_period_start}"
        )));
    }
    if tip_age_secs > MAX_TIP_AGE_NEAR_V24_ACTIVATION_SECS {
        return Err(v24_unknown(format!(
            "the SPV header tip is {tip_age_secs}s old and V24 may activate at height \
             {next_period_start}"
        )));
    }
    Ok(false)
}

/// Whether V24 is active at `block`, the block the SPV header chain holds as
/// `header_hash`: Core requires a coinbase payload version of at least 4
/// while V24 is active, and below 4 before it. DAPI's reply is not
/// authenticated, so the block is read only once it hashes to that header
/// and its transactions match the header's merkle root; otherwise nothing is
/// concluded either way.
fn v24_active_at_block(
    block: &Block,
    header_hash: &BlockHash,
) -> Result<bool, PlatformWalletError> {
    if block.block_hash() != *header_hash {
        return Err(v24_unknown(format!(
            "the served block {} is not the SPV header's block {header_hash}",
            block.block_hash()
        )));
    }
    if !block.check_merkle_root() {
        return Err(v24_unknown(
            "the served block's transactions do not match its merkle root",
        ));
    }
    match block.txdata.first() {
        Some(coinbase) if coinbase.is_coin_base() => match &coinbase.special_transaction_payload {
            Some(TransactionPayload::CoinbasePayloadType(payload)) => {
                Ok(payload.version >= V24_COINBASE_PAYLOAD_VERSION)
            }
            _ => Err(v24_unknown("the block's coinbase has no coinbase payload")),
        },
        _ => Err(v24_unknown("the block has no coinbase")),
    }
}

/// The refusal when the wallet cannot tell whether V24 is active: either
/// payload version would be refused by Core on one side of the fork.
fn v24_unknown(reason: impl Display) -> PlatformWalletError {
    PlatformWalletError::TransactionBuild(format!(
        "cannot tell whether Dash Core's V24 fork is active ({reason}); try again"
    ))
}

/// The `operatorReward` (basis points) the masternode was registered with,
/// read from its ProRegTx via DAPI Core. Fails closed when the transaction
/// cannot be fetched — the payout rule cannot be applied without it.
async fn fetch_operator_reward(
    wallet: &PlatformWallet,
    pro_tx_hash: &[u8; 32],
) -> Result<u16, PlatformWalletError> {
    let display = display_hex(pro_tx_hash);
    let fetched = wallet
        .sdk()
        .get_transaction(&display)
        .await
        .map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!(
                "failed to fetch the registration transaction: {e}"
            ))
        })?
        .ok_or_else(|| {
            PlatformWalletError::InvalidParameter(format!(
                "registration transaction {display} was not found; cannot determine the \
                 operator reward"
            ))
        })?;
    operator_reward_from_registration(pro_tx_hash, &fetched.transaction)
}

/// Read `operatorReward` out of a fetched registration transaction,
/// binding the response to the request first: DAPI's get-transaction reply
/// is not authenticated, so the decoded transaction must hash to the
/// requested proTxHash before its payload is trusted. Only the operator
/// reward is read from it; the service values come from the synced
/// masternode list's entry for that proTxHash. Without the hash check a
/// reply carrying an unrelated zero-reward ProRegTx would steer
/// [`resolve_operator_payout_script`] into clearing a real operator payout.
pub(crate) fn operator_reward_from_registration(
    pro_tx_hash: &[u8; 32],
    transaction: &dashcore::Transaction,
) -> Result<u16, PlatformWalletError> {
    let expected = Txid::from_byte_array(*pro_tx_hash);
    let actual = transaction.txid();
    if actual != expected {
        return Err(PlatformWalletError::InvalidIdentityData(format!(
            "DAPI returned transaction {actual} for requested registration transaction \
             {expected}"
        )));
    }
    match &transaction.special_transaction_payload {
        Some(TransactionPayload::ProviderRegistrationPayloadType(registration)) => {
            Ok(registration.operator_reward)
        }
        _ => Err(PlatformWalletError::InvalidParameter(format!(
            "transaction {expected} is not a provider registration transaction"
        ))),
    }
}

/// The operator payout rule, decided with the wallet owner (2026-08-27):
/// the payload's payout script REPLACES the current one at consensus level
/// and an empty script clears it, while consensus also forbids a payout
/// script entirely when the masternode's `operatorReward` is zero. So:
/// reward 0 ⇒ the script is always empty and an address must not be given;
/// reward non-zero ⇒ the caller must supply the address explicitly — never
/// default to empty, which would silently clear the operator's payout.
pub(crate) fn resolve_operator_payout_script(
    operator_reward: u16,
    operator_payout_address: Option<&str>,
    network: Network,
) -> Result<ScriptBuf, PlatformWalletError> {
    match (operator_reward, operator_payout_address) {
        (0, None) => Ok(ScriptBuf::new()),
        (0, Some(_)) => Err(PlatformWalletError::InvalidParameter(
            "this masternode's operatorReward is 0, so an operator payout address is not \
             allowed"
                .to_string(),
        )),
        (reward, None) => Err(PlatformWalletError::InvalidParameter(format!(
            "this masternode pays a {}.{:02}% operator reward; the operator payout address \
             must be confirmed explicitly — an empty payout script would clear it on-chain",
            reward / 100,
            reward % 100
        ))),
        (_, Some(address)) => {
            let address = address
                .parse::<DashAddress<dashcore::address::NetworkUnchecked>>()
                .map_err(|e| {
                    PlatformWalletError::InvalidParameter(format!(
                        "operator payout address is not a valid Dash address: {e}"
                    ))
                })?
                .require_network(network)
                .map_err(|e| {
                    PlatformWalletError::InvalidParameter(format!(
                        "operator payout address is for another network: {e}"
                    ))
                })?;
            Ok(address.script_pubkey())
        }
    }
}

/// Refuse an operator secret whose public key does not match the
/// masternode-list entry, before any network work. The entry may carry the
/// basic (v19+) or legacy serialization of the same key, so both are
/// accepted — mirroring `verify_masternode_key`.
pub(crate) fn verify_operator_secret(
    expected_operator_key: &[u8; 48],
    operator_secret: &[u8; 32],
) -> Result<(), PlatformWalletError> {
    let (basic, legacy) = bls_public_keys(operator_secret).ok_or_else(|| {
        PlatformWalletError::InvalidParameter(
            "the operator key is not a valid BLS secret key".to_string(),
        )
    })?;
    if expected_operator_key != &basic && expected_operator_key != &legacy {
        return Err(PlatformWalletError::InvalidParameter(
            "the operator key does not match this masternode's operator public key".to_string(),
        ));
    }
    Ok(())
}

/// The Platform P2P port Core requires of a mainnet evonode before version 3.
const MAINNET_PLATFORM_P2P_PORT: u16 = 26656;

/// The placeholder payload for the builder: every field final except the two
/// selection-dependent ones (`inputs_hash`, `payload_sig`), which are zeroed
/// and filled by the payload finalizer after input selection.
///
/// The version follows Dash Core v24's rules for re-asserting `entry`:
/// - a version-3 entry is re-asserted at version 3 with its address map
///   copied verbatim; a version-2 payload would replace the whole map with a
///   single address;
/// - a legacy evonode is converted to version 3 once V24 is active
///   (`v24_active`), because Core then refuses any evonode update below
///   version 3 (`bad-protx-version-disallowed`); see
///   [`legacy_evonode_net_info`]. On mainnet the conversion takes only
///   Platform P2P port 26656: Core holds a pre-v3 mainnet evonode to that
///   port, while the version-3 address map has no such rule, so another
///   port would be accepted and kept;
/// - any other legacy entry stays at version 2 (BasicBLS), which Core
///   accepts for a regular masternode on either side of V24 and for an
///   evonode before it.
pub(crate) fn prepare_update_service_placeholder(
    entry: &MasternodeListEntry,
    network: Network,
    v24_active: bool,
    platform_p2p_port: Option<u16>,
    script_payout: ScriptBuf,
) -> Result<ProviderUpdateServicePayload, PlatformWalletError> {
    // The platform fields are serialized only when mn_type is
    // HighPerformance, so the evonode/regular split must be explicit here —
    // a missing mn_type would silently drop them on the wire.
    let (mn_type, platform_node_id) = match &entry.mn_type {
        EntryMasternodeType::HighPerformance {
            platform_node_id, ..
        } => (
            ProviderMasternodeType::HighPerformance,
            Some(*platform_node_id),
        ),
        EntryMasternodeType::Regular => {
            if platform_p2p_port.is_some() {
                return Err(PlatformWalletError::InvalidParameter(
                    "a platform P2P port was given, but this masternode is not an evonode"
                        .to_string(),
                ));
            }
            (ProviderMasternodeType::Regular, None)
        }
    };

    // The version and the address form are chosen together: a payload
    // encodes only as version 3 with an address map, or below 3 with a
    // legacy address.
    let (version, service_address, scalar_platform_ports) =
        match (&entry.service_address, &entry.mn_type) {
            (MasternodeNetInfo::Extended(net_info), _) => {
                if net_info.purposes.is_empty() {
                    return Err(nothing_to_reassert());
                }
                if let Some(port) = platform_p2p_port {
                    require_listed_platform_p2p_port(net_info, port)?;
                }
                (
                    ProTxVersion::ExtAddr,
                    MasternodeNetInfo::Extended(net_info.clone()),
                    None,
                )
            }
            (MasternodeNetInfo::Legacy(service), _)
                if service.ip().is_unspecified() || service.port() == 0 =>
            {
                return Err(nothing_to_reassert());
            }
            (MasternodeNetInfo::Legacy(service), EntryMasternodeType::Regular) => (
                ProTxVersion::BasicBLS,
                MasternodeNetInfo::Legacy(*service),
                None,
            ),
            (
                MasternodeNetInfo::Legacy(service),
                EntryMasternodeType::HighPerformance {
                    platform_http_port, ..
                },
            ) => {
                let p2p_port = platform_p2p_port.ok_or_else(|| {
                    PlatformWalletError::InvalidParameter(
                        "an evonode payload requires the platform P2P port (a pre-v3 masternode \
                         list entry does not carry it)"
                            .to_string(),
                    )
                })?;
                if v24_active {
                    if network == Network::Mainnet && p2p_port != MAINNET_PLATFORM_P2P_PORT {
                        return Err(PlatformWalletError::InvalidParameter(format!(
                            "platform P2P port {p2p_port} is not the mainnet evonode port \
                             {MAINNET_PLATFORM_P2P_PORT}"
                        )));
                    }
                    (
                        ProTxVersion::ExtAddr,
                        MasternodeNetInfo::Extended(legacy_evonode_net_info(
                            *service,
                            p2p_port,
                            *platform_http_port,
                        )),
                        None,
                    )
                } else {
                    (
                        ProTxVersion::BasicBLS,
                        MasternodeNetInfo::Legacy(*service),
                        Some((p2p_port, *platform_http_port)),
                    )
                }
            }
        };
    let (scalar_p2p_port, scalar_http_port) = scalar_platform_ports.unzip();

    Ok(ProviderUpdateServicePayload {
        version: version as u16,
        mn_type: Some(mn_type as u16),
        pro_tx_hash: Txid::from_byte_array(entry.pro_reg_tx_hash.to_byte_array()),
        service_address,
        script_payout,
        inputs_hash: InputsHash::all_zeros(),
        platform_node_id,
        platform_p2p_port: scalar_p2p_port,
        platform_http_port: scalar_http_port,
        payload_sig: BLSSignature::from([0u8; 96]),
    })
}

/// The refusal for a list entry with no address to re-assert: one
/// registered without addresses, or a version-3 entry whose address map
/// this wallet cannot read. Reviving it needs an update that names the
/// addresses.
fn nothing_to_reassert() -> PlatformWalletError {
    PlatformWalletError::InvalidParameter(
        "the masternode list entry has no service address this wallet can re-assert; set its \
         addresses with Dash Core's `protx update_service` or `update_service_evo` first"
            .to_string(),
    )
}

/// A version-3 evonode's address map is re-asserted as is, so a platform
/// P2P port the caller gives must be one the map already lists; any other
/// would be silently dropped.
fn require_listed_platform_p2p_port(
    net_info: &ExtNetInfo,
    port: u16,
) -> Result<(), PlatformWalletError> {
    let listed: Vec<u16> = net_info
        .purposes
        .iter()
        .filter(|(purpose, _)| *purpose == NetInfoPurpose::PlatformP2P)
        .flat_map(|(_, entries)| entries)
        .filter_map(|entry| match entry {
            NetInfoEntry::Service { port, .. } | NetInfoEntry::Domain { port, .. } => Some(*port),
            NetInfoEntry::Invalid => None,
        })
        .collect();
    if listed.contains(&port) {
        return Ok(());
    }
    Err(PlatformWalletError::InvalidParameter(format!(
        "platform P2P port {port} does not match this evonode's on-chain platform P2P ports \
         {listed:?}; its registered endpoints are re-asserted as they are, so give an on-chain \
         port or none"
    )))
}

/// The version-3 address map for a legacy evonode, as Dash Core's `protx
/// update_service_evo` builds it from bare platform ports: the legacy
/// address as CoreP2P, and its IP with each platform port as PlatformP2P
/// and PlatformHttps, one entry each. The purposes are in ascending order,
/// the order Core re-serializes its map in before checking the operator
/// signature; any other order would fail that check.
fn legacy_evonode_net_info(
    service: SocketAddr,
    platform_p2p_port: u16,
    platform_http_port: u16,
) -> ExtNetInfo {
    let ip = service.ip();
    ExtNetInfo {
        // The only ExtNetInfo version Core defines.
        version: 1,
        purposes: vec![
            (
                NetInfoPurpose::CoreP2P,
                vec![service_entry(ip, service.port())],
            ),
            (
                NetInfoPurpose::PlatformP2P,
                vec![service_entry(ip, platform_p2p_port)],
            ),
            (
                NetInfoPurpose::PlatformHttps,
                vec![service_entry(ip, platform_http_port)],
            ),
        ],
    }
}

/// `ip:port` as an address-map entry. An IPv4-mapped IPv6 address is written
/// as the IPv4 address it maps, since Core reads the mapped form there as an
/// invalid address.
fn service_entry(ip: IpAddr, port: u16) -> NetInfoEntry {
    let (network, addr) = match ip.to_canonical() {
        IpAddr::V4(ip) => (Bip155Network::Ipv4, ip.octets().to_vec()),
        IpAddr::V6(ip) => (Bip155Network::Ipv6, ip.octets().to_vec()),
    };
    NetInfoEntry::Service {
        network,
        addr,
        port,
    }
}

/// The funding builder for a ProUpServTx: the placeholder payload, the
/// finalizer that commits it to the selected inputs and signs it, and one
/// zero-value data output.
///
/// A ProUpServTx moves no value, so without that output a selection whose
/// surplus is below the dust limit (all of it booked as fee, no change
/// output) leaves the transaction with no outputs, which consensus refuses.
/// The data output is the placeholder Dash Core's own `protx
/// update_service` adds for the same reason, so the transaction always
/// carries an output; change is still returned whenever the surplus clears
/// the dust limit.
pub(crate) fn update_service_builder(
    placeholder: ProviderUpdateServicePayload,
    operator_secret: Zeroizing<[u8; 32]>,
) -> Result<TransactionBuilder, PlatformWalletError> {
    Ok(TransactionBuilder::new()
        .add_op_return(&[])
        .map_err(|e| PlatformWalletError::TransactionBuild(e.to_string()))?
        .set_special_payload(TransactionPayload::ProviderUpdateServicePayloadType(
            placeholder,
        ))
        .set_payload_finalizer(move |unsigned| {
            finalize_update_service_payload(unsigned, &operator_secret)
        }))
}

/// The payload finalizer: write `inputs_hash` over the selected inputs and
/// the operator-BLS `payload_sig` (basic scheme over the base payload hash,
/// modern serialization: the exact convention `verify_message_digest`
/// checks real mainnet signatures with).
///
/// Refuses a transaction without outputs, or a payload that cannot be
/// encoded, before signing anything. key-wallet releases the build's
/// reservation on any finalizer error, so the refusal leaves the selected
/// inputs spendable.
pub(crate) fn finalize_update_service_payload(
    unsigned: &Transaction,
    operator_secret: &[u8; 32],
) -> Result<TransactionPayload, BuilderError> {
    if unsigned.output.is_empty() {
        return Err(BuilderError::InvalidData(
            "the assembled ProUpServTx has no outputs, which the network refuses".into(),
        ));
    }
    let Some(TransactionPayload::ProviderUpdateServicePayloadType(placeholder)) =
        &unsigned.special_transaction_payload
    else {
        return Err(BuilderError::InvalidData(
            "the ProUpServTx placeholder payload is missing from the assembled transaction".into(),
        ));
    };
    let mut finalized = placeholder.clone();
    finalized.inputs_hash = unsigned.hash_inputs();
    // Encoding fails when the payload version and the address form
    // disagree. `base_payload_hash` panics on that, and across the FFI a
    // panic aborts the host app, so hash here and return the error.
    let mut engine = SpecialTransactionPayloadHash::engine();
    finalized
        .base_payload_data_encode(&mut engine)
        .map_err(|e| {
            BuilderError::InvalidData(format!("the ProUpServTx payload cannot be encoded: {e}"))
        })?;
    let sign_hash = SpecialTransactionPayloadHash::from_engine(engine);

    let secret = Option::<BlsSecretKey<Bls12381G2Impl>>::from(
        BlsSecretKey::<Bls12381G2Impl>::from_be_bytes(operator_secret),
    )
    .ok_or_else(|| {
        BuilderError::SigningFailed("the operator key is not a valid BLS secret".into())
    })?;
    let signature = secret
        .sign(SignatureSchemes::Basic, sign_hash.as_byte_array())
        .map_err(|e| BuilderError::SigningFailed(format!("BLS payload signing failed: {e}")))?;
    let signature_bytes: [u8; 96] = signature
        .to_bytes_with_mode(dashcore::blsful::SerializationFormat::Modern)
        .as_slice()
        .try_into()
        .map_err(|_| {
            BuilderError::SigningFailed("BLS signature did not serialize to 96 bytes".into())
        })?;
    finalized.payload_sig = BLSSignature::from(signature_bytes);
    Ok(TransactionPayload::ProviderUpdateServicePayloadType(
        finalized,
    ))
}

/// Fund and finalize the ProUpServTx: input selection reserves the funding
/// inputs, the payload finalizer writes `inputs_hash` and the operator-BLS
/// `payload_sig`, and only then are the inputs ECDSA-signed, since their
/// sighashes cover the finished payload.
///
/// Stops at the signed transaction; the caller broadcasts or abandons it.
pub(crate) async fn build_sign_update_service<B, S>(
    core: &CoreWallet<B>,
    placeholder: ProviderUpdateServicePayload,
    operator_secret: Zeroizing<[u8; 32]>,
    signer: &S,
) -> Result<SignedCoreTransaction, PlatformWalletError>
where
    B: TransactionBroadcaster + ?Sized,
    S: TransactionSigner + ?Sized + Sync,
{
    let builder = update_service_builder(placeholder, operator_secret)?;
    core.finalize_transaction(builder, &SEND_FUNDING_SOURCES, 0, signer)
        .await
}

fn display_hex(pro_tx_hash: &[u8; 32]) -> String {
    let mut display = *pro_tx_hash;
    display.reverse();
    hex::encode(display)
}

#[cfg(test)]
mod tests {
    use super::super::list::test_support::{evonode_entry, masternode_entry};
    use super::super::list::MasternodeListSummary;
    use super::super::record::core_v24_vectors::{decode, EVO_PRO_UP_SERV_TX};
    use super::*;
    use crate::broadcaster::BroadcastError;
    use crate::test_support::{
        funded_wallet_manager, funded_wallet_manager_with_outputs, WalletSigner,
    };
    use dashcore::block::Header;
    use dashcore::blockdata::transaction::special_transaction::coinbase::CoinbasePayload;
    use dashcore::bls_sig_utils::BLSPublicKey;
    use dashcore::blsful::{PublicKey as BlsPublicKey, Signature as BlsSignature};
    use dashcore::consensus::encode::deserialize;
    use dashcore::hash_types::{
        MerkleRootAssetUnlocks, MerkleRootMasternodeList, MerkleRootQuorums,
    };
    use dashcore::sml::masternode_list_entry::net_info::{
        Bip155Network, ExtNetInfo, NetInfoEntry, NetInfoPurpose,
    };
    use dashcore::sml::masternode_list_entry::{EntryMasternodeType, MasternodeNetInfo};
    use dashcore::{CompactTarget, OutPoint, ProTxHash, TxIn, TxMerkleNode};
    use key_wallet::account::StandardAccountType;
    use key_wallet::wallet::managed_wallet_info::fee::FeeRate;
    use std::sync::{Arc, Mutex};

    /// A fixed valid BLS12-381 secret scalar (big-endian, below the group
    /// order) so the test keypair is deterministic.
    const OPERATOR_SECRET: [u8; 32] = [7u8; 32];

    fn operator_entry(seed: u8, evo: bool) -> MasternodeListEntry {
        let (basic, _) = bls_public_keys(&OPERATOR_SECRET).expect("valid test scalar");
        let mut entry = if evo {
            evonode_entry(seed)
        } else {
            masternode_entry(seed)
        };
        entry.operator_public_key = BLSPublicKey::from(basic);
        entry
    }

    #[derive(Default)]
    struct RecordingBroadcaster {
        sent: Mutex<Vec<Transaction>>,
    }

    #[async_trait::async_trait]
    impl TransactionBroadcaster for RecordingBroadcaster {
        async fn broadcast(&self, transaction: &Transaction) -> Result<Txid, BroadcastError> {
            self.sent
                .lock()
                .expect("broadcaster lock")
                .push(transaction.clone());
            Ok(transaction.txid())
        }
    }

    impl RecordingBroadcaster {
        fn sent_count(&self) -> usize {
            self.sent.lock().expect("broadcaster lock").len()
        }
    }

    /// A BIP44-funded core wallet holding one UTXO per value in `outputs`,
    /// with a broadcaster that records what it is given.
    async fn core_with_outputs(
        outputs: &[u64],
    ) -> (
        CoreWallet<RecordingBroadcaster>,
        Arc<RecordingBroadcaster>,
        WalletSigner,
    ) {
        let (wallet_manager, wallet_id, generation, signer) =
            funded_wallet_manager_with_outputs(StandardAccountType::BIP44Account, outputs).await;
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let broadcaster = Arc::new(RecordingBroadcaster::default());
        let core = CoreWallet::new(
            sdk,
            wallet_manager,
            wallet_id,
            broadcaster.clone(),
            generation,
        );
        (core, broadcaster, signer)
    }

    fn regular_placeholder() -> ProviderUpdateServicePayload {
        let entry = operator_entry(0x44, false);
        prepare_update_service_placeholder(&entry, Network::Devnet, false, None, ScriptBuf::new())
            .expect("placeholder")
    }

    /// The version-3 ProUpServTx Dash Core v24 mined for evonode
    /// `a9cbebcd…`: CoreP2P 68.67.122.237:20001, PlatformP2P port 26656,
    /// PlatformHttps port 1443, no operator payout.
    fn core_v3_update() -> ProviderUpdateServicePayload {
        let Some(TransactionPayload::ProviderUpdateServicePayloadType(payload)) =
            decode(EVO_PRO_UP_SERV_TX).special_transaction_payload
        else {
            panic!("the vector is a ProUpServTx");
        };
        payload
    }

    /// The version-3 list entry that update leaves behind, operated by the
    /// test operator key.
    fn core_v3_evonode_entry() -> MasternodeListEntry {
        let update = core_v3_update();
        MasternodeListEntry {
            version: 3,
            pro_reg_tx_hash: ProTxHash::from_byte_array(update.pro_tx_hash.to_byte_array()),
            service_address: update.service_address,
            mn_type: EntryMasternodeType::HighPerformance {
                platform_http_port: 1443,
                platform_node_id: update.platform_node_id.expect("an evonode update"),
            },
            ..operator_entry(0, true)
        }
    }

    /// The list is keyed by the wire-order proTxHash; the fixtures'
    /// `[seed; 32]` hashes read the same reversed, so the vector's real one
    /// pins the orientation. V24 comes from the whole list: a legacy target
    /// next to a version-3 entry is past V24 too.
    #[test]
    fn should_find_the_entry_in_wire_order_and_read_v24_from_the_whole_list() {
        let display =
            hex::decode("a9cbebcd8f262c822146b9354ce994afe7d0974356d7223284ff510659b2a122")
                .expect("hex");
        let mut wire = [0u8; 32];
        wire.copy_from_slice(&display);
        wire.reverse();

        let v3 = core_v3_evonode_entry();
        let legacy = operator_entry(0x22, true);
        let legacy_hash = legacy.pro_reg_tx_hash.to_byte_array();
        let mixed = vec![legacy.clone(), v3.clone()];

        let (found, v24_active) =
            update_service_target(&mixed, &wire).expect("found by wire order");
        assert_eq!(found, &v3);
        assert!(v24_active);

        let (found, v24_active) =
            update_service_target(&mixed, &legacy_hash).expect("legacy entry found");
        assert_eq!(found, &legacy);
        assert!(
            v24_active,
            "a version-3 neighbour shows V24 for a legacy target"
        );

        let pre_v24 = vec![legacy.clone(), operator_entry(0x33, false)];
        let (_, v24_active) = update_service_target(&pre_v24, &legacy_hash).expect("found");
        assert!(!v24_active, "no version-3 entry, no V24");

        let mut display_order = [0u8; 32];
        display_order.copy_from_slice(&display);
        let err = update_service_target(&mixed, &display_order)
            .expect_err("a display-order hash is not the wire-order key");
        assert!(
            matches!(&err, PlatformWalletError::InvalidParameter(message) if message.contains("not in the masternode list")),
            "got {err:?}"
        );
    }

    /// The vector's operator sign hash in wire order (`27fb283c…a21c72b6`
    /// in display order): SHA256d of Core's base payload bytes, computed
    /// outside dashcore.
    const CORE_V3_SIGN_HASH: &str =
        "b6721ca2ba4de4face21a4db3eae09590049149307057456019ed2323c28fb27";

    /// A legacy form of the vector's evonode, as before V24, at the address
    /// the version-3 update keeps as its CoreP2P entry.
    fn legacy_core_evonode_entry(service_address: &str) -> MasternodeListEntry {
        MasternodeListEntry {
            version: 2,
            service_address: MasternodeNetInfo::Legacy(
                service_address.parse().expect("socket address"),
            ),
            ..core_v3_evonode_entry()
        }
    }

    fn base_payload_bytes(payload: &ProviderUpdateServicePayload) -> Vec<u8> {
        let mut bytes = Vec::new();
        payload
            .base_payload_data_encode(&mut bytes)
            .expect("the payload version and address form agree");
        bytes
    }

    /// The placeholder, committed to the vector's inputs, is the payload
    /// Core produced: the same bytes Core signed (the payload is the
    /// transaction's last 220 bytes, ending in the 96-byte signature), the
    /// same sign hash, and the same payload field by field, so a scalar
    /// port the version-3 encoding skips cannot hide.
    fn assert_matches_core_v3_vector(mut placeholder: ProviderUpdateServicePayload) {
        let core = core_v3_update();
        placeholder.inputs_hash = core.inputs_hash;

        let raw = hex::decode(EVO_PRO_UP_SERV_TX.0.trim()).expect("vector hex");
        assert_eq!(
            hex::encode(base_payload_bytes(&placeholder)),
            hex::encode(&raw[raw.len() - 220..raw.len() - 96]),
            "the base payload is Core's"
        );
        assert_eq!(
            hex::encode(placeholder.base_payload_hash().as_byte_array()),
            CORE_V3_SIGN_HASH
        );
        assert_eq!(
            placeholder,
            ProviderUpdateServicePayload {
                payload_sig: BLSSignature::from([0u8; 96]),
                ..core
            }
        );
    }

    /// The payload signature verifies under the basic scheme against the
    /// operator public key, over base_payload_hash — the exact convention
    /// `verify_message_digest` checks real mainnet signatures with.
    fn assert_operator_signature(payload: &ProviderUpdateServicePayload) {
        let secret = Option::<BlsSecretKey<Bls12381G2Impl>>::from(
            BlsSecretKey::<Bls12381G2Impl>::from_be_bytes(&OPERATOR_SECRET),
        )
        .expect("valid test scalar");
        let public_key = BlsPublicKey::from(&secret);
        let signature: BlsSignature<Bls12381G2Impl> = payload
            .payload_sig
            .try_into()
            .expect("compressed signature decodes");
        signature
            .verify(&public_key, payload.base_payload_hash().as_byte_array())
            .expect("operator BLS signature verifies over base_payload_hash");
    }

    /// A version-3 entry is re-asserted at version 3 with its address map
    /// copied verbatim: from the list entry Core's update left behind, the
    /// wallet rebuilds that update.
    #[test]
    fn should_reassert_a_v3_evonode_entry_as_core_encoded_it() {
        let placeholder = prepare_update_service_placeholder(
            &core_v3_evonode_entry(),
            Network::Devnet,
            true,
            None,
            ScriptBuf::new(),
        )
        .expect("a version-3 evonode is re-asserted");
        assert_matches_core_v3_vector(placeholder);
    }

    /// After V24 Core refuses an evonode update below version 3, so a
    /// legacy evonode's address and platform ports become a version-3
    /// address map, as `protx update_service_evo` builds one from bare
    /// ports. The vector has exactly that shape, so the conversion must
    /// reproduce Core's bytes, also from an IPv4-mapped address.
    #[test]
    fn should_convert_a_legacy_evonode_to_cores_v3_encoding_after_v24() {
        for address in ["68.67.122.237:20001", "[::ffff:68.67.122.237]:20001"] {
            let placeholder = prepare_update_service_placeholder(
                &legacy_core_evonode_entry(address),
                Network::Devnet,
                true,
                Some(26656),
                ScriptBuf::new(),
            )
            .expect("a legacy evonode is converted");
            assert_matches_core_v3_vector(placeholder);
        }

        let err = prepare_update_service_placeholder(
            &legacy_core_evonode_entry("68.67.122.237:20001"),
            Network::Devnet,
            true,
            None,
            ScriptBuf::new(),
        )
        .expect_err("the converted address map needs the caller's P2P port");
        assert!(
            matches!(&err, PlatformWalletError::InvalidParameter(message) if message.contains("requires the platform P2P port")),
            "got {err:?}"
        );
    }

    /// Before version 3 Core holds a mainnet evonode to Platform P2P port
    /// 26656; the version-3 address map has no such rule. On mainnet the
    /// conversion therefore takes only 26656, so a mistyped port cannot
    /// reach the chain and stay there. Elsewhere, and on the version-2 path
    /// Core still checks, the port is left to Core.
    #[test]
    fn should_hold_a_mainnet_legacy_evonode_conversion_to_port_26656() {
        let entry = legacy_core_evonode_entry("68.67.122.237:20001");
        let build = |network, v24_active, port| {
            prepare_update_service_placeholder(
                &entry,
                network,
                v24_active,
                Some(port),
                ScriptBuf::new(),
            )
        };

        let err =
            build(Network::Mainnet, true, 26657).expect_err("another port is refused on mainnet");
        assert!(
            matches!(&err, PlatformWalletError::InvalidParameter(message) if message.contains("26657") && message.contains("26656")),
            "got {err:?}"
        );

        let converted = build(Network::Mainnet, true, 26656).expect("26656 is converted");
        assert_eq!(converted.version, ProTxVersion::ExtAddr as u16);
        let off_mainnet = build(Network::Testnet, true, 36656).expect("left to Core");
        assert_eq!(off_mainnet.version, ProTxVersion::ExtAddr as u16);
        let before_v24 = build(Network::Mainnet, false, 26657).expect("left to Core");
        assert_eq!(before_v24.version, ProTxVersion::BasicBLS as u16);
    }

    /// A version-3 regular masternode is re-asserted at version 3 with its
    /// address map verbatim. Core also accepts a version-2 update for it,
    /// but that replaces the whole map with one address.
    #[test]
    fn should_reassert_a_v3_regular_masternode_entry_verbatim() {
        let core_p2p = |network, addr: Vec<u8>| NetInfoEntry::Service {
            network,
            addr,
            port: 9999,
        };
        let entry = MasternodeListEntry {
            version: 3,
            service_address: MasternodeNetInfo::Extended(ExtNetInfo {
                version: 1,
                purposes: vec![(
                    NetInfoPurpose::CoreP2P,
                    vec![
                        core_p2p(Bip155Network::Ipv4, vec![10, 0, 0, 0x66]),
                        core_p2p(
                            Bip155Network::Ipv6,
                            "2001:db8::66"
                                .parse::<std::net::Ipv6Addr>()
                                .expect("ipv6")
                                .octets()
                                .to_vec(),
                        ),
                    ],
                )],
            }),
            ..operator_entry(0x66, false)
        };

        let placeholder = prepare_update_service_placeholder(
            &entry,
            Network::Devnet,
            true,
            None,
            ScriptBuf::new(),
        )
        .expect("a version-3 regular masternode is re-asserted");
        assert_eq!(placeholder.version, ProTxVersion::ExtAddr as u16);
        assert_eq!(placeholder.service_address, entry.service_address);
        assert_eq!(
            placeholder.mn_type,
            Some(ProviderMasternodeType::Regular as u16)
        );
        assert_eq!(placeholder.platform_node_id, None);
        assert_eq!(placeholder.platform_p2p_port, None);
        assert_eq!(placeholder.platform_http_port, None);
        assert_eq!(
            base_payload_bytes(&placeholder).len() + 96,
            placeholder.size()
        );
    }

    /// The payload's payout script replaces the on-chain one, so both
    /// version-3 paths carry the resolved script. The vector's is empty, so
    /// the byte comparisons cannot catch a dropped one.
    #[test]
    fn should_keep_the_operator_payout_script_at_version_3() {
        let payout = DashAddress::dummy(Network::Devnet, 3).script_pubkey();
        let v3 = core_v3_evonode_entry();
        let legacy = legacy_core_evonode_entry("68.67.122.237:20001");
        for (entry, platform_p2p_port) in [(&v3, None), (&legacy, Some(26656))] {
            let placeholder = prepare_update_service_placeholder(
                entry,
                Network::Devnet,
                true,
                platform_p2p_port,
                payout.clone(),
            )
            .expect("version-3 placeholder");
            assert_eq!(placeholder.version, ProTxVersion::ExtAddr as u16);
            assert_eq!(placeholder.script_payout, payout);
        }
    }

    /// Hosts cannot tell a version-3 evonode from a legacy one and pass the
    /// platform P2P port for every evonode. The on-chain port, or none, is
    /// accepted; any other is refused rather than silently ignored, since
    /// the re-asserted address map keeps the on-chain one.
    #[test]
    fn should_accept_a_v3_evonode_p2p_port_only_when_it_matches() {
        let entry = core_v3_evonode_entry();
        let with_port = prepare_update_service_placeholder(
            &entry,
            Network::Devnet,
            true,
            Some(26656),
            ScriptBuf::new(),
        )
        .expect("the on-chain port is accepted");
        let without_port = prepare_update_service_placeholder(
            &entry,
            Network::Devnet,
            true,
            None,
            ScriptBuf::new(),
        )
        .expect("no port is accepted");
        assert_eq!(with_port, without_port);
        assert_eq!(with_port.platform_p2p_port, None);

        let err = prepare_update_service_placeholder(
            &entry,
            Network::Devnet,
            true,
            Some(26657),
            ScriptBuf::new(),
        )
        .expect_err("another port is refused");
        assert!(
            matches!(&err, PlatformWalletError::InvalidParameter(message) if message.contains("26657") && message.contains("26656")),
            "the refusal names both ports, got {err:?}"
        );
        prepare_update_service_placeholder(
            &entry,
            Network::Devnet,
            true,
            Some(1443),
            ScriptBuf::new(),
        )
        .expect_err("the PlatformHttps port is not a Platform P2P port");
    }

    /// Before V24 Core rejects version 3, so a legacy evonode keeps the
    /// version-2 payload with scalar platform ports: what mainnet and
    /// testnet build until V24 activates there.
    #[test]
    fn should_keep_version_2_for_a_legacy_evonode_before_v24() {
        let entry = operator_entry(0x22, true);
        let placeholder = prepare_update_service_placeholder(
            &entry,
            Network::Devnet,
            false,
            Some(26656),
            ScriptBuf::new(),
        )
        .expect("version-2 evonode placeholder");
        assert_eq!(placeholder.version, ProTxVersion::BasicBLS as u16);
        assert_eq!(placeholder.service_address, entry.service_address);
        assert_eq!(placeholder.platform_p2p_port, Some(26656));
        assert_eq!(placeholder.platform_http_port, Some(443));
        assert_eq!(
            base_payload_bytes(&placeholder).len() + 96,
            placeholder.size()
        );
    }

    /// Core accepts a version-2 update for a regular masternode on either
    /// side of V24, and it leaves a version-2 state at version 2.
    #[test]
    fn should_keep_version_2_for_a_legacy_regular_masternode_after_v24() {
        let entry = operator_entry(0x11, false);
        let placeholder = prepare_update_service_placeholder(
            &entry,
            Network::Devnet,
            true,
            None,
            ScriptBuf::new(),
        )
        .expect("version-2 regular placeholder");
        assert_eq!(placeholder.version, ProTxVersion::BasicBLS as u16);
        assert_eq!(placeholder.service_address, entry.service_address);
        assert_eq!(
            base_payload_bytes(&placeholder).len() + 96,
            placeholder.size()
        );
    }

    /// A block whose only transaction is a coinbase with payload version
    /// `version`, its merkle root committed in the header.
    fn block_with_coinbase_payload_version(version: u16) -> Block {
        let coinbase = Transaction {
            version: 3,
            lock_time: 0,
            input: vec![TxIn::default()],
            output: Vec::new(),
            special_transaction_payload: Some(TransactionPayload::CoinbasePayloadType(
                CoinbasePayload {
                    version,
                    height: 1,
                    merkle_root_masternode_list: MerkleRootMasternodeList::all_zeros(),
                    merkle_root_quorums: MerkleRootQuorums::all_zeros(),
                    best_cl_height: Some(0),
                    best_cl_signature: Some(BLSSignature::from([0u8; 96])),
                    asset_locked_amount: Some(0),
                    merkle_root_asset_unlocks: (version >= 4)
                        .then(MerkleRootAssetUnlocks::all_zeros),
                },
            )),
        };
        let mut block = Block {
            header: Header {
                version: BlockVersion::default(),
                prev_blockhash: BlockHash::all_zeros(),
                merkle_root: TxMerkleNode::all_zeros(),
                time: 0,
                bits: CompactTarget::from_consensus(0x1d00ffff),
                nonce: 0,
            },
            txdata: vec![coinbase],
        };
        block.header.merkle_root = block.compute_merkle_root().expect("non-empty");
        block
    }

    /// A tip the build must not read: the payload does not depend on V24.
    async fn tip_not_read() -> Result<bool, PlatformWalletError> {
        panic!("the tip block must not be read for this payload")
    }

    /// The placeholder for `entry` as the wallet builds it for the list's
    /// view of V24 and the tip's.
    async fn placeholder_for(
        entry: &MasternodeListEntry,
        list_shows_v24: bool,
        platform_p2p_port: Option<u16>,
        tip_shows_v24: impl Future<Output = Result<bool, PlatformWalletError>>,
    ) -> Result<ProviderUpdateServicePayload, PlatformWalletError> {
        update_service_placeholder(
            list_shows_v24,
            |v24_active| {
                prepare_update_service_placeholder(
                    entry,
                    Network::Devnet,
                    v24_active,
                    platform_p2p_port,
                    ScriptBuf::new(),
                )
            },
            tip_shows_v24,
        )
        .await
    }

    /// A list entry turns version 3 only when a version-3 ProRegTx or
    /// ProUpServTx for it is mined, so right after V24 activates every entry
    /// is still legacy, while Core already refuses an evonode service update
    /// below version 3 (`bad-protx-version-disallowed`). The tip's coinbase
    /// shows V24 then, so reviving a banned legacy evonode must build the
    /// version-3 payload Core accepts.
    #[tokio::test]
    async fn should_revive_a_legacy_evonode_at_version_3_after_v24_before_any_v3_entry_exists() {
        let target = legacy_core_evonode_entry("68.67.122.237:20001");
        let entries = vec![
            target.clone(),
            operator_entry(0x11, false),
            operator_entry(0x22, true),
        ];
        let (entry, list_shows_v24) =
            update_service_target(&entries, &target.pro_reg_tx_hash.to_byte_array())
                .expect("found");
        assert!(!list_shows_v24, "every entry is still legacy");

        let tip = &block_with_coinbase_payload_version(V24_COINBASE_PAYLOAD_VERSION);
        let placeholder = placeholder_for(entry, list_shows_v24, Some(26656), async move {
            v24_active_at_block(tip, &tip.block_hash())
        })
        .await
        .expect("a legacy evonode is converted");
        assert_matches_core_v3_vector(placeholder);
    }

    /// Before V24 the tip's coinbase is below
    /// version 4 and Core refuses version 3, so a legacy evonode keeps its
    /// version-2 payload.
    #[tokio::test]
    async fn should_keep_version_2_for_a_legacy_evonode_while_the_tip_shows_no_v24() {
        let entry = legacy_core_evonode_entry("68.67.122.237:20001");
        let tip = &block_with_coinbase_payload_version(V24_COINBASE_PAYLOAD_VERSION - 1);
        let placeholder = placeholder_for(&entry, false, Some(26656), async move {
            v24_active_at_block(tip, &tip.block_hash())
        })
        .await
        .expect("version-2 evonode placeholder");
        assert_eq!(placeholder.version, ProTxVersion::BasicBLS as u16);
        assert_eq!(placeholder.platform_p2p_port, Some(26656));
    }

    /// Only an evonode payload below version 3 changes with V24, so only
    /// then is the tip read: regular and version-3 unbans never depend on
    /// DAPI's block, a version-3 neighbour already proves V24, and a request
    /// the wallet refuses anyway fails on its own error before any network
    /// work.
    #[tokio::test]
    async fn should_read_the_tip_only_for_a_payload_core_refuses_after_v24() {
        let legacy_evonode = legacy_core_evonode_entry("68.67.122.237:20001");
        let legacy_regular = operator_entry(0x11, false);
        let v3_regular = MasternodeListEntry {
            version: 3,
            service_address: MasternodeNetInfo::Extended(ExtNetInfo {
                version: 1,
                purposes: vec![(
                    NetInfoPurpose::CoreP2P,
                    vec![NetInfoEntry::Service {
                        network: Bip155Network::Ipv4,
                        addr: vec![10, 0, 0, 0x11],
                        port: 9999,
                    }],
                )],
            }),
            ..legacy_regular.clone()
        };

        let regular = placeholder_for(&legacy_regular, false, None, tip_not_read())
            .await
            .expect("a regular masternode needs no tip");
        assert_eq!(regular.version, ProTxVersion::BasicBLS as u16);
        let v3 = placeholder_for(&v3_regular, true, None, tip_not_read())
            .await
            .expect("a version-3 regular masternode needs no tip");
        assert_eq!(v3.version, ProTxVersion::ExtAddr as u16);
        let v3_evonode = placeholder_for(&core_v3_evonode_entry(), true, None, tip_not_read())
            .await
            .expect("a version-3 evonode needs no tip");
        assert_matches_core_v3_vector(v3_evonode);
        let shown_by_list = placeholder_for(&legacy_evonode, true, Some(26656), tip_not_read())
            .await
            .expect("a version-3 neighbour shows V24");
        assert_matches_core_v3_vector(shown_by_list);

        let err = placeholder_for(&legacy_evonode, false, None, tip_not_read())
            .await
            .expect_err("an evonode request without its P2P port is refused");
        assert!(
            matches!(&err, PlatformWalletError::InvalidParameter(message) if message.contains("requires the platform P2P port")),
            "got {err:?}"
        );
    }

    /// When the tip cannot be read, either payload may be refused by Core,
    /// so nothing is built.
    #[tokio::test]
    async fn should_fail_the_build_when_the_tip_cannot_be_read() {
        let entry = legacy_core_evonode_entry("68.67.122.237:20001");
        let err = placeholder_for(&entry, false, Some(26656), async {
            Err(PlatformWalletError::TransactionBuild(
                "block not served".to_string(),
            ))
        })
        .await
        .expect_err("an unread tip fails the build");
        assert!(
            matches!(&err, PlatformWalletError::TransactionBuild(message) if message.contains("not served")),
            "got {err:?}"
        );
    }

    /// Core requires coinbase payload version 4 exactly while V24 is
    /// active, so the version-3 coinbase of the last block before the fork
    /// reads as pre-V24, and the version-4 one of the first block after it,
    /// or any later version, as V24.
    #[test]
    fn should_read_v24_from_the_coinbase_payload_version() {
        for (version, active) in [
            (V24_COINBASE_PAYLOAD_VERSION - 1, false),
            (V24_COINBASE_PAYLOAD_VERSION, true),
            (V24_COINBASE_PAYLOAD_VERSION + 1, true),
        ] {
            let block = block_with_coinbase_payload_version(version);
            assert_eq!(
                v24_active_at_block(&block, &block.block_hash()).expect("verifies"),
                active,
                "coinbase payload version {version}"
            );
        }
    }

    /// Blocks Core v24 mined on either side of the fork read as such: devnet
    /// block 15146, after V24 activated there at 15000, and testnet block
    /// 1575845, before V24 activated on testnet.
    #[test]
    fn should_read_v24_from_blocks_core_mined() {
        let devnet: Block = deserialize(
            &hex::decode(
                include_str!("../../tests/vectors/core_v24_devnet/block_15146.hex").trim(),
            )
            .expect("vector hex"),
        )
        .expect("Core's block decodes");
        let testnet: Block = deserialize(
            &hex::decode(
                include_str!("../../tests/vectors/core_v24_testnet/block_1575845.hex").trim(),
            )
            .expect("vector hex"),
        )
        .expect("Core's block decodes");
        let devnet_hash: BlockHash =
            "74b7a302c6d34ee6050817524d11d6b3fb4e6f640513faa10bd4ec09d8d1a56c"
                .parse()
                .expect("display-order block hash");
        let testnet_hash: BlockHash =
            "000000787345d5662bdb4107b9749a0e8f552cee5196e9531523986d94b90bb2"
                .parse()
                .expect("display-order block hash");

        for (block, header_hash, height, active) in [
            (&devnet, devnet_hash, 15146, true),
            (&testnet, testnet_hash, 1575845, false),
        ] {
            let Some(TransactionPayload::CoinbasePayloadType(coinbase)) =
                &block.txdata[0].special_transaction_payload
            else {
                panic!("the block's first transaction is a coinbase");
            };
            assert_eq!(
                coinbase.height, height,
                "the vector is the documented block"
            );
            assert_eq!(
                v24_active_at_block(block, &header_hash).expect("Core's block verifies"),
                active,
                "block {height}"
            );
        }
    }

    fn tip_of(block: &Block, height: u32, time: u32) -> SpvTip {
        SpvTip {
            height,
            hash: block.block_hash(),
            time,
        }
    }

    /// One period of header versions, `signalling` of them signalling V24 by
    /// Core's rule (top bits `001` and bit 13). Of the rest, every other one
    /// sets bit 13 under other top bits, which Core does not count.
    fn signal_period(network: Network, signalling: usize) -> Vec<BlockVersion> {
        (0..v24_deployment(network).period as usize)
            .map(|index| {
                BlockVersion::from_consensus(match index {
                    index if index < signalling => 0x2000_2000,
                    index if index % 2 == 0 => 0x6000_2000,
                    _ => 0x2000_0000,
                })
            })
            .collect()
    }

    /// The pre-V24 tips of the period before height `period * 401` that a
    /// pre-V24 coinbase does not stand for, with `signal_versions` as the
    /// signalling headers. Every refusal names that height; every other tip
    /// reads as pre-V24.
    fn refused_tips(network: Network, signal_versions: Option<&[BlockVersion]>) -> Vec<u32> {
        let now = 1_792_470_316;
        let pre = block_with_coinbase_payload_version(V24_COINBASE_PAYLOAD_VERSION - 1);
        let period_start = v24_deployment(network).period * 401;
        (period_start - v24_deployment(network).period..period_start)
            .filter(|&height| {
                match v24_active_after_tip(
                    &pre,
                    &tip_of(&pre, height, now as u32),
                    network,
                    now,
                    signal_versions,
                ) {
                    Ok(active) => {
                        assert!(!active, "{network:?} tip {height}");
                        false
                    }
                    Err(err) => {
                        let expected = format!("may activate at height {period_start}");
                        assert!(
                            matches!(&err, PlatformWalletError::TransactionBuild(message) if message.contains(&expected)),
                            "{network:?} tip {height}: got {err:?}"
                        );
                        true
                    }
                }
            })
            .collect()
    }

    /// The tips within [`V24_ACTIVATION_MARGIN_BLOCKS`] of `period_start`.
    fn margin_before(period_start: u32) -> Vec<u32> {
        (period_start - V24_ACTIVATION_MARGIN_BLOCKS..period_start).collect()
    }

    /// The parameters are Dash Core v24.0.0-rc.4's DEPLOYMENT_V24
    /// `nWindowSize` and `nThresholdMin` (`src/chainparams.cpp`). V24 is
    /// active from a period start only if the period before it was locked
    /// in, which takes that many signalling blocks in the period before
    /// that, so those are the headers the wallet reads.
    #[test]
    fn should_read_v24_signalling_from_two_periods_before_the_next_period_start() {
        for (network, period, min_threshold) in [
            (Network::Mainnet, 4032, 2420),
            (Network::Testnet, 100, 60),
            (Network::Devnet, 120, 72),
            (Network::Regtest, 250, 150),
        ] {
            let deployment = v24_deployment(network);
            assert_eq!(
                (deployment.period, deployment.min_threshold),
                (period, min_threshold),
                "{network:?}"
            );
            let start = period * 401;
            for tip in [start - period, start - 1] {
                assert_eq!(
                    next_v24_activation_window(tip, network),
                    (u64::from(start), Some(start - 2 * period..start - period)),
                    "{network:?} tip {tip}"
                );
            }
            assert_eq!(
                next_v24_activation_window(period - 1, network),
                (u64::from(period), None),
                "{network:?}: the chain has no signalling period before the first"
            );
        }
    }

    /// When enough headers signalled for V24 to lock in at the next period
    /// start, a pre-V24 tip within 30 blocks of it may be behind a chain
    /// that already activated V24, or the transaction may reach peers after
    /// it, so the build is refused until the fork is seen. Further from the
    /// period start the version-2 payload is still built.
    #[test]
    fn should_refuse_a_pre_v24_tip_within_30_blocks_of_a_period_v24_may_lock_in_for() {
        for network in [
            Network::Mainnet,
            Network::Testnet,
            Network::Devnet,
            Network::Regtest,
        ] {
            let deployment = v24_deployment(network);
            let period_start = deployment.period * 401;
            let enough = signal_period(network, deployment.min_threshold);
            assert_eq!(
                refused_tips(network, Some(&enough)),
                margin_before(period_start),
                "{network:?}"
            );

            let post = block_with_coinbase_payload_version(V24_COINBASE_PAYLOAD_VERSION);
            let now = 1_792_470_316;
            assert!(
                v24_active_after_tip(
                    &post,
                    &tip_of(&post, period_start - 1, now as u32),
                    network,
                    now,
                    Some(&enough),
                )
                .expect("a version-4 tip"),
                "{network:?}: a version-4 tip proves V24 anywhere"
            );
        }
    }

    /// One signalling block short of the lowest threshold, V24 cannot lock
    /// in for the next period start, so its earliest activation is a period
    /// further on and every tip of this period reads as pre-V24. Bit 13
    /// under other top bits is not a V24 signal.
    #[test]
    fn should_build_pre_v24_payloads_up_to_a_period_start_v24_cannot_lock_in_for() {
        for network in [
            Network::Mainnet,
            Network::Testnet,
            Network::Devnet,
            Network::Regtest,
        ] {
            let too_few = signal_period(network, v24_deployment(network).min_threshold - 1);
            assert_eq!(
                refused_tips(network, Some(&too_few)),
                Vec::<u32>::new(),
                "{network:?}"
            );
        }
    }

    /// An SPV client that started syncing after those headers cannot rule
    /// out a lock-in, so only the distance to the next period start decides.
    /// A window that is not one whole period is read the same way.
    #[test]
    fn should_fall_back_to_the_activation_margin_without_the_signalling_headers() {
        for network in [
            Network::Mainnet,
            Network::Testnet,
            Network::Devnet,
            Network::Regtest,
        ] {
            let deployment = v24_deployment(network);
            let margin = margin_before(deployment.period * 401);
            assert_eq!(refused_tips(network, None), margin, "{network:?}");

            let too_few = signal_period(network, deployment.min_threshold - 1);
            let partial = &too_few[1..];
            assert_eq!(
                refused_tips(network, Some(partial)),
                margin,
                "{network:?}: a partial window is no evidence"
            );
        }
    }

    /// While V24 may lock in for the next period start, the 30-block margin
    /// is counted from the tip, so the tip must be at most 15 minutes old for
    /// few blocks to hide behind it. Without enough signalling the hour-long
    /// allowance stands, since the earliest activation is a period away.
    #[test]
    fn should_require_a_fresh_tip_while_v24_may_lock_in() {
        let network = Network::Testnet;
        let deployment = v24_deployment(network);
        let period_start = deployment.period * 401;
        let tip_height = period_start - V24_ACTIVATION_MARGIN_BLOCKS - 1;
        let time = 1_792_470_316u32;
        let pre = block_with_coinbase_payload_version(V24_COINBASE_PAYLOAD_VERSION - 1);
        let enough = signal_period(network, deployment.min_threshold);
        let too_few = signal_period(network, deployment.min_threshold - 1);
        let read = |signal_versions: Option<&[BlockVersion]>, age_secs: u64| {
            v24_active_after_tip(
                &pre,
                &tip_of(&pre, tip_height, time),
                network,
                u64::from(time) + age_secs,
                signal_versions,
            )
        };
        let fresh = MAX_TIP_AGE_NEAR_V24_ACTIVATION_SECS;
        let not_fresh = MAX_TIP_AGE_NEAR_V24_ACTIVATION_SECS + 1;

        assert!(!read(Some(&enough), fresh).expect("a fresh tip"));
        for signal_versions in [Some(enough.as_slice()), None] {
            let err = read(signal_versions, not_fresh).expect_err("a tip that is not fresh");
            assert!(
                matches!(&err, PlatformWalletError::TransactionBuild(message)
                    if message.contains("901s old") && message.contains(&format!("may activate at height {period_start}"))),
                "got {err:?}"
            );
        }
        assert!(
            !read(Some(&too_few), MAX_PRE_V24_TIP_AGE_SECS).expect("no lock-in possible"),
            "an hour-old tip stands when V24 cannot lock in"
        );
    }

    /// A tip older than an hour means the header chain has stalled, and V24
    /// may have activated in blocks the SPV client has not seen, so its
    /// pre-V24 coinbase is not trusted. A tip more than Core's two-hour
    /// future limit ahead of the local clock means the clock is behind and
    /// cannot judge the tip's age. Both refusals are retryable. A version-4
    /// coinbase proves V24 either way.
    #[test]
    fn should_refuse_a_pre_v24_reading_from_a_stalled_spv_tip() {
        let time = 1_792_470_316u32;
        let height = 1_575_845;
        let pre = block_with_coinbase_payload_version(V24_COINBASE_PAYLOAD_VERSION - 1);
        let post = block_with_coinbase_payload_version(V24_COINBASE_PAYLOAD_VERSION);
        let quiet = signal_period(Network::Testnet, 0);
        let read = |block: &Block, now| {
            v24_active_after_tip(
                block,
                &tip_of(block, height, time),
                Network::Testnet,
                now,
                Some(&quiet),
            )
        };
        let refused_with = |now, reason: &str| {
            let err = read(&pre, now).expect_err(reason);
            assert!(
                matches!(&err, PlatformWalletError::TransactionBuild(message) if message.contains(reason)),
                "expected {reason:?}, got {err:?}"
            );
        };
        let stalled = u64::from(time) + MAX_PRE_V24_TIP_AGE_SECS + 1;
        let clock_behind = u64::from(time) - MAX_FUTURE_BLOCK_TIME_SECS - 1;

        refused_with(stalled, "3601s old");
        assert!(!read(&pre, u64::from(time) + MAX_PRE_V24_TIP_AGE_SECS).expect("an hour old"));
        refused_with(clock_behind, "past the local clock");
        refused_with(0, "past the local clock");
        assert!(
            !read(&pre, u64::from(time) - MAX_FUTURE_BLOCK_TIME_SECS).expect("within Core's limit"),
            "a tip timestamped ahead within Core's future limit is recent"
        );

        assert!(read(&post, stalled).expect("a version-4 tip proves V24 however old"));
        assert!(read(&post, clock_behind).expect("whatever the clock says"));
    }

    /// The block comes from DAPI, which is not authenticated, so it is read
    /// only once it is the SPV header's block and its transactions are the
    /// ones that header commits to; otherwise nothing about V24 is
    /// concluded, either way.
    #[test]
    fn should_refuse_a_tip_block_that_does_not_verify_against_the_spv_header() {
        let block = block_with_coinbase_payload_version(V24_COINBASE_PAYLOAD_VERSION);
        let refused = |block: &Block, header_hash: &BlockHash, reason: &str| {
            let err = v24_active_at_block(block, header_hash).expect_err(reason);
            assert!(
                matches!(&err, PlatformWalletError::TransactionBuild(message) if message.contains(reason)),
                "expected {reason:?}, got {err:?}"
            );
        };
        let recommitted = |mut block: Block| {
            block.header.merkle_root = block.compute_merkle_root().expect("non-empty");
            block
        };

        refused(
            &block,
            &BlockHash::from_byte_array([3u8; 32]),
            "is not the SPV header's block",
        );

        let mut swapped = block.clone();
        swapped.txdata[0] =
            block_with_coinbase_payload_version(V24_COINBASE_PAYLOAD_VERSION - 1).txdata[0].clone();
        refused(&swapped, &block.block_hash(), "merkle root");

        let mut without_payload = block.clone();
        without_payload.txdata[0].special_transaction_payload = None;
        let without_payload = recommitted(without_payload);
        refused(
            &without_payload,
            &without_payload.block_hash(),
            "has no coinbase payload",
        );

        let mut spends_an_output = block.clone();
        spends_an_output.txdata[0].input[0].previous_output =
            OutPoint::new(Txid::from_byte_array([1u8; 32]), 0);
        let spends_an_output = recommitted(spends_an_output);
        refused(
            &spends_an_output,
            &spends_an_output.block_hash(),
            "has no coinbase",
        );
    }

    /// A masternode registered without addresses, or whose address map
    /// this wallet cannot read, has nothing to re-assert. It is refused
    /// before anything is funded.
    #[test]
    fn should_refuse_an_entry_with_nothing_to_reassert() {
        let mut empty_map = core_v3_evonode_entry();
        empty_map.service_address = MasternodeNetInfo::Extended(ExtNetInfo {
            version: 1,
            purposes: vec![],
        });
        // An address-less registration reads back as `[::]:0`.
        let legacy = |address: &str| MasternodeListEntry {
            service_address: MasternodeNetInfo::Legacy(address.parse().expect("socket address")),
            ..operator_entry(0x33, false)
        };

        for entry in [
            empty_map,
            legacy("[::]:0"),
            legacy("0.0.0.0:9999"),
            legacy("10.0.0.51:0"),
        ] {
            let err = prepare_update_service_placeholder(
                &entry,
                Network::Devnet,
                true,
                None,
                ScriptBuf::new(),
            )
            .expect_err("nothing to re-assert");
            assert!(
                matches!(&err, PlatformWalletError::InvalidParameter(message) if message.contains("no service address")),
                "got {err:?}"
            );
        }
    }

    /// A version-3 payload is funded and signed like a version-2 one: it
    /// commits to the selected inputs, its operator signature verifies,
    /// and its size, which prices the fee, is the 220 bytes of Core's own
    /// version-3 payload.
    #[tokio::test]
    async fn should_size_and_sign_a_version_3_pro_up_serv_tx() {
        let (core, broadcaster, signer) = core_with_outputs(&[1_000_000]).await;
        let placeholder = ProviderUpdateServicePayload {
            inputs_hash: InputsHash::all_zeros(),
            payload_sig: BLSSignature::from([0u8; 96]),
            ..core_v3_update()
        };
        let placeholder_size =
            TransactionPayload::ProviderUpdateServicePayloadType(placeholder.clone())
                .max_signed_size();

        let prepared =
            build_sign_update_service(&core, placeholder, Zeroizing::new(OPERATOR_SECRET), &signer)
                .await
                .expect("a version-3 update service builds and signs");

        let tx = prepared.transaction();
        let Some(TransactionPayload::ProviderUpdateServicePayloadType(payload)) =
            &tx.special_transaction_payload
        else {
            panic!("the transaction must carry the ProUpServTx payload");
        };
        assert_eq!(payload.version, ProTxVersion::ExtAddr as u16);
        assert_eq!(payload.inputs_hash, tx.hash_inputs());
        assert_operator_signature(payload);
        assert!(
            !tx.output.is_empty(),
            "a ProUpServTx always carries an output"
        );
        assert_eq!(payload.size(), placeholder_size);
        assert_eq!(payload.size(), 220);
        let serialized_len = dashcore::consensus::encode::serialize(tx).len();
        assert!(
            prepared.fee() >= FeeRate::normal().calculate_fee(serialized_len),
            "fee {} pays for {serialized_len} bytes",
            prepared.fee()
        );
        assert_eq!(broadcaster.sent_count(), 0, "preparing must not broadcast");
    }

    /// A payload whose version and address form disagree cannot be
    /// encoded. The finalizer reports that as a build error, which releases
    /// the reservation, instead of panicking: a panic would abort a host
    /// app across the FFI.
    #[test]
    fn should_refuse_rather_than_panic_on_a_version_and_address_mismatch() {
        let version_3_legacy = ProviderUpdateServicePayload {
            version: ProTxVersion::ExtAddr as u16,
            ..regular_placeholder()
        };
        let version_2_extended = ProviderUpdateServicePayload {
            version: ProTxVersion::BasicBLS as u16,
            ..core_v3_update()
        };
        for mismatched in [version_3_legacy, version_2_extended] {
            let unsigned = Transaction {
                version: 3,
                lock_time: 0,
                input: vec![],
                output: vec![dashcore::TxOut {
                    value: 0,
                    script_pubkey: ScriptBuf::new(),
                }],
                special_transaction_payload: Some(
                    TransactionPayload::ProviderUpdateServicePayloadType(mismatched),
                ),
            };
            let err = finalize_update_service_payload(&unsigned, &OPERATOR_SECRET)
                .expect_err("a mismatched payload is refused");
            assert!(
                matches!(&err, BuilderError::InvalidData(message) if message.contains("cannot be encoded")),
                "got {err:?}"
            );
        }
    }

    /// The placeholder re-asserts the list entry's address as a pre-v3
    /// (legacy) entry; its wire encoding is dashcore's.
    #[test]
    fn should_carry_the_entry_address_as_a_legacy_entry() {
        let entry = operator_entry(0x44, false);
        let payload = prepare_update_service_placeholder(
            &entry,
            Network::Devnet,
            false,
            None,
            ScriptBuf::new(),
        )
        .expect("placeholder");
        assert!(matches!(
            entry.service_address,
            MasternodeNetInfo::Legacy(_)
        ));
        assert_eq!(payload.service_address, entry.service_address);
    }

    #[test]
    fn payout_rule_reward_zero_requires_no_address() {
        let script = resolve_operator_payout_script(0, None, Network::Testnet)
            .expect("reward 0 with no address");
        assert!(script.is_empty(), "reward 0 always sends the empty script");

        let dummy = DashAddress::dummy(Network::Testnet, 3).to_string();
        let err = resolve_operator_payout_script(0, Some(&dummy), Network::Testnet)
            .expect_err("reward 0 with an address must be refused");
        assert!(matches!(err, PlatformWalletError::InvalidParameter(_)));
    }

    #[test]
    fn payout_rule_nonzero_reward_requires_an_explicit_address() {
        let err = resolve_operator_payout_script(500, None, Network::Testnet)
            .expect_err("a non-zero reward with no address must be refused, never cleared");
        assert!(matches!(err, PlatformWalletError::InvalidParameter(_)));

        let dummy = DashAddress::dummy(Network::Testnet, 3);
        let script =
            resolve_operator_payout_script(500, Some(&dummy.to_string()), Network::Testnet)
                .expect("explicit address accepted");
        assert_eq!(script, dummy.script_pubkey());
    }

    #[test]
    fn payout_rule_rejects_an_address_for_another_network() {
        let mainnet = DashAddress::dummy(Network::Mainnet, 3).to_string();
        let err = resolve_operator_payout_script(500, Some(&mainnet), Network::Testnet)
            .expect_err("network mismatch must be refused");
        assert!(matches!(err, PlatformWalletError::InvalidParameter(_)));
    }

    #[test]
    fn operator_secret_matches_basic_or_legacy_serialization_only() {
        let (basic, legacy) = bls_public_keys(&OPERATOR_SECRET).expect("valid test scalar");

        verify_operator_secret(&basic, &OPERATOR_SECRET).expect("basic serialization matches");
        verify_operator_secret(&legacy, &OPERATOR_SECRET).expect("legacy serialization matches");

        let err = verify_operator_secret(&[0x42; 48], &OPERATOR_SECRET)
            .expect_err("a different operator key must be refused");
        assert!(matches!(err, PlatformWalletError::InvalidParameter(_)));

        // 0xFF.. exceeds the BLS12-381 scalar field order.
        let err = verify_operator_secret(&basic, &[0xFF; 32])
            .expect_err("an out-of-range scalar must be refused");
        assert!(matches!(err, PlatformWalletError::InvalidParameter(_)));
    }

    #[test]
    fn placeholder_for_a_regular_masternode_omits_the_platform_fields() {
        let entry = operator_entry(0x11, false);
        let payload = prepare_update_service_placeholder(
            &entry,
            Network::Devnet,
            false,
            None,
            ScriptBuf::new(),
        )
        .expect("regular placeholder");
        assert_eq!(
            payload.version,
            ProviderUpdateServicePayload::CURRENT_VERSION
        );
        assert_eq!(
            payload.mn_type,
            Some(ProviderMasternodeType::Regular as u16)
        );
        assert_eq!(
            payload.pro_tx_hash,
            Txid::from_byte_array(entry.pro_reg_tx_hash.to_byte_array())
        );
        assert_eq!(payload.platform_node_id, None);
        assert_eq!(payload.platform_p2p_port, None);
        assert_eq!(payload.platform_http_port, None);
        assert_eq!(payload.inputs_hash, InputsHash::all_zeros());
        assert_eq!(payload.payload_sig, BLSSignature::from([0u8; 96]));

        let err = prepare_update_service_placeholder(
            &entry,
            Network::Devnet,
            false,
            Some(26656),
            ScriptBuf::new(),
        )
        .expect_err("a platform P2P port on a regular masternode must be refused");
        assert!(matches!(err, PlatformWalletError::InvalidParameter(_)));
    }

    #[test]
    fn placeholder_for_an_evonode_requires_and_carries_the_platform_triplet() {
        let entry = operator_entry(0x22, true);
        let EntryMasternodeType::HighPerformance {
            platform_http_port,
            platform_node_id,
        } = entry.mn_type
        else {
            panic!("an evonode entry");
        };
        let payload = prepare_update_service_placeholder(
            &entry,
            Network::Devnet,
            false,
            Some(26656),
            ScriptBuf::new(),
        )
        .expect("evonode placeholder");
        assert_eq!(
            payload.mn_type,
            Some(ProviderMasternodeType::HighPerformance as u16)
        );
        assert_eq!(payload.platform_node_id, Some(platform_node_id));
        assert_eq!(payload.platform_p2p_port, Some(26656));
        assert_eq!(payload.platform_http_port, Some(platform_http_port));

        let err = prepare_update_service_placeholder(
            &entry,
            Network::Devnet,
            false,
            None,
            ScriptBuf::new(),
        )
        .expect_err("an evonode payload without the P2P port must be refused");
        assert!(matches!(err, PlatformWalletError::InvalidParameter(_)));
    }

    fn registration_transaction(operator_reward: u16) -> Transaction {
        use dashcore::blockdata::transaction::special_transaction::provider_registration::ProviderRegistrationPayload;
        use dashcore::bls_sig_utils::BLSPublicKey;
        use dashcore::{OutPoint, PubkeyHash};

        Transaction {
            version: 3,
            lock_time: 0,
            input: vec![],
            output: vec![],
            special_transaction_payload: Some(TransactionPayload::ProviderRegistrationPayloadType(
                ProviderRegistrationPayload {
                    version: ProviderRegistrationPayload::CURRENT_VERSION,
                    masternode_type: ProviderMasternodeType::Regular,
                    masternode_mode: 0,
                    collateral_outpoint: OutPoint {
                        txid: Txid::all_zeros(),
                        vout: 0,
                    },
                    service_address: MasternodeNetInfo::Legacy(
                        "10.0.0.1:9999".parse().expect("socket address"),
                    ),
                    owner_key_hash: PubkeyHash::from_byte_array([1; 20]),
                    operator_public_key: BLSPublicKey::from([2; 48]),
                    voting_key_hash: PubkeyHash::from_byte_array([3; 20]),
                    operator_reward,
                    script_payout: ScriptBuf::new(),
                    inputs_hash: InputsHash::all_zeros(),
                    signature: vec![],
                    platform_node_id: None,
                    platform_p2p_port: None,
                    platform_http_port: None,
                    payouts: None,
                    shared_collateral: None,
                },
            )),
        }
    }

    /// The DAPI get-transaction reply is unauthenticated: the payload is
    /// trusted only after the decoded transaction hashes to the requested
    /// proTxHash.
    #[test]
    fn operator_reward_binds_the_fetched_transaction_to_the_request() {
        let transaction = registration_transaction(500);
        let matching = transaction.txid().to_byte_array();

        let reward = operator_reward_from_registration(&matching, &transaction)
            .expect("a matching registration transaction is accepted");
        assert_eq!(reward, 500);

        let err = operator_reward_from_registration(&[0x99; 32], &transaction)
            .expect_err("a transaction that does not hash to the request must be refused");
        assert!(matches!(err, PlatformWalletError::InvalidIdentityData(_)));

        // The right txid but not a ProRegTx payload.
        let mut not_registration = registration_transaction(0);
        not_registration.special_transaction_payload = None;
        let plain_txid = not_registration.txid().to_byte_array();
        let err = operator_reward_from_registration(&plain_txid, &not_registration)
            .expect_err("a non-registration transaction must be refused");
        assert!(matches!(err, PlatformWalletError::InvalidParameter(_)));
    }

    #[tokio::test]
    async fn builds_signs_and_broadcasts_a_pro_up_serv_tx() {
        let (wallet_manager, wallet_id, generation, signer) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let broadcaster = Arc::new(RecordingBroadcaster::default());
        let core = CoreWallet::new(
            sdk,
            wallet_manager,
            wallet_id,
            broadcaster.clone(),
            generation,
        );

        let entry = operator_entry(0x44, true);
        let placeholder = prepare_update_service_placeholder(
            &entry,
            Network::Devnet,
            false,
            Some(26656),
            ScriptBuf::new(),
        )
        .expect("placeholder");

        let prepared =
            build_sign_update_service(&core, placeholder, Zeroizing::new(OPERATOR_SECRET), &signer)
                .await
                .expect("update service builds and signs");

        // Building signs but must not send: the preview flow shows this exact
        // transaction before the user decides.
        assert!(
            broadcaster
                .sent
                .lock()
                .expect("broadcaster lock")
                .is_empty(),
            "preparing must not broadcast"
        );

        let txid = core
            .broadcast_finalized_transaction(&prepared)
            .await
            .expect("prepared transaction broadcasts");

        let sent = broadcaster.sent.lock().expect("broadcaster lock");
        assert_eq!(sent.len(), 1, "exactly one transaction broadcast");
        let tx = &sent[0];
        assert_eq!(tx.txid(), txid);
        assert_eq!(tx.version, 3);
        assert!(
            !tx.output.is_empty(),
            "a ProUpServTx always carries an output"
        );
        assert!(
            tx.input.iter().all(|input| !input.script_sig.is_empty()),
            "every funding input is ECDSA-signed"
        );

        let Some(TransactionPayload::ProviderUpdateServicePayloadType(payload)) =
            &tx.special_transaction_payload
        else {
            panic!("the broadcast transaction must carry the ProUpServTx payload");
        };
        assert_eq!(
            payload.inputs_hash,
            tx.hash_inputs(),
            "inputs_hash commits to the selected inputs"
        );
        assert_eq!(
            payload.mn_type,
            Some(ProviderMasternodeType::HighPerformance as u16)
        );
        assert_eq!(payload.platform_p2p_port, Some(26656));
        assert_eq!(
            payload.platform_http_port,
            MasternodeListSummary::from_entry(&entry).platform_http_port
        );
        assert_ne!(payload.payload_sig, BLSSignature::from([0u8; 96]));

        assert_operator_signature(payload);
    }

    /// A lone 600-duff UTXO covers the fee but leaves less than the dust
    /// limit over, so no change output is added. The zero-value data output
    /// still gives the transaction an output.
    #[tokio::test]
    async fn should_carry_an_output_when_the_only_utxo_leaves_no_change() {
        let (core, broadcaster, signer) = core_with_outputs(&[600]).await;

        let prepared = build_sign_update_service(
            &core,
            regular_placeholder(),
            Zeroizing::new(OPERATOR_SECRET),
            &signer,
        )
        .await
        .expect("600 duffs pay the fee");

        let tx = prepared.transaction();
        assert_eq!(tx.input.len(), 1);
        assert_eq!(tx.output.len(), 1, "one output even with no change");
        assert!(tx.output[0].script_pubkey.is_op_return());
        assert_eq!(tx.output[0].value, 0, "the data output moves no value");
        assert_eq!(prepared.fee(), 600, "the sub-dust surplus is the fee");
        assert_eq!(broadcaster.sent_count(), 0, "preparing must not broadcast");
    }

    /// Branch-and-bound prefers the small UTXO, whose surplus is booked as
    /// fee; the transaction still carries an output.
    #[tokio::test]
    async fn should_carry_an_output_when_selection_prefers_the_small_utxo() {
        let (core, _broadcaster, signer) = core_with_outputs(&[1_000_000_000, 600]).await;

        let prepared = build_sign_update_service(
            &core,
            regular_placeholder(),
            Zeroizing::new(OPERATOR_SECRET),
            &signer,
        )
        .await
        .expect("update service builds");

        let tx = prepared.transaction();
        assert!(
            !tx.output.is_empty(),
            "a ProUpServTx always carries an output"
        );
        let Some(TransactionPayload::ProviderUpdateServicePayloadType(payload)) =
            &tx.special_transaction_payload
        else {
            panic!("the transaction must carry the ProUpServTx payload");
        };
        assert_eq!(payload.inputs_hash, tx.hash_inputs());
    }

    /// A wallet that cannot cover the fee fails in coin selection, before
    /// anything is reserved, signed or sent.
    #[tokio::test]
    async fn should_fail_before_broadcast_when_the_fee_cannot_be_covered() {
        let (core, broadcaster, signer) = core_with_outputs(&[200]).await;

        let err = build_sign_update_service(
            &core,
            regular_placeholder(),
            Zeroizing::new(OPERATOR_SECRET),
            &signer,
        )
        .await
        .expect_err("200 duffs cannot pay the fee");

        assert!(
            matches!(err, PlatformWalletError::CorePooledInsufficientFunds { .. }),
            "an insufficient-funds error, got {err:?}"
        );
        assert_eq!(broadcaster.sent_count(), 0, "nothing is broadcast");
    }

    /// Without the data output a lone 600-duff UTXO assembles into a
    /// transaction with no outputs. The finalizer refuses it before signing,
    /// and the refusal releases the reservation: the same UTXO funds the
    /// next build.
    #[tokio::test]
    async fn should_release_the_reservation_when_the_transaction_has_no_outputs() {
        let (core, broadcaster, signer) = core_with_outputs(&[600]).await;

        let secret = Zeroizing::new(OPERATOR_SECRET);
        let without_output = TransactionBuilder::new()
            .set_special_payload(TransactionPayload::ProviderUpdateServicePayloadType(
                regular_placeholder(),
            ))
            .set_payload_finalizer(move |unsigned| {
                finalize_update_service_payload(unsigned, &secret)
            });
        let err = core
            .finalize_transaction(without_output, &SEND_FUNDING_SOURCES, 0, &signer)
            .await
            .expect_err("a transaction with no outputs is refused");
        assert!(
            matches!(&err, PlatformWalletError::TransactionBuild(message) if message.contains("no outputs")),
            "the refusal names the missing outputs, got {err:?}"
        );
        assert_eq!(broadcaster.sent_count(), 0, "nothing is broadcast");

        let rebuilt = build_sign_update_service(
            &core,
            regular_placeholder(),
            Zeroizing::new(OPERATOR_SECRET),
            &signer,
        )
        .await
        .expect("the released UTXO funds the next build");
        assert_eq!(rebuilt.transaction().input.len(), 1);
    }
}
