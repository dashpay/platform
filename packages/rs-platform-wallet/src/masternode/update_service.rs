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
//! script is copied verbatim from the live masternode-list entry, and the
//! payout script follows a hard rule (see
//! [`resolve_operator_payout_script`]) so an unban can never silently clear
//! an operator payout on-chain.

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
use dashcore::{Address as DashAddress, Network, Txid};
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

/// What an update-service (unban) request lets the caller choose. Everything
/// else — service address, masternode type, platform node id and HTTP port —
/// is copied from the live masternode-list entry.
#[derive(Debug, Clone)]
pub struct MasternodeUpdateServiceParams {
    /// ProRegTx hash of the masternode to update, in WIRE order (the same
    /// order `MasternodeListSummary::pro_tx_hash` uses).
    pub pro_tx_hash: [u8; 32],
    /// Platform P2P port for an evonode payload. The masternode list does
    /// not carry it, so the caller must supply it for an evonode; it must be
    /// `None` for a regular masternode.
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
pub async fn prepare_masternode_update_service<S: TransactionSigner + ?Sized + Sync>(
    wallet: &PlatformWallet,
    spv: &SpvRuntime,
    params: MasternodeUpdateServiceParams,
    operator_secret: Zeroizing<[u8; 32]>,
    signer: &S,
) -> Result<SignedCoreTransaction, PlatformWalletError> {
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

    verify_operator_secret(&entry.operator_public_key, &operator_secret)?;

    let operator_reward = fetch_operator_reward(wallet, &params.pro_tx_hash).await?;
    let script_payout = resolve_operator_payout_script(
        operator_reward,
        params.operator_payout_address.as_deref(),
        wallet.network(),
    )?;

    let placeholder =
        prepare_update_service_placeholder(entry, params.platform_p2p_port, script_payout)?;

    build_sign_update_service(wallet.core(), placeholder, operator_secret, signer).await
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

/// Read `operatorReward` out of a fetched registration transaction —
/// binding the response to the request first: DAPI's get-transaction reply
/// is not authenticated, so the decoded transaction must hash to the
/// SPV-authenticated proTxHash before its payload is trusted. Without this
/// check a faulty or malicious endpoint could answer with an unrelated
/// zero-reward ProRegTx and steer [`resolve_operator_payout_script`] into
/// clearing a real operator payout.
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

/// The placeholder payload for the builder: every field final except the two
/// selection-dependent ones (`inputs_hash`, `payload_sig`), which are zeroed
/// and filled by the payload finalizer after input selection. Always version
/// 2 (BasicBLS) — every current network is past the v19 hard fork.
pub(crate) fn prepare_update_service_placeholder(
    entry: &MasternodeListSummary,
    platform_p2p_port: Option<u16>,
    script_payout: ScriptBuf,
) -> Result<ProviderUpdateServicePayload, PlatformWalletError> {
    // A v3 extended entry advertises an endpoint map the version-2 payload
    // cannot express: Core would replace the whole map with the single
    // address below, downgrading the entry and discarding live endpoints.
    // Refuse until a v3 ProUpServTx payload exists to re-assert it.
    if entry.has_extended_net_info {
        return Err(PlatformWalletError::InvalidParameter(
            "this masternode advertises v3 extended network info; a version-2 update-service \
             payload would replace its whole endpoint map with a single address, so it cannot \
             be re-asserted from this wallet yet"
                .to_string(),
        ));
    }
    let service = entry.service_address.ok_or_else(|| {
        PlatformWalletError::InvalidParameter(
            "the masternode's service address is not a plain IP:port entry, so it cannot be \
             re-asserted from the masternode list"
                .to_string(),
        )
    })?;
    let (ip_address, port) = service_payload_fields(service);

    // The platform triplet is serialized only when mn_type is HighPerformance,
    // so the evonode/regular split must be explicit here — a missing mn_type
    // would silently drop the platform fields on the wire.
    let (mn_type, platform_node_id, platform_p2p_port, platform_http_port) = if entry.is_evonode {
        let node_id = entry.platform_node_id.ok_or_else(|| {
            PlatformWalletError::InvalidParameter(
                "the masternode list entry is an evonode without a platform node id".to_string(),
            )
        })?;
        let http_port = entry.platform_http_port.ok_or_else(|| {
            PlatformWalletError::InvalidParameter(
                "the masternode list entry is an evonode without a platform HTTP port".to_string(),
            )
        })?;
        let p2p_port = platform_p2p_port.ok_or_else(|| {
            PlatformWalletError::InvalidParameter(
                "an evonode payload requires the platform P2P port (the masternode list does \
                 not carry it)"
                    .to_string(),
            )
        })?;
        (
            Some(ProviderMasternodeType::HighPerformance as u16),
            Some(PlatformNodeId::from_byte_array(node_id)),
            Some(p2p_port),
            Some(http_port),
        )
    } else {
        if platform_p2p_port.is_some() {
            return Err(PlatformWalletError::InvalidParameter(
                "a platform P2P port was given, but this masternode is not an evonode".to_string(),
            ));
        }
        (
            Some(ProviderMasternodeType::Regular as u16),
            None,
            None,
            None,
        )
    };

    Ok(ProviderUpdateServicePayload::new(
        mn_type,
        Txid::from_byte_array(entry.pro_tx_hash),
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

/// Fund and finalize the ProUpServTx: input selection reserves the funding
/// inputs, the payload finalizer writes `inputs_hash` and the operator-BLS
/// `payload_sig` (basic scheme over `base_payload_hash()`, modern
/// serialization — the exact convention `verify_message_digest` checks real
/// mainnet signatures with), and only then are the inputs ECDSA-signed,
/// since their sighashes cover the finished payload.
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
    let builder = TransactionBuilder::new()
        .set_special_payload(TransactionPayload::ProviderUpdateServicePayloadType(
            placeholder,
        ))
        .set_payload_finalizer(move |unsigned| {
            let Some(TransactionPayload::ProviderUpdateServicePayloadType(placeholder)) =
                &unsigned.special_transaction_payload
            else {
                return Err(BuilderError::InvalidData(
                    "the ProUpServTx placeholder payload is missing from the assembled \
                     transaction"
                        .into(),
                ));
            };
            let mut finalized = placeholder.clone();
            finalized.inputs_hash = unsigned.hash_inputs();

            let secret = Option::<BlsSecretKey<Bls12381G2Impl>>::from(
                BlsSecretKey::<Bls12381G2Impl>::from_be_bytes(&operator_secret),
            )
            .ok_or_else(|| {
                BuilderError::SigningFailed("the operator key is not a valid BLS secret".into())
            })?;
            let signature = secret
                .sign(
                    SignatureSchemes::Basic,
                    finalized.base_payload_hash().as_byte_array(),
                )
                .map_err(|e| {
                    BuilderError::SigningFailed(format!("BLS payload signing failed: {e}"))
                })?;
            let signature_bytes: [u8; 96] = signature
                .to_bytes_with_mode(dashcore::blsful::SerializationFormat::Modern)
                .as_slice()
                .try_into()
                .map_err(|_| {
                    BuilderError::SigningFailed(
                        "BLS signature did not serialize to 96 bytes".into(),
                    )
                })?;
            finalized.payload_sig = BLSSignature::from(signature_bytes);
            Ok(TransactionPayload::ProviderUpdateServicePayloadType(
                finalized,
            ))
        });

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
    use crate::test_support::funded_wallet_manager;
    use dashcore::blsful::{PublicKey as BlsPublicKey, Signature as BlsSignature};
    use dashcore::Transaction;
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

    #[test]
    fn placeholder_for_a_regular_masternode_omits_the_platform_fields() {
        let entry = operator_entry(0x11, false);
        let payload = prepare_update_service_placeholder(&entry, None, ScriptBuf::new())
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
            Txid::from_byte_array(entry.pro_tx_hash)
        );
        assert_eq!(payload.platform_node_id, None);
        assert_eq!(payload.platform_p2p_port, None);
        assert_eq!(payload.platform_http_port, None);
        assert_eq!(payload.inputs_hash, InputsHash::all_zeros());
        assert_eq!(payload.payload_sig, BLSSignature::from([0u8; 96]));

        let err = prepare_update_service_placeholder(&entry, Some(26656), ScriptBuf::new())
            .expect_err("a platform P2P port on a regular masternode must be refused");
        assert!(matches!(err, PlatformWalletError::InvalidParameter(_)));
    }

    #[test]
    fn placeholder_for_an_evonode_requires_and_carries_the_platform_triplet() {
        let entry = operator_entry(0x22, true);
        let payload = prepare_update_service_placeholder(&entry, Some(26656), ScriptBuf::new())
            .expect("evonode placeholder");
        assert_eq!(
            payload.mn_type,
            Some(ProviderMasternodeType::HighPerformance as u16)
        );
        assert_eq!(
            payload.platform_node_id,
            entry.platform_node_id.map(PlatformNodeId::from_byte_array)
        );
        assert_eq!(payload.platform_p2p_port, Some(26656));
        assert_eq!(payload.platform_http_port, entry.platform_http_port);

        let err = prepare_update_service_placeholder(&entry, None, ScriptBuf::new())
            .expect_err("an evonode payload without the P2P port must be refused");
        assert!(matches!(err, PlatformWalletError::InvalidParameter(_)));
    }

    #[test]
    fn placeholder_refuses_a_v3_extended_net_info_entry() {
        // A v2 payload would replace the whole endpoint map with the primary
        // address — refuse even though a routable primary exists.
        let mut entry = operator_entry(0x55, false);
        entry.has_extended_net_info = true;
        assert!(
            entry.service_address.is_some(),
            "primary address present and routable"
        );
        let err = prepare_update_service_placeholder(&entry, None, ScriptBuf::new())
            .expect_err("an extended-net-info entry must be refused");
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

    #[test]
    fn placeholder_refuses_an_entry_without_a_plain_service_address() {
        let mut entry = operator_entry(0x33, false);
        entry.service_address = None;
        let err = prepare_update_service_placeholder(&entry, None, ScriptBuf::new())
            .expect_err("no service address to re-assert");
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
        let placeholder = prepare_update_service_placeholder(&entry, Some(26656), ScriptBuf::new())
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
        assert_eq!(payload.platform_http_port, entry.platform_http_port);
        assert_ne!(payload.payload_sig, BLSSignature::from([0u8; 96]));

        // The payload signature verifies under the basic scheme against the
        // operator public key, over base_payload_hash — the exact convention
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
}
