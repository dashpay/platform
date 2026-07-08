//! DashPay invitation link (`dashpay://invite`) codec — DIP-13 sub-feature 3'.
//!
//! An invitation packages a one-time ECDSA **voucher** private key together with
//! the InstantSend asset-lock proof that funds it, so an invitee with no Dash can
//! register their own identity from it. The inviter optionally includes their own
//! identity id + username so the invitee can send a contact request back.
//!
//! The link is a single versioned, self-contained blob:
//! `dashpay://invite?data=<base58(payload)>`. Only the off-chain envelope is ours;
//! the embedded `AssetLockProof` and the on-chain acts are consensus formats.
//! See `docs/dashpay/DIP15_INVITATIONS_SPEC.md`.
//!
//! The payload uses a small hand-rolled little-endian binary format (rather than
//! serde/bincode) so the codec has no dependency on the crate's optional `serde`
//! feature — create/claim need it unconditionally.
//!
//! # Security
//!
//! The `voucher_key` is **bearer money** — whoever holds the link can claim the
//! funded identity. The URI is a secret: callers MUST NOT log or persist it, and
//! the voucher key is never stored (it is HD-derived and re-derivable from the
//! funding index). Parsing is bounded before decode (base58 length cap) so a
//! hostile link can't force a large allocation, and [`validate_claimable`] fails
//! fast on a stale, wrong-type, or mismatched link before any network call.

use dashcore::secp256k1::{PublicKey, Secp256k1, SecretKey};
use dashcore::ScriptBuf;
use dpp::bincode::config;
use dpp::prelude::AssetLockProof;
use zeroize::Zeroizing;

use crate::error::PlatformWalletError;

/// URI prefix for an invitation deep link. The `dashpay://invite` scheme matches
/// the reference wallets for familiarity; the payload is our own.
const INVITATION_URI_PREFIX: &str = "dashpay://invite?data=";

/// Max base58 chars of the `data=` value accepted **before** decoding (anti-DoS).
/// A real payload — voucher key (32 B) + an InstantSend proof (funding tx + islock,
/// ~0.5–1 KB) + small metadata — base58-encodes to roughly 1.5–2 K chars; 8192 is
/// comfortable headroom while still bounding the base58 allocation a hostile link
/// can force. Mirrors the `dapk` cap in `auto_accept::parse_dashpay_contact_uri`.
const MAX_INVITATION_DATA_B58_LEN: usize = 8192;

/// Hard byte cap on the decoded payload (defense in depth alongside the b58 cap).
const MAX_INVITATION_PAYLOAD_BYTES: usize = 64 * 1024;

/// Max length (bytes) of a UTF-8 string field (username / display name). DPNS
/// labels are short; this only bounds a hostile link.
const MAX_STR_BYTES: usize = 256;

/// Current invitation payload version.
const INVITATION_PAYLOAD_VERSION: u8 = 0;

/// Inviter contact-bootstrap info — present iff the inviter opted in to "send a
/// contact request back to me". Absent ⇒ the invitation is a pure funding voucher.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InviterInfo {
    /// The inviter's identity id (32 bytes) — the target of the invitee's
    /// contact request.
    pub identity_id: [u8; 32],
    /// The inviter's DPNS username, shown to the invitee and used to label the
    /// contact.
    pub username: String,
    /// Optional display name for the claim UI.
    pub display_name: Option<String>,
}

/// A decoded invitation, ready for [`validate_claimable`] + claim.
pub struct ParsedInvitation {
    /// One-time ECDSA voucher private key that funds the invitee's identity
    /// create (signs the asset-lock's outer state-transition signature).
    pub voucher_key: SecretKey,
    /// The InstantSend asset-lock proof funding the voucher (embeds tx + islock).
    pub asset_lock: AssetLockProof,
    /// Advisory expiry (unix seconds). Not consensus-enforced; the claim path
    /// refuses a past-expiry link so a stale IS proof is never submitted.
    pub expiry_unix: u32,
    /// Inviter contact-bootstrap info; `None` ⇒ pure funding voucher.
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
    /// Redacts the voucher key — the whole point of the type is to carry a
    /// bearer secret, which must never reach a log.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParsedInvitation")
            .field("voucher_key", &"<redacted>")
            .field("expiry_unix", &self.expiry_unix)
            .field("inviter", &self.inviter)
            .finish_non_exhaustive()
    }
}

