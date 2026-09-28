//! ProUpServTx (provider update service) orchestration.
//!
//! Asserts a masternode's or evonode's service values in a ProUpServTx signed
//! with the operator BLS key, which also revives the node if it is
//! PoSe-banned. The payload commits to the funding inputs (`inputs_hash`) and
//! each funding input's ECDSA sighash covers the finished payload, so the
//! build order is fixed: select and reserve inputs → write `inputs_hash` →
//! BLS-sign the payload → ECDSA-sign the inputs → broadcast. key-wallet's
//! `TransactionBuilder::set_payload_finalizer` is the seam that makes steps
//! two and three possible between selection and input signing.
//!
//! The service values are an explicit input the caller confirms
//! ([`ConfirmedMasternodeService`]), and the payload is built from that input
//! alone. [`masternode_update_service_suggestion`] reads what the synced
//! masternode list shows so a host can prefill its form; that is a hint the
//! user confirms, never a payload source. The confirmed masternode type is
//! checked against the registration transaction, and the payout script
//! follows a hard rule (see [`resolve_operator_payout_script`]) so an update
//! can never silently clear an operator payout on-chain.

use dashcore::blockdata::script::ScriptBuf;
use dashcore::blockdata::transaction::special_transaction::provider_registration::ProviderMasternodeType;
use dashcore::blockdata::transaction::special_transaction::provider_update_service::ProviderUpdateServicePayload;
use dashcore::blockdata::transaction::special_transaction::{
    SpecialTransactionBasePayloadEncodable, TransactionPayload,
};
use dashcore::bls_sig_utils::BLSSignature;
use dashcore::blsful::{Bls12381G2Impl, SecretKey as BlsSecretKey, SignatureSchemes};
use dashcore::hash_types::InputsHash;
use dashcore::hashes::Hash;
use dashcore::platform_node_id::PlatformNodeId;
use dashcore::{Address as DashAddress, Network, Transaction, Txid};
use key_wallet::wallet::managed_wallet_info::transaction_builder::{
    BuilderError, TransactionBuilder, TransactionSigner,
};
use std::net::{IpAddr, SocketAddr};
use zeroize::Zeroizing;

use super::list::MasternodeListSummary;
use super::locator::bls_public_keys;
use crate::broadcaster::TransactionBroadcaster;
use crate::error::PlatformWalletError;
use crate::spv::SpvRuntime;
use crate::wallet::core::{CoreWallet, SignedCoreTransaction, SEND_FUNDING_SOURCES};
use crate::wallet::platform_wallet::PlatformWallet;

/// What an update-service request asserts and chooses. Everything the
/// payload carries comes from here; nothing is copied from the synced
/// masternode list.
#[derive(Debug, Clone)]
pub struct MasternodeUpdateServiceParams {
    /// ProRegTx hash of the masternode to update, in WIRE order (the same
    /// order `MasternodeListSummary::pro_tx_hash` uses).
    pub pro_tx_hash: [u8; 32],
    /// The service values the payload asserts, as the caller confirmed them.
    pub service: ConfirmedMasternodeService,
    /// Operator payout address. Consensus REPLACES the current operator
    /// payout script with this payload's, and an empty script clears it —
    /// see [`resolve_operator_payout_script`] for the rule that keeps that
    /// from happening silently.
    pub operator_payout_address: Option<String>,
}

/// The service values a ProUpServTx asserts, as the caller confirmed them.
///
/// Hosts may prefill these from [`masternode_update_service_suggestion`],
/// but the operator is the one asserting them, so every value must be shown
/// to the user and confirmed (or corrected) before it is passed in. The
/// variant must match how the masternode was registered; the update refuses
/// a mismatch against the registration transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmedMasternodeService {
    /// A regular masternode: the Core P2P endpoint only.
    Regular {
        /// Core P2P endpoint. The version-2 payload carries an IPv4 address.
        service_address: SocketAddr,
    },
    /// An evonode: the Core P2P endpoint plus the platform fields the
    /// payload serializes for the high-performance type.
    Evonode {
        /// Core P2P endpoint. The version-2 payload carries an IPv4 address.
        service_address: SocketAddr,
        /// Tenderdash node id (`SHA256(ed25519 pk)[..20]`).
        platform_node_id: [u8; 20],
        /// Tenderdash P2P port.
        platform_p2p_port: u16,
        /// Platform HTTP (DAPI) port.
        platform_http_port: u16,
    },
}

