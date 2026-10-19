//! DashPay invitation link codec and claim — DIP-13 sub-feature 3', legacy-compatible.
//!
//! An invitation packages a one-time ECDSA **voucher** private key together with
//! a reference to the InstantSend-locked funding transaction, so an invitee with
//! no Dash can register their own identity from it. The inviter optionally
//! includes their own DPNS username so the invitee can send a contact request
//! back.
//!
//! The link is the **legacy query form** shared with the reference wallets
//! (`dash-wallet` Android, `dashwallet-ios`), so a link produced here is
//! field-level cross-claimable with those wallets and vice versa:
//!
//! ```text
//! dashpay://invite
//!   ?du=<inviter DPNS username>
//!   &assetlocktx=<funding txid, lowercase big-endian display hex>
//!   &pk=<voucher credit-burn key, WIF, compressed, network-correct>
//!   &islock=<InstantSend lock, lowercase hex>   # or omitted / "null"
//!   [&display-name=<inviter display name>]
//!   [&avatar-url=<inviter avatar url, single %-encoded>]
//! ```
//!
//! The interop contract is **emit strict/canonical, parse lenient** — exactly as
//! tolerantly as the live wallets:
//! - Parse accepts both the `dashpay://invite` scheme and the
//!   `https://invitations.dashpay.io/applink` host, by field name and
//!   order-independent (the two legacy wallets differ in param order).
//! - `islock` is optional: a missing param **and** the literal string `"null"`
//!   (which Android emits for chainlock-confirmed invites) both mean "no instant
//!   lock" — the claim reconstructs a ChainLock proof instead. The hex decodes
//!   as the deterministic InstantSend lock; a lock in the older
//!   non-deterministic format does not decode, so the claim fails rather than
//!   guessing (no live wallet emits that format).
//! - `assetlocktx` is kept as the raw hex string; the claim tries it as-given
//!   then byte-reversed on a fetch miss, mirroring the legacy endianness retry.
//!
//! # Security
//!
//! The `voucher_key` is **bearer money** — whoever holds the link can claim the
//! funded identity. The URI is a secret: callers MUST NOT log or persist it, and
//! the voucher key is never stored (it is HD-derived and re-derivable from the
//! funding index). Parsing is bounded (URI length cap) so a hostile link can't
//! force a large allocation; the WIF is decoded + compression-checked at parse,
//! and its network is validated against the wallet at claim (a wrong-network WIF
//! is a valid key on the wrong chain, caught before the funding fetch).
//!
//! What the design does and does not protect against:
//! - Every theft reduces to "who holds the link". Platform checks the claim's
//!   signature against the key of the asset-lock output, and the new identity's
//!   id is derived from the funding outpoint, so someone who watches a claim in
//!   flight but lacks the voucher key cannot redirect it, and two racing claims
//!   target the same identity (exactly one commits).
//! - The inviter can always re-derive the voucher key, so after handing over a
//!   link it can still claim or reclaim the voucher first. Nothing is stolen
//!   (the funds were the inviter's), but the invitee is denied onboarding with
//!   no sign of why. For the same reason, reclaiming a leaked link is a race the
//!   inviter can lose.
//! - Nothing signs the link, so whoever controls the channel it travels over
//!   can swap in a different invite. The worst outcome is the invitee sending a
//!   contact request to the attacker's identity, which an ordinary contact
//!   request achieves anyway. Signing would not help: the channel is the trust
//!   root.
//! - Because the identity id comes from the funding outpoint, the inviter knows
//!   the invitee's identity id before the invitee claims it.
//! - The legacy wallets wrapped the link in an AppsFlyer OneLink, which sends
//!   the plaintext `pk` to AppsFlyer's servers. We parse that host but never
//!   emit it.

use std::collections::BTreeMap;

use dpp::dashcore::consensus::Decodable;
use dpp::dashcore::secp256k1::{PublicKey, SecretKey};
use dpp::dashcore::transaction::special_transaction::TransactionPayload;
use dpp::dashcore::{InstantLock, Network, OutPoint, PrivateKey, ScriptBuf, Transaction, TxOut};
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
use dpp::identity::state_transition::asset_lock_proof::InstantAssetLockProof;
use dpp::identity::v0::IdentityV0;
use dpp::identity::{Identity, IdentityPublicKey, KeyID, Purpose, SecurityLevel};
use dpp::prelude::{AssetLockProof, Identifier};

use crate::core::FetchedCoreTransaction;
use crate::platform::transition::asset_lock_submission::{
    is_asset_lock_already_consumed, is_instant_lock_proof_invalid, submit_with_cl_height_retry,
};
use crate::platform::transition::put_identity::PutIdentity;
use crate::platform::transition::put_settings::PutSettings;
use crate::platform::Fetch;
use crate::{Error, Sdk};

/// Hard cap on the amount an invitation can lock (0.26 DASH). The voucher is a
/// bearer credential, so the blast radius of a leaked link is bounded here in
/// Rust — not just in the UI. Sized for onboarding at BOTH username tiers: the
/// invitee spends the voucher on identity creation **plus** a DPNS name — a
/// normal name (~0.03 DASH, the legacy `DASH_PAY_FEE`) or a contested/premium
/// name (~0.25 DASH, `DASH_PAY_FEE_CONTESTED`), with the 0.01 margin covering
/// the create/claim fees. The earlier 0.05 value deferred the contested tier
/// "until contested-name-via-invite claim exists" — the claim path is
/// amount-agnostic and contested-invite claims are verified working, and the
/// Android wallet has funded contested invitations at 0.25 since 2024, so the
/// deferral is over. (The pre-merge 0.01 iteration was below even a usable
/// non-contested invitation and rejected its own onboarding default.)
pub const MAX_INVITATION_DUFFS: u64 = 26_000_000;

/// Floor on the amount an invitation can lock (0.003 DASH). A voucher funds a
/// Platform identity operation, and creating an identity — which is what the
/// invitee's claim does, and what a register-target reclaim does — requires the
/// asset lock to carry at least the identity-registration minimum (~0.00228 DASH
/// in credits on the current network; the state transition is rejected with
/// `IdentityAssetLockTransactionOutPointNotEnoughBalanceError` below it). A
/// voucher under this floor produces an invitation that can be neither claimed
/// nor reclaimed, so reject it at creation. Set above the bare floor to leave the
/// new identity a small usable starting balance; tune if the floor changes.
pub const MIN_INVITATION_DUFFS: u64 = 300_000;

/// Default TTL (24h) for an invitation's advisory expiry. The creating wallet
/// sets `expiry_unix = now + MAX_INVITATION_TTL_SECS`. The expiry is **advisory** — a
/// leaked-link finder holds the voucher key and ignores it — so it bounds only
/// the honest UI (don't submit an about-to-go-stale IS proof) and the reclaim
/// signal, NOT a leaked-link window. The real leak bound is `MAX_INVITATION_DUFFS`.
pub const MAX_INVITATION_TTL_SECS: u32 = 24 * 60 * 60;

/// Claim-by-fetch bound: how many times to look up the funding tx (each attempt
/// tries both byte orders) before giving up. InstantSend/ChainLock finality does
/// not guarantee the invitee's DAPI node has indexed the tx yet, so a freshly
/// shared invitation can miss purely from propagation lag; retrying bounds that.
const CLAIM_FETCH_MAX_ATTEMPTS: u32 = 5;

/// Fixed backoff between claim-by-fetch attempts. 5 attempts × 3s ≈ 12s of
/// tolerance for propagation delay before surfacing a "not found" error.
const CLAIM_FETCH_RETRY_DELAY: std::time::Duration = std::time::Duration::from_secs(3);

/// Why an invitation could not be encoded, parsed, inspected or claimed.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, thiserror::Error)]
pub enum InvitationError {
    /// The link, or the funding transaction it names, is not a usable
    /// invitation.
    #[error("invalid invitation: {0}")]
    Invalid(String),

    /// The voucher key is for the other network (a testnet link opened in a
    /// mainnet wallet, or the reverse). Definitive: the claim refuses the link
    /// before any network work.
    #[error("invitation is for the {invitation:?} network but this wallet is on {wallet:?}")]
    NetworkMismatch {
        invitation: Network,
        wallet: Network,
    },

    /// Retry later: the link may be fine but cannot be used yet. Either the
    /// funding transaction has not reached the queried node, or it is not
    /// chain-locked yet and the claim needs a ChainLock proof (the link has
    /// no islock, or Platform rejected its islock as stale). The message
    /// says which.
    #[error("{0}")]
    NotChainLocked(String),

    /// An SDK call failed.
    #[error(transparent)]
    Sdk(#[from] Error),
}

/// The `dashpay://invite` custom scheme — the canonical form we emit and the
/// primary form we parse (QR / in-person / deep link). `invite` is the URI
/// authority (Android's `URI_PREFIX`), so the accepted host is exactly `invite`.
const INVITATION_SCHEME: &str = "dashpay";
const INVITATION_SCHEME_HOST: &str = "invite";
/// The full canonical prefix we emit.
const INVITATION_SCHEME_PREFIX: &str = "dashpay://invite";

/// The AppsFlyer OneLink applink the iOS reference wallet emits
/// (`https://invitations.dashpay.io/applink?…`). Parsed as a first-class
/// alternative to the custom scheme (field-level interop).
const INVITATION_APPLINK_HOST: &str = "invitations.dashpay.io";
const INVITATION_APPLINK_PATH: &str = "/applink";

/// Query parameter names — identical to the legacy wallets' contract.
const PARAM_USER: &str = "du";
const PARAM_ASSET_LOCK_TX: &str = "assetlocktx";
const PARAM_PRIVATE_KEY: &str = "pk";
const PARAM_IS_LOCK: &str = "islock";
const PARAM_DISPLAY_NAME: &str = "display-name";
const PARAM_AVATAR_URL: &str = "avatar-url";

/// Android emits this literal for the `islock` value when the funding was
/// confirmed by a ChainLock rather than an InstantSend lock; treat it as "no
/// instant lock" (reconstruct a ChainLock proof at claim), NOT as hex to decode.
const IS_LOCK_NULL_SENTINEL: &str = "null";

/// Max chars of the whole URI accepted before parsing (anti-DoS). A real link —
/// username + txid (64) + WIF (~52) + islock hex (~400) + optional avatar url —
/// is well under 2 KB; 8192 is comfortable headroom while bounding the
/// allocation a hostile link can force.
const MAX_INVITATION_URI_LEN: usize = 8192;

/// Max length (bytes) of a UTF-8 string field (username / display name / avatar
/// url). DPNS labels are short; this only bounds a hostile link.
const MAX_STR_BYTES: usize = 2048;

/// Inviter contact-bootstrap info — present iff the link carries any of `du`,
/// `display-name`, or `avatar-url`. Absent ⇒ pure funding voucher. The link does
/// not carry the inviter's identity id (the legacy format has no such field);
/// the invitee resolves it from `username` via DPNS at contact-bootstrap time.
///
/// `username` is optional: the reference wallets emit `display-name`/`avatar-url`
/// in blocks independent of `du`, so a `du`-less link can still carry inviter
/// metadata (surfaced in the preview even though no contact request is possible
/// without a username).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InviterInfo {
    /// The inviter's DPNS username (`du`), shown to the invitee and used to
    /// resolve the inviter's identity for the contact request. `None` for a
    /// metadata-only (`du`-less) link.
    pub username: Option<String>,
    /// Optional display name (`display-name`) for the claim UI.
    pub display_name: Option<String>,
    /// Optional avatar url (`avatar-url`) for the claim UI.
    pub avatar_url: Option<String>,
}

/// A decoded invitation, ready for claim.
///
/// Unlike the funding proof (which is fetched at claim), everything here comes
/// straight from the link: the bearer voucher key, the funding txid to fetch,
/// and the optional InstantSend lock hex.
pub struct ParsedInvitation {
    /// One-time ECDSA voucher private key that funds the invitee's identity
    /// create (signs the asset-lock's outer state-transition signature).
    pub voucher_key: SecretKey,
    /// The network the voucher key's WIF was encoded for (`0xCC` ⇒ Mainnet,
    /// `0xEF` ⇒ the testnet family). The claim path rejects a link whose WIF
    /// network does not match the wallet's, turning a wrong-network link into a
    /// clear error instead of a mysterious fetch miss.
    pub voucher_key_network: Network,
    /// The funding transaction id as carried in the link (`assetlocktx`),
    /// lowercased. Kept as the raw hex string so the claim can try it as-given
    /// and byte-reversed on a fetch miss (old iOS links are little-endian).
    pub funding_txid: String,
    /// The InstantSend lock, lowercase consensus hex — `None` when the link
    /// omitted `islock` or set it to `"null"` (a ChainLock-confirmed invite).
    pub islock_hex: Option<String>,
    /// Inviter contact-bootstrap info; `None` ⇒ pure funding voucher (no `du`).
    pub inviter: Option<InviterInfo>,
}

