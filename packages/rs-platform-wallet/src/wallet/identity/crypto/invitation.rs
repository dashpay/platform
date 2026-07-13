//! DashPay invitation link codec — DIP-13 sub-feature 3', legacy-compatible.
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
//!   lock" — the claim reconstructs a ChainLock proof instead.
//! - `assetlocktx` is kept as the raw hex string; the claim tries it as-given
//!   then byte-reversed on a fetch miss, mirroring the legacy endianness retry.
//!
//! # Security
//!
//! The `voucher_key` is **bearer money** — whoever holds the link can claim the
//! funded identity. The URI is a secret: callers MUST NOT log or persist it, and
//! the voucher key is never stored (it is HD-derived and re-derivable from the
//! funding index). Parsing is bounded (URI length cap) so a hostile link can't
//! force a large allocation, and the WIF is network- and compression-checked so
//! a malformed key is rejected before any network call.

use dashcore::secp256k1::{PublicKey, Secp256k1, SecretKey};
use dashcore::transaction::special_transaction::TransactionPayload;
use dashcore::{Network, PrivateKey, ScriptBuf, Transaction};
use dpp::prelude::AssetLockProof;

use crate::error::PlatformWalletError;

/// The `dashpay://invite` custom scheme — the canonical form we emit and the
/// primary form we parse (QR / in-person / deep link).
const INVITATION_SCHEME_PREFIX: &str = "dashpay://invite";

/// The AppsFlyer OneLink applink host the iOS reference wallet emits. Parsed as
/// a first-class alternative to the custom scheme (field-level interop).
const INVITATION_APPLINK_HOST: &str = "invitations.dashpay.io/applink";

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

/// Inviter contact-bootstrap info — present iff the link carries a `du`
/// (username). Absent ⇒ the invitation is a pure funding voucher. The link does
/// not carry the inviter's identity id (the legacy format has no such field);
/// the invitee resolves it from `username` via DPNS at contact-bootstrap time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InviterInfo {
    /// The inviter's DPNS username (`du`), shown to the invitee and used to
    /// resolve the inviter's identity for the contact request.
    pub username: String,
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

fn invalid(msg: impl Into<String>) -> PlatformWalletError {
    PlatformWalletError::InvalidIdentityData(msg.into())
}