impl ConfirmedMasternodeService {
    /// The Core P2P endpoint, whichever the type.
    pub fn service_address(&self) -> SocketAddr {
        match self {
            Self::Regular { service_address }
            | Self::Evonode {
                service_address, ..
            } => *service_address,
        }
    }

    /// Whether this is the evonode (high-performance) shape.
    pub fn is_evonode(&self) -> bool {
        matches!(self, Self::Evonode { .. })
    }
}

/// What the synced masternode list currently shows for a masternode's
/// service, for a host to prefill its update-service form.
///
/// A hint only. The host must show these values and have the user confirm
/// or correct them; the confirmed values, not these, become the
/// [`ConfirmedMasternodeService`] the payload is built from. The list does
/// not carry the platform P2P port, so an evonode's always has to be
/// entered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MasternodeServiceSuggestion {
    /// Primary Core P2P endpoint. `None` for Tor / I2P / CJDNS /
    /// domain-only entries, which have no IP:port form.
    pub service_address: Option<SocketAddr>,
    /// The list shows a high-performance (evonode) entry.
    pub is_evonode: bool,
    /// Tenderdash node id, evonodes only.
    pub platform_node_id: Option<[u8; 20]>,
    /// Platform HTTP (DAPI) port, evonodes only.
    pub platform_http_port: Option<u16>,
    /// The list shows the masternode PoSe-banned. When it does not, a host
    /// may say so before the user sends: updating a healthy node's service
    /// is legitimate, so nothing refuses on it.
    pub pose_banned: bool,
    /// The list shows v3 extended network info. The update refuses such an
    /// entry, since a version-2 payload would replace its whole endpoint map
    /// with one address, so a host can explain that before the user fills
    /// in the form.
    pub has_extended_net_info: bool,
}

impl MasternodeServiceSuggestion {
    /// The suggestion for one list entry.
    pub fn from_summary(entry: &MasternodeListSummary) -> Self {
        Self {
            service_address: entry.service_address,
            is_evonode: entry.is_evonode,
            platform_node_id: entry.platform_node_id,
            platform_http_port: entry.platform_http_port,
            pose_banned: !entry.is_valid,
            has_extended_net_info: entry.has_extended_net_info,
        }
    }
}

/// What the synced masternode list shows for `pro_tx_hash` (wire order), as
/// a [`MasternodeServiceSuggestion`] a host prefills its form with and the
/// user then confirms. `None` when the list has no such masternode; errors
/// with [`PlatformWalletError::MasternodeListUnavailable`] before the list
/// has synced. Read-only and local.
pub async fn masternode_update_service_suggestion(
    spv: &SpvRuntime,
    pro_tx_hash: &[u8; 32],
) -> Result<Option<MasternodeServiceSuggestion>, PlatformWalletError> {
    let summaries = spv
        .masternode_list_summaries()
        .await
        .ok_or(PlatformWalletError::MasternodeListUnavailable)?;
    Ok(summaries
        .iter()
        .find(|entry| &entry.pro_tx_hash == pro_tx_hash)
        .map(MasternodeServiceSuggestion::from_summary))
}

/// Parse a host-entered `"a.b.c.d:port"` service address. The port is
/// required: the Core P2P port differs between mainnet and the test
/// networks, so it is never guessed.
pub fn parse_service_address(text: &str) -> Result<SocketAddr, PlatformWalletError> {
    let text = text.trim();
    text.parse::<SocketAddr>().map_err(|_| {
        if text.parse::<IpAddr>().is_ok() {
            PlatformWalletError::InvalidParameter(format!(
                "the service address {text} needs a port, as IP:port"
            ))
        } else {
            PlatformWalletError::InvalidParameter(format!(
                "{text:?} is not an IP:port service address"
            ))
        }
    })
}