impl Drop for ParsedInvitation {
    /// Scrub the voucher scalar on drop — it is literal bearer money. Mirrors
    /// the resolver signer's key hygiene (`WipingSecretKey`).
    fn drop(&mut self) {
        self.voucher_key.non_secure_erase();
    }
}

impl std::fmt::Debug for ParsedInvitation {
    /// Redacts the voucher key and the funding txid — the whole point of the
    /// type is to carry a bearer secret, which must never reach a log.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParsedInvitation")
            .field("voucher_key", &"<redacted>")
            .field("funding_txid", &"<redacted>")
            .field("has_islock", &self.islock_hex.is_some())
            .field("inviter", &self.inviter)
            .finish_non_exhaustive()
    }
}

fn invalid(msg: impl Into<String>) -> InvitationError {
    InvitationError::Invalid(msg.into())
}

/// The P2PKH script the voucher key controls (compressed-pubkey hash160). This
/// is the selector that binds the voucher key to its funded credit output.
fn voucher_credit_script(voucher_key: &SecretKey) -> ScriptBuf {
    let pubkey = PublicKey::from_secret_key(voucher_key);
    let hash = dpp::dashcore::PublicKey::new(pubkey).pubkey_hash();
    ScriptBuf::new_p2pkh(&hash)
}

// ---------------------------------------------------------------------------
// Percent-encoding (URI query values). Only %XX escapes — never `+` for space
// (that is form encoding; `Uri.getQueryParameter` on the legacy wallets does
// not treat `+` as space, so neither do we).
// ---------------------------------------------------------------------------

/// Percent-encode a query value, passing the RFC 3986 unreserved set through
/// untouched (which covers hex, base58 WIF, and DPNS labels) and `%`-escaping
/// everything else. Encoding an already-safe value is a no-op.
fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Decode a percent-encoded query value (`%XX` → byte). Leaves `+` literal.
/// Errors on a malformed escape or non-UTF-8 result.
fn percent_decode(s: &str) -> Result<String, InvitationError> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hi = bytes
                .get(i + 1)
                .and_then(|c| (*c as char).to_digit(16))
                .ok_or_else(|| invalid("malformed percent-escape in invitation link"))?;
            let lo = bytes
                .get(i + 2)
                .and_then(|c| (*c as char).to_digit(16))
                .ok_or_else(|| invalid("malformed percent-escape in invitation link"))?;
            out.push((hi * 16 + lo) as u8);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|_| invalid("invitation link field is not valid UTF-8"))
}

/// Split a query string (`k1=v1&k2=v2`) into percent-decoded key/value pairs,
/// order-independent. A blank segment or a segment without `=` is skipped.
fn parse_query(query: &str) -> Result<Vec<(String, String)>, InvitationError> {
    let mut pairs = Vec::new();
    for segment in query.split('&') {
        if segment.is_empty() {
            continue;
        }
        let Some((raw_key, raw_val)) = segment.split_once('=') else {
            continue;
        };
        pairs.push((percent_decode(raw_key)?, percent_decode(raw_val)?));
    }
    Ok(pairs)
}

/// Look up a field by name (first match wins), returning it trimmed. `None` for
/// a missing or blank value.
fn field<'a>(pairs: &'a [(String, String)], name: &str) -> Option<&'a str> {
    pairs
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.trim())
        .filter(|v| !v.is_empty())
}

/// The parts of an invitation URI we gate on. Extracted with a small hand parser
/// (no `url` crate in this crate's deps) so the transport check is anchored to
/// the real scheme/host/path — not a substring/prefix that a decoy URL could
/// smuggle (`dashpay://inviteXYZ`, `https://evil/?next=invitations.dashpay.io/applink&…`).
struct UriParts<'a> {
    scheme: &'a str,
    /// Authority host, lowercased for comparison (userinfo + port stripped).
    host: String,
    path: &'a str,
    query: Option<&'a str>,
}

/// Strip `userinfo@` and `:port` from an authority, leaving the bare host.
fn authority_host(authority: &str) -> &str {
    let after_userinfo = authority.rsplit_once('@').map_or(authority, |(_, h)| h);
    // A bare `host:port` — take the host. (Our hosts are never bracketed IPv6.)
    after_userinfo.split(':').next().unwrap_or(after_userinfo)
}

/// Split `scheme://authority/path?query#fragment` into the parts we gate on.
/// Requires the `://` authority form (both accepted transports use it) and drops
/// the fragment. Returns `None` for a shape we don't recognize as a URI.
fn split_uri(uri: &str) -> Option<UriParts<'_>> {
    let (scheme, rest) = uri.split_once(':')?;
    let rest = rest.strip_prefix("//")?;
    let auth_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let authority = &rest[..auth_end];
    let after_auth = &rest[auth_end..];
    // Drop the fragment, then split path from query.
    let after_auth = after_auth.split('#').next().unwrap_or(after_auth);
    let (path, query) = match after_auth.split_once('?') {
        Some((p, q)) => (p, Some(q)),
        None => (after_auth, None),
    };
    Some(UriParts {
        scheme,
        host: authority_host(authority).to_ascii_lowercase(),
        path,
        query,
    })
}

/// Encode an invitation into a legacy-compatible `dashpay://invite?…` link.
///
/// The returned URI **contains the plaintext voucher key** (WIF) — treat it as a
/// secret (do not log or persist it). `network` sets the WIF network byte
/// (`0xCC` mainnet / `0xEF` testnet) and the key is emitted **compressed** (the
/// credit-output hash160 uses the compressed pubkey). The funding txid and
/// InstantSend lock are read from `asset_lock`; a ChainLock proof emits no
/// `islock`. `inviter` is `Some` only when the inviter opted into the
/// contact-bootstrap (the link then carries `du`/`display-name`/`avatar-url`).
pub fn encode_invitation_uri(
    voucher_key: &SecretKey,
    network: Network,
    asset_lock: &AssetLockProof,
    inviter: Option<&InviterInfo>,
) -> Result<String, InvitationError> {
    // txid (big-endian display hex) + optional islock hex from the proof.
    let (funding_txid, islock_hex) = match asset_lock {
        AssetLockProof::Instant(instant) => {
            let txid = instant.transaction().txid().to_string();
            let islock = hex::encode(dpp::dashcore::consensus::serialize(instant.instant_lock()));
            (txid, Some(islock))
        }
        AssetLockProof::Chain(chain) => (chain.out_point.txid.to_string(), None),
    };

    // WIF, compressed (the default for `PrivateKey::new`), network-correct.
    let wif = PrivateKey::new(*voucher_key, network).to_wif();

    let mut query = String::new();
    let mut push = |key: &str, val: &str| {
        if !query.is_empty() {
            query.push('&');
        }
        query.push_str(key);
        query.push('=');
        query.push_str(&percent_encode(val));
    };

    // `du` is emitted first (canonical) when the inviter opted in with a
    // username; a pure funding voucher emits a `du`-less link (still parseable —
    // iOS accepts it).
    if let Some(username) = inviter.and_then(|info| info.username.as_ref()) {
        if username.len() > MAX_STR_BYTES {
            return Err(invalid("inviter username too long"));
        }
        push(PARAM_USER, username);
    }
    push(PARAM_ASSET_LOCK_TX, &funding_txid);
    push(PARAM_PRIVATE_KEY, &wif);
    if let Some(islock) = &islock_hex {
        push(PARAM_IS_LOCK, islock);
    }
    if let Some(info) = inviter {
        if let Some(display_name) = &info.display_name {
            if display_name.len() > MAX_STR_BYTES {
                return Err(invalid("inviter display name too long"));
            }
            push(PARAM_DISPLAY_NAME, display_name);
        }
        if let Some(avatar_url) = &info.avatar_url {
            if avatar_url.len() > MAX_STR_BYTES {
                return Err(invalid("inviter avatar url too long"));
            }
            push(PARAM_AVATAR_URL, avatar_url);
        }
    }

    let uri = format!("{INVITATION_SCHEME_PREFIX}?{query}");
    // Reject a link the parser itself would reject. `MAX_STR_BYTES` bounds each
    // field's RAW bytes, but percent-encoding can expand a field up to 3×, so
    // several near-cap fields can push the formatted URI past
    // `MAX_INVITATION_URI_LEN` (the exact cap `parse_invitation_uri` enforces).
    // Fail here rather than emit a self-unparseable link — the create path
    // surfaces this as a hard error *after* the funded voucher is recorded, so it
    // stays reclaimable.
    if uri.len() > MAX_INVITATION_URI_LEN {
        return Err(invalid(format!(
            "invitation link too long ({} chars; max {MAX_INVITATION_URI_LEN})",
            uri.len()
        )));
    }
    Ok(uri)
}

/// Parse a legacy-compatible invitation link into a [`ParsedInvitation`].
///
/// Accepts exactly the `dashpay://invite` scheme (host `invite`) and the
/// `https://invitations.dashpay.io/applink` URL (host + path anchored, so a
/// decoy URL that merely mentions the applink cannot smuggle a query in). Fields
/// are matched by name, order-independent. Requires `assetlocktx` + `pk`
/// (non-blank, each appearing at most once); `du` and `islock` are optional (a
/// missing/`"null"` `islock` is a ChainLock invite). The WIF is decoded and
/// compression-checked here; its **network** is captured for the claim path to
/// validate against the wallet (a wrong-network WIF is a valid key on the wrong
/// chain, so it must be caught there, not here). Does **not** fetch or validate
/// the funding tx — that happens at claim.
pub fn parse_invitation_uri(uri: &str) -> Result<ParsedInvitation, InvitationError> {
    if uri.len() > MAX_INVITATION_URI_LEN {
        return Err(invalid(format!(
            "invitation link too long ({} chars; max {MAX_INVITATION_URI_LEN})",
            uri.len()
        )));
    }

    // Anchor the transport gate to the real scheme/host/path, not a prefix or
    // substring: `dashpay://invite` (host == `invite`) or the applink URL
    // (host == invitations.dashpay.io, path == /applink). A trailing slash on
    // the applink path is tolerated.
    let parts = split_uri(uri).ok_or_else(|| invalid("not a valid invitation link URI"))?;
    let accepted = match parts.scheme.to_ascii_lowercase().as_str() {
        INVITATION_SCHEME => {
            parts.host == INVITATION_SCHEME_HOST && (parts.path.is_empty() || parts.path == "/")
        }
        "https" => {
            parts.host == INVITATION_APPLINK_HOST
                && parts.path.trim_end_matches('/') == INVITATION_APPLINK_PATH
        }
        _ => false,
    };
    if !accepted {
        return Err(invalid(
            "not a dashpay://invite or https://invitations.dashpay.io/applink link",
        ));
    }

    let query = parts
        .query
        .ok_or_else(|| invalid("invitation link has no query parameters"))?;
    let pairs = parse_query(query)?;

    // A duplicated required key is ambiguous and dangerous: the two reference
    // wallets bind different occurrences (iOS the last, Android the first), so a
    // `?pk=A&pk=B` link would claim different keys per wallet. Reject it.
    for key in [PARAM_PRIVATE_KEY, PARAM_ASSET_LOCK_TX] {
        if pairs.iter().filter(|(k, _)| k == key).count() > 1 {
            return Err(invalid(format!(
                "invitation link has a duplicated `{key}` field"
            )));
        }
    }

    // iOS minimum: `assetlocktx` + `pk` present and non-blank. Never reject on a
    // missing/`"null"` `islock` or a missing `du`.
    let assetlocktx = field(&pairs, PARAM_ASSET_LOCK_TX)
        .ok_or_else(|| invalid("invitation link is missing the assetlocktx field"))?;
    let pk = field(&pairs, PARAM_PRIVATE_KEY)
        .ok_or_else(|| invalid("invitation link is missing the pk field"))?;

    // WIF: decode + require compression (the credit-output hash uses the
    // compressed pubkey — an uncompressed key would produce a mismatching
    // hash160 and a dead claim). `from_wif` rejects a non-Dash network byte but
    // cannot tell mainnet from testnet against *this* wallet; the claim path does
    // that (see `voucher_key_network`).
    // `PrivateKey`/`SecretKey` are `Copy`, so the decoded binding is a second
    // bearer-scalar copy alongside `voucher_key` — erase it on every exit
    // (rejection included; `ParsedInvitation`'s wipe-on-drop guard doesn't
    // exist yet on that path) to keep the parser's wiping policy airtight.
    let mut private_key = PrivateKey::from_wif(pk)
        .map_err(|e| invalid(format!("invitation pk is not a valid WIF key: {e}")))?;
    if !private_key.compressed {
        private_key.inner.non_secure_erase();
        return Err(invalid("invitation pk must be a compressed WIF key"));
    }
    let voucher_key = private_key.inner;
    let voucher_key_network = private_key.network;
    private_key.inner.non_secure_erase();

    let funding_txid = assetlocktx.to_lowercase();

    // `islock`: a missing param and the literal `"null"` both mean no instant
    // lock. Kept as lowercase hex; decoded to an `InstantLock` only at claim.
    let islock_hex = field(&pairs, PARAM_IS_LOCK)
        .filter(|v| *v != IS_LOCK_NULL_SENTINEL)
        .map(|v| v.to_lowercase());

    // Inviter metadata is parsed field-by-field, independent of `du` (the
    // reference wallets emit `display-name`/`avatar-url` in blocks separate from
    // `du`, so a `du`-less link can still carry them). Present iff any field is.
    // The identity id is not in the link; it is resolved from the username at
    // contact-bootstrap time.
    //
    // Note: an unencoded `&` inside a legacy `avatar-url` would truncate that
    // field at the naive `&` split. Cosmetic (avatar only, low likelihood) — not
    // worth a full nested-query parser; the required fields are unaffected.
    let username = field(&pairs, PARAM_USER).map(str::to_string);
    let display_name = field(&pairs, PARAM_DISPLAY_NAME).map(str::to_string);
    let avatar_url = field(&pairs, PARAM_AVATAR_URL).map(str::to_string);
    let inviter = if username.is_some() || display_name.is_some() || avatar_url.is_some() {
        Some(InviterInfo {
            username,
            display_name,
            avatar_url,
        })
    } else {
        None
    };

    Ok(ParsedInvitation {
        voucher_key,
        voucher_key_network,
        funding_txid,
        islock_hex,
        inviter,
    })
}

