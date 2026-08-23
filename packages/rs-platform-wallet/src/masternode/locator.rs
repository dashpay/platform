//! Find a masternode from whatever the user has in hand.
//!
//! A host pastes one string — an IP (`1.2.3.4`, `1.2.3.4:9999`, a DAPI URL),
//! a proTxHash, or a private key (owner / voting WIF or hex, operator BLS hex,
//! Tenderdash node key in dashmate's base64 or hex) — and gets back the
//! masternode(s) it names, plus, for a key, the role that key fills on each.
//!
//! Three layers, each pure and testable on its own:
//!
//! 1. [`parse_locator_input`] turns the text into *candidates*. A 64-hex
//!    string is ambiguous (proTxHash, secp256k1 secret, BLS secret, ed25519
//!    seed), so it yields every reading and the list decides.
//! 2. [`locate_in_summaries`] resolves candidates against a DML snapshot:
//!    proTxHash / IP directly; secrets by deriving the public side and
//!    matching the list's voting key id, operator key (basic **and** legacy
//!    serialization) or platform node id.
//! 3. [`MasternodeLocator::locate`] adds the opt-in Platform step for secp256k1
//!    keys: owner and payout keys are not on the list, but Platform's
//!    masternode identities carry them (owner identity = proTxHash: key 0 =
//!    payout address TRANSFER, key 1 = owner OWNER; operator identity:
//!    operator payout TRANSFER), all registered non-unique, so
//!    `getIdentityByNonUniquePublicKeyHash` finds them.
//!
//! [`verify_masternode_key`] is the same derive-and-compare used when a host
//! attaches a key to a role: it answers `Matches` / `DoesNotMatch` or
//! `Unverifiable` when the reference (owner key hash, payout hash) isn't known
//! yet — never a false pass.

use std::collections::{BTreeSet, HashMap};
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use dash_sdk::platform::types::identity::NonUniquePublicKeyHashQuery;
use dash_sdk::platform::Fetch;
use dashcore::blsful::{
    Bls12381G2Impl, PublicKey as BlsPublicKey, SecretKey as BlsSecretKey, SerializationFormat,
};
use dashcore::ed25519_dalek::SigningKey;
use dashcore::hashes::{hash160, Hash};
use dashcore::secp256k1::{PublicKey as SecpPublicKey, Secp256k1, SecretKey as SecpSecretKey};
use dashcore::{Network, PlatformNodeId, PrivateKey};
use dpp::identifier::MasternodeIdentifiers;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{Identity, Purpose};
use dpp::prelude::Identifier;
use zeroize::Zeroizing;

use super::list::{find_in_summaries, MasternodeListQuery, MasternodeListSummary};
use super::record::MasternodeRecord;
use crate::spv::SpvRuntime;
use crate::wallet::platform_wallet::WalletId;

// ---------------------------------------------------------------------------
// Roles
// ---------------------------------------------------------------------------

/// A private key's job on a masternode. The discriminants are the FFI wire
/// values and line up with the Android wallet's `MasternodeKeyType` for the
/// first four.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MasternodeKeyRole {
    /// secp256k1; signs ProUpRegTx, and is the OWNER key of the Platform
    /// owner identity (can sign withdrawals).
    Owner = 0,
    /// secp256k1; governance / contested-resource voting.
    Voting = 1,
    /// BLS12-381; signs ProUpServTx, SYSTEM key of the operator identity.
    Operator = 2,
    /// ed25519; the Tenderdash node key. Identifies an evonode, signs
    /// nothing a wallet does.
    PlatformNode = 3,
    /// secp256k1 key of the owner payout address — the TRANSFER key of the
    /// owner identity, i.e. what withdraws owner rewards.
    OwnerPayout = 4,
    /// secp256k1 key of the operator payout address — the TRANSFER key of
    /// the operator identity.
    OperatorPayout = 5,
}

impl MasternodeKeyRole {
    pub const ALL: [MasternodeKeyRole; 6] = [
        Self::Owner,
        Self::Voting,
        Self::Operator,
        Self::PlatformNode,
        Self::OwnerPayout,
        Self::OperatorPayout,
    ];

    pub fn as_u8(self) -> u8 {
        self as u8
    }

    pub fn from_u8(value: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|r| r.as_u8() == value)
    }

    /// Whether the role's key is a secp256k1 key (WIF / hex input).
    pub fn is_ecdsa(self) -> bool {
        matches!(
            self,
            Self::Owner | Self::Voting | Self::OwnerPayout | Self::OperatorPayout
        )
    }
}

// ---------------------------------------------------------------------------
// Parsed input
// ---------------------------------------------------------------------------

/// Decoded secret material from the locator text. Zeroized on drop.
#[derive(Clone)]
pub enum LocatorSecret {
    /// secp256k1 secret; `compressed` follows the WIF flag (hex input is
    /// taken as compressed, which is what every Dash tool emits).
    Ecdsa {
        secret: Zeroizing<[u8; 32]>,
        compressed: bool,
    },
    /// BLS12-381 secret scalar (32 bytes, big-endian).
    Bls(Zeroizing<[u8; 32]>),
    /// ed25519 seed (32 bytes).
    Ed25519(Zeroizing<[u8; 32]>),
}

impl std::fmt::Debug for LocatorSecret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ecdsa { compressed, .. } => f
                .debug_struct("Ecdsa")
                .field("compressed", compressed)
                .finish_non_exhaustive(),
            Self::Bls(_) => f.write_str("Bls(..)"),
            Self::Ed25519(_) => f.write_str("Ed25519(..)"),
        }
    }
}

/// One reading of the locator text.
#[derive(Debug, Clone)]
pub enum MasternodeLocatorInput {
    /// proTxHash in wire order.
    ProTxHash([u8; 32]),
    /// Service IP, optionally with the Core P2P port.
    ServiceAddress { ip: IpAddr, port: Option<u16> },
    /// A private key of some role.
    Secret(LocatorSecret),
}

/// Every plausible reading of the text. Ambiguous input (64 hex chars)
/// yields several candidates; the list disambiguates.
#[derive(Debug, Clone, Default)]
pub struct ParsedLocatorInput {
    pub candidates: Vec<MasternodeLocatorInput>,
}

impl ParsedLocatorInput {
    pub fn has_secret(&self) -> bool {
        self.candidates
            .iter()
            .any(|c| matches!(c, MasternodeLocatorInput::Secret(_)))
    }
}

/// Why the locator text couldn't be read at all.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LocatorParseError {
    #[error("nothing to look up")]
    Empty,
    #[error("not an IP address, proTxHash or private key")]
    Unrecognized,
    /// A WIF key for another network (mainnet key on testnet or the reverse).
    #[error("this key is for {key_network}, the wallet is on {expected}")]
    WrongNetworkKey {
        key_network: Network,
        expected: Network,
    },
    /// A 64-byte node key whose public half does not match its seed — the
    /// pasted value is corrupt or not a Tenderdash node key.
    #[error("the node key's public half does not match its seed")]
    NodeKeyMismatch,
    /// Hex / base64 that decodes to 32 bytes but is not a valid secret on
    /// any of the three curves (and isn't a proTxHash either).
    #[error("not a valid private key")]
    InvalidSecret,
}

