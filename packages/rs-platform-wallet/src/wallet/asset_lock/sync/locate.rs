//! Locating the block a funding transaction was mined in when the wallet's
//! own record cannot say.
//!
//! A ChainLock asset-lock proof needs only the outpoint and a chain-locked
//! height at or above the transaction's block. The wallet normally reads that
//! height off its transaction record, but a record can be left without one —
//! an `InstantSend` context carries the lock and no `BlockInfo` — and ChainLock
//! promotion only ever advances `InBlock` records, so waiting for the record
//! to change can wait forever. This module asks DAPI where the transaction was
//! mined and accepts the answer only when the SPV header chain has the same
//! block at that height.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use dash_sdk::RequestSettings;
use dashcore::hashes::Hash;
use dashcore::{BlockHash, Txid};

/// Source of SPV-verified block headers, by height.
#[async_trait]
pub(crate) trait BlockHeaderSource: Send + Sync {
    /// Hash of the stored header at `height`, or `None` when the store does
    /// not hold it.
    async fn header_hash_at(&self, height: u32) -> Option<BlockHash>;
}

/// Where a lookup placed a transaction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Located {
    /// DAPI placed the transaction at `height`, and the SPV header chain holds
    /// the same block there.
    Mined { height: u32 },
    /// DAPI knows the transaction but reports no block for it.
    NotMined,
    /// DAPI does not know the transaction.
    NotFound,
    /// DAPI reported a block the SPV header store does not hold.
    HeaderMissing { height: u32 },
    /// DAPI reported a block that differs from the SPV header at that height.
    HeaderMismatch { height: u32 },
    /// No answer: transport failure, or no lookup is configured.
    Unavailable(String),
}

/// Finds the block a transaction was mined in.
#[async_trait]
pub(crate) trait MinedHeightLocator: Send + Sync {
    /// Where `txid` was mined, as far as this locator can establish.
    async fn locate(&self, txid: &Txid) -> Located;
}

/// Locator for managers built without a DAPI + SPV pair. Answers
/// `Unavailable`, so the ChainLock wait relies on the record alone.
pub(crate) struct NoMinedHeightLocator;

#[async_trait]
impl MinedHeightLocator for NoMinedHeightLocator {
    async fn locate(&self, _txid: &Txid) -> Located {
        Located::Unavailable("no mined-height locator configured".to_string())
    }
}

/// Request settings for one placement lookup: a short per-request timeout and a
/// single retry, instead of the SDK defaults (several retries at a longer
/// timeout). The ChainLock wait caps each attempt as well; these keep the
/// transport from spending that whole budget on one node.
const LOCATE_REQUEST_SETTINGS: RequestSettings = RequestSettings {
    connect_timeout: None,
    timeout: Some(Duration::from_secs(5)),
    retries: Some(1),
    ban_failed_address: None,
    max_decoding_message_size: None,
};

/// Production locator: DAPI `getTransaction` for the placement, the SPV
/// header store to verify it.
pub(crate) struct DapiSpvLocator {
    sdk: Arc<dash_sdk::Sdk>,
    headers: Arc<dyn BlockHeaderSource>,
}

impl DapiSpvLocator {
    /// A locator asking `sdk` for placements and checking them against `headers`.
    pub(crate) fn new(sdk: Arc<dash_sdk::Sdk>, headers: Arc<dyn BlockHeaderSource>) -> Self {
        Self { sdk, headers }
    }
}

#[async_trait]
impl MinedHeightLocator for DapiSpvLocator {
    async fn locate(&self, txid: &Txid) -> Located {
        // Core's RPC takes the display-order hex that `Txid` formats to.
        let fetched = match self
            .sdk
            .get_transaction_placement(&txid.to_string(), LOCATE_REQUEST_SETTINGS)
            .await
        {
            Ok(Some(fetched)) => fetched,
            Ok(None) => return Located::NotFound,
            Err(e) => return Located::Unavailable(e.to_string()),
        };
        let spv_hash = if fetched.height > 0 {
            self.headers.header_hash_at(fetched.height).await
        } else {
            None
        };
        classify_reported_block(fetched.height, fetched.block_hash, spv_hash)
    }
}