/// Whether a voucher WIF encoded for `pk_network` is usable on a wallet running
/// `wallet_network`.
///
/// WIF carries only two network bytes — `0xCC` (Mainnet) and `0xEF` (the whole
/// testnet family: Testnet/Devnet/Regtest) — so the meaningful check is
/// mainnet-vs-not: a testnet/devnet/regtest wallet accepts any `0xEF` WIF, and a
/// mainnet wallet accepts only a `0xCC` WIF. This catches a testnet link pasted
/// into a mainnet wallet (a valid key on the wrong chain) as a clear error
/// rather than a mysterious funding-tx fetch miss.
pub fn wif_network_matches(pk_network: Network, wallet_network: Network) -> bool {
    matches!(pk_network, Network::Mainnet) == matches!(wallet_network, Network::Mainnet)
}

/// Select the funded credit output the voucher key controls.
///
/// The link carries the voucher key but not the output index; scan the fetched
/// asset-lock transaction's credit outputs for the one whose `script_pubkey`
/// matches the voucher key's P2PKH (compressed pubkey), and return its index.
/// Rejects a transaction with no matching credit output. The pk↔output binding
/// is itself consensus-enforced, so this is a fail-fast + correct-index guard,
/// not a security boundary — but selecting (rather than hard-coding index 0) is
/// required: a legacy invite's credit output need not be at index 0.
pub fn voucher_output_index(
    transaction: &Transaction,
    voucher_key: &SecretKey,
) -> Result<u32, InvitationError> {
    voucher_credit_output(transaction, voucher_key).map(|(index, _)| index)
}

/// [`voucher_output_index`] plus the selected credit output itself, so a
/// caller that needs the voucher's value reads it from the same match.
pub fn voucher_credit_output<'a>(
    transaction: &'a Transaction,
    voucher_key: &SecretKey,
) -> Result<(u32, &'a TxOut), InvitationError> {
    let Some(TransactionPayload::AssetLockPayloadType(payload)) =
        &transaction.special_transaction_payload
    else {
        return Err(invalid(
            "funding transaction is not an asset-lock special transaction",
        ));
    };
    let expected = voucher_credit_script(voucher_key);
    payload
        .credit_outputs
        .iter()
        .enumerate()
        .find(|(_, out)| out.script_pubkey == expected)
        .map(|(index, out)| (index as u32, out))
        .ok_or_else(|| {
            invalid("voucher key does not control any credit output of the funding transaction")
        })
}

/// The invitee's pre-claim view of an invitation, from
/// [`Sdk::invitation_claim_status`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvitationClaimStatus {
    /// The identity the claim would create (derived from the credit outpoint).
    pub prospective_identity_id: Identifier,
    /// Value of the credit output the voucher key controls (duffs).
    pub amount_duffs: u64,
    /// The claim would submit an InstantSend proof (the link carried an islock).
    pub is_instant: bool,
    /// The funding transaction is chain-locked. `false` with `is_instant ==
    /// false` is a ChainLock-only invitation whose claim must wait for it.
    pub is_chain_locked: bool,
    /// An identity exists at `prospective_identity_id` — definitively claimed.
    /// `false` does NOT prove the voucher is unspent (see
    /// [`Sdk::invitation_prospective_identity_id`]).
    pub already_claimed: bool,
}

/// Pre-flight the caller-supplied identity keys map: id=0 must be a MASTER-level
/// AUTHENTICATION key (it signs the IdentityCreate transition).
fn preflight_keys_map(keys_map: &BTreeMap<u32, IdentityPublicKey>) -> Result<(), InvitationError> {
    if keys_map.is_empty() {
        return Err(InvitationError::Invalid(
            "keys_map must contain at least one identity public key".to_string(),
        ));
    }
    match keys_map.get(&0) {
        Some(k)
            if k.security_level() == SecurityLevel::MASTER
                && k.purpose() == Purpose::AUTHENTICATION => {}
        Some(_) => {
            return Err(InvitationError::Invalid(
                "keys_map[0] must be a MASTER-level AUTHENTICATION key \
                 (required to sign the IdentityCreate transition)"
                    .to_string(),
            ))
        }
        None => {
            return Err(InvitationError::Invalid(
                "keys_map must include key id=0 with MASTER security level".to_string(),
            ))
        }
    }
    Ok(())
}

/// Byte-reverse a txid hex string. Old iOS invitation links carry the funding
/// txid in little-endian internal order; DAPI keys `getTransaction` by the
/// big-endian display id, so the claim retries with the id reversed on a miss
/// (mirrors Android's `Sha256Hash.wrap(id).reversedBytes` retry).
fn reverse_txid_hex(txid_hex: &str) -> Result<String, InvitationError> {
    let mut bytes = hex::decode(txid_hex).map_err(|e| {
        InvitationError::Invalid(format!("invitation funding txid is not valid hex: {e}"))
    })?;
    bytes.reverse();
    Ok(hex::encode(bytes))
}

/// RAII scrub for the voucher `PrivateKey` copy used on the claim path.
/// `dashcore::PrivateKey` wraps a `secp256k1::SecretKey` that has no
/// Drop-zeroize, so the imported bearer scalar would otherwise linger in memory
/// after `claim_invitation` returns on every exit path (success or error).
struct WipingPrivateKey(PrivateKey);

impl Drop for WipingPrivateKey {
    fn drop(&mut self) {
        self.0.inner.non_secure_erase();
    }
}

impl Sdk {
    /// The identity id this invitation WOULD create, without claiming it.
    ///
    /// Platform derives a created identity's id from the asset-lock outpoint,
    /// so the id is knowable before the claim. The claim is otherwise the only
    /// way to learn a voucher is spent, which is why a used invitation surfaces
    /// as a raw "asset lock … already completely used" after the invitee has
    /// picked a username and entered their PIN.
    ///
    /// # Detects claims, not consumption — the signal is ONE-WAY
    ///
    /// An identity existing under the returned id means the voucher was
    /// **definitely** claimed. Its absence does **not** mean the voucher is
    /// usable.
    ///
    /// The lock can also be consumed by `IdentityTopUp` — an inviter reclaiming
    /// the voucher into one of its existing identities — which credits an
    /// EXISTING identity rather than creating the derived one. Afterwards no
    /// identity exists at this id, yet a claim still fails deterministically
    /// because the asset-lock output is already spent.
    ///
    /// Platform exposes no client query for spent asset locks (drive tracks
    /// them under `SpentAssetLockTransactions`, but no DAPI endpoint surfaces
    /// it), so consumption cannot be checked from here. Callers must therefore
    /// treat this as a fast-fail for the common case only: reject the
    /// invitation when an identity exists, and otherwise proceed WITHOUT
    /// concluding the voucher is usable. It narrows when the late failure
    /// happens; it does not remove it.
    ///
    /// Costs one funding-tx fetch: the outpoint is not in the link (the credit
    /// output is selected by pk↔script match, not by index), so the tx has to
    /// be refetched exactly as the claim does. Same wrong-network fail-fast as
    /// [`Sdk::claim_invitation`], so a testnet link on mainnet reports the
    /// network mismatch rather than a confusing fetch miss.
    pub async fn invitation_prospective_identity_id(
        &self,
        invitation: &ParsedInvitation,
    ) -> Result<Identifier, InvitationError> {
        let funding = inspect_invitation_funding(self, invitation).await?;
        Ok(identity_id_for_out_point(funding.out_point))
    }

    /// What an invitation is worth and whether it was already claimed — the
    /// invitee's pre-claim check, one network round trip before any username
    /// is picked.
    ///
    /// The amount is the tier signal: the link does not say whether it funds a
    /// contested or a non-contested username, the value of the voucher's credit
    /// output does (the inviter chose it). It is read from the same refetched
    /// funding transaction the claim uses, so it is exactly what the claim
    /// would spend.
    ///
    /// `already_claimed` carries the one-way caveat of
    /// [`Sdk::invitation_prospective_identity_id`]: `true` is definitive,
    /// `false` does not prove the voucher is unspent (a reclaim top-up consumes
    /// it without creating the derived identity).
    ///
    /// It describes the funding transaction rather than requiring a buildable
    /// proof: a ChainLock-only link whose funding tx is not chain-locked yet
    /// reports `is_instant == false, is_chain_locked == false` and its amount
    /// instead of failing (the claim of such a link waits for the ChainLock).
    /// A failed identity lookup still fails the whole call.
    ///
    /// Costs one funding-tx fetch (with the claim's bounded propagation retry)
    /// and one identity fetch. Same wrong-network fail-fast as the claim.
    pub async fn invitation_claim_status(
        &self,
        invitation: &ParsedInvitation,
    ) -> Result<InvitationClaimStatus, InvitationError> {
        let funding = inspect_invitation_funding(self, invitation).await?;
        let prospective_identity_id = identity_id_for_out_point(funding.out_point);
        let already_claimed = Identity::fetch(self, prospective_identity_id)
            .await?
            .is_some();
        Ok(InvitationClaimStatus {
            prospective_identity_id,
            amount_duffs: funding.credit_output_duffs,
            is_instant: funding.instant_lock.is_some(),
            is_chain_locked: funding.is_chain_locked,
            already_claimed,
        })
    }