fn strip_url(text: &str) -> &str {
    let lower = text.to_ascii_lowercase();
    let rest = if let Some(stripped) = lower.strip_prefix("https://") {
        &text[text.len() - stripped.len()..]
    } else if let Some(stripped) = lower.strip_prefix("http://") {
        &text[text.len() - stripped.len()..]
    } else {
        text
    };
    // Cut at the first path / query separator, drop a trailing slash.
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    rest[..end].trim_end_matches('/')
}

fn parse_service_address(text: &str) -> Option<MasternodeLocatorInput> {
    if let Ok(addr) = text.parse::<SocketAddr>() {
        return Some(MasternodeLocatorInput::ServiceAddress {
            ip: addr.ip(),
            port: Some(addr.port()),
        });
    }
    let bare = text.trim_start_matches('[').trim_end_matches(']');
    if let Ok(ip) = bare.parse::<IpAddr>() {
        return Some(MasternodeLocatorInput::ServiceAddress { ip, port: None });
    }
    None
}

fn is_mainnet(network: Network) -> bool {
    matches!(network, Network::Mainnet)
}

/// The 32-byte secret candidates a raw 32-byte blob can be, on every curve
/// it is a valid secret for. `Ecdsa` only when the scalar is in range, `Bls`
/// only when below the group order; ed25519 accepts any 32 bytes as a seed.
fn secret_candidates(bytes: &[u8; 32]) -> Vec<MasternodeLocatorInput> {
    let mut out = Vec::with_capacity(3);
    if SecpSecretKey::from_slice(bytes).is_ok() {
        out.push(MasternodeLocatorInput::Secret(LocatorSecret::Ecdsa {
            secret: Zeroizing::new(*bytes),
            compressed: true,
        }));
    }
    if bls_public_keys(bytes).is_some() {
        out.push(MasternodeLocatorInput::Secret(LocatorSecret::Bls(
            Zeroizing::new(*bytes),
        )));
    }
    out.push(MasternodeLocatorInput::Secret(LocatorSecret::Ed25519(
        Zeroizing::new(*bytes),
    )));
    out
}

/// Split a 64-byte `seed ‖ public key` node key (dashmate / Tenderdash
/// `node_key.json`) and check the public half against the seed.
fn ed25519_seed_from_node_key(bytes: &[u8]) -> Result<[u8; 32], LocatorParseError> {
    let seed: [u8; 32] = bytes[..32]
        .try_into()
        .map_err(|_| LocatorParseError::Unrecognized)?;
    let declared_pub = &bytes[32..];
    let derived_pub = SigningKey::from_bytes(&seed).verifying_key().to_bytes();
    if declared_pub != derived_pub {
        return Err(LocatorParseError::NodeKeyMismatch);
    }
    Ok(seed)
}

/// Read the locator text into candidates. `network` is the wallet's
/// network, used to reject a WIF key for the other network.
pub fn parse_locator_input(
    text: &str,
    network: Network,
) -> Result<ParsedLocatorInput, LocatorParseError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(LocatorParseError::Empty);
    }
    let host = strip_url(trimmed);

    if let Some(addr) = parse_service_address(host) {
        return Ok(ParsedLocatorInput {
            candidates: vec![addr],
        });
    }

    // Hex: 64 chars is ambiguous, 128 is a seed‖pub node key.
    if trimmed.len().is_multiple_of(2) && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        let bytes = hex::decode(trimmed).map_err(|_| LocatorParseError::Unrecognized)?;
        match bytes.len() {
            32 => {
                let raw: [u8; 32] = bytes.as_slice().try_into().expect("len checked");
                let mut candidates = Vec::with_capacity(5);
                // Display-order proTxHash (explorers, dashmate, Tenderdash,
                // `protx list`) — reverse to wire order.
                let mut wire = raw;
                wire.reverse();
                candidates.push(MasternodeLocatorInput::ProTxHash(wire));
                // The wire orientation itself, should a tool ever print it
                // that way; harmless when the two coincide.
                if wire != raw {
                    candidates.push(MasternodeLocatorInput::ProTxHash(raw));
                }
                candidates.extend(secret_candidates(&raw));
                return Ok(ParsedLocatorInput { candidates });
            }
            64 => {
                let seed = ed25519_seed_from_node_key(&bytes)?;
                return Ok(ParsedLocatorInput {
                    candidates: vec![MasternodeLocatorInput::Secret(LocatorSecret::Ed25519(
                        Zeroizing::new(seed),
                    ))],
                });
            }
            _ => return Err(LocatorParseError::Unrecognized),
        }
    }

    // WIF (owner / voting / payout keys as Core's `dumpprivkey` prints them).
    if let Ok(key) = PrivateKey::from_wif(trimmed) {
        if is_mainnet(key.network) != is_mainnet(network) {
            return Err(LocatorParseError::WrongNetworkKey {
                key_network: key.network,
                expected: network,
            });
        }
        return Ok(ParsedLocatorInput {
            candidates: vec![MasternodeLocatorInput::Secret(LocatorSecret::Ecdsa {
                secret: Zeroizing::new(key.inner.secret_bytes()),
                compressed: key.compressed,
            })],
        });
    }

    // base64: dashmate's 64-byte node key, or a bare 32-byte secret.
    {
        if let Ok(bytes) = dashcore::base64::decode(trimmed) {
            match bytes.len() {
                64 => {
                    let seed = ed25519_seed_from_node_key(&bytes)?;
                    return Ok(ParsedLocatorInput {
                        candidates: vec![MasternodeLocatorInput::Secret(LocatorSecret::Ed25519(
                            Zeroizing::new(seed),
                        ))],
                    });
                }
                32 => {
                    let raw: [u8; 32] = bytes.as_slice().try_into().expect("len checked");
                    return Ok(ParsedLocatorInput {
                        candidates: secret_candidates(&raw),
                    });
                }
                _ => {}
            }
        }
    }

    Err(LocatorParseError::Unrecognized)
}

// ---------------------------------------------------------------------------
// Public-side derivations
// ---------------------------------------------------------------------------

/// hash160 of the secp256k1 public key for `secret`, or `None` when the
/// scalar is out of range.
pub fn ecdsa_key_id(secret: &[u8; 32], compressed: bool) -> Option<[u8; 20]> {
    let sk = SecpSecretKey::from_slice(secret).ok()?;
    let pk = SecpPublicKey::from_secret_key(&Secp256k1::signing_only(), &sk);
    let bytes: Vec<u8> = if compressed {
        pk.serialize().to_vec()
    } else {
        pk.serialize_uncompressed().to_vec()
    };
    Some(hash160::Hash::hash(&bytes).to_byte_array())
}