/// The P2PKH script the voucher key controls (compressed-pubkey hash160). This
/// is the selector that binds the voucher key to its funded credit output.
fn voucher_credit_script(voucher_key: &SecretKey) -> ScriptBuf {
    let secp = Secp256k1::new();
    let pubkey = PublicKey::from_secret_key(&secp, voucher_key);
    let hash = dashcore::PublicKey::new(pubkey).pubkey_hash();
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
fn percent_decode(s: &str) -> Result<String, PlatformWalletError> {
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
fn parse_query(query: &str) -> Result<Vec<(String, String)>, PlatformWalletError> {
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
) -> Result<String, PlatformWalletError> {
    // txid (big-endian display hex) + optional islock hex from the proof.
    let (funding_txid, islock_hex) = match asset_lock {
        AssetLockProof::Instant(instant) => {
            let txid = instant.transaction().txid().to_string();
            let islock = hex::encode(dashcore::consensus::serialize(instant.instant_lock()));
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

    // `du` is emitted first (canonical) when the inviter opted in; a pure
    // funding voucher emits a `du`-less link (still parseable — iOS accepts it).
    if let Some(info) = inviter {
        if info.username.len() > MAX_STR_BYTES {
            return Err(invalid("inviter username too long"));
        }
        push(PARAM_USER, &info.username);
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

    Ok(format!("{INVITATION_SCHEME_PREFIX}?{query}"))
}

/// Parse a legacy-compatible invitation link into a [`ParsedInvitation`].
///
/// Accepts both the `dashpay://invite` scheme and the
/// `https://invitations.dashpay.io/applink` host, by field name and
/// order-independent. Requires `assetlocktx` + `pk` (non-blank); `du` and
/// `islock` are optional (a missing/`"null"` `islock` is a ChainLock invite).
/// The WIF is network- and compression-checked. Does **not** fetch or validate
/// the funding tx — that happens at claim.
pub fn parse_invitation_uri(uri: &str) -> Result<ParsedInvitation, PlatformWalletError> {
    if uri.len() > MAX_INVITATION_URI_LEN {
        return Err(invalid(format!(
            "invitation link too long ({} chars; max {MAX_INVITATION_URI_LEN})",
            uri.len()
        )));
    }

    // Accept the custom scheme or the applink host; the transport differs but
    // the query contract is identical (field-level, not byte-level, parity).
    let is_scheme = uri.starts_with(INVITATION_SCHEME_PREFIX);
    let is_applink = uri.contains(INVITATION_APPLINK_HOST);
    if !is_scheme && !is_applink {
        return Err(invalid(
            "not a dashpay://invite or invitations.dashpay.io/applink link",
        ));
    }

    // Everything after the first `?`, up to an optional fragment.
    let query = uri
        .split_once('?')
        .map(|(_, q)| q)
        .ok_or_else(|| invalid("invitation link has no query parameters"))?;
    let query = query.split('#').next().unwrap_or(query);
    let pairs = parse_query(query)?;

    // iOS minimum: `assetlocktx` + `pk` present and non-blank. Never reject on a
    // missing/`"null"` `islock` or a missing `du`.
    let assetlocktx = field(&pairs, PARAM_ASSET_LOCK_TX)
        .ok_or_else(|| invalid("invitation link is missing the assetlocktx field"))?;
    let pk = field(&pairs, PARAM_PRIVATE_KEY)
        .ok_or_else(|| invalid("invitation link is missing the pk field"))?;

    // WIF: network-checked decode (`from_wif` rejects a foreign network byte),
    // compression required (the credit-output hash uses the compressed pubkey —
    // an uncompressed key would produce a mismatching hash160 and a dead claim).
    let private_key = PrivateKey::from_wif(pk)
        .map_err(|e| invalid(format!("invitation pk is not a valid WIF key: {e}")))?;
    if !private_key.compressed {
        return Err(invalid("invitation pk must be a compressed WIF key"));
    }
    let voucher_key = private_key.inner;

    let funding_txid = assetlocktx.to_lowercase();

    // `islock`: a missing param and the literal `"null"` both mean no instant
    // lock. Kept as lowercase hex; decoded to an `InstantLock` only at claim.
    let islock_hex = field(&pairs, PARAM_IS_LOCK)
        .filter(|v| *v != IS_LOCK_NULL_SENTINEL)
        .map(|v| v.to_lowercase());

    // Inviter present iff `du` is present. The identity id is not in the link;
    // it is resolved from the username at contact-bootstrap time.
    let inviter = field(&pairs, PARAM_USER).map(|username| InviterInfo {
        username: username.to_string(),
        display_name: field(&pairs, PARAM_DISPLAY_NAME).map(str::to_string),
        avatar_url: field(&pairs, PARAM_AVATAR_URL).map(str::to_string),
    });

    Ok(ParsedInvitation {
        voucher_key,
        funding_txid,
        islock_hex,
        inviter,
    })
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
) -> Result<u32, PlatformWalletError> {
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
        .position(|out| out.script_pubkey == expected)
        .map(|idx| idx as u32)
        .ok_or_else(|| {
            invalid("voucher key does not control any credit output of the funding transaction")
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashcore::ephemerealdata::instant_lock::InstantLock;
    use dashcore::transaction::special_transaction::asset_lock::AssetLockPayload;
    use dashcore::{Transaction, TxOut};
    use dpp::identity::state_transition::asset_lock_proof::InstantAssetLockProof;

    fn voucher() -> SecretKey {
        SecretKey::from_slice(&[0x11u8; 32]).expect("valid scalar")
    }

    fn inviter_info() -> InviterInfo {
        InviterInfo {
            username: "alice".to_string(),
            display_name: Some("Alice Example".to_string()),
            avatar_url: Some("https://example.com/a b.png?x=1".to_string()),
        }
    }

    /// Build an asset-lock tx whose credit output at `index` pays the voucher
    /// key (and `index` decoy outputs before it that do not).
    fn asset_lock_tx_paying_voucher_at(key: &SecretKey, index: usize) -> Transaction {
        let decoy = SecretKey::from_slice(&[0x22u8; 32]).unwrap();
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
    fn wif_round_trip_preserves_compression_and_network() {
        for (network, first_byte) in [(Network::Mainnet, 204u8), (Network::Testnet, 239u8)] {
            let wif = PrivateKey::new(voucher(), network).to_wif();
            let decoded = PrivateKey::from_wif(&wif).expect("wif decodes");
            assert!(decoded.compressed, "voucher WIF must be compressed");
            assert_eq!(decoded.network, network, "network preserved");
            assert_eq!(decoded.inner.secret_bytes(), voucher().secret_bytes());
            // Network byte matches bitcoinj/legacy (0xCC mainnet, 0xEF testnet).
            let raw = bs58::decode(&wif).into_vec().unwrap();
            assert_eq!(raw[0], first_byte);
        }
    }

    #[test]
    fn encode_parse_round_trip_with_inviter() {
        let proof = instant_proof_paying_voucher();
        let info = inviter_info();
        let uri = encode_invitation_uri(&voucher(), Network::Testnet, &proof, Some(&info))
            .expect("encode");
        assert!(uri.starts_with("dashpay://invite?"));

        let parsed = parse_invitation_uri(&uri).expect("parse");
        assert_eq!(parsed.voucher_key.secret_bytes(), voucher().secret_bytes());
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
    fn encode_parse_round_trip_pure_voucher_no_inviter() {
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
    fn parse_is_order_independent() {
        let wif = PrivateKey::new(voucher(), Network::Testnet).to_wif();
        let uri = format!("dashpay://invite?islock=deadbeef&pk={wif}&du=bob&assetlocktx=aabbcc");
        let parsed = parse_invitation_uri(&uri).expect("parse");
        assert_eq!(parsed.funding_txid, "aabbcc");
        assert_eq!(parsed.islock_hex.as_deref(), Some("deadbeef"));
        assert_eq!(parsed.inviter.as_ref().unwrap().username, "bob");
    }

    /// The `https://invitations.dashpay.io/applink` host is accepted as a
    /// first-class alternative to the custom scheme (iOS emits it).
    #[test]
    fn parse_accepts_applink_host() {
        let wif = PrivateKey::new(voucher(), Network::Testnet).to_wif();
        let uri = format!(
            "https://invitations.dashpay.io/applink?du=carol&assetlocktx=aabb&pk={wif}&islock=cc"
        );
        let parsed = parse_invitation_uri(&uri).expect("parse applink");
        assert_eq!(parsed.inviter.as_ref().unwrap().username, "carol");
        assert_eq!(parsed.funding_txid, "aabb");
    }

    /// `islock` present / absent / `"null"`: only a real hex value yields
    /// `Some`; both a missing param and the literal `"null"` yield `None`
    /// (a ChainLock-confirmed invite Android's own validator accepts).
    #[test]
    fn parse_islock_present_absent_and_null() {
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
    fn parse_accepts_du_less_link() {
        let wif = PrivateKey::new(voucher(), Network::Testnet).to_wif();
        let uri = format!("dashpay://invite?assetlocktx=aa&pk={wif}&islock=bb");
        let parsed = parse_invitation_uri(&uri).expect("parse");
        assert!(parsed.inviter.is_none());
    }

    #[test]
    fn parse_rejects_missing_pk_or_assetlocktx() {
        let wif = PrivateKey::new(voucher(), Network::Testnet).to_wif();
        // No pk.
        assert!(parse_invitation_uri("dashpay://invite?assetlocktx=aa").is_err());
        // No assetlocktx.
        assert!(parse_invitation_uri(&format!("dashpay://invite?pk={wif}")).is_err());
        // Blank assetlocktx is treated as missing.
        assert!(parse_invitation_uri(&format!("dashpay://invite?assetlocktx=&pk={wif}")).is_err());
    }

    #[test]
    fn parse_rejects_wrong_scheme_and_host() {
        assert!(parse_invitation_uri("https://example.com/foo?pk=x").is_err());
        assert!(parse_invitation_uri("dashpay://contact?pk=x").is_err());
    }

    #[test]
    fn parse_rejects_uncompressed_wif() {
        let uncompressed = PrivateKey::new_uncompressed(voucher(), Network::Testnet).to_wif();
        let uri = format!("dashpay://invite?assetlocktx=aa&pk={uncompressed}");
        let err = parse_invitation_uri(&uri).unwrap_err();
        assert!(err.to_string().contains("compressed"));
    }

    #[test]
    fn parse_rejects_bad_wif() {
        let err = parse_invitation_uri("dashpay://invite?assetlocktx=aa&pk=not-a-wif").unwrap_err();
        assert!(err.to_string().contains("WIF"));
    }

    #[test]
    fn parse_rejects_oversized_uri() {
        let huge = format!("dashpay://invite?pk={}", "z".repeat(MAX_INVITATION_URI_LEN));
        let err = parse_invitation_uri(&huge).unwrap_err();
        assert!(err.to_string().contains("too long"));
    }

    /// A hand-crafted Android-style link (params in Android's emit order, with
    /// display-name + avatar-url) parses to the right fields.
    #[test]
    fn parse_android_style_link() {
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
        assert_eq!(inviter.username, "satoshi");
        assert_eq!(inviter.display_name.as_deref(), Some("Sat Oshi"));
        assert_eq!(
            inviter.avatar_url.as_deref(),
            Some("https://img.example/a.png")
        );
    }

    /// The voucher output is selected by pk↔script match, not hard-coded to 0.
    #[test]
    fn voucher_output_index_selects_matching_output() {
        let key = voucher();
        // Voucher output sits at index 2, behind two decoy outputs.
        let tx = asset_lock_tx_paying_voucher_at(&key, 2);
        assert_eq!(voucher_output_index(&tx, &key).unwrap(), 2);
    }

    #[test]
    fn voucher_output_index_rejects_no_match() {
        let key = voucher();
        let other = SecretKey::from_slice(&[0x33u8; 32]).unwrap();
        let tx = asset_lock_tx_paying_voucher_at(&other, 0);
        let err = voucher_output_index(&tx, &key).unwrap_err();
        assert!(err.to_string().contains("does not control"));
    }

    #[test]
    fn voucher_output_index_rejects_non_asset_lock_tx() {
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
}