    /// Claim a DashPay invitation: register a NEW identity for the invitee,
    /// funded by the imported voucher.
    ///
    /// The link carries only the voucher key + funding txid (+ optional islock),
    /// not the funding proof — so this **refetches** the funding transaction
    /// from Core and reconstructs the asset-lock proof, mirroring the legacy
    /// Android claim (`TopUpRepository.obtainAssetLockTransaction`):
    /// 1. Fetch the tx by `funding_txid`; retry byte-reversed on a miss (old iOS
    ///    links are little-endian).
    /// 2. Fail-fast that the fetched tx is really the funding tx, and (if an
    ///    islock is present) that the islock locks it.
    /// 3. Select the funded credit output by pk↔script match (not index 0).
    /// 4. Build an `InstantAssetLockProof` when an islock is present (the fast
    ///    path), else a `ChainAssetLockProof` once the funding tx is chain-locked.
    ///
    /// The IS proof is the *fast* path only: a voucher islock is signed at
    /// creation, so by the time the invitee claims (minutes-to-hours later, and on
    /// testnet possibly across a quorum rotation) Drive may reject the islock as
    /// stale with `InvalidInstantAssetLockProofSignatureError` — its own message
    /// asks to "try chain asset lock proof instead". When the funding tx is
    /// already chain-locked (the usual case by claim time), this resubmits a
    /// `ChainAssetLockProof` over the SAME credit output. If the tx is not yet
    /// chain-locked no fallback is possible and the claim returns
    /// [`InvitationError::NotChainLocked`] as a retry signal.
    ///
    /// The invitee's own identity keys (`keys_map`) are signed by
    /// `identity_signer`; the asset-lock's outer state-transition signature is
    /// produced from the **imported voucher key** (`invitation.voucher_key`)
    /// via the SDK's in-process raw-key path. The invitee owns neither the
    /// lock's inputs nor its tracking. `keys_map[0]` must be a MASTER-level
    /// AUTHENTICATION key.
    ///
    /// A claim repeated after an earlier one landed is refused by Platform
    /// because its outpoint is consumed; the identity at the prospective id is
    /// returned when it carries exactly the submitted keys.
    ///
    /// The caller records the returned identity in its own wallet; nothing is
    /// persisted here. The contact-bootstrap is a separate step: on success the
    /// caller may send the inviter a contact request.
    pub async fn claim_invitation<S>(
        &self,
        invitation: ParsedInvitation,
        keys_map: BTreeMap<KeyID, IdentityPublicKey>,
        identity_signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<Identity, InvitationError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        preflight_keys_map(&keys_map)?;
        ensure_invitation_network(&invitation, self.network)?;

        // Reconstruct the funding asset-lock proof by refetching the tx. Consensus
        // enforces pk↔output, islock↔tx, and identity_id↔outpoint, so the local
        // guards are for fast-fail + correct-index selection, not theft
        // prevention (a crafted link at worst yields a failed claim).
        let fetched = fetch_funding_tx(self, &invitation).await?;
        let ReconstructedProof {
            primary,
            chain_fallback,
            out_point,
        } = assemble_asset_lock_proof(
            fetched.transaction,
            fetched.is_chain_locked,
            fetched.height,
            &invitation,
        )?;
        let prospective_identity_id = identity_id_for_out_point(out_point);

        // The voucher key signs the asset lock's outer ST signature (ECDSA over
        // the credit-output pubkey hash), scrubbed on every exit path by the
        // `WipingPrivateKey` guard.
        let voucher_priv = WipingPrivateKey(PrivateKey::new(invitation.voucher_key, self.network));

        // Build the placeholder identity ONCE so both the primary attempt and the
        // IS→CL fallback submit the same key set without a `keys_map` clone.
        let placeholder = Identity::V0(IdentityV0 {
            id: Identifier::default(),
            public_keys: keys_map,
            balance: 0,
            revision: 0,
        });

        // Submit the primary proof first and — only on Platform's stale-islock
        // rejection — the ChainLock fallback over the SAME credit output. Each
        // submission is wrapped in the CL-height-too-low retry (Platform's
        // observed Core tip briefly behind the proof's chain-locked height;
        // harmless for an IS proof, which never triggers it).
        let placeholder = &placeholder;
        let voucher_priv = &voucher_priv;
        let submitted =
            submit_claim_with_stale_islock_fallback(primary, chain_fallback, move |proof| {
                submit_with_cl_height_retry(settings, move |s| {
                    placeholder.put_to_platform_and_wait_for_response_with_private_key(
                        self,
                        proof.clone(),
                        &voucher_priv.0,
                        identity_signer,
                        s,
                    )
                })
            })
            .await;

        // A lost-ACK duplicate of this very submission ("already in the
        // mempool / in chain") is resolved inside the result wait. What
        // reaches here is a claim repeated after an earlier one landed, refused
        // because its outpoint is consumed: adopt the identity when it carries
        // exactly the keys we submitted.
        adopt_already_landed_claim(submitted, &out_point, placeholder.public_keys(), || {
            Identity::fetch(self, prospective_identity_id)
        })
        .await
    }
}

/// The wrong-network guard, the funding-tx fetch and its inspection: what
/// the invitee's read-only queries need, without requiring the tx to be
/// claimable yet (see [`inspect_funding_tx`]).
async fn inspect_invitation_funding(
    sdk: &Sdk,
    invitation: &ParsedInvitation,
) -> Result<InvitationFunding, InvitationError> {
    ensure_invitation_network(invitation, sdk.network)?;
    let fetched = fetch_funding_tx(sdk, invitation).await?;
    inspect_funding_tx(&fetched.transaction, fetched.is_chain_locked, invitation)
}

/// Fetch the invitation's funding transaction with the bounded propagation
/// retry (see [`fetch_funding_tx_with_retry`]); a miss after every attempt
/// is [`InvitationError::NotChainLocked`], a retry-later signal.
async fn fetch_funding_tx(
    sdk: &Sdk,
    invitation: &ParsedInvitation,
) -> Result<FetchedCoreTransaction, InvitationError> {
    fetch_funding_tx_with_retry(
        &invitation.funding_txid,
        |txid| async move { Ok(sdk.get_transaction(&txid).await?) },
        || rs_dapi_client::transport::sleep(CLAIM_FETCH_RETRY_DELAY),
    )
    .await
}

/// Fetch the claim's funding transaction with the bounded propagation retry —
/// the injectable orchestration seam of [`fetch_funding_tx`] (tests
/// script `fetch`/`delay`; production passes `Sdk::get_transaction` and
/// `rs_dapi_client::transport::sleep`, which also runs in the browser where
/// `tokio::time::sleep` does not).
///
/// Two independent concerns are layered here:
///   1. Byte order — old iOS links carry the txid little-endian, so a
///      canonical miss is retried byte-reversed **within the same attempt**
///      (a compatibility fallback, not a temporal retry). A transport error is
///      NOT masked as a miss — it propagates immediately (it is not a "not
///      indexed yet" signal, so retrying it would only hide it).
///   2. Propagation lag — InstantSend/ChainLock finality does not guarantee
///      the queried DAPI node has indexed the tx yet, so a fresh invitation
///      can miss on both byte orders purely from propagation delay. Retry up
///      to [`CLAIM_FETCH_MAX_ATTEMPTS`] attempts with one `delay` between
///      attempts — and none after the last (the caller surfaces the miss
///      immediately).
///
/// Every attempt missing on both byte orders is
/// [`InvitationError::NotChainLocked`]: the transaction may simply not have
/// reached the queried node yet, so the caller should retry later rather than
/// treat the link as invalid.
async fn fetch_funding_tx_with_retry<T, F, FFut, D, DFut>(
    funding_txid: &str,
    mut fetch: F,
    mut delay: D,
) -> Result<T, InvitationError>
where
    F: FnMut(String) -> FFut,
    FFut: std::future::Future<Output = Result<Option<T>, InvitationError>>,
    D: FnMut() -> DFut,
    DFut: std::future::Future<Output = ()>,
{
    for attempt in 0..CLAIM_FETCH_MAX_ATTEMPTS {
        if let Some(tx) = fetch(funding_txid.to_string()).await? {
            return Ok(tx);
        }
        // Reversed lookup is computed lazily on a canonical miss, so a hit
        // never depends on the txid being reversible hex.
        let reversed = reverse_txid_hex(funding_txid)?;
        if let Some(tx) = fetch(reversed).await? {
            return Ok(tx);
        }
        if attempt + 1 < CLAIM_FETCH_MAX_ATTEMPTS {
            delay().await;
        }
    }
    Err(InvitationError::NotChainLocked(
        "invitation funding transaction not found (tried both byte orders across \
         repeated attempts); it may not have propagated to the queried DAPI node yet — \
         retry shortly"
            .to_string(),
    ))
}

/// Submit the claim's reconstructed proof with the stale-islock → ChainLock
/// fallback — the injectable orchestration seam of the claim submission (tests
/// script `submit`; production passes the placeholder-identity put wrapped in
/// `submit_with_cl_height_retry`).
///
/// `primary` is submitted first. Exactly ONE recovery is attempted, and only
/// for the one rejection that indicts the proof kind rather than the claim:
/// `is_instant_lock_proof_invalid` (Platform rejected a stale islock — its
/// signing quorum rotated out, or it is no longer "recent"; Platform's own
/// message asks us to "try chain asset lock proof instead"). On that rejection
/// the ChainLock `chain_fallback` over the SAME credit output is submitted —
/// or, when the funding tx was not yet chain-locked so no fallback could be
/// built, [`InvitationError::NotChainLocked`] surfaces a clear retry signal instead of
/// the raw consensus error. Every other error — from the primary or from the
/// fallback itself — propagates as [`InvitationError::Sdk`] with no
/// further submission.
async fn submit_claim_with_stale_islock_fallback<T, F, Fut>(
    primary: AssetLockProof,
    chain_fallback: Option<AssetLockProof>,
    mut submit: F,
) -> Result<T, InvitationError>
where
    F: FnMut(AssetLockProof) -> Fut,
    Fut: std::future::Future<Output = Result<T, Error>>,
{
    match submit(primary).await {
        Ok(created) => Ok(created),
        Err(e) if is_instant_lock_proof_invalid(&e) => {
            let Some(chain_proof) = chain_fallback else {
                return Err(InvitationError::NotChainLocked(
                    "invitation islock proof was rejected by Platform (stale — quorum \
                     rotated or no longer recent) and the funding transaction is not yet \
                     chain-locked, so no ChainLock fallback is possible; retry once the \
                     funding block is chain-locked"
                        .to_string(),
                ));
            };
            tracing::warn!(
                "invitation IS-lock proof rejected by Platform on claim; retrying with a \
                 ChainLock proof over the same funding outpoint"
            );
            submit(chain_proof).await.map_err(InvitationError::Sdk)
        }
        Err(e) => Err(InvitationError::Sdk(e)),
    }
}

/// Refuse a link whose voucher key is for the other network before any
/// network work: a testnet WIF is a valid key on the wrong chain, so it would
/// otherwise surface as a confusing funding-tx fetch miss.
fn ensure_invitation_network(
    invitation: &ParsedInvitation,
    wallet_network: Network,
) -> Result<(), InvitationError> {
    if wif_network_matches(invitation.voucher_key_network, wallet_network) {
        Ok(())
    } else {
        Err(InvitationError::NetworkMismatch {
            invitation: invitation.voucher_key_network,
            wallet: wallet_network,
        })
    }
}

/// The identity id Platform derives from an asset-lock outpoint. Both proof
/// kinds hash only the outpoint, so the ChainLock derivation (which ignores
/// its height) gives the id for either.
fn identity_id_for_out_point(out_point: OutPoint) -> Identifier {
    ChainAssetLockProof::new(0, out_point.into()).create_identifier()
}

/// Recognize a claim Platform already executed. A claim repeated after an
/// earlier attempt landed (reported as a failure, or interrupted) is refused
/// because its outpoint is consumed; the identity at the prospective id is
/// ours when it carries exactly the keys we submitted, and is then the
/// claim's result. Anything else (no identity, someone else's, a failed
/// lookup) reports the original rejection.
///
/// `fetch` looks up the identity at the prospective id: the injectable seam
/// (production passes `Identity::fetch`).
async fn adopt_already_landed_claim<F, Fut>(
    submitted: Result<Identity, InvitationError>,
    out_point: &OutPoint,
    submitted_keys: &BTreeMap<KeyID, IdentityPublicKey>,
    fetch: F,
) -> Result<Identity, InvitationError>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<Option<Identity>, Error>>,
{
    let error = match submitted {
        Err(InvitationError::Sdk(error)) if is_asset_lock_already_consumed(&error, out_point) => {
            error
        }
        other => return other,
    };
    match fetch().await {
        Ok(Some(identity)) if identity_carries_keys(&identity, submitted_keys) => {
            tracing::info!(
                identity_id = %identity.id(),
                "invitation claim was already accepted by Platform; adopting its identity"
            );
            Ok(identity)
        }
        Ok(_) => Err(InvitationError::Sdk(error)),
        Err(lookup) => {
            tracing::warn!(
                error = %lookup,
                "could not look up the identity of a consumed invitation; reporting the rejection"
            );
            Err(InvitationError::Sdk(error))
        }
    }
}

/// The identity carries exactly `expected` public keys — same ids, same key
/// data. Key data is what only our signer could have produced, so a match
/// means the identity was created by our claim, not by a finder of the link.
fn identity_carries_keys(
    identity: &Identity,
    expected: &BTreeMap<KeyID, IdentityPublicKey>,
) -> bool {
    let actual = identity.public_keys();
    actual.len() == expected.len()
        && expected.iter().all(|(id, key)| {
            actual.get(id).is_some_and(|found| {
                found.data() == key.data() && found.key_type() == key.key_type()
            })
        })
}

/// The claim's reconstructed funding proof, plus an optional ChainLock fallback.
///
/// `primary` is submitted first: an [`AssetLockProof::Instant`] when the link
/// carried an islock (the fast path), or an [`AssetLockProof::Chain`] when it did
/// not. `chain_fallback` is a ChainLock proof over the SAME credit output,
/// populated ONLY when `primary` is InstantSend AND the funding tx is already
/// chain-locked — [`Sdk::claim_invitation`]'s stale-islock recovery
/// resubmits it if Platform rejects the primary IS proof with
/// `InvalidInstantAssetLockProofSignatureError`. It is `None` when the primary is
/// already a ChainLock proof (nothing to fall back to) or when the funding tx is
/// not yet chain-locked (no ChainLock proof can be built — the claim must be
/// retried once the block confirms).
#[derive(Debug)]
struct ReconstructedProof {
    primary: AssetLockProof,
    chain_fallback: Option<AssetLockProof>,
    /// The voucher's credit outpoint both proofs spend.
    out_point: OutPoint,
}