/// `(basic, legacy)` 48-byte serializations of the BLS public key for
/// `secret`, or `None` when the scalar is not below the group order.
pub fn bls_public_keys(secret: &[u8; 32]) -> Option<([u8; 48], [u8; 48])> {
    let sk: BlsSecretKey<Bls12381G2Impl> =
        Option::from(BlsSecretKey::<Bls12381G2Impl>::from_be_bytes(secret))?;
    let pk = BlsPublicKey::from(&sk);
    let basic: [u8; 48] = pk.to_bytes().as_slice().try_into().ok()?;
    let legacy: [u8; 48] = pk
        .to_bytes_with_mode(SerializationFormat::Legacy)
        .as_slice()
        .try_into()
        .ok()?;
    Some((basic, legacy))
}

/// Tenderdash node id for an ed25519 `seed`.
pub fn ed25519_node_id(seed: &[u8; 32]) -> [u8; 20] {
    let public = SigningKey::from_bytes(seed).verifying_key().to_bytes();
    PlatformNodeId::from_ed25519_public_key(&public).to_byte_array()
}

// ---------------------------------------------------------------------------
// Resolution
// ---------------------------------------------------------------------------

/// How a match was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocatorMatchKind {
    ProTxHash = 0,
    ServiceAddress = 1,
    /// A pasted private key — `matched_keys` says which role(s).
    Key = 2,
}

impl LocatorMatchKind {
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

/// One masternode the locator text names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MasternodeLocateMatch {
    pub summary: MasternodeListSummary,
    pub matched_by: LocatorMatchKind,
    /// Roles the pasted key fills on this masternode (empty unless
    /// `matched_by == Key`). Sorted, no duplicates. Usually one; a key used
    /// as both owner and voting key yields two.
    pub matched_keys: Vec<MasternodeKeyRole>,
    /// This masternode is already one of a loaded wallet's own (registered
    /// with that wallet's keys) — hosts show "already in wallet" instead of
    /// offering to track it.
    pub in_wallet: Option<WalletId>,
}

/// Outcome of the optional Platform step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlatformLookup {
    /// The input had no secp256k1 key, so Platform had nothing to add.
    NotNeeded = 0,
    /// A secp256k1 key was given but the host didn't opt in.
    NotRequested = 1,
    /// Ran to completion.
    Done = 2,
    /// Attempted and failed (network / DAPI); the local matches stand, the
    /// owner / payout roles simply weren't checked.
    Unavailable = 3,
}