fn invalid(msg: impl Into<String>) -> PlatformWalletError {
    PlatformWalletError::InvalidIdentityData(msg.into())
}

/// The P2PKH script the voucher key controls (compressed-pubkey hash160).
fn voucher_credit_script(voucher_key: &SecretKey) -> ScriptBuf {
    let secp = Secp256k1::new();
    let pubkey = PublicKey::from_secret_key(&secp, voucher_key);
    let hash = dashcore::PublicKey::new(pubkey).pubkey_hash();
    ScriptBuf::new_p2pkh(&hash)
}

// ---------------------------------------------------------------------------
// Wire encoding (little-endian, length-prefixed)
// ---------------------------------------------------------------------------

fn put_len_prefixed(buf: &mut Vec<u8>, bytes: &[u8]) {
    buf.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    buf.extend_from_slice(bytes);
}

/// Cursor over the payload bytes with bounds-checked, non-panicking reads.
struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], PlatformWalletError> {
        let end = self
            .pos
            .checked_add(n)
            .ok_or_else(|| invalid("invitation payload length overflow"))?;
        if end > self.buf.len() {
            return Err(invalid("invitation payload truncated"));
        }
        let out = &self.buf[self.pos..end];
        self.pos = end;
        Ok(out)
    }

    fn u8(&mut self) -> Result<u8, PlatformWalletError> {
        Ok(self.take(1)?[0])
    }

    fn u32(&mut self) -> Result<u32, PlatformWalletError> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn arr32(&mut self) -> Result<[u8; 32], PlatformWalletError> {
        let mut out = [0u8; 32];
        out.copy_from_slice(self.take(32)?);
        Ok(out)
    }

    fn len_prefixed(&mut self, max: usize) -> Result<&'a [u8], PlatformWalletError> {
        let len = self.u32()? as usize;
        if len > max {
            return Err(invalid("invitation payload field exceeds size cap"));
        }
        self.take(len)
    }

    fn string(&mut self) -> Result<String, PlatformWalletError> {
        let bytes = self.len_prefixed(MAX_STR_BYTES)?;
        String::from_utf8(bytes.to_vec())
            .map_err(|_| invalid("invitation payload string is not valid UTF-8"))
    }

    fn finish(self) -> Result<(), PlatformWalletError> {
        if self.pos != self.buf.len() {
            return Err(invalid("unexpected trailing bytes in invitation payload"));
        }
        Ok(())
    }
}

/// Encode an invitation into a `dashpay://invite?data=<base58>` link.
///
/// The returned URI **contains the plaintext voucher key** — treat it as a
/// secret (do not log or persist it).
pub fn encode_invitation_uri(
    voucher_key: &SecretKey,
    asset_lock: &AssetLockProof,
    expiry_unix: u32,
    inviter: Option<&InviterInfo>,
) -> Result<String, PlatformWalletError> {
    let asset_lock_bytes = dpp::bincode::encode_to_vec(asset_lock, config::standard())
        .map_err(|e| invalid(format!("failed to encode asset-lock proof: {e}")))?;

    // Zeroized: `buf` holds the plaintext voucher scalar until it is base58'd
    // into the (secret) URI; scrub the intermediate on drop.
    let mut buf = Zeroizing::new(Vec::with_capacity(64 + asset_lock_bytes.len()));
    buf.push(INVITATION_PAYLOAD_VERSION);
    buf.extend_from_slice(&voucher_key.secret_bytes());
    buf.extend_from_slice(&expiry_unix.to_le_bytes());
    match inviter {
        Some(info) => {
            if info.username.len() > MAX_STR_BYTES
                || info
                    .display_name
                    .as_ref()
                    .is_some_and(|d| d.len() > MAX_STR_BYTES)
            {
                return Err(invalid("inviter username/display name too long"));
            }
            buf.push(1);
            buf.extend_from_slice(&info.identity_id);
            put_len_prefixed(&mut buf, info.username.as_bytes());
            match &info.display_name {
                Some(d) => {
                    buf.push(1);
                    put_len_prefixed(&mut buf, d.as_bytes());
                }
                None => buf.push(0),
            }
        }
        None => buf.push(0),
    }
    put_len_prefixed(&mut buf, &asset_lock_bytes);

    Ok(format!(
        "{INVITATION_URI_PREFIX}{}",
        bs58::encode(buf.as_slice()).into_string()
    ))
}