/// What an invitation's funding transaction says before any proof is built.
#[derive(Debug)]
struct InvitationFunding {
    /// The voucher's credit outpoint; the claim's identity id derives from it.
    out_point: OutPoint,
    /// Value of that credit output (duffs): what the invitation is worth.
    credit_output_duffs: u64,
    /// The link's islock, decoded and checked to lock this transaction.
    instant_lock: Option<InstantLock>,
    /// The funding transaction is chain-locked.
    is_chain_locked: bool,
}

/// Check a fetched funding transaction against the link and select the
/// voucher's credit output: the pure core shared by the claim's proof
/// assembly and the read-only queries. Validates the tx is the funding tx
/// (either byte order), selects the voucher's credit output, and decodes the
/// link's islock (checking it locks this tx). Chain-lock finality is reported,
/// not required: only building a ChainLock proof needs it.
fn inspect_funding_tx(
    transaction: &Transaction,
    is_chain_locked: bool,
    invitation: &ParsedInvitation,
) -> Result<InvitationFunding, InvitationError> {
    // Fail-fast: the fetched tx must actually be the funding tx (either byte
    // order). DAPI returns whatever tx matches the id we asked for, so this
    // guards a backend that answers with an unrelated tx.
    let fetched_txid = transaction.txid().to_string();
    let reversed_txid = reverse_txid_hex(&invitation.funding_txid).ok();
    if fetched_txid != invitation.funding_txid
        && reversed_txid.as_deref() != Some(fetched_txid.as_str())
    {
        return Err(InvitationError::Invalid(
            "fetched transaction id does not match the invitation funding txid".to_string(),
        ));
    }

    // Select the funded credit output the voucher key controls (not index 0
    // — a legacy invite's credit output need not be first).
    let (output_index, credit_output) =
        voucher_credit_output(transaction, &invitation.voucher_key)?;

    let instant_lock = match &invitation.islock_hex {
        Some(islock_hex) => {
            let islock_bytes = hex::decode(islock_hex).map_err(|e| {
                InvitationError::Invalid(format!("invitation islock is not valid hex: {e}"))
            })?;
            // The hex is not self-describing; the modern deterministic islock
            // (ISDLOCK) carries its version byte, which is exactly what the
            // consensus decoder reads first.
            let instant_lock = InstantLock::consensus_decode(&mut islock_bytes.as_slice())
                .map_err(|e| {
                    InvitationError::Invalid(format!("invitation islock could not be decoded: {e}"))
                })?;
            if instant_lock.txid != transaction.txid() {
                return Err(InvitationError::Invalid(
                    "invitation islock does not lock the funding transaction".to_string(),
                ));
            }
            Some(instant_lock)
        }
        None => None,
    };

    Ok(InvitationFunding {
        out_point: OutPoint::new(transaction.txid(), output_index),
        credit_output_duffs: credit_output.value,
        instant_lock,
        is_chain_locked,
    })
}