/// Build, operator-BLS-sign, fund, input-sign, and broadcast a ProUpServTx
/// asserting `params.service` for `params.pro_tx_hash` (the transaction
/// Core produces for `protx update_service`), which also revives the
/// masternode if it is PoSe-banned.
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
/// Whether the list shows the node PoSe-banned is reported by
/// [`masternode_update_service_suggestion`], not checked here: updating a
/// healthy node's service is legitimate.
pub async fn prepare_masternode_update_service<S: TransactionSigner + ?Sized + Sync>(
    wallet: &PlatformWallet,
    spv: &SpvRuntime,
    params: MasternodeUpdateServiceParams,
    operator_secret: Zeroizing<[u8; 32]>,
    signer: &S,
) -> Result<SignedCoreTransaction, PlatformWalletError> {
    validate_confirmed_service(&params.service)?;

    let summaries = spv
        .masternode_list_summaries()
        .await
        .ok_or(PlatformWalletError::MasternodeListUnavailable)?;
    let entry = summaries
        .iter()
        .find(|entry| entry.pro_tx_hash == params.pro_tx_hash)
        .ok_or_else(|| {
            PlatformWalletError::InvalidParameter(format!(
                "masternode {} is not in the masternode list",
                display_hex(&params.pro_tx_hash)
            ))
        })?;
    check_list_entry(entry, &operator_secret)?;

    let registration = fetch_registration_terms(wallet, &params.pro_tx_hash).await?;
    let placeholder = update_service_placeholder(&registration, &params, wallet.network())?;

    build_sign_update_service(wallet.core(), placeholder, operator_secret, signer).await
}

/// The preflights that read the synced masternode list. Both only ever
/// refuse; neither feeds the payload.
///
/// - The operator secret must match the entry's operator public key, so a
///   wrong key fails here rather than as a payload signature the network
///   refuses.
/// - An entry advertising v3 extended network info is refused: the
///   version-2 payload cannot express an endpoint map, and Core would
///   replace the whole map with its single address, discarding live
///   endpoints. It stays refused until a v3 ProUpServTx payload exists.
pub(crate) fn check_list_entry(
    entry: &MasternodeListSummary,
    operator_secret: &[u8; 32],
) -> Result<(), PlatformWalletError> {
    verify_operator_secret(&entry.operator_public_key, operator_secret)?;
    if entry.has_extended_net_info {
        return Err(PlatformWalletError::InvalidParameter(
            "this masternode advertises v3 extended network info; a version-2 update-service \
             payload would replace its whole endpoint map with a single address, so it cannot \
             be updated from this wallet yet"
                .to_string(),
        ));
    }
    Ok(())
}

/// What the ProRegTx fixed for the masternode's lifetime, read from the
/// fetched registration transaction once it is bound to the requested
/// proTxHash.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RegistrationTerms {
    /// `operatorReward` in basis points.
    pub(crate) operator_reward: u16,
    /// Regular or evonode. No transaction changes it after registration,
    /// and a ProUpServTx must carry the same type.
    pub(crate) masternode_type: ProviderMasternodeType,
}

/// The [`RegistrationTerms`] of `pro_tx_hash`, from its ProRegTx fetched via
/// DAPI Core. Fails closed when the transaction cannot be fetched: neither
/// the payout rule nor the type check can be applied without it.
async fn fetch_registration_terms(
    wallet: &PlatformWallet,
    pro_tx_hash: &[u8; 32],
) -> Result<RegistrationTerms, PlatformWalletError> {
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
    registration_terms(pro_tx_hash, &fetched.transaction)
}