/// Classify DAPI's placement against the SPV header at the same height.
///
/// The hash is compared in both byte orders: the orientation of the reported
/// bytes is a DAPI serialization detail, and a reversed 32-byte match binds the
/// block just as well. Getting it wrong the other way would turn every lookup
/// into a mismatch and leave the wait unresolvable.
pub(crate) fn classify_reported_block(
    reported_height: u32,
    reported_hash: Option<BlockHash>,
    spv_hash: Option<BlockHash>,
) -> Located {
    let Some(reported_hash) = reported_hash.filter(|_| reported_height > 0) else {
        return Located::NotMined;
    };
    let Some(spv_hash) = spv_hash else {
        return Located::HeaderMissing {
            height: reported_height,
        };
    };
    let mut reversed = reported_hash.to_byte_array();
    reversed.reverse();
    if spv_hash == reported_hash || spv_hash == BlockHash::from_byte_array(reversed) {
        Located::Mined {
            height: reported_height,
        }
    } else {
        Located::HeaderMismatch {
            height: reported_height,
        }
    }
}

/// The ChainLock proof height a lookup supports, if any: the transaction is
/// verifiably mined, the wallet's own BLS-verified ChainLock already covers
/// its block, and that ChainLock belongs to the SDK's network.
pub(crate) fn chain_proof_height_from_lookup(
    located: &Located,
    wallet_chain_lock_height: Option<u32>,
    networks_match: bool,
) -> Option<u32> {
    match located {
        Located::Mined { height }
            if networks_match && wallet_chain_lock_height.is_some_and(|cl| cl >= *height) =>
        {
            Some(*height)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A distinct, non-palindromic block hash per `byte`.
    fn hash(byte: u8) -> BlockHash {
        let mut bytes = [0u8; 32];
        bytes[0] = byte;
        bytes[31] = byte.wrapping_add(1);
        BlockHash::from_byte_array(bytes)
    }

    /// `h` with its bytes reversed.
    fn reversed(h: BlockHash) -> BlockHash {
        let mut bytes = h.to_byte_array();
        bytes.reverse();
        BlockHash::from_byte_array(bytes)
    }

    /// A reported block the SPV headers hold places the transaction.
    #[test]
    fn matching_header_places_the_transaction() {
        assert_eq!(
            classify_reported_block(100, Some(hash(7)), Some(hash(7))),
            Located::Mined { height: 100 }
        );
    }

    /// The reported hash matches in either byte order.
    #[test]
    fn reversed_reported_hash_still_matches() {
        assert_eq!(
            classify_reported_block(100, Some(reversed(hash(7))), Some(hash(7))),
            Located::Mined { height: 100 }
        );
    }

    /// A different SPV header at that height is a mismatch.
    #[test]
    fn differing_header_is_a_mismatch() {
        assert_eq!(
            classify_reported_block(100, Some(hash(7)), Some(hash(8))),
            Located::HeaderMismatch { height: 100 }
        );
    }

    /// No SPV header at that height is reported as missing.
    #[test]
    fn absent_header_is_reported_as_missing() {
        assert_eq!(
            classify_reported_block(100, Some(hash(7)), None),
            Located::HeaderMissing { height: 100 }
        );
    }

    /// A zero height or an absent hash means not mined.
    #[test]
    fn no_height_or_no_hash_means_not_mined() {
        assert_eq!(
            classify_reported_block(0, Some(hash(7)), Some(hash(7))),
            Located::NotMined
        );
        assert_eq!(
            classify_reported_block(100, None, Some(hash(7))),
            Located::NotMined
        );
    }

    /// A placement yields a height only under the wallet's ChainLock.
    #[test]
    fn proof_height_needs_wallet_chain_lock_coverage() {
        let mined = Located::Mined { height: 100 };
        assert_eq!(
            chain_proof_height_from_lookup(&mined, Some(100), true),
            Some(100)
        );
        assert_eq!(
            chain_proof_height_from_lookup(&mined, Some(150), true),
            Some(100)
        );
        assert_eq!(chain_proof_height_from_lookup(&mined, Some(99), true), None);
        assert_eq!(chain_proof_height_from_lookup(&mined, None, true), None);
    }

    /// A ChainLock from another network yields no height.
    #[test]
    fn proof_height_refuses_a_network_mismatch() {
        let mined = Located::Mined { height: 100 };
        assert_eq!(
            chain_proof_height_from_lookup(&mined, Some(150), false),
            None
        );
    }

    /// Every answer but a verified placement yields no height.
    #[test]
    fn only_a_verified_placement_yields_a_height() {
        for located in [
            Located::NotMined,
            Located::NotFound,
            Located::HeaderMissing { height: 100 },
            Located::HeaderMismatch { height: 100 },
            Located::Unavailable("offline".to_string()),
        ] {
            assert_eq!(
                chain_proof_height_from_lookup(&located, Some(150), true),
                None,
                "{located:?}"
            );
        }
    }
}