impl PlatformLookup {
    pub fn as_u8(&self) -> u8 {
        match self {
            Self::NotNeeded => 0,
            Self::NotRequested => 1,
            Self::Done => 2,
            Self::Unavailable => 3,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MasternodeLocateResult {
    /// In the list's order; one entry per distinct proTxHash.
    pub matches: Vec<MasternodeLocateMatch>,
    pub platform_lookup: PlatformLookup,
    /// Human-readable reason when `platform_lookup == Unavailable`.
    pub platform_error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LocateOptions {
    /// Also ask Platform for owner / payout roles of a pasted secp256k1
    /// key (one `getIdentityByNonUniquePublicKeyHash` per key). Off by
    /// default: it tells DAPI which key hash the user is interested in.
    pub search_platform: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum MasternodeLocateError {
    #[error(transparent)]
    Parse(#[from] LocatorParseError),
    /// The deterministic masternode list isn't available (SPV not running
    /// or masternode sync incomplete) — there is nothing to search yet.
    #[error("the masternode list is not available yet")]
    ListUnavailable,
}

/// secp256k1 key ids derived from the input, remembered for the Platform
/// step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EcdsaKeyCandidate {
    pub key_id: [u8; 20],
}

/// Resolve `parsed` against a DML snapshot. Pure. Returns the matches and
/// the secp256k1 key ids the input produced (for the optional Platform
/// step). A secret's *voting* role is matched here; owner / payout roles
/// are not on the list.
pub fn locate_in_summaries(
    parsed: &ParsedLocatorInput,
    summaries: &[MasternodeListSummary],
    in_wallet: &HashMap<[u8; 32], WalletId>,
) -> (Vec<MasternodeLocateMatch>, Vec<EcdsaKeyCandidate>) {
    let mut matches: Vec<MasternodeLocateMatch> = Vec::new();
    let mut ecdsa = Vec::new();

    let mut add = |summary: &MasternodeListSummary,
                   kind: LocatorMatchKind,
                   role: Option<MasternodeKeyRole>| {
        if let Some(existing) = matches
            .iter_mut()
            .find(|m| m.summary.pro_tx_hash == summary.pro_tx_hash)
        {
            if let Some(role) = role {
                if !existing.matched_keys.contains(&role) {
                    existing.matched_keys.push(role);
                    existing.matched_keys.sort();
                }
            }
            return;
        }
        matches.push(MasternodeLocateMatch {
            summary: summary.clone(),
            matched_by: kind,
            matched_keys: role.into_iter().collect(),
            in_wallet: in_wallet.get(&summary.pro_tx_hash).copied(),
        });
    };

    for candidate in &parsed.candidates {
        match candidate {
            MasternodeLocatorInput::ProTxHash(hash) => {
                for s in find_in_summaries(summaries, &MasternodeListQuery::ProTxHash(*hash)) {
                    add(s, LocatorMatchKind::ProTxHash, None);
                }
            }
            MasternodeLocatorInput::ServiceAddress { ip, port } => {
                let q = MasternodeListQuery::ServiceAddress {
                    ip: *ip,
                    port: *port,
                };
                for s in find_in_summaries(summaries, &q) {
                    add(s, LocatorMatchKind::ServiceAddress, None);
                }
            }
            MasternodeLocatorInput::Secret(LocatorSecret::Ecdsa { secret, compressed }) => {
                if let Some(key_id) = ecdsa_key_id(secret, *compressed) {
                    ecdsa.push(EcdsaKeyCandidate { key_id });
                    for s in find_in_summaries(summaries, &MasternodeListQuery::VotingKeyId(key_id))
                    {
                        add(s, LocatorMatchKind::Key, Some(MasternodeKeyRole::Voting));
                    }
                }
            }
            MasternodeLocatorInput::Secret(LocatorSecret::Bls(secret)) => {
                if let Some((basic, legacy)) = bls_public_keys(secret) {
                    for key in [basic, legacy] {
                        for s in find_in_summaries(
                            summaries,
                            &MasternodeListQuery::OperatorPublicKey(key),
                        ) {
                            add(s, LocatorMatchKind::Key, Some(MasternodeKeyRole::Operator));
                        }
                    }
                }
            }
            MasternodeLocatorInput::Secret(LocatorSecret::Ed25519(seed)) => {
                let node_id = ed25519_node_id(seed);
                for s in find_in_summaries(summaries, &MasternodeListQuery::PlatformNodeId(node_id))
                {
                    add(
                        s,
                        LocatorMatchKind::Key,
                        Some(MasternodeKeyRole::PlatformNode),
                    );
                }
            }
        }
    }

    // Keep the list's order so repeated lookups render stably.
    let order: HashMap<[u8; 32], usize> = summaries
        .iter()
        .enumerate()
        .map(|(i, s)| (s.pro_tx_hash, i))
        .collect();
    matches.sort_by_key(|m| {
        order
            .get(&m.summary.pro_tx_hash)
            .copied()
            .unwrap_or(usize::MAX)
    });
    (matches, ecdsa)
}

/// Which masternode (wire proTxHash) and role a Platform masternode identity
/// says `key_id` fills.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformKeyRole {
    pub pro_tx_hash: [u8; 32],
    pub role: MasternodeKeyRole,
}

/// Interpret one identity returned by the non-unique-key-hash query for
/// `key_id`: an owner identity (id == display proTxHash of a listed node)
/// yields `Owner` / `OwnerPayout` by the matching key's purpose; an operator
/// identity (id == `create_operator_identifier(proTxHash, operator key)` of a
/// listed node) yields `OperatorPayout`. Anything else — a voter identity
/// (voting keys are matched from the list), a non-masternode identity that
/// happens to hold the key — yields nothing. Pure; the network step is the
/// caller's.
pub fn platform_roles_from_identity(
    identity: &Identity,
    key_id: &[u8; 20],
    summaries: &[MasternodeListSummary],
) -> Vec<PlatformKeyRole> {
    let id_bytes: [u8; 32] = identity.id().to_buffer();
    let mut wire = id_bytes;
    wire.reverse();

    if let Some(summary) = summaries.iter().find(|s| s.pro_tx_hash == wire) {
        // Owner identity. Which of its keys is ours decides the role.
        let mut roles = BTreeSet::new();
        for key in identity.public_keys().values() {
            if key.data().as_slice() != key_id {
                continue;
            }
            match key.purpose() {
                Purpose::OWNER => {
                    roles.insert(MasternodeKeyRole::Owner);
                }
                Purpose::TRANSFER => {
                    roles.insert(MasternodeKeyRole::OwnerPayout);
                }
                _ => {}
            }
        }
        return roles
            .into_iter()
            .map(|role| PlatformKeyRole {
                pro_tx_hash: summary.pro_tx_hash,
                role,
            })
            .collect();
    }

    // Operator identity of some listed node?
    let id = identity.id();
    summaries
        .iter()
        .filter(|s| {
            Identifier::create_operator_identifier(&s.pro_tx_hash_display(), &s.operator_public_key)
                == id
        })
        .filter(|_| {
            identity
                .public_keys()
                .values()
                .any(|k| k.data().as_slice() == key_id && k.purpose() == Purpose::TRANSFER)
        })
        .map(|s| PlatformKeyRole {
            pro_tx_hash: s.pro_tx_hash,
            role: MasternodeKeyRole::OperatorPayout,
        })
        .collect()
}

/// Upper bound on identities paged through per key hash. Masternode keys
/// are rarely shared by more than a handful of identities; this caps the
/// round trips for a key that happens to be widely reused.
const PLATFORM_LOOKUP_MAX_PAGES: usize = 16;

/// The snapshot a locate runs against: SPV for the list, the SDK for the
/// Platform step, the network for WIF checks, and the wallets' own
/// masternodes so matches can say "already in wallet". Built by
/// [`crate::PlatformWalletManager::masternode_locator_blocking`]; `Send +
/// Sync`, so hosts can run [`Self::locate`] on a worker without holding the
/// manager.
#[derive(Clone)]
pub struct MasternodeLocator {
    pub spv: Arc<SpvRuntime>,
    pub sdk: Arc<dash_sdk::Sdk>,
    pub network: Network,
    /// proTxHash (wire) ⇒ wallet id, for every loaded wallet's masternodes.
    pub in_wallet: HashMap<[u8; 32], WalletId>,
}

impl MasternodeLocator {
    /// Find the masternode(s) `text` names. See the module docs for the
    /// three layers. `Err(ListUnavailable)` when the DML isn't synced yet;
    /// parse errors surface as `Err(Parse(..))`.
    pub async fn locate(
        &self,
        text: &str,
        options: LocateOptions,
    ) -> Result<MasternodeLocateResult, MasternodeLocateError> {
        let parsed = parse_locator_input(text, self.network)?;
        let summaries = self
            .spv
            .masternode_list_summaries()
            .await
            .ok_or(MasternodeLocateError::ListUnavailable)?;
        let (mut matches, ecdsa) = locate_in_summaries(&parsed, &summaries, &self.in_wallet);

        let (platform_lookup, platform_error) = if ecdsa.is_empty() {
            (PlatformLookup::NotNeeded, None)
        } else if !options.search_platform {
            (PlatformLookup::NotRequested, None)
        } else {
            match self.platform_roles(&ecdsa, &summaries).await {
                Ok(roles) => {
                    for found in roles {
                        let Some(summary) = summaries
                            .iter()
                            .find(|s| s.pro_tx_hash == found.pro_tx_hash)
                        else {
                            continue;
                        };
                        if let Some(existing) = matches
                            .iter_mut()
                            .find(|m| m.summary.pro_tx_hash == found.pro_tx_hash)
                        {
                            if !existing.matched_keys.contains(&found.role) {
                                existing.matched_keys.push(found.role);
                                existing.matched_keys.sort();
                            }
                        } else {
                            matches.push(MasternodeLocateMatch {
                                summary: summary.clone(),
                                matched_by: LocatorMatchKind::Key,
                                matched_keys: vec![found.role],
                                in_wallet: self.in_wallet.get(&found.pro_tx_hash).copied(),
                            });
                        }
                    }
                    (PlatformLookup::Done, None)
                }
                Err(message) => (PlatformLookup::Unavailable, Some(message)),
            }
        };

        Ok(MasternodeLocateResult {
            matches,
            platform_lookup,
            platform_error,
        })
    }

    async fn platform_roles(
        &self,
        ecdsa: &[EcdsaKeyCandidate],
        summaries: &[MasternodeListSummary],
    ) -> Result<Vec<PlatformKeyRole>, String> {
        let mut out = Vec::new();
        for candidate in ecdsa {
            let mut after: Option<[u8; 32]> = None;
            for _ in 0..PLATFORM_LOOKUP_MAX_PAGES {
                let query = NonUniquePublicKeyHashQuery {
                    key_hash: candidate.key_id,
                    after,
                };
                let identity = Identity::fetch(self.sdk.as_ref(), query)
                    .await
                    .map_err(|e| e.to_string())?;
                let Some(identity) = identity else {
                    break;
                };
                after = Some(identity.id().to_buffer());
                out.extend(platform_roles_from_identity(
                    &identity,
                    &candidate.key_id,
                    summaries,
                ));
            }
        }
        Ok(out)
    }
}

// ---------------------------------------------------------------------------
// Verification
// ---------------------------------------------------------------------------

/// The on-chain / on-Platform references a key is checked against. Built
/// from whatever the caller knows: the DML summary (voting / operator /
/// platform node), the wallet record or a tracked node's enrichment (owner
/// / payout hashes). Missing references make a role `Unverifiable`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MasternodeKeyReference {
    pub owner_key_hash: Option<[u8; 20]>,
    pub voting_key_id: Option<[u8; 20]>,
    pub operator_public_key: Option<[u8; 48]>,
    pub platform_node_id: Option<[u8; 20]>,
    /// hash160 behind the owner payout address (P2PKH payout script).
    pub payout_key_hash: Option<[u8; 20]>,
    /// hash160 behind the operator payout address.
    pub operator_payout_key_hash: Option<[u8; 20]>,
}

impl MasternodeKeyReference {
    /// What the list knows: voting, operator, platform node.
    pub fn from_summary(summary: &MasternodeListSummary) -> Self {
        Self {
            voting_key_id: Some(summary.voting_key_id),
            operator_public_key: Some(summary.operator_public_key),
            platform_node_id: summary.platform_node_id,
            ..Default::default()
        }
    }

    /// What a wallet record knows (everything the provider transactions
    /// carry). The payout hash is extracted only from a P2PKH payout
    /// script; other script kinds leave it `None`.
    pub fn from_record(record: &MasternodeRecord) -> Self {
        Self {
            owner_key_hash: record.owner_key_hash,
            voting_key_id: record.voting_key_hash,
            operator_public_key: record.operator_public_key,
            platform_node_id: record.platform_node_id,
            payout_key_hash: record.payout_script.as_deref().and_then(p2pkh_script_hash),
            operator_payout_key_hash: None,
        }
    }

    /// Fill `None`s of `self` from `other` (the summary refreshes the
    /// list-known fields, a record / enrichment supplies the rest).
    pub fn merged_with(mut self, other: &Self) -> Self {
        self.owner_key_hash = self.owner_key_hash.or(other.owner_key_hash);
        self.voting_key_id = self.voting_key_id.or(other.voting_key_id);
        self.operator_public_key = self.operator_public_key.or(other.operator_public_key);
        self.platform_node_id = self.platform_node_id.or(other.platform_node_id);
        self.payout_key_hash = self.payout_key_hash.or(other.payout_key_hash);
        self.operator_payout_key_hash = self
            .operator_payout_key_hash
            .or(other.operator_payout_key_hash);
        self
    }
}

/// hash160 of a standard P2PKH script (`OP_DUP OP_HASH160 <20> OP_EQUALVERIFY
/// OP_CHECKSIG`), else `None`.
pub fn p2pkh_script_hash(script: &[u8]) -> Option<[u8; 20]> {
    if script.len() == 25
        && script[0] == 0x76
        && script[1] == 0xa9
        && script[2] == 0x14
        && script[23] == 0x88
        && script[24] == 0xac
    {
        let mut out = [0u8; 20];
        out.copy_from_slice(&script[3..23]);
        Some(out)
    } else {
        None
    }
}

/// Result of checking a key against a role.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyVerification {
    Matches = 0,
    DoesNotMatch = 1,
    /// The reference for this role isn't known (e.g. owner / payout of a
    /// node whose registration details haven't been fetched) — not a pass.
    Unverifiable = 2,
}

impl KeyVerification {
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

/// Parse `text` as a key of `role`'s curve: WIF / hex for secp256k1 roles,
/// hex for BLS, base64 or hex (32 or 64 bytes) for ed25519.
pub fn parse_secret_for_role(
    text: &str,
    role: MasternodeKeyRole,
    network: Network,
) -> Result<LocatorSecret, LocatorParseError> {
    let parsed = parse_locator_input(text, network)?;
    let wanted = parsed.candidates.into_iter().find_map(|c| match (c, role) {
        (
            MasternodeLocatorInput::Secret(s @ LocatorSecret::Ecdsa { .. }),
            MasternodeKeyRole::Owner
            | MasternodeKeyRole::Voting
            | MasternodeKeyRole::OwnerPayout
            | MasternodeKeyRole::OperatorPayout,
        ) => Some(s),
        (
            MasternodeLocatorInput::Secret(s @ LocatorSecret::Bls(_)),
            MasternodeKeyRole::Operator,
        ) => Some(s),
        (
            MasternodeLocatorInput::Secret(s @ LocatorSecret::Ed25519(_)),
            MasternodeKeyRole::PlatformNode,
        ) => Some(s),
        _ => None,
    });
    wanted.ok_or(LocatorParseError::InvalidSecret)
}

/// Derive-and-compare `secret` against `reference` for `role`.
pub fn verify_masternode_key(
    reference: &MasternodeKeyReference,
    role: MasternodeKeyRole,
    secret: &LocatorSecret,
) -> KeyVerification {
    fn compare<T: PartialEq>(expected: Option<T>, actual: Option<T>) -> KeyVerification {
        match (expected, actual) {
            (Some(e), Some(a)) if e == a => KeyVerification::Matches,
            (Some(_), Some(_)) => KeyVerification::DoesNotMatch,
            (Some(_), None) => KeyVerification::DoesNotMatch,
            (None, _) => KeyVerification::Unverifiable,
        }
    }

    match (role, secret) {
        (MasternodeKeyRole::Owner, LocatorSecret::Ecdsa { secret, compressed }) => {
            compare(reference.owner_key_hash, ecdsa_key_id(secret, *compressed))
        }
        (MasternodeKeyRole::Voting, LocatorSecret::Ecdsa { secret, compressed }) => {
            compare(reference.voting_key_id, ecdsa_key_id(secret, *compressed))
        }
        (MasternodeKeyRole::OwnerPayout, LocatorSecret::Ecdsa { secret, compressed }) => {
            compare(reference.payout_key_hash, ecdsa_key_id(secret, *compressed))
        }
        (MasternodeKeyRole::OperatorPayout, LocatorSecret::Ecdsa { secret, compressed }) => {
            compare(
                reference.operator_payout_key_hash,
                ecdsa_key_id(secret, *compressed),
            )
        }
        (MasternodeKeyRole::Operator, LocatorSecret::Bls(secret)) => {
            match (reference.operator_public_key, bls_public_keys(secret)) {
                (Some(expected), Some((basic, legacy))) => {
                    if expected == basic || expected == legacy {
                        KeyVerification::Matches
                    } else {
                        KeyVerification::DoesNotMatch
                    }
                }
                (Some(_), None) => KeyVerification::DoesNotMatch,
                (None, _) => KeyVerification::Unverifiable,
            }
        }
        (MasternodeKeyRole::PlatformNode, LocatorSecret::Ed25519(seed)) => {
            compare(reference.platform_node_id, Some(ed25519_node_id(seed)))
        }
        // Curve mismatch: the caller parsed with `parse_secret_for_role`, so
        // this only happens on misuse; it is simply not a match.
        _ => KeyVerification::DoesNotMatch,
    }
}

/// Convenience: parse + verify in one step.
pub fn verify_masternode_key_text(
    reference: &MasternodeKeyReference,
    role: MasternodeKeyRole,
    text: &str,
    network: Network,
) -> Result<KeyVerification, LocatorParseError> {
    let secret = parse_secret_for_role(text, role, network)?;
    Ok(verify_masternode_key(reference, role, &secret))
}

#[cfg(test)]
mod tests {
    use super::super::list::test_support::{evonode, ip, masternode};
    use super::*;

    // A fixed secp256k1 secret and its derived ids.
    const SECP_SECRET: [u8; 32] = [0x11u8; 32];

    fn secp_key_id() -> [u8; 20] {
        ecdsa_key_id(&SECP_SECRET, true).unwrap()
    }

    fn wif(network: Network) -> String {
        PrivateKey::from_byte_array(&SECP_SECRET, network)
            .unwrap()
            .to_wif()
    }

    // --- parsing -----------------------------------------------------------

    #[test]
    fn empty_and_garbage_are_rejected() {
        assert_eq!(
            parse_locator_input("   ", Network::Mainnet).unwrap_err(),
            LocatorParseError::Empty
        );
        assert_eq!(
            parse_locator_input("not a thing", Network::Mainnet).unwrap_err(),
            LocatorParseError::Unrecognized
        );
        assert_eq!(
            parse_locator_input("abcd", Network::Mainnet).unwrap_err(),
            LocatorParseError::Unrecognized,
            "short hex is not a proTxHash or a key"
        );
    }

    #[test]
    fn ip_forms_parse_to_a_service_address() {
        let cases: [(&str, Option<u16>); 6] = [
            ("1.2.3.4", None),
            ("1.2.3.4:9999", Some(9999)),
            (" https://1.2.3.4:443/ ", Some(443)),
            ("http://1.2.3.4", None),
            ("[2001:db8::1]", None),
            ("[2001:db8::1]:9999", Some(9999)),
        ];
        for (text, port) in cases {
            let parsed = parse_locator_input(text, Network::Mainnet).unwrap();
            assert_eq!(parsed.candidates.len(), 1, "{text}");
            match &parsed.candidates[0] {
                MasternodeLocatorInput::ServiceAddress { port: p, .. } => {
                    assert_eq!(*p, port, "{text}")
                }
                other => panic!("{text}: {other:?}"),
            }
        }
    }

    #[test]
    fn sixty_four_hex_yields_every_reading() {
        // Not a palindrome, so the two proTxHash orientations differ.
        let mut raw = SECP_SECRET;
        raw[0] = 0x01;
        let hex = hex::encode(raw);
        let parsed = parse_locator_input(&hex, Network::Mainnet).unwrap();
        let kinds: Vec<&str> = parsed
            .candidates
            .iter()
            .map(|c| match c {
                MasternodeLocatorInput::ProTxHash(_) => "protx",
                MasternodeLocatorInput::Secret(LocatorSecret::Ecdsa { .. }) => "ecdsa",
                MasternodeLocatorInput::Secret(LocatorSecret::Bls(_)) => "bls",
                MasternodeLocatorInput::Secret(LocatorSecret::Ed25519(_)) => "ed25519",
                MasternodeLocatorInput::ServiceAddress { .. } => "ip",
            })
            .collect();
        assert_eq!(kinds, ["protx", "protx", "ecdsa", "bls", "ed25519"]);
        // First proTxHash reading is the reversal (display → wire), the
        // second the bytes as given.
        match (&parsed.candidates[0], &parsed.candidates[1]) {
            (MasternodeLocatorInput::ProTxHash(a), MasternodeLocatorInput::ProTxHash(b)) => {
                let mut expected = raw;
                expected.reverse();
                assert_eq!(*a, expected);
                assert_eq!(*b, raw);
            }
            _ => unreachable!(),
        }
        // A palindromic hash yields the orientation once.
        let parsed = parse_locator_input(&hex::encode(SECP_SECRET), Network::Mainnet).unwrap();
        assert_eq!(
            parsed
                .candidates
                .iter()
                .filter(|c| matches!(c, MasternodeLocatorInput::ProTxHash(_)))
                .count(),
            1
        );
    }

    #[test]
    fn out_of_range_secp_scalar_drops_the_ecdsa_reading() {
        // 0xFF.. is above the secp256k1 group order, so it cannot be an
        // owner / voting / payout key; it is still a proTxHash reading and
        // an ed25519 seed (any 32 bytes).
        let hex = "ff".repeat(32);
        let parsed = parse_locator_input(&hex, Network::Mainnet).unwrap();
        assert!(parsed.candidates.iter().all(|c| !matches!(
            c,
            MasternodeLocatorInput::Secret(LocatorSecret::Ecdsa { .. })
        )));
        assert!(parsed.has_secret(), "ed25519 still accepts any 32 bytes");
        assert!(parsed
            .candidates
            .iter()
            .any(|c| matches!(c, MasternodeLocatorInput::ProTxHash(_))));
    }

    #[test]
    fn wif_parses_and_checks_the_network() {
        let parsed = parse_locator_input(&wif(Network::Mainnet), Network::Mainnet).unwrap();
        assert_eq!(parsed.candidates.len(), 1);
        match &parsed.candidates[0] {
            MasternodeLocatorInput::Secret(LocatorSecret::Ecdsa { secret, compressed }) => {
                assert_eq!(**secret, SECP_SECRET);
                assert!(compressed);
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(
            parse_locator_input(&wif(Network::Testnet), Network::Mainnet).unwrap_err(),
            LocatorParseError::WrongNetworkKey {
                key_network: Network::Testnet,
                expected: Network::Mainnet
            }
        );
        // Devnet / regtest share the testnet WIF prefix, so a testnet WIF is
        // fine there.
        assert!(parse_locator_input(&wif(Network::Testnet), Network::Devnet).is_ok());
    }

    #[test]
    fn dashmate_node_key_parses_when_consistent() {
        let seed = [0x42u8; 32];
        let public = SigningKey::from_bytes(&seed).verifying_key().to_bytes();
        let mut node_key = seed.to_vec();
        node_key.extend_from_slice(&public);
        let b64 = dashcore::base64::encode(&node_key);

        let parsed = parse_locator_input(&b64, Network::Mainnet).unwrap();
        assert_eq!(parsed.candidates.len(), 1);
        assert!(matches!(
            &parsed.candidates[0],
            MasternodeLocatorInput::Secret(LocatorSecret::Ed25519(s)) if **s == seed
        ));
        // Same key as 128 hex chars.
        let parsed = parse_locator_input(&hex::encode(&node_key), Network::Mainnet).unwrap();
        assert!(matches!(
            &parsed.candidates[0],
            MasternodeLocatorInput::Secret(LocatorSecret::Ed25519(_))
        ));

        // A corrupted public half is rejected, not silently accepted.
        node_key[40] ^= 0x01;
        assert_eq!(
            parse_locator_input(&dashcore::base64::encode(&node_key), Network::Mainnet)
                .unwrap_err(),
            LocatorParseError::NodeKeyMismatch
        );
    }

    // --- derivations ---------------------------------------------------------

    #[test]
    fn ecdsa_key_id_matches_dashcore_address_hash() {
        let key = PrivateKey::from_byte_array(&SECP_SECRET, Network::Mainnet).unwrap();
        let expected: [u8; 20] = key
            .public_key(&Secp256k1::new())
            .pubkey_hash()
            .to_byte_array();
        assert_eq!(secp_key_id(), expected);
        assert_ne!(
            ecdsa_key_id(&SECP_SECRET, false).unwrap(),
            expected,
            "uncompressed keys hash differently"
        );
    }

    #[test]
    fn bls_keys_have_distinct_basic_and_legacy_forms() {
        let (basic, legacy) = bls_public_keys(&[0x33u8; 32]).unwrap();
        assert_ne!(basic, legacy);
        assert_eq!(basic[0] & 0x80, 0x80, "compressed flag set in both");
        assert_eq!(legacy[0] & 0x80, 0x80);
    }

    #[test]
    fn node_id_is_sha256_prefix_of_the_public_key() {
        use dashcore::hashes::sha256;
        let seed = [0x42u8; 32];
        let public = SigningKey::from_bytes(&seed).verifying_key().to_bytes();
        let digest = sha256::Hash::hash(&public).to_byte_array();
        assert_eq!(ed25519_node_id(&seed), digest[..20]);
    }

    // --- local resolution -------------------------------------------------

    fn list() -> Vec<MasternodeListSummary> {
        let mut voting = masternode(1);
        voting.voting_key_id = secp_key_id();
        let mut operator = evonode(2);
        operator.operator_public_key = bls_public_keys(&[0x33u8; 32]).unwrap().0;
        let mut legacy_operator = masternode(3);
        legacy_operator.operator_public_key = bls_public_keys(&[0x33u8; 32]).unwrap().1;
        let mut node = evonode(4);
        node.platform_node_id = Some(ed25519_node_id(&[0x42u8; 32]));
        vec![voting, operator, legacy_operator, node, masternode(5)]
    }

    #[test]
    fn locates_by_pro_tx_hash_in_display_order() {
        let mut display = [5u8; 32];
        display.reverse();
        let parsed = parse_locator_input(&hex::encode(display), Network::Mainnet).unwrap();
        let (matches, _) = locate_in_summaries(&parsed, &list(), &HashMap::new());
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].summary.pro_tx_hash, [5u8; 32]);
        assert_eq!(matches[0].matched_by, LocatorMatchKind::ProTxHash);
        assert!(matches[0].matched_keys.is_empty());
    }

    #[test]
    fn locates_by_ip_and_marks_wallet_membership() {
        let parsed = parse_locator_input("10.0.0.5", Network::Mainnet).unwrap();
        let mut in_wallet = HashMap::new();
        in_wallet.insert([5u8; 32], [9u8; 32]);
        let (matches, _) = locate_in_summaries(&parsed, &list(), &in_wallet);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].matched_by, LocatorMatchKind::ServiceAddress);
        assert_eq!(matches[0].in_wallet, Some([9u8; 32]));
    }

    #[test]
    fn voting_key_resolves_locally_and_is_remembered_for_platform() {
        let parsed = parse_locator_input(&wif(Network::Mainnet), Network::Mainnet).unwrap();
        let (matches, ecdsa) = locate_in_summaries(&parsed, &list(), &HashMap::new());
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].summary.pro_tx_hash, [1u8; 32]);
        assert_eq!(matches[0].matched_by, LocatorMatchKind::Key);
        assert_eq!(matches[0].matched_keys, vec![MasternodeKeyRole::Voting]);
        assert_eq!(
            ecdsa,
            vec![EcdsaKeyCandidate {
                key_id: secp_key_id()
            }]
        );
    }

    #[test]
    fn operator_secret_matches_basic_and_legacy_entries() {
        let parsed = parse_locator_input(&hex::encode([0x33u8; 32]), Network::Mainnet).unwrap();
        let (matches, _) = locate_in_summaries(&parsed, &list(), &HashMap::new());
        let hashes: Vec<[u8; 32]> = matches.iter().map(|m| m.summary.pro_tx_hash).collect();
        assert_eq!(hashes, vec![[2u8; 32], [3u8; 32]]);
        for m in &matches {
            assert_eq!(m.matched_keys, vec![MasternodeKeyRole::Operator]);
        }
    }

    #[test]
    fn platform_node_seed_matches_the_evonode() {
        let parsed = parse_locator_input(&hex::encode([0x42u8; 32]), Network::Mainnet).unwrap();
        let (matches, _) = locate_in_summaries(&parsed, &list(), &HashMap::new());
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].summary.pro_tx_hash, [4u8; 32]);
        assert_eq!(
            matches[0].matched_keys,
            vec![MasternodeKeyRole::PlatformNode]
        );
    }

    #[test]
    fn shared_key_across_roles_merges_into_one_match() {
        // A node whose voting key id equals its platform node id bytes is
        // contrived, but exercises the merge: two candidates, one match.
        let mut both = evonode(7);
        both.voting_key_id = secp_key_id();
        both.platform_node_id = Some(ed25519_node_id(&SECP_SECRET));
        let parsed = parse_locator_input(&hex::encode(SECP_SECRET), Network::Mainnet).unwrap();
        let (matches, _) = locate_in_summaries(&parsed, &[both], &HashMap::new());
        assert_eq!(matches.len(), 1);
        assert_eq!(
            matches[0].matched_keys,
            vec![MasternodeKeyRole::Voting, MasternodeKeyRole::PlatformNode]
        );
    }

    #[test]
    fn nothing_found_is_an_empty_match_list() {
        let parsed = parse_locator_input("192.168.9.9", Network::Mainnet).unwrap();
        let (matches, ecdsa) = locate_in_summaries(&parsed, &list(), &HashMap::new());
        assert!(matches.is_empty());
        assert!(ecdsa.is_empty());
        let _ = ip(1);
    }

    // --- platform role interpretation ---------------------------------------

    fn identity_with(id: [u8; 32], keys: Vec<(Purpose, [u8; 20])>) -> Identity {
        use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
        use dpp::identity::v0::IdentityV0;
        use dpp::identity::{IdentityPublicKey, KeyType, SecurityLevel};
        use dpp::platform_value::BinaryData;
        let public_keys = keys
            .into_iter()
            .enumerate()
            .map(|(i, (purpose, data))| {
                let key: IdentityPublicKey = IdentityPublicKeyV0 {
                    id: i as u32,
                    purpose,
                    security_level: SecurityLevel::CRITICAL,
                    contract_bounds: None,
                    key_type: KeyType::ECDSA_HASH160,
                    read_only: true,
                    data: BinaryData::new(data.to_vec()),
                    disabled_at: None,
                }
                .into();
                (i as u32, key)
            })
            .collect();
        IdentityV0 {
            id: Identifier::from(id),
            public_keys,
            balance: 0,
            revision: 0,
        }
        .into()
    }

    #[test]
    fn owner_identity_yields_owner_or_payout_role_by_purpose() {
        let summaries = list();
        let owner_hash = secp_key_id();
        let payout_hash = [0x77u8; 20];
        // Owner identity id = display-order proTxHash of node 5.
        let mut display = [5u8; 32];
        display.reverse();
        let identity = identity_with(
            display,
            vec![
                (Purpose::TRANSFER, payout_hash),
                (Purpose::OWNER, owner_hash),
            ],
        );
        assert_eq!(
            platform_roles_from_identity(&identity, &owner_hash, &summaries),
            vec![PlatformKeyRole {
                pro_tx_hash: [5u8; 32],
                role: MasternodeKeyRole::Owner
            }]
        );
        assert_eq!(
            platform_roles_from_identity(&identity, &payout_hash, &summaries),
            vec![PlatformKeyRole {
                pro_tx_hash: [5u8; 32],
                role: MasternodeKeyRole::OwnerPayout
            }]
        );
        // A key the identity doesn't hold yields nothing.
        assert!(platform_roles_from_identity(&identity, &[0x01u8; 20], &summaries).is_empty());
    }

    #[test]
    fn operator_identity_yields_operator_payout_role() {
        let summaries = list();
        let node = &summaries[1]; // evonode 2
        let payout_hash = [0x66u8; 20];
        let id = Identifier::create_operator_identifier(
            &node.pro_tx_hash_display(),
            &node.operator_public_key,
        );
        let identity = identity_with(id.to_buffer(), vec![(Purpose::TRANSFER, payout_hash)]);
        assert_eq!(
            platform_roles_from_identity(&identity, &payout_hash, &summaries),
            vec![PlatformKeyRole {
                pro_tx_hash: node.pro_tx_hash,
                role: MasternodeKeyRole::OperatorPayout
            }]
        );
    }

    #[test]
    fn unrelated_identity_yields_nothing() {
        let identity = identity_with([0xEEu8; 32], vec![(Purpose::TRANSFER, secp_key_id())]);
        assert!(platform_roles_from_identity(&identity, &secp_key_id(), &list()).is_empty());
    }

    // --- verification ------------------------------------------------------

    #[test]
    fn verifies_each_role_against_its_reference() {
        let reference = MasternodeKeyReference {
            owner_key_hash: Some(secp_key_id()),
            voting_key_id: Some([0xABu8; 20]),
            operator_public_key: Some(bls_public_keys(&[0x33u8; 32]).unwrap().1),
            platform_node_id: Some(ed25519_node_id(&[0x42u8; 32])),
            payout_key_hash: None,
            operator_payout_key_hash: None,
        };
        let net = Network::Mainnet;
        let owner = wif(net);
        assert_eq!(
            verify_masternode_key_text(&reference, MasternodeKeyRole::Owner, &owner, net).unwrap(),
            KeyVerification::Matches
        );
        assert_eq!(
            verify_masternode_key_text(&reference, MasternodeKeyRole::Voting, &owner, net).unwrap(),
            KeyVerification::DoesNotMatch
        );
        assert_eq!(
            verify_masternode_key_text(&reference, MasternodeKeyRole::OwnerPayout, &owner, net)
                .unwrap(),
            KeyVerification::Unverifiable,
            "no payout reference ⇒ unverifiable, never a pass"
        );
        // Legacy-serialized operator key still matches from the secret.
        assert_eq!(
            verify_masternode_key_text(
                &reference,
                MasternodeKeyRole::Operator,
                &hex::encode([0x33u8; 32]),
                net
            )
            .unwrap(),
            KeyVerification::Matches
        );
        assert_eq!(
            verify_masternode_key_text(
                &reference,
                MasternodeKeyRole::Operator,
                &hex::encode([0x34u8; 32]),
                net
            )
            .unwrap(),
            KeyVerification::DoesNotMatch
        );
        assert_eq!(
            verify_masternode_key_text(
                &reference,
                MasternodeKeyRole::PlatformNode,
                &hex::encode([0x42u8; 32]),
                net
            )
            .unwrap(),
            KeyVerification::Matches
        );
        // A WIF in the operator field is not a BLS key.
        assert_eq!(
            verify_masternode_key_text(&reference, MasternodeKeyRole::Operator, &owner, net)
                .unwrap_err(),
            LocatorParseError::InvalidSecret
        );
    }

    #[test]
    fn reference_from_record_extracts_the_p2pkh_payout_hash() {
        let mut record = MasternodeRecord::default();
        let mut script = vec![0x76, 0xa9, 0x14];
        script.extend_from_slice(&[0x55u8; 20]);
        script.extend_from_slice(&[0x88, 0xac]);
        record.payout_script = Some(script);
        record.owner_key_hash = Some([1u8; 20]);
        let reference = MasternodeKeyReference::from_record(&record);
        assert_eq!(reference.payout_key_hash, Some([0x55u8; 20]));
        assert_eq!(reference.owner_key_hash, Some([1u8; 20]));
        // Merge: the summary supplies list fields, the record the rest.
        let merged = MasternodeKeyReference::from_summary(&masternode(1)).merged_with(&reference);
        assert_eq!(merged.voting_key_id, Some([1u8; 20]));
        assert_eq!(merged.payout_key_hash, Some([0x55u8; 20]));
        assert_eq!(p2pkh_script_hash(&[0x00, 0x14]), None);
    }

    #[test]
    fn role_codes_round_trip_and_align_with_android() {
        for role in MasternodeKeyRole::ALL {
            assert_eq!(MasternodeKeyRole::from_u8(role.as_u8()), Some(role));
        }
        assert_eq!(MasternodeKeyRole::Owner.as_u8(), 0);
        assert_eq!(MasternodeKeyRole::Voting.as_u8(), 1);
        assert_eq!(MasternodeKeyRole::Operator.as_u8(), 2);
        assert_eq!(MasternodeKeyRole::PlatformNode.as_u8(), 3);
        assert_eq!(MasternodeKeyRole::from_u8(6), None);
    }
}