/// Parse a `dashpay://invite?data=<base58>` link into a [`ParsedInvitation`].
///
/// Bounds the base58 input before decoding and rejects trailing bytes, an
/// unsupported version, and malformed keys/proofs. Does **not** check expiry or
/// the credit-output binding — call [`validate_claimable`] for that before use.
pub fn parse_invitation_uri(uri: &str) -> Result<ParsedInvitation, PlatformWalletError> {
    let data = uri
        .strip_prefix(INVITATION_URI_PREFIX)
        .ok_or_else(|| invalid("not a dashpay://invite?data= URI"))?;
    // Tolerate trailing query params after the payload (`…?data=X&foo=Y`).
    let data = data.split('&').next().unwrap_or(data);
    if data.len() > MAX_INVITATION_DATA_B58_LEN {
        return Err(invalid(format!(
            "invitation data too long ({} chars; max {MAX_INVITATION_DATA_B58_LEN})",
            data.len()
        )));
    }
    // Zeroized: the decoded payload carries the plaintext voucher scalar (at
    // offset 1..33); scrub it once parsed.
    let bytes = Zeroizing::new(
        bs58::decode(data)
            .into_vec()
            .map_err(|e| invalid(format!("invitation data is not valid base58: {e}")))?,
    );
    if bytes.len() > MAX_INVITATION_PAYLOAD_BYTES {
        return Err(invalid(format!(
            "invitation payload too large ({} bytes; max {MAX_INVITATION_PAYLOAD_BYTES})",
            bytes.len()
        )));
    }

    let mut r = Reader::new(bytes.as_slice());
    let version = r.u8()?;
    if version != INVITATION_PAYLOAD_VERSION {
        return Err(invalid(format!(
            "unsupported invitation version {version} (expected {INVITATION_PAYLOAD_VERSION})"
        )));
    }
    let voucher_key = SecretKey::from_slice(r.take(32)?)
        .map_err(|e| invalid(format!("invalid voucher private key: {e}")))?;
    let expiry_unix = r.u32()?;
    let inviter = match r.u8()? {
        0 => None,
        1 => {
            let identity_id = r.arr32()?;
            let username = r.string()?;
            let display_name = match r.u8()? {
                0 => None,
                1 => Some(r.string()?),
                other => return Err(invalid(format!("invalid display-name flag {other}"))),
            };
            Some(InviterInfo {
                identity_id,
                username,
                display_name,
            })
        }
        other => return Err(invalid(format!("invalid inviter-present flag {other}"))),
    };
    let asset_lock_bytes = r.len_prefixed(MAX_INVITATION_PAYLOAD_BYTES)?;
    let (asset_lock, consumed): (AssetLockProof, usize) =
        dpp::bincode::decode_from_slice(asset_lock_bytes, config::standard())
            .map_err(|e| invalid(format!("failed to decode asset-lock proof: {e}")))?;
    if consumed != asset_lock_bytes.len() {
        return Err(invalid("trailing bytes in embedded asset-lock proof"));
    }
    r.finish()?;

    Ok(ParsedInvitation {
        voucher_key,
        asset_lock,
        expiry_unix,
        inviter,
    })
}

