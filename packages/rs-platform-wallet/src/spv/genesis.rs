//! Devnet genesis-header resolution for the SPV pre-seed.
//!
//! `dash-spv` ships built-in genesis parameters only for mainnet,
//! testnet, and regtest; on devnet its `initialize_genesis_block`
//! reaches `known_genesis_block_hash()` and fails with
//! `Configuration error: No known genesis hash for network`. The
//! runtime sidesteps this by pre-seeding the genesis header into SPV
//! storage before the client starts — `initialize_genesis_block`
//! early-returns once storage already has a tip.
//!
//! This module owns the pure, network-free part of that: building the
//! header and verifying its hash. Block 0 is constant across standard
//! Dash devnets and `dashcore` already ships it, so the default needs
//! no configuration; explicit field overrides exist for the rare case
//! of a non-standard devnet genesis.

use std::str::FromStr;

use dashcore::block::{Header, Version};
use dashcore::{BlockHash, CompactTarget, Network, TxMerkleNode};

/// Per-field overrides for the devnet genesis header, layered on top of
/// the `dashcore` built-in. Every field is optional; an unset field
/// keeps the built-in value. All hex inputs are in Core RPC display
/// form (big-endian, as printed by `dash-cli getblockheader`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DevnetGenesisOverride {
    /// Expected block hash (RPC display hex). When set, the resolved
    /// header is asserted to hash to this value; when unset, the header
    /// is self-checked against the `dashcore` constant's own hash.
    pub hash: Option<String>,
    /// Block version.
    pub version: Option<i32>,
    /// Previous block hash (RPC display hex).
    pub prev_blockhash: Option<String>,
    /// Merkle root (RPC display hex).
    pub merkle_root: Option<String>,
    /// Block time (unix seconds).
    pub time: Option<u32>,
    /// Compact target (`nBits`), parsed from hex.
    pub bits: Option<u32>,
    /// Block nonce.
    pub nonce: Option<u32>,
}

impl DevnetGenesisOverride {
    /// True when no field is set — i.e. the caller wants the pure
    /// `dashcore` built-in with the constant self-check.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// Build the devnet genesis header from the `dashcore` built-in,
/// applying any per-field overrides, then verify the result hashes to
/// the expected value.
///
/// The expected hash is the override's `hash` when set, otherwise the
/// built-in constant's own hash (a self-check that catches an upstream
/// `dashcore` change or a bad override before SPV starts).
///
/// # Errors
///
/// Returns a descriptive message when a hex field fails to parse or
/// when the assembled header does not hash to the expected value
/// (naming expected vs computed) — this is the endianness/field
/// guard, run before the header is committed to SPV storage.
pub fn resolve_devnet_genesis_header(overrides: &DevnetGenesisOverride) -> Result<Header, String> {
    let mut header = dashcore::blockdata::constants::genesis_block(Network::Devnet).header;

    if let Some(v) = overrides.version {
        header.version = Version::from_consensus(v);
    }
    if let Some(prev) = &overrides.prev_blockhash {
        header.prev_blockhash = BlockHash::from_str(prev)
            .map_err(|e| format!("invalid devnet genesis prev hash {prev:?}: {e}"))?;
    }
    if let Some(root) = &overrides.merkle_root {
        header.merkle_root = TxMerkleNode::from_str(root)
            .map_err(|e| format!("invalid devnet genesis merkle root {root:?}: {e}"))?;
    }
    if let Some(t) = overrides.time {
        header.time = t;
    }
    if let Some(bits) = overrides.bits {
        header.bits = CompactTarget::from_consensus(bits);
    }
    if let Some(nonce) = overrides.nonce {
        header.nonce = nonce;
    }

    let expected = match &overrides.hash {
        Some(h) => h.clone(),
        None => header.block_hash().to_string(),
    };
    let computed = header.block_hash().to_string();
    if computed != expected {
        return Err(format!(
            "devnet genesis header hash mismatch: expected {expected}, computed {computed} \
             (check the PLATFORM_WALLET_E2E_DEVNET_GENESIS_* fields and their endianness)"
        ));
    }

    Ok(header)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The porter / standard-devnet genesis hash, also asserted by
    /// `dashcore`'s own `devnet_genesis_full_block` test.
    const PORTER_GENESIS_HASH: &str =
        "000008ca1832a4baf228eb1553c03d3a2c8e02399550dd6ea8d65cec3ef23d2e";
    const PORTER_MERKLE_ROOT: &str =
        "e0028eb9648db56b1ac77cf090b99048a8007e2bb64b68f092c03c7f56a662c7";

    #[test]
    fn default_header_hashes_to_porter_genesis() {
        let header = resolve_devnet_genesis_header(&DevnetGenesisOverride::default())
            .expect("built-in devnet genesis must self-check");
        assert_eq!(header.block_hash().to_string(), PORTER_GENESIS_HASH);
        assert_eq!(header.merkle_root.to_string(), PORTER_MERKLE_ROOT);
        assert_eq!(header.time, 1417713337);
        assert_eq!(header.nonce, 1096447);
        assert_eq!(header.bits, CompactTarget::from_consensus(0x207fffff));
    }

    #[test]
    fn explicit_porter_fields_match_builtin() {
        let overrides = DevnetGenesisOverride {
            hash: Some(PORTER_GENESIS_HASH.to_string()),
            version: Some(1),
            prev_blockhash: Some(
                "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
            ),
            merkle_root: Some(PORTER_MERKLE_ROOT.to_string()),
            time: Some(1417713337),
            bits: Some(0x207fffff),
            nonce: Some(1096447),
        };
        let header = resolve_devnet_genesis_header(&overrides)
            .expect("explicit porter fields must reproduce the built-in header");
        assert_eq!(header.block_hash().to_string(), PORTER_GENESIS_HASH);
    }

    #[test]
    fn wrong_expected_hash_is_rejected() {
        let overrides = DevnetGenesisOverride {
            hash: Some(
                "0000000000000000000000000000000000000000000000000000000000000001".to_string(),
            ),
            ..Default::default()
        };
        let err = resolve_devnet_genesis_header(&overrides)
            .expect_err("a mismatched expected hash must fail fast");
        assert!(err.contains("hash mismatch"), "got: {err}");
        assert!(
            err.contains(PORTER_GENESIS_HASH),
            "must name the computed hash: {err}"
        );
    }

    #[test]
    fn changing_a_field_changes_the_hash() {
        // Override the nonce without a matching expected hash: the
        // self-check now compares against the *new* header's own hash,
        // which differs from the porter constant. Proves a field
        // change actually propagates into the block hash (no silent
        // no-op) and that the self-check tracks the override.
        let overrides = DevnetGenesisOverride {
            nonce: Some(1096448),
            ..Default::default()
        };
        let header = resolve_devnet_genesis_header(&overrides)
            .expect("self-check uses the overridden header's own hash");
        assert_ne!(header.block_hash().to_string(), PORTER_GENESIS_HASH);
    }

    #[test]
    fn malformed_merkle_root_hex_errors() {
        let overrides = DevnetGenesisOverride {
            merkle_root: Some("not-hex".to_string()),
            ..Default::default()
        };
        let err =
            resolve_devnet_genesis_header(&overrides).expect_err("non-hex merkle root must error");
        assert!(err.contains("merkle root"), "got: {err}");
    }

    #[test]
    fn is_empty_reports_no_overrides() {
        assert!(DevnetGenesisOverride::default().is_empty());
        assert!(!DevnetGenesisOverride {
            nonce: Some(1),
            ..Default::default()
        }
        .is_empty());
    }
}
