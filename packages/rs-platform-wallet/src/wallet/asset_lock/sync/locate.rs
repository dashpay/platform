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
//! block at that height and that block — fetched by the SPV header's hash and
//! checked against its merkle root — actually contains the transaction. DAPI
//! is only a data source: nothing it says is used unverified.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use dash_sdk::RequestSettings;
use dashcore::hashes::Hash;
use dashcore::{Block, BlockHash, Txid};

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
    /// The SPV header chain holds a block at `height`, and that block's
    /// transactions — checked against its merkle root — include this one.
    Mined {
        /// Height of the block.
        height: u32,
        /// Hash of the SPV header the inclusion was verified against;
        /// re-checked before every use.
        block_hash: BlockHash,
    },
    /// DAPI knows the transaction but reports no block for it.
    NotMined,
    /// DAPI does not know the transaction.
    NotFound,
    /// DAPI reported a block the SPV header store does not hold.
    HeaderMissing { height: u32 },
    /// DAPI reported a block that differs from the SPV header at that height.
    HeaderMismatch { height: u32 },
    /// The SPV-verified block at `height` does not contain the transaction.
    NotIncluded { height: u32 },
    /// The block at `height` could not be checked for the transaction.
    BlockUnverifiable { height: u32, reason: String },
    /// No answer: transport failure, or no lookup is configured.
    Unavailable(String),
}

/// Finds the block a transaction was mined in.
#[async_trait]
pub(crate) trait MinedHeightLocator: Send + Sync {
    /// Where `txid` was mined, as far as this locator can establish.
    async fn locate(&self, txid: &Txid) -> Located;

    /// Hash of the SPV header this locator verifies against at `height`, or
    /// `None` when it has no such header.
    async fn header_hash_at(&self, height: u32) -> Option<BlockHash>;
}

/// Locator for managers built without a DAPI + SPV pair. Answers
/// `Unavailable`, so the ChainLock wait relies on the record alone.
pub(crate) struct NoMinedHeightLocator;

#[async_trait]
impl MinedHeightLocator for NoMinedHeightLocator {
    async fn locate(&self, _txid: &Txid) -> Located {
        Located::Unavailable("no mined-height locator configured".to_string())
    }

    async fn header_hash_at(&self, _height: u32) -> Option<BlockHash> {
        None
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

/// Request settings for one block fetch: blocks are larger than a transaction
/// reply, so a longer timeout and a decoding limit sized for a full block, but
/// still a single retry.
const BLOCK_REQUEST_SETTINGS: RequestSettings = RequestSettings {
    connect_timeout: None,
    timeout: Some(Duration::from_secs(10)),
    retries: Some(1),
    ban_failed_address: None,
    max_decoding_message_size: Some(4 << 20),
};

/// What DAPI reports about the block a transaction is in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ReportedPlacement {
    /// Height of the block (0 if unconfirmed).
    pub(crate) height: u32,
    /// Hash of the block, if DAPI reported a well-formed one.
    pub(crate) block_hash: Option<BlockHash>,
}

/// Core chain data served by DAPI. Everything it returns is unverified.
#[async_trait]
pub(crate) trait CoreBlockSource: Send + Sync {
    /// DAPI's placement of `txid`; `Ok(None)` when DAPI does not know it.
    async fn placement(&self, txid: &Txid) -> Result<Option<ReportedPlacement>, String>;

    /// The block with hash `hash`; `Ok(None)` when DAPI does not serve it.
    async fn block(&self, hash: &BlockHash) -> Result<Option<Block>, String>;
}

/// The SDK's DAPI client as a [`CoreBlockSource`].
struct SdkCoreBlockSource(Arc<dash_sdk::Sdk>);

#[async_trait]
impl CoreBlockSource for SdkCoreBlockSource {
    async fn placement(&self, txid: &Txid) -> Result<Option<ReportedPlacement>, String> {
        // Core's RPC takes the display-order hex that `Txid` formats to.
        self.0
            .get_transaction_placement(&txid.to_string(), LOCATE_REQUEST_SETTINGS)
            .await
            .map(|placement| {
                placement.map(|p| ReportedPlacement {
                    height: p.height,
                    block_hash: p.block_hash,
                })
            })
            .map_err(|e| e.to_string())
    }