/// Read the [`RegistrationTerms`] out of a fetched registration transaction,
/// binding the response to the request first: DAPI's get-transaction reply
/// is not authenticated, so the decoded transaction must hash to the
/// requested proTxHash before its payload is trusted. Without this check a
/// reply carrying an unrelated zero-reward ProRegTx would steer
/// [`resolve_operator_payout_script`] into clearing a real operator payout.
pub(crate) fn registration_terms(
    pro_tx_hash: &[u8; 32],
    transaction: &Transaction,
) -> Result<RegistrationTerms, PlatformWalletError> {
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
            Ok(RegistrationTerms {
                operator_reward: registration.operator_reward,
                masternode_type: registration.masternode_type,
            })
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

/// Refuse confirmed service values no ProUpServTx can carry, before any
/// network work. Core validates the rest (routability, each network's
/// required ports, uniqueness across the list) when the transaction
/// arrives; these are the checks a mistyped value most often fails, and
/// catching them here keeps a transaction the network would refuse from
/// being built at all.
pub(crate) fn validate_confirmed_service(
    service: &ConfirmedMasternodeService,
) -> Result<(), PlatformWalletError> {
    let address = service.service_address();
    let ipv4 = match address.ip() {
        IpAddr::V4(v4) => Some(v4),
        IpAddr::V6(v6) => v6.to_ipv4_mapped(),
    };
    let Some(ipv4) = ipv4 else {
        return Err(PlatformWalletError::InvalidParameter(format!(
            "the service address {address} is not IPv4; a version-2 update-service payload \
             carries an IPv4 address"
        )));
    };
    if ipv4.is_unspecified() || address.port() == 0 {
        return Err(PlatformWalletError::InvalidParameter(format!(
            "the service address {address} needs a real IP and port"
        )));
    }
    if let ConfirmedMasternodeService::Evonode {
        platform_node_id,
        platform_p2p_port,
        platform_http_port,
        ..
    } = service
    {
        if platform_node_id == &[0u8; 20] {
            return Err(PlatformWalletError::InvalidParameter(
                "an evonode payload needs the platform node id".to_string(),
            ));
        }
        if *platform_p2p_port == 0 || *platform_http_port == 0 {
            return Err(PlatformWalletError::InvalidParameter(
                "an evonode payload needs both the platform P2P port and the platform HTTP port"
                    .to_string(),
            ));
        }
        if platform_p2p_port == platform_http_port
            || *platform_p2p_port == address.port()
            || *platform_http_port == address.port()
        {
            return Err(PlatformWalletError::InvalidParameter(
                "the Core P2P port, platform P2P port and platform HTTP port must all differ"
                    .to_string(),
            ));
        }
    }
    Ok(())
}

/// Refuse a confirmed service whose type differs from the registration's.
/// The type is fixed at registration and a ProUpServTx must repeat it, so
/// the registration transaction (bound to the proTxHash) decides.
pub(crate) fn check_registered_type(
    registered: ProviderMasternodeType,
    service: &ConfirmedMasternodeService,
) -> Result<(), PlatformWalletError> {
    match (registered, service.is_evonode()) {
        (ProviderMasternodeType::Regular, false)
        | (ProviderMasternodeType::HighPerformance, true) => Ok(()),
        (ProviderMasternodeType::HighPerformance, false) => {
            Err(PlatformWalletError::InvalidParameter(
                "this masternode is registered as an evonode; confirm its platform node id, \
                 platform P2P port and platform HTTP port"
                    .to_string(),
            ))
        }
        (ProviderMasternodeType::Regular, true) => Err(PlatformWalletError::InvalidParameter(
            "this masternode is registered as a regular masternode, so it has no platform \
             fields to update"
                .to_string(),
        )),
    }
}

/// The placeholder payload for `params`, checked against the registration:
/// the confirmed service values, the registered type, and the payout script
/// the payout rule resolves. Nothing here reads the masternode list.
pub(crate) fn update_service_placeholder(
    registration: &RegistrationTerms,
    params: &MasternodeUpdateServiceParams,
    network: Network,
) -> Result<ProviderUpdateServicePayload, PlatformWalletError> {
    check_registered_type(registration.masternode_type, &params.service)?;
    let script_payout = resolve_operator_payout_script(
        registration.operator_reward,
        params.operator_payout_address.as_deref(),
        network,
    )?;
    prepare_update_service_placeholder(params.pro_tx_hash, &params.service, script_payout)
}

/// The placeholder payload for the builder, built from the confirmed
/// service values alone: every field final except the two
/// selection-dependent ones (`inputs_hash`, `payload_sig`), which are zeroed
/// and filled by the payload finalizer after input selection. Always version
/// 2 (BasicBLS) — every current network is past the v19 hard fork.
pub(crate) fn prepare_update_service_placeholder(
    pro_tx_hash: [u8; 32],
    service: &ConfirmedMasternodeService,
    script_payout: ScriptBuf,
) -> Result<ProviderUpdateServicePayload, PlatformWalletError> {
    validate_confirmed_service(service)?;
    let (ip_address, port) = service_payload_fields(service.service_address());

    // The platform triplet is serialized only when mn_type is HighPerformance,
    // so the evonode/regular split must be explicit here — a missing mn_type
    // would silently drop the platform fields on the wire.
    let (mn_type, platform_node_id, platform_p2p_port, platform_http_port) = match service {
        ConfirmedMasternodeService::Evonode {
            platform_node_id,
            platform_p2p_port,
            platform_http_port,
            ..
        } => (
            Some(ProviderMasternodeType::HighPerformance as u16),
            Some(PlatformNodeId::from_byte_array(*platform_node_id)),
            Some(*platform_p2p_port),
            Some(*platform_http_port),
        ),
        ConfirmedMasternodeService::Regular { .. } => (
            Some(ProviderMasternodeType::Regular as u16),
            None,
            None,
            None,
        ),
    };

    Ok(ProviderUpdateServicePayload::new(
        mn_type,
        Txid::from_byte_array(pro_tx_hash),
        ip_address,
        port,
        script_payout,
        InputsHash::all_zeros(),
        platform_node_id,
        platform_p2p_port,
        platform_http_port,
        BLSSignature::from([0u8; 96]),
    ))
}

/// A service socket address as the payload encodes it: the IPv6 (or
/// IPv4-mapped-IPv6) octets as a little-endian `u128`, and the port in host
/// order (the payload serializer byte-swaps it on the wire).
pub(crate) fn service_payload_fields(service: SocketAddr) -> (u128, u16) {
    let octets = match service.ip() {
        IpAddr::V4(v4) => v4.to_ipv6_mapped().octets(),
        IpAddr::V6(v6) => v6.octets(),
    };
    (u128::from_le_bytes(octets), service.port())
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
/// the operator-BLS `payload_sig` (basic scheme over `base_payload_hash()`,
/// modern serialization: the exact convention `verify_message_digest`
/// checks real mainnet signatures with).
///
/// Refuses a transaction without outputs before signing anything. key-wallet
/// releases the build's reservation on any finalizer error, so the refusal
/// leaves the selected inputs spendable.
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

    let secret = Option::<BlsSecretKey<Bls12381G2Impl>>::from(
        BlsSecretKey::<Bls12381G2Impl>::from_be_bytes(operator_secret),
    )
    .ok_or_else(|| {
        BuilderError::SigningFailed("the operator key is not a valid BLS secret".into())
    })?;
    let signature = secret
        .sign(
            SignatureSchemes::Basic,
            finalized.base_payload_hash().as_byte_array(),
        )
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
    use super::super::list::test_support::{evonode, masternode};
    use super::*;
    use crate::broadcaster::BroadcastError;
    use crate::test_support::{funded_wallet_manager, funded_wallet_manager_with_outputs};
    use dashcore::blsful::{PublicKey as BlsPublicKey, Signature as BlsSignature};
    use key_wallet::account::StandardAccountType;
    use std::sync::{Arc, Mutex};

    /// A fixed valid BLS12-381 secret scalar (big-endian, below the group
    /// order) so the test keypair is deterministic.
    const OPERATOR_SECRET: [u8; 32] = [7u8; 32];

    fn operator_entry(seed: u8, evo: bool) -> MasternodeListSummary {
        let (basic, _) = bls_public_keys(&OPERATOR_SECRET).expect("valid test scalar");
        let mut entry = if evo { evonode(seed) } else { masternode(seed) };
        entry.operator_public_key = basic;
        entry
    }

    /// A confirmed regular service at a documentation address, distinct
    /// from every `masternode(seed)` list fixture (`10.0.0.<seed>:9999`).
    fn confirmed_regular() -> ConfirmedMasternodeService {
        ConfirmedMasternodeService::Regular {
            service_address: "203.0.113.7:19999".parse().expect("socket address"),
        }
    }

    /// A confirmed evonode service whose every field differs from the
    /// `evonode(seed)` list fixture (node id `seed ^ 0xFF`, HTTP port 443).
    fn confirmed_evonode() -> ConfirmedMasternodeService {
        ConfirmedMasternodeService::Evonode {
            service_address: "203.0.113.8:19999".parse().expect("socket address"),
            platform_node_id: [0x5A; 20],
            platform_p2p_port: 36656,
            platform_http_port: 1443,
        }
    }

    fn params(
        pro_tx_hash: [u8; 32],
        service: ConfirmedMasternodeService,
    ) -> MasternodeUpdateServiceParams {
        MasternodeUpdateServiceParams {
            pro_tx_hash,
            service,
            operator_payout_address: None,
        }
    }

    fn registered(masternode_type: ProviderMasternodeType) -> RegistrationTerms {
        RegistrationTerms {
            operator_reward: 0,
            masternode_type,
        }
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
        crate::test_support::WalletSigner,
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
        prepare_update_service_placeholder([0x44; 32], &confirmed_regular(), ScriptBuf::new())
            .expect("placeholder")
    }

    /// The IPv4-mapped little-endian encoding, pinned against the known
    /// testnet ProUpServTx vector in dashcore's own payload tests
    /// (52.36.64.148:19999).
    #[test]
    fn service_fields_match_the_known_testnet_vector() {
        let service: SocketAddr = "52.36.64.148:19999".parse().expect("socket address");
        let (ip_address, port) = service_payload_fields(service);
        let expected: [u8; 16] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xFF, 0xFF, 52, 36, 64, 148];
        assert_eq!(ip_address.to_le_bytes(), expected);
        assert_eq!(port, 19999);
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

    /// The list entry sits at `10.0.0.17:9999`; the caller confirmed
    /// `203.0.113.7:19999`. The list preflights pass and the payload carries
    /// the confirmed address, because the placeholder never reads the list.
    #[test]
    fn should_build_the_payload_from_the_confirmed_address_not_the_list_entry() {
        let entry = operator_entry(0x11, false);
        let listed = entry.service_address.expect("fixture has a plain address");
        let confirmed = confirmed_regular();
        assert_ne!(
            listed,
            confirmed.service_address(),
            "fixture addresses differ"
        );

        check_list_entry(&entry, &OPERATOR_SECRET).expect("list preflights pass");
        let payload = update_service_placeholder(
            &registered(ProviderMasternodeType::Regular),
            &params(entry.pro_tx_hash, confirmed),
            Network::Testnet,
        )
        .expect("regular placeholder");

        assert_eq!(
            (payload.ip_address, payload.port),
            service_payload_fields(confirmed.service_address()),
            "the payload asserts the confirmed address"
        );
        assert_ne!(
            (payload.ip_address, payload.port),
            service_payload_fields(listed),
            "the list entry's address never reaches the payload"
        );
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
            Txid::from_byte_array(entry.pro_tx_hash)
        );
        assert_eq!(payload.platform_node_id, None);
        assert_eq!(payload.platform_p2p_port, None);
        assert_eq!(payload.platform_http_port, None);
        assert_eq!(payload.inputs_hash, InputsHash::all_zeros());
        assert_eq!(payload.payload_sig, BLSSignature::from([0u8; 96]));
    }

    /// Every evonode field comes from the confirmed input: the list
    /// fixture's node id and HTTP port differ from the confirmed ones.
    #[test]
    fn should_take_every_evonode_field_from_the_confirmed_input() {
        let entry = operator_entry(0x22, true);
        let ConfirmedMasternodeService::Evonode {
            service_address,
            platform_node_id,
            platform_p2p_port,
            platform_http_port,
        } = confirmed_evonode()
        else {
            panic!("confirmed_evonode is the evonode shape");
        };
        assert_ne!(entry.platform_node_id, Some(platform_node_id));
        assert_ne!(entry.platform_http_port, Some(platform_http_port));

        check_list_entry(&entry, &OPERATOR_SECRET).expect("list preflights pass");
        let payload = update_service_placeholder(
            &registered(ProviderMasternodeType::HighPerformance),
            &params(entry.pro_tx_hash, confirmed_evonode()),
            Network::Testnet,
        )
        .expect("evonode placeholder");

        assert_eq!(
            payload.mn_type,
            Some(ProviderMasternodeType::HighPerformance as u16)
        );
        assert_eq!(
            (payload.ip_address, payload.port),
            service_payload_fields(service_address)
        );
        assert_eq!(
            payload.platform_node_id,
            Some(PlatformNodeId::from_byte_array(platform_node_id))
        );
        assert_eq!(payload.platform_p2p_port, Some(platform_p2p_port));
        assert_eq!(payload.platform_http_port, Some(platform_http_port));
    }

    /// The registration fixes the type, so a confirmed service of the other
    /// shape is refused before any payload exists.
    #[test]
    fn should_refuse_a_service_type_that_differs_from_the_registration() {
        let err = update_service_placeholder(
            &registered(ProviderMasternodeType::HighPerformance),
            &params([0x33; 32], confirmed_regular()),
            Network::Testnet,
        )
        .expect_err("a registered evonode needs its platform fields");
        assert!(matches!(err, PlatformWalletError::InvalidParameter(_)));

        let err = update_service_placeholder(
            &registered(ProviderMasternodeType::Regular),
            &params([0x33; 32], confirmed_evonode()),
            Network::Testnet,
        )
        .expect_err("a registered regular masternode has no platform fields");
        assert!(matches!(err, PlatformWalletError::InvalidParameter(_)));
    }

    #[test]
    fn should_refuse_confirmed_service_values_no_payload_can_carry() {
        let regular = |address: &str| ConfirmedMasternodeService::Regular {
            service_address: address.parse().expect("socket address"),
        };
        for address in ["[2001:db8::1]:19999", "0.0.0.0:19999", "203.0.113.7:0"] {
            let err = validate_confirmed_service(&regular(address))
                .expect_err("an address no version-2 payload can carry");
            assert!(
                matches!(err, PlatformWalletError::InvalidParameter(_)),
                "{address}"
            );
        }
        validate_confirmed_service(&regular("[::ffff:203.0.113.7]:19999"))
            .expect("an IPv4-mapped address is IPv4");

        let evonode =
            |node_id: [u8; 20], p2p: u16, http: u16| ConfirmedMasternodeService::Evonode {
                service_address: "203.0.113.8:19999".parse().expect("socket address"),
                platform_node_id: node_id,
                platform_p2p_port: p2p,
                platform_http_port: http,
            };
        for (label, service) in [
            ("empty node id", evonode([0; 20], 36656, 1443)),
            ("no P2P port", evonode([1; 20], 0, 1443)),
            ("no HTTP port", evonode([1; 20], 36656, 0)),
            ("P2P equals HTTP", evonode([1; 20], 1443, 1443)),
            ("P2P equals Core", evonode([1; 20], 19999, 1443)),
            ("HTTP equals Core", evonode([1; 20], 36656, 19999)),
        ] {
            let err = validate_confirmed_service(&service).expect_err(label);
            assert!(
                matches!(err, PlatformWalletError::InvalidParameter(_)),
                "{label}"
            );
        }
        validate_confirmed_service(&confirmed_evonode()).expect("a complete evonode service");
    }

    #[test]
    fn should_parse_a_service_address_only_with_a_port() {
        assert_eq!(
            parse_service_address(" 203.0.113.7:19999 ").expect("IP:port"),
            "203.0.113.7:19999"
                .parse::<SocketAddr>()
                .expect("socket address")
        );
        for text in ["203.0.113.7", "node.example:19999", ""] {
            let err = parse_service_address(text).expect_err(text);
            assert!(matches!(err, PlatformWalletError::InvalidParameter(_)));
        }
    }

    /// A v2 payload would replace the whole endpoint map with one address,
    /// so the list's extended-net-info flag refuses even though a routable
    /// primary exists.
    #[test]
    fn should_refuse_a_list_entry_with_extended_net_info() {
        let mut entry = operator_entry(0x55, false);
        entry.has_extended_net_info = true;
        assert!(
            entry.service_address.is_some(),
            "primary address present and routable"
        );
        let err = check_list_entry(&entry, &OPERATOR_SECRET)
            .expect_err("an extended-net-info entry must be refused");
        assert!(matches!(err, PlatformWalletError::InvalidParameter(_)));
    }

    /// The caller supplies the address, so a list entry without a plain
    /// IP:port (a Tor-only entry, say) no longer blocks the update.
    #[test]
    fn should_accept_a_list_entry_without_a_plain_service_address() {
        let mut entry = operator_entry(0x33, false);
        entry.service_address = None;
        check_list_entry(&entry, &OPERATOR_SECRET).expect("the list address is not needed");
    }

    #[test]
    fn should_suggest_the_list_entry_with_its_banned_status() {
        let mut entry = evonode(0x66);
        entry.is_valid = false;
        let suggestion = MasternodeServiceSuggestion::from_summary(&entry);
        assert_eq!(suggestion.service_address, entry.service_address);
        assert!(suggestion.is_evonode);
        assert_eq!(suggestion.platform_node_id, entry.platform_node_id);
        assert_eq!(suggestion.platform_http_port, entry.platform_http_port);
        assert!(suggestion.pose_banned);
        assert!(!suggestion.has_extended_net_info);

        let healthy = MasternodeServiceSuggestion::from_summary(&masternode(0x67));
        assert!(
            !healthy.pose_banned,
            "a valid entry is reported as not banned"
        );
        assert!(!healthy.is_evonode);
        assert_eq!(healthy.platform_node_id, None);
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
                    service_address: "10.0.0.1:9999".parse().expect("socket address"),
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
                },
            )),
        }
    }

    /// The DAPI get-transaction reply is unauthenticated: the payload is
    /// trusted only after the decoded transaction hashes to the requested
    /// proTxHash.
    #[test]
    fn should_bind_the_registration_to_the_requested_pro_tx_hash() {
        let transaction = registration_transaction(500);
        let matching = transaction.txid().to_byte_array();

        let terms = registration_terms(&matching, &transaction)
            .expect("a matching registration transaction is accepted");
        assert_eq!(
            terms,
            RegistrationTerms {
                operator_reward: 500,
                masternode_type: ProviderMasternodeType::Regular,
            }
        );

        let err = registration_terms(&[0x99; 32], &transaction)
            .expect_err("a transaction that does not hash to the request must be refused");
        assert!(matches!(err, PlatformWalletError::InvalidIdentityData(_)));

        // The right txid but not a ProRegTx payload.
        let mut not_registration = registration_transaction(0);
        not_registration.special_transaction_payload = None;
        let plain_txid = not_registration.txid().to_byte_array();
        let err = registration_terms(&plain_txid, &not_registration)
            .expect_err("a non-registration transaction must be refused");
        assert!(matches!(err, PlatformWalletError::InvalidParameter(_)));
    }

    #[tokio::test]
    async fn should_build_sign_and_broadcast_a_pro_up_serv_tx() {
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

        let placeholder =
            prepare_update_service_placeholder([0x44; 32], &confirmed_evonode(), ScriptBuf::new())
                .expect("placeholder");

        let prepared =
            build_sign_update_service(&core, placeholder, Zeroizing::new(OPERATOR_SECRET), &signer)
                .await
                .expect("update service builds and signs");

        // Building signs but must not send: the preview flow shows this exact
        // transaction before the user decides.
        assert_eq!(broadcaster.sent_count(), 0, "preparing must not broadcast");

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
        assert_eq!(payload.platform_p2p_port, Some(36656));
        assert_eq!(payload.platform_http_port, Some(1443));
        assert_ne!(payload.payload_sig, BLSSignature::from([0u8; 96]));

        // The payload signature verifies under the basic scheme against the
        // operator public key, over base_payload_hash: the exact convention
        // `verify_message_digest` checks real mainnet signatures with.
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