/// Fail-fast validation before any network call.
///
/// Rejects a link with a zero clock read, whose advisory expiry has passed,
/// whose proof is not an InstantSend proof (per the owner's proof-type
/// decision), or whose voucher key does not control the funded credit output —
/// turning an otherwise opaque consensus rejection into a clear, local error.
/// The credit-output binding is itself consensus-enforced, so this is a UX
/// guard, not a security boundary.
pub fn validate_claimable(
    invitation: &ParsedInvitation,
    now_unix: u32,
) -> Result<(), PlatformWalletError> {
    // A zero `now` would make the `now > expiry` test below pass for any
    // positive expiry, silently treating an expired link as fresh. Reject it up
    // front, mirroring the create side's non-zero timestamp guard.
    if now_unix == 0 {
        return Err(invalid(
            "invitation claim requires a valid clock (now_unix is zero)",
        ));
    }
    if now_unix > invitation.expiry_unix {
        return Err(invalid(format!(
            "invitation expired (expiry {}, now {now_unix}) — ask the sender for a new one",
            invitation.expiry_unix
        )));
    }
    let instant = match &invitation.asset_lock {
        AssetLockProof::Instant(instant) => instant,
        AssetLockProof::Chain(_) => {
            return Err(invalid(
                "invitation asset-lock proof must be an InstantSend proof",
            ))
        }
    };
    let output = instant
        .output()
        .ok_or_else(|| invalid("asset-lock proof has no credit output at its output index"))?;
    if output.script_pubkey != voucher_credit_script(&invitation.voucher_key) {
        return Err(invalid(
            "voucher key does not control the funded credit output",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashcore::ephemerealdata::instant_lock::InstantLock;
    use dashcore::transaction::special_transaction::asset_lock::AssetLockPayload;
    use dashcore::transaction::special_transaction::TransactionPayload;
    use dashcore::{Transaction, TxOut};
    use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
    use dpp::identity::state_transition::asset_lock_proof::InstantAssetLockProof;

    fn voucher() -> SecretKey {
        SecretKey::from_slice(&[0x11u8; 32]).expect("valid scalar")
    }

    fn inviter_info() -> InviterInfo {
        InviterInfo {
            identity_id: [0xAB; 32],
            username: "alice".to_string(),
            display_name: Some("Alice".to_string()),
        }
    }

    /// An InstantSend proof whose single credit output pays to `key`'s P2PKH.
    fn instant_proof_paying_to(key: &SecretKey) -> AssetLockProof {
        let credit = TxOut {
            value: 100_000,
            script_pubkey: voucher_credit_script(key),
        };
        let payload = AssetLockPayload {
            version: 1,
            credit_outputs: vec![credit],
        };
        let tx = Transaction {
            version: 3,
            lock_time: 0,
            input: vec![],
            output: vec![],
            special_transaction_payload: Some(TransactionPayload::AssetLockPayloadType(payload)),
        };
        AssetLockProof::Instant(InstantAssetLockProof::new(InstantLock::default(), tx, 0))
    }

    fn proof_bytes(proof: &AssetLockProof) -> Vec<u8> {
        dpp::bincode::encode_to_vec(proof, config::standard()).unwrap()
    }

    #[test]
    fn round_trip_with_inviter() {
        let key = voucher();
        let proof = instant_proof_paying_to(&key);
        let uri = encode_invitation_uri(&key, &proof, 1_800_000_000, Some(&inviter_info()))
            .expect("encode");
        assert!(uri.starts_with(INVITATION_URI_PREFIX));

        let parsed = parse_invitation_uri(&uri).expect("parse");
        assert_eq!(parsed.voucher_key.secret_bytes(), key.secret_bytes());
        assert_eq!(parsed.expiry_unix, 1_800_000_000);
        assert_eq!(parsed.inviter, Some(inviter_info()));
        // Proof round-trips (compare re-encoded bytes — AssetLockProof is not Eq).
        assert_eq!(proof_bytes(&parsed.asset_lock), proof_bytes(&proof));
    }

    #[test]
    fn round_trip_pure_voucher_no_inviter() {
        let key = voucher();
        let proof = instant_proof_paying_to(&key);
        let uri = encode_invitation_uri(&key, &proof, 42, None).expect("encode");
        let parsed = parse_invitation_uri(&uri).expect("parse");
        assert!(parsed.inviter.is_none());
        assert_eq!(parsed.expiry_unix, 42);
    }

    #[test]
    fn round_trip_inviter_without_display_name() {
        let key = voucher();
        let proof = instant_proof_paying_to(&key);
        let info = InviterInfo {
            identity_id: [0x01; 32],
            username: "bob".to_string(),
            display_name: None,
        };
        let uri = encode_invitation_uri(&key, &proof, 7, Some(&info)).expect("encode");
        let parsed = parse_invitation_uri(&uri).expect("parse");
        assert_eq!(parsed.inviter, Some(info));
    }

    #[test]
    fn rejects_wrong_scheme() {
        assert!(parse_invitation_uri("https://invite?data=abc").is_err());
        assert!(parse_invitation_uri("dashpay://contact?data=abc").is_err());
    }

    #[test]
    fn rejects_bad_base58() {
        // '0','O','I','l' are not in the base58 alphabet.
        let err = parse_invitation_uri("dashpay://invite?data=0OIl").unwrap_err();
        assert!(err.to_string().contains("base58"));
    }

    #[test]
    fn rejects_oversized_data_before_decoding() {
        let huge = "z".repeat(MAX_INVITATION_DATA_B58_LEN + 1);
        let uri = format!("{INVITATION_URI_PREFIX}{huge}");
        let err = parse_invitation_uri(&uri).unwrap_err();
        assert!(err.to_string().contains("too long"));
    }

    #[test]
    fn rejects_trailing_bytes() {
        let key = voucher();
        let proof = instant_proof_paying_to(&key);
        let uri = encode_invitation_uri(&key, &proof, 1, None).expect("encode");
        let data = uri.strip_prefix(INVITATION_URI_PREFIX).unwrap();
        let mut bytes = bs58::decode(data).into_vec().unwrap();
        bytes.push(0x00);
        let tampered = format!(
            "{INVITATION_URI_PREFIX}{}",
            bs58::encode(&bytes).into_string()
        );
        let err = parse_invitation_uri(&tampered).unwrap_err();
        assert!(err.to_string().contains("trailing"));
    }

    #[test]
    fn rejects_unsupported_version() {
        let key = voucher();
        let proof = instant_proof_paying_to(&key);
        let uri = encode_invitation_uri(&key, &proof, 1, None).expect("encode");
        let data = uri.strip_prefix(INVITATION_URI_PREFIX).unwrap();
        let mut bytes = bs58::decode(data).into_vec().unwrap();
        bytes[0] = 99; // corrupt the version byte
        let tampered = format!(
            "{INVITATION_URI_PREFIX}{}",
            bs58::encode(&bytes).into_string()
        );
        let err = parse_invitation_uri(&tampered).unwrap_err();
        assert!(err.to_string().contains("unsupported invitation version"));
    }

    #[test]
    fn rejects_truncated_payload() {
        let key = voucher();
        let proof = instant_proof_paying_to(&key);
        let uri = encode_invitation_uri(&key, &proof, 1, None).expect("encode");
        let data = uri.strip_prefix(INVITATION_URI_PREFIX).unwrap();
        let bytes = bs58::decode(data).into_vec().unwrap();
        // Drop the tail so the embedded proof length prefix overruns.
        let truncated = format!(
            "{INVITATION_URI_PREFIX}{}",
            bs58::encode(&bytes[..bytes.len() - 5]).into_string()
        );
        assert!(parse_invitation_uri(&truncated).is_err());
    }

    #[test]
    fn validate_ok_for_fresh_matching_instant_proof() {
        let key = voucher();
        let parsed = ParsedInvitation {
            voucher_key: key,
            asset_lock: instant_proof_paying_to(&key),
            expiry_unix: 2_000_000_000,
            inviter: None,
        };
        assert!(validate_claimable(&parsed, 1_000_000_000).is_ok());
    }

    #[test]
    fn validate_rejects_expired() {
        let key = voucher();
        let parsed = ParsedInvitation {
            voucher_key: key,
            asset_lock: instant_proof_paying_to(&key),
            expiry_unix: 1_000,
            inviter: None,
        };
        let err = validate_claimable(&parsed, 2_000).unwrap_err();
        assert!(err.to_string().contains("expired"));
    }

    #[test]
    fn validate_rejects_zero_clock() {
        // A zero clock read must be rejected up front: otherwise `now(0) >
        // expiry` is false and even a long-expired link looks claimable. Here
        // the expiry is well in the past, so only the zero-clock guard can
        // reject it — a regression that dropped the guard would return `Ok`.
        let key = voucher();
        let parsed = ParsedInvitation {
            voucher_key: key,
            asset_lock: instant_proof_paying_to(&key),
            expiry_unix: 1_000,
            inviter: None,
        };
        let err = validate_claimable(&parsed, 0).unwrap_err();
        assert!(err.to_string().contains("clock"));
    }

    #[test]
    fn validate_rejects_chain_proof() {
        let key = voucher();
        let chain = AssetLockProof::Chain(ChainAssetLockProof::new(42, [0x7u8; 36]));
        let parsed = ParsedInvitation {
            voucher_key: key,
            asset_lock: chain,
            expiry_unix: 2_000_000_000,
            inviter: None,
        };
        let err = validate_claimable(&parsed, 1).unwrap_err();
        assert!(err.to_string().contains("InstantSend"));
    }

    #[test]
    fn validate_rejects_voucher_not_controlling_output() {
        let key = voucher();
        let other = SecretKey::from_slice(&[0x22u8; 32]).unwrap();
        // Proof pays to `other`, but the parsed voucher key is `key`.
        let parsed = ParsedInvitation {
            voucher_key: key,
            asset_lock: instant_proof_paying_to(&other),
            expiry_unix: 2_000_000_000,
            inviter: None,
        };
        let err = validate_claimable(&parsed, 1).unwrap_err();
        assert!(err.to_string().contains("does not control"));
    }
}