    async fn block(&self, hash: &BlockHash) -> Result<Option<Block>, String> {
        self.0
            .get_block_by_hash(hash, BLOCK_REQUEST_SETTINGS)
            .await
            .map_err(|e| e.to_string())
    }
}

/// Production locator: DAPI for the placement and the block, the SPV header
/// store to decide which block to trust.
pub(crate) struct DapiSpvLocator {
    core: Arc<dyn CoreBlockSource>,
    headers: Arc<dyn BlockHeaderSource>,
}

impl DapiSpvLocator {
    /// A locator asking `sdk` for placements and blocks and checking them
    /// against `headers`.
    pub(crate) fn new(sdk: Arc<dash_sdk::Sdk>, headers: Arc<dyn BlockHeaderSource>) -> Self {
        Self {
            core: Arc::new(SdkCoreBlockSource(sdk)),
            headers,
        }
    }

    /// A locator over an arbitrary chain-data source, for tests.
    #[cfg(test)]
    pub(crate) fn with_core_source(
        core: Arc<dyn CoreBlockSource>,
        headers: Arc<dyn BlockHeaderSource>,
    ) -> Self {
        Self { core, headers }
    }
}

#[async_trait]
impl MinedHeightLocator for DapiSpvLocator {
    async fn locate(&self, txid: &Txid) -> Located {
        let placement = match self.core.placement(txid).await {
            Ok(Some(placement)) => placement,
            Ok(None) => return Located::NotFound,
            Err(e) => return Located::Unavailable(e),
        };
        let spv_hash = if placement.height > 0 {
            self.headers.header_hash_at(placement.height).await
        } else {
            None
        };
        let (height, spv_hash) =
            match classify_reported_block(placement.height, placement.block_hash, spv_hash) {
                PlacementCheck::HeaderMatches { height, spv_hash } => (height, spv_hash),
                PlacementCheck::Rejected(located) => return located,
            };

        // Fetch by the SPV header's hash, not DAPI's: the block has to be the
        // one the verified header chain holds.
        let block = match self.core.block(&spv_hash).await {
            Ok(Some(block)) => block,
            Ok(None) => {
                return Located::BlockUnverifiable {
                    height,
                    reason: "block not served".to_string(),
                }
            }
            Err(e) => return Located::Unavailable(e),
        };
        match verify_inclusion(&block, spv_hash, txid) {
            Inclusion::Included => Located::Mined {
                height,
                block_hash: spv_hash,
            },
            Inclusion::NotIncluded => Located::NotIncluded { height },
            Inclusion::BlockHashMismatch => Located::BlockUnverifiable {
                height,
                reason: "served block does not hash to the SPV header".to_string(),
            },
            Inclusion::MerkleRootMismatch => Located::BlockUnverifiable {
                height,
                reason: "served block's transactions do not match its merkle root".to_string(),
            },
        }
    }

    async fn header_hash_at(&self, height: u32) -> Option<BlockHash> {
        self.headers.header_hash_at(height).await
    }
}

/// DAPI's placement checked against the SPV header chain, before inclusion.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PlacementCheck {
    /// The SPV header at `height` is the block DAPI named; its hash is `spv_hash`.
    HeaderMatches { height: u32, spv_hash: BlockHash },
    /// The placement cannot be used; the reason as a lookup answer.
    Rejected(Located),
}

/// Whether a served block contains a transaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Inclusion {
    /// The block is the SPV header's, its merkle root holds, and it has the tx.
    Included,
    /// The block is verified but does not have the tx.
    NotIncluded,
    /// The block does not hash to the SPV header.
    BlockHashMismatch,
    /// The block's transactions do not produce its header's merkle root.
    MerkleRootMismatch,
}