/// Assemble the asset-lock proof from an already-fetched funding transaction — the
/// pure, testable core of the claim reconstruction (the fetch/retry lives in
/// [`fetch_funding_tx`]). Checks the tx with [`inspect_funding_tx`], then
/// builds an InstantSend proof (link carried an islock) or a ChainLock proof
/// (islock absent), requiring chain-lock finality for the latter.
///
/// When an islock is present AND the funding tx is already chain-locked, the
/// returned [`ReconstructedProof`] also carries a `chain_fallback` ChainLock proof
/// over the same credit output, so the caller can recover from a stale islock that
/// Platform rejects without refetching the tx.
fn assemble_asset_lock_proof(
    transaction: Transaction,
    is_chain_locked: bool,
    height: u32,
    invitation: &ParsedInvitation,
) -> Result<ReconstructedProof, InvitationError> {
    let InvitationFunding {
        out_point,
        instant_lock,
        ..
    } = inspect_funding_tx(&transaction, is_chain_locked, invitation)?;

    // A ChainLock proof over the selected credit output. Buildable only once the
    // funding block is chain-locked; `height` is the tx's mined height (the
    // `ChainAssetLockProof`'s `core_chain_locked_height`). Reused both as the
    // primary for an islock-less invite and as the stale-islock fallback.
    let chain_lock_proof =
        || AssetLockProof::Chain(ChainAssetLockProof::new(height, out_point.into()));

    match instant_lock {
        Some(instant_lock) => {
            // Fast path: submit the InstantSend proof. If the islock is stale
            // (quorum rotated / no longer "recent") Platform rejects it, and the
            // claim falls back to `chain_fallback` — available only when the
            // funding tx is already chain-locked (the usual case by claim time,
            // since the voucher was funded minutes-to-hours earlier).
            let chain_fallback = is_chain_locked.then(chain_lock_proof);
            let primary = AssetLockProof::Instant(InstantAssetLockProof::new(
                instant_lock,
                transaction,
                out_point.vout,
            ));
            Ok(ReconstructedProof {
                primary,
                chain_fallback,
                out_point,
            })
        }
        None => {
            // ChainLock invite: the proof references the outpoint + a chain-locked
            // core height. Require the funding tx to be chain-locked so the proof
            // proves finality; the inviter/invitee retries once the block is
            // chain-locked otherwise. There is no separate fallback — this IS the
            // ChainLock proof.
            if !is_chain_locked {
                return Err(InvitationError::NotChainLocked(
                    "chainlock invitation funding transaction is not yet chain-locked; \
                     retry once it confirms"
                        .to_string(),
                ));
            }
            Ok(ReconstructedProof {
                primary: chain_lock_proof(),
                chain_fallback: None,
                out_point,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::dashcore::consensus::Encodable;
    use dpp::dashcore::transaction::special_transaction::asset_lock::AssetLockPayload;

    fn voucher() -> SecretKey {
        SecretKey::from_secret_bytes([0x11u8; 32]).expect("valid scalar")
    }

    fn inviter_info() -> InviterInfo {
        InviterInfo {
            username: Some("alice".to_string()),
            display_name: Some("Alice Example".to_string()),
            avatar_url: Some("https://example.com/a b.png?x=1".to_string()),
        }
    }

    /// Build an asset-lock tx whose credit output at `index` pays the voucher
    /// key (and `index` decoy outputs before it that do not).
    fn asset_lock_tx_paying_voucher_at(key: &SecretKey, index: usize) -> Transaction {
        let decoy = SecretKey::from_secret_bytes([0x22u8; 32]).unwrap();
        let mut credit_outputs = Vec::new();
        for _ in 0..index {
            credit_outputs.push(TxOut {
                value: 50_000,
                script_pubkey: voucher_credit_script(&decoy),
            });
        }
        credit_outputs.push(TxOut {
            value: 100_000,
            script_pubkey: voucher_credit_script(key),
        });
        let payload = AssetLockPayload {
            version: 1,
            credit_outputs,
        };
        Transaction {
            version: 3,
            lock_time: 0,
            input: vec![],
            output: vec![],
            special_transaction_payload: Some(TransactionPayload::AssetLockPayloadType(payload)),
        }
    }

    fn instant_proof_paying_voucher() -> AssetLockProof {
        let key = voucher();
        let tx = asset_lock_tx_paying_voucher_at(&key, 0);
        AssetLockProof::Instant(InstantAssetLockProof::new(InstantLock::default(), tx, 0))
    }

    #[test]
    fn should_round_trip_a_wif_keeping_compression_and_network() {
        for (network, first_byte) in [(Network::Mainnet, 204u8), (Network::Testnet, 239u8)] {
            let wif = PrivateKey::new(voucher(), network).to_wif();
            let decoded = PrivateKey::from_wif(&wif).expect("wif decodes");
            assert!(decoded.compressed, "voucher WIF must be compressed");
            assert_eq!(decoded.network, network, "network preserved");
            assert_eq!(decoded.inner.to_secret_bytes(), voucher().to_secret_bytes());
            // Network byte matches bitcoinj/legacy (0xCC mainnet, 0xEF testnet).
            let raw = dpp::dashcore::base58::decode(&wif).unwrap();
            assert_eq!(raw[0], first_byte);
        }
    }

    #[test]
    fn should_round_trip_an_encoded_link_with_inviter_info() {
        let proof = instant_proof_paying_voucher();
        let info = inviter_info();
        let uri = encode_invitation_uri(&voucher(), Network::Testnet, &proof, Some(&info))
            .expect("encode");
        assert!(uri.starts_with("dashpay://invite?"));

        let parsed = parse_invitation_uri(&uri).expect("parse");
        assert_eq!(
            parsed.voucher_key.to_secret_bytes(),
            voucher().to_secret_bytes()
        );
        assert_eq!(parsed.inviter, Some(info));
        assert!(parsed.islock_hex.is_some());
        // The parsed txid matches the proof's transaction id (big-endian).
        let expected_txid = match &proof {
            AssetLockProof::Instant(i) => i.transaction().txid().to_string(),
            _ => unreachable!(),
        };
        assert_eq!(parsed.funding_txid, expected_txid);
    }

    #[test]
    fn should_round_trip_an_encoded_link_without_inviter_info() {
        let proof = instant_proof_paying_voucher();
        let uri =
            encode_invitation_uri(&voucher(), Network::Mainnet, &proof, None).expect("encode");
        let parsed = parse_invitation_uri(&uri).expect("parse");
        assert!(parsed.inviter.is_none(), "du-less link ⇒ no inviter");
        assert!(!uri.contains("du="), "pure voucher emits no du");
    }

    /// Params in a non-canonical order must still parse (field-level, not
    /// byte-level, interop — the two legacy wallets differ in order).
    #[test]
    fn should_parse_a_link_regardless_of_field_order() {
        let wif = PrivateKey::new(voucher(), Network::Testnet).to_wif();
        let uri = format!("dashpay://invite?islock=deadbeef&pk={wif}&du=bob&assetlocktx=aabbcc");
        let parsed = parse_invitation_uri(&uri).expect("parse");
        assert_eq!(parsed.funding_txid, "aabbcc");
        assert_eq!(parsed.islock_hex.as_deref(), Some("deadbeef"));
        assert_eq!(
            parsed.inviter.as_ref().unwrap().username.as_deref(),
            Some("bob")
        );
    }

    /// The `https://invitations.dashpay.io/applink` host is accepted as a
    /// first-class alternative to the custom scheme (iOS emits it).
    #[test]
    fn should_parse_a_link_on_the_applink_host() {
        let wif = PrivateKey::new(voucher(), Network::Testnet).to_wif();
        let uri = format!(
            "https://invitations.dashpay.io/applink?du=carol&assetlocktx=aabb&pk={wif}&islock=cc"
        );
        let parsed = parse_invitation_uri(&uri).expect("parse applink");
        assert_eq!(
            parsed.inviter.as_ref().unwrap().username.as_deref(),
            Some("carol")
        );
        assert_eq!(parsed.funding_txid, "aabb");
    }

    /// The link carries the plaintext voucher WIF, so a bearer link must not be
    /// accepted over cleartext `http`. Both mobile wallets emit only `https`.
    #[test]
    fn should_reject_an_applink_over_plain_http() {
        let wif = PrivateKey::new(voucher(), Network::Testnet).to_wif();
        let uri =
            format!("http://invitations.dashpay.io/applink?du=carol&assetlocktx=aabb&pk={wif}");
        assert!(matches!(
            parse_invitation_uri(&uri),
            Err(InvitationError::Invalid(_))
        ));
    }

    /// `islock` present / absent / `"null"`: only a real hex value yields
    /// `Some`; both a missing param and the literal `"null"` yield `None`
    /// (a ChainLock-confirmed invite Android's own validator accepts).
    #[test]
    fn should_parse_an_islock_that_is_present_absent_or_null() {
        let wif = PrivateKey::new(voucher(), Network::Testnet).to_wif();
        let present = format!("dashpay://invite?assetlocktx=aa&pk={wif}&islock=00aa11");
        assert_eq!(
            parse_invitation_uri(&present)
                .unwrap()
                .islock_hex
                .as_deref(),
            Some("00aa11")
        );

        let absent = format!("dashpay://invite?assetlocktx=aa&pk={wif}");
        assert!(parse_invitation_uri(&absent).unwrap().islock_hex.is_none());

        let null = format!("dashpay://invite?assetlocktx=aa&pk={wif}&islock=null");
        assert!(
            parse_invitation_uri(&null).unwrap().islock_hex.is_none(),
            "islock=null must be treated as no instant lock"
        );
    }

    /// A `du`-less link still parses (iOS accepts du-less links).
    #[test]
    fn should_parse_a_link_without_a_du_field() {
        let wif = PrivateKey::new(voucher(), Network::Testnet).to_wif();
        let uri = format!("dashpay://invite?assetlocktx=aa&pk={wif}&islock=bb");
        let parsed = parse_invitation_uri(&uri).expect("parse");
        assert!(parsed.inviter.is_none());
    }

    #[test]
    fn should_reject_a_link_missing_pk_or_assetlocktx() {
        let wif = PrivateKey::new(voucher(), Network::Testnet).to_wif();
        // No pk.
        assert!(parse_invitation_uri("dashpay://invite?assetlocktx=aa").is_err());
        // No assetlocktx.
        assert!(parse_invitation_uri(&format!("dashpay://invite?pk={wif}")).is_err());
        // Blank assetlocktx is treated as missing.
        assert!(parse_invitation_uri(&format!("dashpay://invite?assetlocktx=&pk={wif}")).is_err());
    }

    #[test]
    fn should_reject_a_link_with_the_wrong_scheme_or_host() {
        assert!(parse_invitation_uri("https://example.com/foo?pk=x").is_err());
        assert!(parse_invitation_uri("dashpay://contact?pk=x").is_err());
    }

    #[test]
    fn should_reject_an_uncompressed_wif() {
        let uncompressed = PrivateKey::new_uncompressed(voucher(), Network::Testnet).to_wif();
        let uri = format!("dashpay://invite?assetlocktx=aa&pk={uncompressed}");
        let err = parse_invitation_uri(&uri).unwrap_err();
        assert!(err.to_string().contains("compressed"));
    }

    #[test]
    fn should_reject_a_malformed_wif() {
        let err = parse_invitation_uri("dashpay://invite?assetlocktx=aa&pk=not-a-wif").unwrap_err();
        assert!(err.to_string().contains("WIF"));
    }

    #[test]
    fn should_reject_an_oversized_uri() {
        let huge = format!("dashpay://invite?pk={}", "z".repeat(MAX_INVITATION_URI_LEN));
        let err = parse_invitation_uri(&huge).unwrap_err();
        assert!(err.to_string().contains("too long"));
    }

    /// A hand-crafted Android-style link (params in Android's emit order, with
    /// display-name + avatar-url) parses to the right fields.
    #[test]
    fn should_parse_an_android_style_link() {
        let wif = PrivateKey::new(voucher(), Network::Mainnet).to_wif();
        // Android order: du, assetlocktx, pk, islock, display-name, avatar-url.
        let uri = format!(
            "dashpay://invite?du=satoshi&assetlocktx={txid}&pk={wif}&islock={islock}&display-name=Sat%20Oshi&avatar-url=https%3A%2F%2Fimg.example%2Fa.png",
            txid = "e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d",
            islock = "01"
        );
        let parsed = parse_invitation_uri(&uri).expect("parse android link");
        assert_eq!(
            parsed.funding_txid,
            "e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d"
        );
        assert_eq!(parsed.islock_hex.as_deref(), Some("01"));
        let inviter = parsed.inviter.as_ref().expect("inviter");
        assert_eq!(inviter.username.as_deref(), Some("satoshi"));
        assert_eq!(inviter.display_name.as_deref(), Some("Sat Oshi"));
        assert_eq!(
            inviter.avatar_url.as_deref(),
            Some("https://img.example/a.png")
        );
    }

    /// The voucher output is selected by pk↔script match, not hard-coded to 0.
    #[test]
    fn should_select_the_output_that_pays_the_voucher_key() {
        let key = voucher();
        // Voucher output sits at index 2, behind two decoy outputs.
        let tx = asset_lock_tx_paying_voucher_at(&key, 2);
        assert_eq!(voucher_output_index(&tx, &key).unwrap(), 2);
    }

    /// The credit output comes back with its index, so its value is the
    /// voucher's, not a decoy's.
    #[test]
    fn should_return_the_voucher_credit_output_with_its_index() {
        let key = voucher();
        let tx = asset_lock_tx_paying_voucher_at(&key, 2);
        let (index, output) = voucher_credit_output(&tx, &key).unwrap();
        assert_eq!(index, 2);
        assert_eq!(output.value, 100_000, "the decoys before it carry 50_000");
        assert_eq!(output.script_pubkey, voucher_credit_script(&key));
    }

    #[test]
    fn should_reject_a_funding_tx_with_no_output_paying_the_voucher() {
        let key = voucher();
        let other = SecretKey::from_secret_bytes([0x33u8; 32]).unwrap();
        let tx = asset_lock_tx_paying_voucher_at(&other, 0);
        let err = voucher_output_index(&tx, &key).unwrap_err();
        assert!(err.to_string().contains("does not control"));
    }

    #[test]
    fn should_reject_a_funding_tx_that_is_not_an_asset_lock() {
        let key = voucher();
        let tx = Transaction {
            version: 2,
            lock_time: 0,
            input: vec![],
            output: vec![],
            special_transaction_payload: None,
        };
        assert!(voucher_output_index(&tx, &key).is_err());
    }

    /// A `du`-less link that still carries `display-name`/`avatar-url` must
    /// surface them (the reference wallets emit those independently of `du`).
    #[test]
    fn should_keep_inviter_metadata_when_the_du_field_is_missing() {
        let wif = PrivateKey::new(voucher(), Network::Testnet).to_wif();
        let uri = format!(
            "dashpay://invite?assetlocktx=aa&pk={wif}&display-name=No%20User&avatar-url=https%3A%2F%2Fimg%2Fx.png"
        );
        let parsed = parse_invitation_uri(&uri).expect("parse");
        let inviter = parsed.inviter.as_ref().expect("metadata-only inviter");
        assert!(inviter.username.is_none(), "no du ⇒ no username");
        assert_eq!(inviter.display_name.as_deref(), Some("No User"));
        assert_eq!(inviter.avatar_url.as_deref(), Some("https://img/x.png"));
    }

    /// A duplicated required key is ambiguous (iOS binds the last, Android the
    /// first) — reject rather than claim a wallet-dependent key.
    #[test]
    fn should_reject_a_link_with_duplicate_required_keys() {
        let wif = PrivateKey::new(voucher(), Network::Testnet).to_wif();
        let other = PrivateKey::new(
            SecretKey::from_secret_bytes([0x44u8; 32]).unwrap(),
            Network::Testnet,
        )
        .to_wif();
        let dup_pk = format!("dashpay://invite?assetlocktx=aa&pk={wif}&pk={other}");
        assert!(parse_invitation_uri(&dup_pk)
            .unwrap_err()
            .to_string()
            .contains("duplicated"));

        let dup_tx = format!("dashpay://invite?assetlocktx=aa&assetlocktx=bb&pk={wif}");
        assert!(parse_invitation_uri(&dup_tx)
            .unwrap_err()
            .to_string()
            .contains("duplicated"));
    }

    /// The scheme/host gate must be anchored: a suffixed scheme host
    /// (`dashpay://inviteXYZ`) and a decoy URL that merely *mentions* the applink
    /// host in its own query must both be rejected — otherwise the decoy's query
    /// (with a `pk`/`assetlocktx`) would be parsed.
    #[test]
    fn should_reject_a_link_with_a_decoy_scheme_or_host() {
        let wif = PrivateKey::new(voucher(), Network::Testnet).to_wif();
        // Suffixed authority — not exactly `invite`.
        let suffixed = format!("dashpay://inviteXYZ?assetlocktx=aa&pk={wif}");
        assert!(parse_invitation_uri(&suffixed).is_err());
        // Decoy host: the real host is evil.example; the applink string only
        // appears inside the decoy's own query value.
        let decoy = format!(
            "https://evil.example/r?next=invitations.dashpay.io/applink&assetlocktx=aa&pk={wif}"
        );
        assert!(parse_invitation_uri(&decoy).is_err());
        // Wrong path on the right host.
        let wrong_path = format!("https://invitations.dashpay.io/evil?assetlocktx=aa&pk={wif}");
        assert!(parse_invitation_uri(&wrong_path).is_err());
    }

    /// WIF network compatibility is mainnet-vs-not: a testnet WIF is rejected on
    /// a mainnet wallet, but accepted on any testnet-family wallet.
    #[test]
    fn should_match_wif_networks_by_mainnet_versus_testnet_family() {
        assert!(wif_network_matches(Network::Mainnet, Network::Mainnet));
        assert!(wif_network_matches(Network::Testnet, Network::Testnet));
        // WIF decodes 0xEF to Testnet; a devnet/regtest wallet must still accept.
        assert!(wif_network_matches(Network::Testnet, Network::Devnet));
        assert!(wif_network_matches(Network::Testnet, Network::Regtest));
        // Cross mainnet/testnet is rejected (the wrong-chain paste).
        assert!(!wif_network_matches(Network::Testnet, Network::Mainnet));
        assert!(!wif_network_matches(Network::Mainnet, Network::Testnet));
    }

    // --- assemble_asset_lock_proof: the claim-time proof reconstruction guards ---

    fn parsed(
        key: SecretKey,
        funding_txid: String,
        islock_hex: Option<String>,
    ) -> ParsedInvitation {
        ParsedInvitation {
            voucher_key: key,
            voucher_key_network: Network::Testnet,
            funding_txid,
            islock_hex,
            inviter: None,
        }
    }

    /// A backend answering with an unrelated tx (id matches neither byte order) is
    /// rejected before any proof is built.
    #[test]
    fn should_reject_a_fetched_tx_whose_id_does_not_match() {
        let key = voucher();
        let tx = asset_lock_tx_paying_voucher_at(&key, 0);
        let inv = parsed(key, "00".repeat(32), None);
        let err = assemble_asset_lock_proof(tx, true, 100, &inv).unwrap_err();
        assert!(format!("{err}").contains("does not match"));
    }

    /// A ChainLock invite (no islock) whose funding tx is not yet chain-locked is
    /// not claimable yet — the proof must prove finality — and that is the
    /// retry-later signal, not an invalid link.
    #[test]
    fn should_ask_to_retry_a_chainlock_invitation_until_it_is_chain_locked() {
        let key = voucher();
        let tx = asset_lock_tx_paying_voucher_at(&key, 0);
        let txid = tx.txid().to_string();
        let inv = parsed(key, txid, None);
        let err = assemble_asset_lock_proof(tx, false, 100, &inv).unwrap_err();
        assert!(
            matches!(err, InvitationError::NotChainLocked(_)),
            "expected the retry-later signal, got {err:?}"
        );
    }

    /// A chain-locked ChainLock invite (no islock) assembles a ChainLock proof at
    /// the tx's voucher output, with no separate fallback (the primary IS the CL
    /// proof).
    #[test]
    fn should_assemble_a_chainlock_proof_once_chain_locked() {
        let key = voucher();
        let tx = asset_lock_tx_paying_voucher_at(&key, 0);
        let txid = tx.txid().to_string();
        let inv = parsed(key, txid, None);
        let reconstructed = assemble_asset_lock_proof(tx, true, 100, &inv).unwrap();
        assert!(matches!(reconstructed.primary, AssetLockProof::Chain(_)));
        assert!(
            reconstructed.chain_fallback.is_none(),
            "an islock-less chainlock invite has no separate fallback"
        );
    }

    /// An islock that locks the funding tx, with the tx already chain-locked,
    /// yields an InstantSend primary (fast path) PLUS a ChainLock fallback over the
    /// same credit output — the stale-islock recovery the claim path submits if
    /// Platform rejects the IS proof with `InvalidInstantAssetLockProofSignatureError`.
    #[test]
    fn should_carry_a_chain_fallback_for_a_chain_locked_islock_invitation() {
        let key = voucher();
        let tx = asset_lock_tx_paying_voucher_at(&key, 0);
        let txid = tx.txid().to_string();
        let islock = InstantLock {
            txid: tx.txid(),
            ..Default::default()
        };
        let mut islock_bytes = Vec::new();
        islock.consensus_encode(&mut islock_bytes).unwrap();
        let inv = parsed(key, txid, Some(hex::encode(islock_bytes)));
        let reconstructed = assemble_asset_lock_proof(tx, true, 100, &inv).unwrap();
        assert!(
            matches!(reconstructed.primary, AssetLockProof::Instant(_)),
            "the islock fast path must be tried first"
        );
        assert!(
            matches!(reconstructed.chain_fallback, Some(AssetLockProof::Chain(_))),
            "a chain-locked funding tx must carry a ChainLock fallback for stale-islock recovery"
        );
    }

    /// An islock that locks the funding tx, but the tx NOT yet chain-locked, yields
    /// the IS primary with NO fallback: if that islock is later rejected there is
    /// no ChainLock proof to fall back to, and the claim must be retried once the
    /// block confirms.
    #[test]
    fn should_have_no_chain_fallback_for_an_islock_invitation_that_is_not_chain_locked() {
        let key = voucher();
        let tx = asset_lock_tx_paying_voucher_at(&key, 0);
        let txid = tx.txid().to_string();
        let islock = InstantLock {
            txid: tx.txid(),
            ..Default::default()
        };
        let mut islock_bytes = Vec::new();
        islock.consensus_encode(&mut islock_bytes).unwrap();
        let inv = parsed(key, txid, Some(hex::encode(islock_bytes)));
        let reconstructed = assemble_asset_lock_proof(tx, false, 100, &inv).unwrap();
        assert!(matches!(reconstructed.primary, AssetLockProof::Instant(_)));
        assert!(
            reconstructed.chain_fallback.is_none(),
            "an un-chain-locked funding tx cannot produce a ChainLock fallback"
        );
    }

    /// The prospective identity id is derived from the *selected* credit
    /// output's outpoint. Selection itself is pinned next to
    /// `voucher_output_index`; what matters here is that the id follows it — a
    /// voucher sitting behind a decoy must not yield the index-0 id, or the
    /// "has this invitation been used?" check answers about a stranger's
    /// identity and reports a perfectly good voucher as spent.
    #[test]
    fn should_derive_the_prospective_id_from_the_selected_credit_output() {
        let key = voucher();
        let decoy = SecretKey::from_secret_bytes([0x22u8; 32]).unwrap();
        let payload = AssetLockPayload {
            version: 1,
            credit_outputs: vec![
                TxOut {
                    value: 100_000,
                    script_pubkey: voucher_credit_script(&decoy),
                },
                TxOut {
                    value: 25_000_000,
                    script_pubkey: voucher_credit_script(&key),
                },
            ],
        };
        let tx = Transaction {
            version: 3,
            lock_time: 0,
            input: vec![],
            output: vec![],
            special_transaction_payload: Some(TransactionPayload::AssetLockPayloadType(payload)),
        };
        let txid = tx.txid();
        let inv = parsed(key, txid.to_string(), None);

        let funding = inspect_funding_tx(&tx, true, &inv).unwrap();
        assert_eq!(
            funding.credit_output_duffs, 25_000_000,
            "the amount must come from the voucher's output, not the decoy at index 0"
        );
        let proof = assemble_asset_lock_proof(tx, true, 100, &inv).unwrap();
        let id = proof.primary.create_identifier().unwrap();
        assert_eq!(
            identity_id_for_out_point(funding.out_point),
            id,
            "the read-only queries must derive the id the claim creates"
        );

        let from_index_0 =
            ChainAssetLockProof::new(100, OutPoint::new(txid, 0).into()).create_identifier();
        let from_index_1 =
            ChainAssetLockProof::new(100, OutPoint::new(txid, 1).into()).create_identifier();

        assert_ne!(id, from_index_0, "id must not come from credit output 0");
        assert_eq!(id, from_index_1);
    }

    /// The prospective id is a pure function of the asset-lock OUTPOINT, so it
    /// carries no information about whether that lock has been consumed.
    ///
    /// This is why the claimed-check is one-way. A voucher claimed normally
    /// creates the identity at this id, and the check sees it. A voucher
    /// RECLAIMED into an existing identity — an `IdentityTopUp` the inviter
    /// funds with the voucher's asset lock — spends the very same outpoint but
    /// creates nothing here, so the check still finds no identity while a claim
    /// would fail deterministically.
    ///
    /// Pinned as an executable fact because the id derivation is what a reader
    /// would otherwise assume encodes "spent": it does not, and Platform
    /// exposes no client query for spent asset locks to fill the gap.
    #[test]
    fn should_derive_the_prospective_id_from_the_outpoint_alone() {
        let key = voucher();
        let payload = AssetLockPayload {
            version: 1,
            credit_outputs: vec![TxOut {
                value: 100_000,
                script_pubkey: voucher_credit_script(&key),
            }],
        };
        let tx = Transaction {
            version: 3,
            lock_time: 0,
            input: vec![],
            output: vec![],
            special_transaction_payload: Some(TransactionPayload::AssetLockPayloadType(payload)),
        };
        let txid = tx.txid();
        let inv = parsed(key, txid.to_string(), None);

        let id = assemble_asset_lock_proof(tx, true, 100, &inv)
            .unwrap()
            .primary
            .create_identifier()
            .unwrap();

        // Nothing but (txid, vout) feeds it — the same value a reclaim would
        // leave behind untouched.
        let from_outpoint =
            ChainAssetLockProof::new(100, OutPoint::new(txid, 0).into()).create_identifier();
        assert_eq!(
            id, from_outpoint,
            "the id must be derivable from the outpoint alone, which is exactly \
             why its absence cannot prove the lock is unspent"
        );
    }

    /// A test identity key: `byte` fills the key data, so two keys differ
    /// exactly when their bytes do.
    fn test_key(id: KeyID, byte: u8) -> IdentityPublicKey {
        use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
        use dpp::identity::KeyType;
        use dpp::platform_value::BinaryData;

        IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::MASTER,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(vec![byte; 33]),
            disabled_at: None,
        })
    }

    fn identity_with_keys(keys: BTreeMap<KeyID, IdentityPublicKey>) -> Identity {
        Identity::V0(IdentityV0 {
            id: Identifier::default(),
            public_keys: keys,
            balance: 0,
            revision: 0,
        })
    }

    fn our_keys() -> BTreeMap<KeyID, IdentityPublicKey> {
        [(0, test_key(0, 1)), (1, test_key(1, 2))].into()
    }

    /// A claim that already landed is recognized as ours only by its keys:
    /// the same key set is ours, anything else (a finder of the link claiming
    /// first) is not.
    #[test]
    fn should_recognize_a_claimed_identity_only_by_our_keys() {
        let ours = our_keys();
        assert!(identity_carries_keys(
            &identity_with_keys(ours.clone()),
            &ours
        ));
        let theirs: BTreeMap<KeyID, IdentityPublicKey> =
            [(0, test_key(0, 9)), (1, test_key(1, 2))].into();
        assert!(!identity_carries_keys(&identity_with_keys(theirs), &ours));
        let fewer: BTreeMap<KeyID, IdentityPublicKey> = [(0, test_key(0, 1))].into();
        assert!(!identity_carries_keys(&identity_with_keys(fewer), &ours));
    }

    /// A testnet link in a mainnet wallet is refused with the typed
    /// `InvitationError::NetworkMismatch`.
    #[test]
    fn should_refuse_an_invitation_for_the_other_network() {
        let inv = parsed(voucher(), "00".repeat(32), None);
        assert!(ensure_invitation_network(&inv, Network::Testnet).is_ok());
        assert!(matches!(
            ensure_invitation_network(&inv, Network::Mainnet),
            Err(InvitationError::NetworkMismatch {
                invitation: Network::Testnet,
                wallet: Network::Mainnet,
            })
        ));
    }

    /// A ChainLock-only link whose funding tx is not chain-locked yet cannot
    /// be claimed, but the read-only view still reads its amount and outpoint
    /// and reports the missing ChainLock instead of failing.
    #[test]
    fn should_inspect_a_chainlock_invitation_before_it_is_chain_locked() {
        let key = voucher();
        let tx = asset_lock_tx_paying_voucher_at(&key, 0);
        let txid = tx.txid();
        let inv = parsed(key, txid.to_string(), None);

        let funding = inspect_funding_tx(&tx, false, &inv).unwrap();
        assert_eq!(funding.credit_output_duffs, 100_000);
        assert_eq!(funding.out_point, OutPoint::new(txid, 0));
        assert!(funding.instant_lock.is_none());
        assert!(!funding.is_chain_locked);
        assert!(
            assemble_asset_lock_proof(tx, false, 100, &inv).is_err(),
            "the claim itself still waits for the ChainLock"
        );
    }

    /// The InstantSend read-only view carries the decoded islock, and its id
    /// matches the claim's InstantSend primary.
    #[test]
    fn should_inspect_an_instant_invitation_with_the_claims_identity_id() {
        let key = voucher();
        let tx = asset_lock_tx_paying_voucher_at(&key, 0);
        let txid = tx.txid().to_string();
        let islock = InstantLock {
            txid: tx.txid(),
            ..Default::default()
        };
        let mut islock_bytes = Vec::new();
        islock.consensus_encode(&mut islock_bytes).unwrap();
        let inv = parsed(key, txid, Some(hex::encode(islock_bytes)));

        let funding = inspect_funding_tx(&tx, true, &inv).unwrap();
        assert!(funding.instant_lock.is_some());
        assert_eq!(funding.credit_output_duffs, 100_000);
        let primary = assemble_asset_lock_proof(tx, true, 100, &inv)
            .unwrap()
            .primary;
        assert_eq!(
            identity_id_for_out_point(funding.out_point),
            primary.create_identifier().unwrap()
        );
    }

    // --- adopt_already_landed_claim: recognizing our own earlier claim ---

    mod claim_adoption {
        use std::cell::Cell;

        use super::super::adopt_already_landed_claim;
        use super::super::{Error, InvitationError};
        use super::{identity_with_keys, our_keys, test_key};
        use dpp::consensus::basic::identity::IdentityAssetLockTransactionOutPointAlreadyConsumedError;
        use dpp::consensus::basic::BasicError;
        use dpp::consensus::ConsensusError;
        use dpp::dashcore::hashes::Hash;
        use dpp::dashcore::{OutPoint, Txid};
        use dpp::identity::{Identity, IdentityPublicKey, KeyID};
        use std::collections::BTreeMap;

        fn out_point() -> OutPoint {
            OutPoint::new(Txid::from_byte_array([7u8; 32]), 1)
        }

        /// Platform's consumed-outpoint rejection for `out_point`.
        fn consumed(out_point: OutPoint) -> InvitationError {
            InvitationError::Sdk(Error::Protocol(dpp::ProtocolError::ConsensusError(
                Box::new(ConsensusError::BasicError(
                    BasicError::IdentityAssetLockTransactionOutPointAlreadyConsumedError(
                        IdentityAssetLockTransactionOutPointAlreadyConsumedError::new(
                            out_point.txid,
                            out_point.vout as usize,
                        ),
                    ),
                )),
            )))
        }

        fn is_consumed(result: &Result<Identity, InvitationError>) -> bool {
            matches!(result, Err(InvitationError::Sdk(Error::Protocol(_))))
        }

        /// Run the seam with a scripted lookup, counting how often it is called.
        async fn adopt(
            submitted: Result<Identity, InvitationError>,
            lookup: Result<Option<Identity>, Error>,
        ) -> (Result<Identity, InvitationError>, u32) {
            let calls = Cell::new(0);
            let keys = our_keys();
            let result = adopt_already_landed_claim(submitted, &out_point(), &keys, || {
                calls.set(calls.get() + 1);
                async { lookup }
            })
            .await;
            (result, calls.get())
        }

        #[tokio::test]
        async fn should_pass_a_successful_claim_through_without_a_lookup() {
            let (result, lookups) = adopt(Ok(identity_with_keys(our_keys())), Ok(None)).await;
            assert!(result.is_ok());
            assert_eq!(lookups, 0);
        }

        #[tokio::test]
        async fn should_adopt_our_identity_behind_a_consumed_outpoint() {
            let (result, lookups) = adopt(
                Err(consumed(out_point())),
                Ok(Some(identity_with_keys(our_keys()))),
            )
            .await;
            assert!(result.is_ok(), "our own landed claim is the claim's result");
            assert_eq!(lookups, 1);
        }

        #[tokio::test]
        async fn should_report_the_rejection_when_someone_else_claimed_first() {
            let theirs: BTreeMap<KeyID, IdentityPublicKey> = [(0, test_key(0, 9))].into();
            let (result, _) = adopt(
                Err(consumed(out_point())),
                Ok(Some(identity_with_keys(theirs))),
            )
            .await;
            assert!(is_consumed(&result));
        }

        /// A reclaim top-up consumes the outpoint without creating the
        /// identity: nothing to adopt.
        #[tokio::test]
        async fn should_report_the_rejection_when_no_identity_exists() {
            let (result, _) = adopt(Err(consumed(out_point())), Ok(None)).await;
            assert!(is_consumed(&result));
        }

        /// A failed lookup must not replace the rejection that explains the
        /// failure.
        #[tokio::test]
        async fn should_keep_the_rejection_when_the_lookup_fails() {
            let (result, _) = adopt(
                Err(consumed(out_point())),
                Err(Error::Generic("timeout".to_string())),
            )
            .await;
            assert!(is_consumed(&result));
        }

        /// A consumed report for another outpoint is not about this claim.
        #[tokio::test]
        async fn should_ignore_a_consumed_report_for_another_outpoint() {
            let other = OutPoint::new(Txid::from_byte_array([8u8; 32]), 1);
            let (result, lookups) = adopt(
                Err(consumed(other)),
                Ok(Some(identity_with_keys(our_keys()))),
            )
            .await;
            assert!(is_consumed(&result));
            assert_eq!(lookups, 0);
        }

        #[tokio::test]
        async fn should_pass_unrelated_errors_through_without_a_lookup() {
            let (result, lookups) = adopt(
                Err(InvitationError::Sdk(Error::Generic(
                    "unrelated".to_string(),
                ))),
                Ok(Some(identity_with_keys(our_keys()))),
            )
            .await;
            assert!(matches!(
                result,
                Err(InvitationError::Sdk(Error::Generic(_)))
            ));
            assert_eq!(lookups, 0);
        }
    }

    /// An islock that locks a DIFFERENT tx than the funding tx is rejected (the
    /// txid-binding guard), so a link can't pair a valid islock with a foreign tx.
    #[test]
    fn should_reject_an_islock_that_locks_another_tx() {
        let key = voucher();
        let tx = asset_lock_tx_paying_voucher_at(&key, 0);
        let txid = tx.txid().to_string();
        // A default islock carries a zeroed txid, which won't equal the funding tx.
        let mut islock_bytes = Vec::new();
        InstantLock::default()
            .consensus_encode(&mut islock_bytes)
            .unwrap();
        let inv = parsed(key, txid, Some(hex::encode(islock_bytes)));
        let err = assemble_asset_lock_proof(tx, true, 100, &inv).unwrap_err();
        assert!(format!("{err}").contains("does not lock the funding transaction"));
    }

    // --- fetch_funding_tx_with_retry: the claim propagation-retry seam ---

    mod fetch_retry {
        use std::collections::VecDeque;
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::{Arc, Mutex};

        use super::super::InvitationError;
        use super::super::{
            fetch_funding_tx_with_retry, reverse_txid_hex, CLAIM_FETCH_MAX_ATTEMPTS,
        };

        /// A canonical 64-hex txid whose byte-reversed form differs from it.
        fn canonical_txid() -> String {
            format!("{}{}", "11".repeat(31), "22")
        }

        /// Scripted fetch: pops the next result per call and records the txid
        /// each call asked for; plus a counter-only delay.
        #[allow(clippy::type_complexity)]
        fn harness(
            script: Vec<Result<Option<u32>, InvitationError>>,
        ) -> (
            Arc<Mutex<VecDeque<Result<Option<u32>, InvitationError>>>>,
            Arc<Mutex<Vec<String>>>,
            Arc<AtomicU32>,
        ) {
            (
                Arc::new(Mutex::new(script.into_iter().collect())),
                Arc::new(Mutex::new(Vec::new())),
                Arc::new(AtomicU32::new(0)),
            )
        }

        async fn run(
            script: Vec<Result<Option<u32>, InvitationError>>,
        ) -> (Result<u32, InvitationError>, Vec<String>, u32) {
            let canonical = canonical_txid();
            let (queue, calls, delays) = harness(script);
            let (queue_f, calls_f, delays_f) =
                (Arc::clone(&queue), Arc::clone(&calls), Arc::clone(&delays));
            let result = fetch_funding_tx_with_retry(
                &canonical,
                move |txid| {
                    let queue = Arc::clone(&queue_f);
                    let calls = Arc::clone(&calls_f);
                    async move {
                        calls.lock().expect("calls").push(txid);
                        queue
                            .lock()
                            .expect("script")
                            .pop_front()
                            .expect("script exhausted — the loop fetched more than scripted")
                    }
                },
                move || {
                    let delays = Arc::clone(&delays_f);
                    async move {
                        delays.fetch_add(1, Ordering::SeqCst);
                    }
                },
            )
            .await;
            let recorded = calls.lock().expect("calls").clone();
            (result, recorded, delays.load(Ordering::SeqCst))
        }

        /// A first-call canonical hit returns immediately: one lookup, no
        /// reversed fallback, no delay.
        #[tokio::test]
        async fn should_return_a_canonical_hit_immediately() {
            let (result, calls, delays) = run(vec![Ok(Some(7))]).await;
            assert_eq!(result.expect("no error"), 7);
            assert_eq!(calls, vec![canonical_txid()]);
            assert_eq!(delays, 0);
        }

        /// A canonical miss falls back to the byte-reversed lookup WITHIN the
        /// same attempt (the legacy-iOS little-endian compatibility path) — no
        /// delay is spent on the byte-order fallback.
        #[tokio::test]
        async fn should_try_the_reversed_txid_in_the_same_attempt_without_a_delay() {
            let (result, calls, delays) = run(vec![Ok(None), Ok(Some(9))]).await;
            assert_eq!(result.expect("no error"), 9);
            assert_eq!(
                calls,
                vec![
                    canonical_txid(),
                    reverse_txid_hex(&canonical_txid()).expect("reversible")
                ]
            );
            assert_eq!(delays, 0);
        }

        /// A hit on a later attempt (propagation lag) succeeds after exactly
        /// one delay per fully-missed attempt.
        #[tokio::test]
        async fn should_wait_once_per_fully_missed_attempt_before_a_hit() {
            // Attempts 1 and 2 miss on both orders; attempt 3 hits canonically.
            let (result, calls, delays) =
                run(vec![Ok(None), Ok(None), Ok(None), Ok(None), Ok(Some(3))]).await;
            assert_eq!(result.expect("no error"), 3);
            assert_eq!(calls.len(), 5);
            assert_eq!(delays, 2);
        }

        /// Exhausting every attempt is the retry-later signal (the tx may not
        /// have reached the node yet), not an invalid link, after 2 lookups per
        /// attempt, with NO delay after the final attempt.
        #[tokio::test]
        async fn should_ask_to_retry_later_after_every_attempt_misses() {
            let script = (0..CLAIM_FETCH_MAX_ATTEMPTS * 2)
                .map(|_| Ok(None))
                .collect();
            let (result, calls, delays) = run(script).await;
            assert!(
                matches!(result, Err(InvitationError::NotChainLocked(_))),
                "a funding tx that never appears must be a retry-later signal, got {result:?}"
            );
            assert_eq!(calls.len(), (CLAIM_FETCH_MAX_ATTEMPTS * 2) as usize);
            assert_eq!(delays, CLAIM_FETCH_MAX_ATTEMPTS - 1);
        }

        /// A transport error is NOT masked as a miss: it propagates
        /// immediately, with no further lookups and no extra delay.
        #[tokio::test]
        async fn should_propagate_a_transport_error_immediately() {
            // Attempt 1 misses both orders (1 delay); attempt 2's canonical
            // lookup errors.
            let (result, calls, delays) = run(vec![
                Ok(None),
                Ok(None),
                Err(InvitationError::Invalid(
                    "simulated transport failure".to_string(),
                )),
            ])
            .await;
            assert!(
                matches!(result, Err(InvitationError::Invalid(_))),
                "transport error must propagate, got {result:?}"
            );
            assert_eq!(calls.len(), 3, "no lookup may follow the error");
            assert_eq!(delays, 1);
        }
    }

    // --- submit_claim_with_stale_islock_fallback: the claim submission seam ---

    mod claim_submission {
        use std::collections::VecDeque;
        use std::sync::{Arc, Mutex};

        use super::super::submit_claim_with_stale_islock_fallback;
        use super::super::{Error, InvitationError};
        use super::{asset_lock_tx_paying_voucher_at, voucher};
        use dpp::consensus::basic::identity::InvalidInstantAssetLockProofSignatureError;
        use dpp::dashcore::InstantLock;
        use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
        use dpp::identity::state_transition::asset_lock_proof::InstantAssetLockProof;
        use dpp::prelude::AssetLockProof;

        /// The one rejection the seam recovers from: Platform's stale-islock
        /// consensus error, in the `Protocol(ConsensusError)` shape
        /// `is_instant_lock_proof_invalid` matches.
        fn stale_islock_rejection() -> Error {
            Error::Protocol(dpp::ProtocolError::ConsensusError(Box::new(
                InvalidInstantAssetLockProofSignatureError::new().into(),
            )))
        }

        /// Any error the seam must NOT treat as a stale islock.
        fn unrelated_sdk_error() -> Error {
            Error::Config("simulated unrelated rejection".to_string())
        }

        /// An InstantSend primary — the variant is what the assertions key on,
        /// so a scripted submission log reads as primary-vs-fallback directly.
        fn instant_primary() -> AssetLockProof {
            AssetLockProof::Instant(InstantAssetLockProof::new(
                InstantLock::default(),
                asset_lock_tx_paying_voucher_at(&voucher(), 0),
                0,
            ))
        }

        fn chain_fallback_proof() -> AssetLockProof {
            AssetLockProof::Chain(ChainAssetLockProof::new(100, [0u8; 36]))
        }

        /// Drive the seam with a scripted submit: each call records the proof
        /// it was handed and pops the next scripted result.
        async fn run(
            chain_fallback: Option<AssetLockProof>,
            script: Vec<Result<u32, Error>>,
        ) -> (Result<u32, InvitationError>, Vec<AssetLockProof>) {
            let queue: Arc<Mutex<VecDeque<Result<u32, Error>>>> =
                Arc::new(Mutex::new(script.into_iter().collect()));
            let submitted: Arc<Mutex<Vec<AssetLockProof>>> = Arc::new(Mutex::new(Vec::new()));
            let (queue_f, submitted_f) = (Arc::clone(&queue), Arc::clone(&submitted));
            let result = submit_claim_with_stale_islock_fallback(
                instant_primary(),
                chain_fallback,
                move |proof| {
                    let queue = Arc::clone(&queue_f);
                    let submitted = Arc::clone(&submitted_f);
                    async move {
                        submitted.lock().expect("submitted").push(proof);
                        queue
                            .lock()
                            .expect("script")
                            .pop_front()
                            .expect("script exhausted — the seam submitted more than scripted")
                    }
                },
            )
            .await;
            let recorded = submitted.lock().expect("submitted").clone();
            (result, recorded)
        }

        /// A successful primary submission returns immediately: one submission,
        /// and the ChainLock fallback is never touched even though it exists.
        #[tokio::test]
        async fn should_not_submit_the_fallback_when_the_primary_succeeds() {
            let (result, submitted) = run(Some(chain_fallback_proof()), vec![Ok(7)]).await;
            assert_eq!(result.expect("primary success"), 7);
            assert_eq!(submitted.len(), 1, "exactly one submission");
            assert!(
                matches!(submitted[0], AssetLockProof::Instant(_)),
                "the IS primary must be what was submitted"
            );
        }

        /// A stale-islock rejection of the primary resubmits the ChainLock
        /// fallback over the same outpoint, and the fallback's success is the
        /// claim's success.
        #[tokio::test]
        async fn should_resubmit_the_chain_fallback_after_a_stale_islock() {
            let (result, submitted) = run(
                Some(chain_fallback_proof()),
                vec![Err(stale_islock_rejection()), Ok(9)],
            )
            .await;
            assert_eq!(result.expect("fallback success"), 9);
            assert_eq!(submitted.len(), 2, "primary then fallback");
            assert!(
                matches!(submitted[0], AssetLockProof::Instant(_)),
                "the IS primary must be tried first"
            );
            assert!(
                matches!(submitted[1], AssetLockProof::Chain(_)),
                "the resubmission must carry the ChainLock fallback"
            );
        }

        /// A stale-islock rejection with NO fallback (funding tx not yet
        /// chain-locked) surfaces `InvitationError::NotChainLocked` — the clear retry
        /// signal — after exactly one submission.
        #[tokio::test]
        async fn should_ask_to_retry_when_a_stale_islock_has_no_chain_fallback() {
            let (result, submitted) = run(None, vec![Err(stale_islock_rejection())]).await;
            assert!(
                matches!(result, Err(InvitationError::NotChainLocked(_))),
                "expected the not-chain-locked retry signal, got {result:?}"
            );
            assert_eq!(
                submitted.len(),
                1,
                "no second submission is possible without a fallback"
            );
        }

        /// An unrelated SDK error propagates unchanged after one submission:
        /// the fallback exists but must NOT be tried for it.
        #[tokio::test]
        async fn should_propagate_an_unrelated_error_without_a_second_submission() {
            let (result, submitted) = run(
                Some(chain_fallback_proof()),
                vec![Err(unrelated_sdk_error())],
            )
            .await;
            assert!(
                matches!(result, Err(InvitationError::Sdk(_))),
                "the unrelated error must propagate as Sdk, got {result:?}"
            );
            assert_eq!(
                submitted.len(),
                1,
                "an unrelated error must not trigger the fallback"
            );
        }

        /// The recovery is attempted exactly once: a fallback rejection — even
        /// one wearing the stale-islock shape again — propagates rather than
        /// looping.
        #[tokio::test]
        async fn should_not_recover_a_rejected_fallback_again() {
            let (result, submitted) = run(
                Some(chain_fallback_proof()),
                vec![Err(stale_islock_rejection()), Err(stale_islock_rejection())],
            )
            .await;
            assert!(
                matches!(result, Err(InvitationError::Sdk(_))),
                "the fallback's own rejection must propagate as Sdk, got {result:?}"
            );
            assert_eq!(submitted.len(), 2, "no third submission may follow");
        }
    }
}