/// Check that `block` is the SPV-verified block `spv_hash`, that its
/// transactions are the ones its header commits to, and that `txid` is among
/// them.
pub(crate) fn verify_inclusion(block: &Block, spv_hash: BlockHash, txid: &Txid) -> Inclusion {
    if block.block_hash() != spv_hash {
        return Inclusion::BlockHashMismatch;
    }
    if !block.check_merkle_root() {
        return Inclusion::MerkleRootMismatch;
    }
    if block.txdata.iter().any(|tx| tx.txid() == *txid) {
        Inclusion::Included
    } else {
        Inclusion::NotIncluded
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
) -> PlacementCheck {
    let Some(reported_hash) = reported_hash.filter(|_| reported_height > 0) else {
        return PlacementCheck::Rejected(Located::NotMined);
    };
    let Some(spv_hash) = spv_hash else {
        return PlacementCheck::Rejected(Located::HeaderMissing {
            height: reported_height,
        });
    };
    let mut reversed = reported_hash.to_byte_array();
    reversed.reverse();
    if spv_hash == reported_hash || spv_hash == BlockHash::from_byte_array(reversed) {
        PlacementCheck::HeaderMatches {
            height: reported_height,
            spv_hash,
        }
    } else {
        PlacementCheck::Rejected(Located::HeaderMismatch {
            height: reported_height,
        })
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
        Located::Mined { height, .. }
            if networks_match && wallet_chain_lock_height.is_some_and(|cl| cl >= *height) =>
        {
            Some(*height)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    use dashcore::block::{Header, Version};
    use dashcore::{CompactTarget, Network, OutPoint, Transaction, TxIn, TxMerkleNode};

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
            PlacementCheck::HeaderMatches {
                height: 100,
                spv_hash: hash(7)
            }
        );
    }

    /// The reported hash matches in either byte order.
    #[test]
    fn reversed_reported_hash_still_matches() {
        assert_eq!(
            classify_reported_block(100, Some(reversed(hash(7))), Some(hash(7))),
            PlacementCheck::HeaderMatches {
                height: 100,
                spv_hash: hash(7)
            }
        );
    }

    /// A different SPV header at that height is a mismatch.
    #[test]
    fn differing_header_is_a_mismatch() {
        assert_eq!(
            classify_reported_block(100, Some(hash(7)), Some(hash(8))),
            PlacementCheck::Rejected(Located::HeaderMismatch { height: 100 })
        );
    }

    /// No SPV header at that height is reported as missing.
    #[test]
    fn absent_header_is_reported_as_missing() {
        assert_eq!(
            classify_reported_block(100, Some(hash(7)), None),
            PlacementCheck::Rejected(Located::HeaderMissing { height: 100 })
        );
    }

    /// A zero height or an absent hash means not mined.
    #[test]
    fn no_height_or_no_hash_means_not_mined() {
        assert_eq!(
            classify_reported_block(0, Some(hash(7)), Some(hash(7))),
            PlacementCheck::Rejected(Located::NotMined)
        );
        assert_eq!(
            classify_reported_block(100, None, Some(hash(7))),
            PlacementCheck::Rejected(Located::NotMined)
        );
    }

    /// A placement yields a height only under the wallet's ChainLock.
    #[test]
    fn proof_height_needs_wallet_chain_lock_coverage() {
        let mined = Located::Mined {
            height: 100,
            block_hash: hash(7),
        };
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
        let mined = Located::Mined {
            height: 100,
            block_hash: hash(7),
        };
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
            Located::NotIncluded { height: 100 },
            Located::BlockUnverifiable {
                height: 100,
                reason: "block not served".to_string(),
            },
            Located::Unavailable("offline".to_string()),
        ] {
            assert_eq!(
                chain_proof_height_from_lookup(&located, Some(150), true),
                None,
                "{located:?}"
            );
        }
    }

    /// Mainnet's genesis block: a real, one-transaction block.
    fn genesis() -> Block {
        dashcore::constants::genesis_block(Network::Mainnet)
    }

    /// A transaction with a txid unique to `seed`.
    fn tx(seed: u8) -> Transaction {
        Transaction {
            version: 1,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: OutPoint::new(Txid::from([seed; 32]), 0),
                ..Default::default()
            }],
            output: Vec::new(),
            special_transaction_payload: None,
        }
    }

    /// The genesis coinbase is found in the genesis block.
    #[test]
    fn genesis_coinbase_is_included() {
        let block = genesis();
        let coinbase = block.txdata[0].txid();
        assert_eq!(
            verify_inclusion(&block, block.block_hash(), &coinbase),
            Inclusion::Included
        );
    }

    /// A txid the verified block does not carry is not included.
    #[test]
    fn unknown_txid_is_not_included() {
        let block = genesis();
        assert_eq!(
            verify_inclusion(&block, block.block_hash(), &tx(9).txid()),
            Inclusion::NotIncluded
        );
    }

    /// A block that is not the SPV header's block is rejected before its
    /// contents are looked at.
    #[test]
    fn block_bytes_not_matching_spv_header_are_rejected() {
        let block = genesis();
        let coinbase = block.txdata[0].txid();
        assert_eq!(
            verify_inclusion(&block, hash(3), &coinbase),
            Inclusion::BlockHashMismatch
        );
    }

    /// Swapping a transaction under an unchanged header breaks the merkle root.
    #[test]
    fn tampered_txdata_fails_merkle_root() {
        let mut block = Block {
            header: Header {
                version: Version::default(),
                prev_blockhash: BlockHash::all_zeros(),
                merkle_root: TxMerkleNode::all_zeros(),
                time: 0,
                bits: CompactTarget::from_consensus(0x1d00ffff),
                nonce: 0,
            },
            txdata: vec![tx(1), tx(2)],
        };
        block.header.merkle_root = block.compute_merkle_root().expect("non-empty");
        let spv_hash = block.block_hash();
        let funding = tx(1).txid();
        assert_eq!(
            verify_inclusion(&block, spv_hash, &funding),
            Inclusion::Included
        );

        block.txdata[1] = tx(3);
        assert_eq!(
            verify_inclusion(&block, spv_hash, &funding),
            Inclusion::MerkleRootMismatch
        );
    }

    /// Scripted DAPI answers, counting block fetches and the hashes asked for.
    struct FakeCoreBlockSource {
        placement: Result<Option<ReportedPlacement>, String>,
        block: Result<Option<Block>, String>,
        block_calls: AtomicUsize,
        requested: Mutex<Vec<BlockHash>>,
    }

    impl FakeCoreBlockSource {
        /// A source answering `placement` and `block` on every call.
        fn new(
            placement: Result<Option<ReportedPlacement>, String>,
            block: Result<Option<Block>, String>,
        ) -> Arc<Self> {
            Arc::new(Self {
                placement,
                block,
                block_calls: AtomicUsize::new(0),
                requested: Mutex::new(Vec::new()),
            })
        }

        /// The block hashes `block()` was asked for, in order.
        fn requested_block_hashes(&self) -> Vec<BlockHash> {
            self.requested.lock().unwrap().clone()
        }

        /// How many blocks were fetched.
        fn block_calls(&self) -> usize {
            self.block_calls.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl CoreBlockSource for FakeCoreBlockSource {
        async fn placement(&self, _txid: &Txid) -> Result<Option<ReportedPlacement>, String> {
            self.placement.clone()
        }

        async fn block(&self, hash: &BlockHash) -> Result<Option<Block>, String> {
            self.block_calls.fetch_add(1, Ordering::SeqCst);
            self.requested.lock().unwrap().push(*hash);
            self.block.clone()
        }
    }

    /// An SPV header store holding `hash` at every height.
    struct FakeHeaders(Option<BlockHash>);

    #[async_trait]
    impl BlockHeaderSource for FakeHeaders {
        async fn header_hash_at(&self, _height: u32) -> Option<BlockHash> {
            self.0
        }
    }

    /// DAPI placing the tx in `block` at height 1.
    fn placed_in(block: &Block) -> Result<Option<ReportedPlacement>, String> {
        Ok(Some(ReportedPlacement {
            height: 1,
            block_hash: Some(block.block_hash()),
        }))
    }

    /// A locator over `core` with the SPV header store holding `spv_hash`.
    fn locator(core: Arc<FakeCoreBlockSource>, spv_hash: Option<BlockHash>) -> DapiSpvLocator {
        DapiSpvLocator::with_core_source(core, Arc::new(FakeHeaders(spv_hash)))
    }

    /// `Mined` only comes back after the block proved to contain the tx.
    #[tokio::test]
    async fn locate_verifies_inclusion_before_mined() {
        let block = genesis();
        let core = FakeCoreBlockSource::new(placed_in(&block), Ok(Some(block.clone())));
        let located = locator(Arc::clone(&core), Some(block.block_hash()))
            .locate(&block.txdata[0].txid())
            .await;
        assert_eq!(
            located,
            Located::Mined {
                height: 1,
                block_hash: block.block_hash(),
            }
        );
        assert_eq!(core.block_calls(), 1);
        assert_eq!(core.requested_block_hashes(), vec![block.block_hash()]);
    }

    /// A verified block without the tx is reported as not including it.
    #[tokio::test]
    async fn locate_reports_not_included_when_block_lacks_txid() {
        let block = genesis();
        let core = FakeCoreBlockSource::new(placed_in(&block), Ok(Some(block.clone())));
        let located = locator(Arc::clone(&core), Some(block.block_hash()))
            .locate(&tx(9).txid())
            .await;
        assert_eq!(located, Located::NotIncluded { height: 1 });
        assert_eq!(core.requested_block_hashes(), vec![block.block_hash()]);
    }

    /// A placement the SPV header contradicts never costs a block fetch.
    #[tokio::test]
    async fn locate_skips_the_block_fetch_on_header_mismatch() {
        let block = genesis();
        let core = FakeCoreBlockSource::new(placed_in(&block), Ok(Some(block.clone())));
        let located = locator(Arc::clone(&core), Some(hash(3)))
            .locate(&block.txdata[0].txid())
            .await;
        assert_eq!(located, Located::HeaderMismatch { height: 1 });
        assert_eq!(core.block_calls(), 0);
    }

    /// A block DAPI does not serve leaves inclusion unverifiable.
    #[tokio::test]
    async fn locate_reports_unverifiable_when_block_is_not_served() {
        let block = genesis();
        let core = FakeCoreBlockSource::new(placed_in(&block), Ok(None));
        let located = locator(core, Some(block.block_hash()))
            .locate(&block.txdata[0].txid())
            .await;
        assert_eq!(
            located,
            Located::BlockUnverifiable {
                height: 1,
                reason: "block not served".to_string(),
            }
        );
    }

    /// The block is fetched by the SPV header's hash even when DAPI reports
    /// the same hash in the other byte order.
    #[tokio::test]
    async fn locate_requests_the_block_by_the_spv_header_hash_not_dapis() {
        let block = genesis();
        let spv = block.block_hash();
        let core = FakeCoreBlockSource::new(
            Ok(Some(ReportedPlacement {
                height: 1,
                block_hash: Some(reversed(spv)),
            })),
            Ok(Some(block.clone())),
        );
        let located = locator(Arc::clone(&core), Some(spv))
            .locate(&block.txdata[0].txid())
            .await;
        assert_eq!(
            located,
            Located::Mined {
                height: 1,
                block_hash: spv,
            }
        );
        assert_eq!(core.requested_block_hashes(), vec![spv]);
    }

    /// The locator answers header checks from its SPV header store.
    #[tokio::test]
    async fn locator_exposes_the_spv_header_hash() {
        let core = FakeCoreBlockSource::new(Ok(None), Ok(None));
        let locator = locator(core, Some(hash(3)));
        assert_eq!(locator.header_hash_at(5).await, Some(hash(3)));
    }
}
