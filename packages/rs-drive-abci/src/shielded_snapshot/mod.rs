//! Shielded-pool genesis snapshot — production module.
//!
//! Reduces shielded-pool seeding from a runtime cost (~3h 41m for 500k notes
//! on macOS Docker, ~65 min native) to a one-shot offline bake + at-boot
//! SST ingest (~few seconds at any N).
//!
//! Two entry points:
//!
//! - [`dump_shielded_subtree`] — runs from a snapshot-bake binary. Reads the
//!   already-populated shielded subtree from a live GroveDB and writes a
//!   portable snapshot file containing one SST blob + header + checksum.
//! - [`apply_shielded_snapshot`] — runs from drive-abci's `InitChain`. Reads
//!   the snapshot file, validates header + checksum, ingests the SST into
//!   the underlying RocksDB, cross-validates the reconstructed state against
//!   the header's `combined_root`, then writes the parent-Merk
//!   `Element::CommitmentTree` leaf via the new
//!   `GroveDb::replace_commitment_tree_subtree_root` public API.
//!
//! Built on three new public methods we added to grovedb on the
//! `feat/snapshot-apply-public-api` branch:
//!
//! 1. `GroveDb::raw_storage()` — escape hatch to the underlying
//!    `RocksDbStorage` so we can open a `StorageContext` for raw iteration.
//! 2. `GroveDb::ingest_subtree_sst(cf, sst_path)` — bulk-ingest an SST file.
//! 3. `GroveDb::replace_commitment_tree_subtree_root(...)` — patch the
//!    parent-Merk `Element::CommitmentTree` leaf with a caller-provided
//!    `combined_root`.
//!
//! See `docs/genesis-snapshot-design.md` for the full design + threat model.

#![allow(missing_docs)]

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use dpp::version::PlatformVersion;
use drive::drive::shielded::paths::{shielded_credit_pool_path_vec, SHIELDED_NOTES_KEY};
use drive::grovedb::{Element, GroveDb, TransactionArg};
use drive::grovedb_path::SubtreePath;
use drive::grovedb_storage::rocksdb_storage::RocksDbStorage;
use drive::grovedb_storage::{RawIterator, Storage, StorageContext};
use grovedb_commitment_tree::{CommitmentTree, DashMemo};

/// File magic — 8 bytes, matches the `b"DRVSHLD\0"` literal.
const MAGIC: [u8; 8] = *b"DRVSHLD\0";
/// Snapshot file format version. Bump on any breaking header/section change.
const FORMAT_VERSION: u32 = 1;

/// Max permitted `chunk_power`. BulkAppendTree internally caps at 31; for
/// genesis snapshots we want a tighter sanity bound — anything > 16 would
/// imply a chunk holding ≥ 65 536 cmx values, which is far beyond anything
/// we'd ever ship.
const MAX_CHUNK_POWER: u8 = 16;

/// CF where the shielded subtree's keys live. Pinned empirically by the
/// `dump_only_default_and_aux_cfs_under_shielded_subtree_prefix` test —
/// EVERY key (BulkAppendTree state, dense-tree buffer, MMR nodes, META,
/// Sinsemilla frontier) is in the default CF for this subtree.
const SUBTREE_CF: &str = rocksdb::DEFAULT_COLUMN_FAMILY_NAME;

/// Errors surfaced by [`dump_shielded_subtree`] and [`apply_shielded_snapshot`].
#[derive(Debug)]
pub enum ShieldedSnapshotError {
    Io(std::io::Error),
    InvalidMagic {
        got: [u8; 8],
    },
    FormatVersionMismatch {
        expected: u32,
        found: u32,
    },
    ChunkPowerTooLarge {
        got: u8,
        max: u8,
    },
    ChecksumMismatch {
        expected: [u8; 32],
        computed: [u8; 32],
    },
    /// Header's `combined_root` doesn't match what reconstructing the
    /// CommitmentTree from the ingested data produces. Indicates tampering,
    /// truncation, or version skew.
    CombinedRootMismatch {
        expected: [u8; 32],
        computed: [u8; 32],
    },
    /// The element at the expected parent-leaf path/key is not
    /// `Element::CommitmentTree`. InitChain must build the parent skeleton
    /// before applying the snapshot.
    ParentLeafWrongType,
    /// Bubbled from GroveDB / grovedb-storage / grovedb-commitment-tree.
    GroveDb(String),
    /// Bubbled from rocksdb (SST writer, ingest).
    RocksDb(String),
    /// Inconsistent header/file state.
    Inconsistent(String),
}

impl std::fmt::Display for ShieldedSnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "i/o: {e}"),
            Self::InvalidMagic { got } => write!(f, "invalid magic: {got:?}"),
            Self::FormatVersionMismatch { expected, found } => {
                write!(
                    f,
                    "format_version mismatch (expected {expected}, got {found})"
                )
            }
            Self::ChunkPowerTooLarge { got, max } => {
                write!(f, "chunk_power {got} exceeds max {max}")
            }
            Self::ChecksumMismatch { .. } => write!(f, "checksum mismatch"),
            Self::CombinedRootMismatch { .. } => write!(
                f,
                "combined_root mismatch — snapshot data doesn't match header"
            ),
            Self::ParentLeafWrongType => write!(f, "parent leaf is not Element::CommitmentTree"),
            Self::GroveDb(s) => write!(f, "grovedb: {s}"),
            Self::RocksDb(s) => write!(f, "rocksdb: {s}"),
            Self::Inconsistent(s) => write!(f, "inconsistent: {s}"),
        }
    }
}

impl std::error::Error for ShieldedSnapshotError {}

impl From<std::io::Error> for ShieldedSnapshotError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// Header of a shielded-pool snapshot file. See module docs for fields.
#[derive(Debug, Clone)]
pub struct SnapshotHeader {
    pub format_version: u32,
    pub total_count: u64,
    pub chunk_power: u8,
    /// First byte of the parent-leaf `Element::CommitmentTree` flags
    /// (`Option<ElementFlags>` where `ElementFlags = Vec<u8>`). We only
    /// encode one byte because the shielded leaf in practice carries either
    /// `None` flags or a single-byte flags vector; if we ever ship multi-
    /// byte flags this needs to widen and `FORMAT_VERSION` must bump.
    pub flags_byte: u8,
    pub combined_root: [u8; 32],
    pub sst_len: u64,
}

impl SnapshotHeader {
    /// Wire size of the encoded header (NOT including the SST blob or the
    /// trailing checksum).
    const ENCODED_LEN: usize = 8 + 4 + 8 + 1 + 1 + 32 + 8; // = 62 bytes

    fn write_to<W: Write>(&self, w: &mut W) -> Result<(), std::io::Error> {
        w.write_all(&MAGIC)?;
        w.write_all(&self.format_version.to_be_bytes())?;
        w.write_all(&self.total_count.to_be_bytes())?;
        w.write_all(&[self.chunk_power])?;
        w.write_all(&[self.flags_byte])?;
        w.write_all(&self.combined_root)?;
        w.write_all(&self.sst_len.to_be_bytes())?;
        Ok(())
    }

    fn read_from<R: Read>(r: &mut R) -> Result<Self, ShieldedSnapshotError> {
        let mut magic = [0u8; 8];
        r.read_exact(&mut magic)?;
        if magic != MAGIC {
            return Err(ShieldedSnapshotError::InvalidMagic { got: magic });
        }
        let mut buf4 = [0u8; 4];
        r.read_exact(&mut buf4)?;
        let format_version = u32::from_be_bytes(buf4);
        if format_version != FORMAT_VERSION {
            return Err(ShieldedSnapshotError::FormatVersionMismatch {
                expected: FORMAT_VERSION,
                found: format_version,
            });
        }
        let mut buf8 = [0u8; 8];
        r.read_exact(&mut buf8)?;
        let total_count = u64::from_be_bytes(buf8);
        let mut one = [0u8; 1];
        r.read_exact(&mut one)?;
        let chunk_power = one[0];
        if chunk_power > MAX_CHUNK_POWER {
            return Err(ShieldedSnapshotError::ChunkPowerTooLarge {
                got: chunk_power,
                max: MAX_CHUNK_POWER,
            });
        }
        r.read_exact(&mut one)?;
        let flags_byte = one[0];
        let mut combined_root = [0u8; 32];
        r.read_exact(&mut combined_root)?;
        r.read_exact(&mut buf8)?;
        let sst_len = u64::from_be_bytes(buf8);
        Ok(Self {
            format_version,
            total_count,
            chunk_power,
            flags_byte,
            combined_root,
            sst_len,
        })
    }
}

/// Stats returned by [`dump_shielded_subtree`].
#[derive(Debug, Clone)]
pub struct DumpStats {
    pub total_count: u64,
    pub key_count: u64,
    pub sst_bytes: u64,
}

/// Stats returned by [`apply_shielded_snapshot`].
#[derive(Debug, Clone)]
pub struct ApplyStats {
    pub total_count: u64,
    pub combined_root: [u8; 32],
}

/// Build the SubtreePath segments for the shielded commitment-tree subtree.
fn shielded_subtree_segments() -> Vec<Vec<u8>> {
    let mut v = shielded_credit_pool_path_vec();
    v.push(vec![SHIELDED_NOTES_KEY]);
    v
}

/// Iterate the live shielded subtree's keys and write them into a snapshot
/// file at `out_path`.
///
/// Uses the public [`GroveDb`] API + raw RocksDB `SstFileWriter` to produce
/// a snapshot the apply side can `ingest_external_file_cf` into a fresh DB.
///
/// Reads the parent-Merk `Element::CommitmentTree` leaf to populate the
/// header's `total_count`/`chunk_power`/`flags`. Computes `combined_root` by
/// reconstructing the CommitmentTree from the same storage we're about to
/// dump — that root is what the apply side cross-validates against.
pub fn dump_shielded_subtree(
    grove: &GroveDb,
    transaction: TransactionArg,
    out_path: &Path,
    platform_version: &PlatformVersion,
) -> Result<DumpStats, ShieldedSnapshotError> {
    use rocksdb::{Options, SstFileWriter};

    let parent_segments = shielded_credit_pool_path_vec();
    let parent_path = SubtreePath::from(parent_segments.as_slice());
    let leaf_key = &[SHIELDED_NOTES_KEY];

    // 1. Read parent leaf for header values.
    let element = grove
        .get_raw(
            parent_path,
            leaf_key,
            transaction,
            &platform_version.drive.grove_version,
        )
        .value
        .map_err(|e| ShieldedSnapshotError::GroveDb(format!("get_raw parent leaf: {e}")))?;
    let (total_count, chunk_power, flags) = match element {
        Element::CommitmentTree(tc, cp, f) => (tc, cp, f),
        _ => return Err(ShieldedSnapshotError::ParentLeafWrongType),
    };
    if chunk_power > MAX_CHUNK_POWER {
        return Err(ShieldedSnapshotError::ChunkPowerTooLarge {
            got: chunk_power,
            max: MAX_CHUNK_POWER,
        });
    }

    // 2. Compute the 32-byte subtree prefix. RocksDB SST keys must be FULL
    //    keys (prefix prepended) because ingest_external_file_cf doesn't
    //    prepend anything — it expects already-final bytes.
    let subtree_segments = shielded_subtree_segments();
    let subtree_path = SubtreePath::from(subtree_segments.as_slice());
    let prefix: [u8; 32] = RocksDbStorage::build_prefix(subtree_path.clone())
        .unwrap()
        .into();

    // 3. Open transactional storage context at the subtree path. We use the
    //    caller's transaction if provided; otherwise start a local one.
    //    The context's raw_iter strips the prefix; we re-add it for the SST.
    let local_tx;
    let tx_ref: &drive::grovedb::Transaction = match transaction {
        Some(t) => t,
        None => {
            local_tx = grove.start_transaction();
            &local_tx
        }
    };
    let storage_ctx = grove
        .raw_storage()
        .get_transactional_storage_context(subtree_path, None, tx_ref)
        .unwrap();

    // 4. Compute `combined_root` for the header by reloading CommitmentTree
    //    from the same storage. Apply side recomputes independently and
    //    cross-validates — drift surfaces as CombinedRootMismatch.
    let ct = CommitmentTree::<_, DashMemo>::open(total_count, chunk_power, storage_ctx)
        .value
        .map_err(|e| ShieldedSnapshotError::GroveDb(format!("CommitmentTree::open: {e}")))?;
    let combined_root = ct
        .compute_current_state_root()
        .map_err(|e| ShieldedSnapshotError::GroveDb(format!("compute_current_state_root: {e}")))?;

    // The CommitmentTree owns the storage_ctx. We need to drop it to free
    // the borrow before opening the iterator on a fresh context.
    drop(ct);

    // 5. Open a SECOND storage context just for iteration. Two contexts on
    //    the same path/txn is safe — they share the underlying transaction.
    let subtree_segments_for_iter = shielded_subtree_segments();
    let iter_path = SubtreePath::from(subtree_segments_for_iter.as_slice());
    let iter_ctx = grove
        .raw_storage()
        .get_transactional_storage_context(iter_path, None, tx_ref)
        .unwrap();

    // 6. Open SST writer. Write SST to tmp file alongside the output path.
    let tmp_dir = out_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    let sst_tmp = tmp_dir.join(format!(
        ".{}.sst.tmp",
        out_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("shielded-snapshot")
    ));
    let sst_opts = Options::default();
    let mut sst = SstFileWriter::create(&sst_opts);
    sst.open(&sst_tmp)
        .map_err(|e| ShieldedSnapshotError::RocksDb(format!("SstFileWriter::open: {e}")))?;

    // 7. Walk subtree keys (sorted by RocksDB iterator), prepend prefix,
    //    write to SST. RocksDB SST requires keys in strictly increasing
    //    order — the raw_iter returns keys in lex order so prefix-prepended
    //    keys are also lex-ordered.
    let mut iter = iter_ctx.raw_iter();
    iter.seek_to_first().unwrap();
    let mut key_count: u64 = 0;
    loop {
        if !iter.valid().unwrap() {
            break;
        }
        let user_key = match iter.key().unwrap() {
            Some(k) => k.to_vec(),
            None => break,
        };
        let value = match iter.value().unwrap() {
            Some(v) => v.to_vec(),
            None => break,
        };
        let mut full_key = Vec::with_capacity(32 + user_key.len());
        full_key.extend_from_slice(&prefix);
        full_key.extend_from_slice(&user_key);
        sst.put(&full_key, &value)
            .map_err(|e| ShieldedSnapshotError::RocksDb(format!("SstFileWriter::put: {e}")))?;
        key_count += 1;
        iter.next().unwrap();
    }
    sst.finish()
        .map_err(|e| ShieldedSnapshotError::RocksDb(format!("SstFileWriter::finish: {e}")))?;
    let sst_bytes_on_disk = std::fs::metadata(&sst_tmp)?.len();

    // Release the iter_ctx borrow before writing the output file.
    drop(iter_ctx);

    // 8. Compose the output file: header || sst_bytes || blake3 checksum.
    let header = SnapshotHeader {
        format_version: FORMAT_VERSION,
        total_count,
        chunk_power,
        flags_byte: flags.as_ref().and_then(|v| v.first().copied()).unwrap_or(0),
        combined_root,
        sst_len: sst_bytes_on_disk,
    };

    let mut hasher = blake3::Hasher::new();
    let mut out = std::fs::File::create(out_path)?;
    let mut header_buf = Vec::with_capacity(SnapshotHeader::ENCODED_LEN);
    header.write_to(&mut header_buf)?;
    hasher.update(&header_buf);
    out.write_all(&header_buf)?;

    // Stream the SST file contents in chunks to avoid loading the full
    // (potentially-huge) SST into RAM.
    let mut sst_file = std::fs::File::open(&sst_tmp)?;
    let mut buf = vec![0u8; 256 * 1024];
    loop {
        let n = sst_file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        out.write_all(&buf[..n])?;
    }

    let checksum = hasher.finalize();
    out.write_all(checksum.as_bytes())?;
    out.sync_all()?;

    let _ = std::fs::remove_file(&sst_tmp);

    Ok(DumpStats {
        total_count,
        key_count,
        sst_bytes: sst_bytes_on_disk,
    })
}

/// Apply a snapshot file produced by [`dump_shielded_subtree`] into a fresh
/// GroveDB. Intended to be called from `InitChain` AFTER the parent tree
/// skeleton (`/platform/shielded_pool/`) has been built but BEFORE the
/// shielded subtree would otherwise be populated by the runtime seeder.
pub fn apply_shielded_snapshot(
    grove: &GroveDb,
    transaction: TransactionArg,
    snapshot_path: &Path,
    platform_version: &PlatformVersion,
) -> Result<ApplyStats, ShieldedSnapshotError> {
    // 1. Read the file into memory (small enough for N up to ~500k).
    let bytes = std::fs::read(snapshot_path)?;
    if bytes.len() < SnapshotHeader::ENCODED_LEN + 32 {
        return Err(ShieldedSnapshotError::Inconsistent(format!(
            "snapshot file shorter than minimal envelope ({} bytes)",
            bytes.len()
        )));
    }

    // 2. Parse + verify checksum.
    let header = {
        let mut cursor = std::io::Cursor::new(&bytes[..SnapshotHeader::ENCODED_LEN]);
        SnapshotHeader::read_from(&mut cursor)?
    };
    let body_start = SnapshotHeader::ENCODED_LEN;
    let body_end = body_start + header.sst_len as usize;
    if body_end + 32 > bytes.len() {
        return Err(ShieldedSnapshotError::Inconsistent(format!(
            "header says sst_len={} but file truncated",
            header.sst_len
        )));
    }
    let sst_slice = &bytes[body_start..body_end];

    let mut stored = [0u8; 32];
    stored.copy_from_slice(&bytes[body_end..body_end + 32]);
    let mut hasher = blake3::Hasher::new();
    hasher.update(&bytes[..body_end]);
    let computed = *hasher.finalize().as_bytes();
    if stored != computed {
        return Err(ShieldedSnapshotError::ChecksumMismatch {
            expected: stored,
            computed,
        });
    }

    // 3. Materialise SST to a tmp file (rocksdb ingest needs a real path).
    let tmp_dir = std::env::temp_dir();
    let sst_tmp = tmp_dir.join(format!(
        "drv-shielded-snapshot-{}-{}.sst",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0),
    ));
    std::fs::write(&sst_tmp, sst_slice)?;
    let _cleanup = SstTmpGuard(sst_tmp.clone());

    // 4. Bulk-ingest. Bypasses any open transaction; OK at InitChain time
    //    (txn abort = wipe-and-restart, so orphan data is unreachable).
    grove
        .ingest_subtree_sst(SUBTREE_CF, &sst_tmp)
        .map_err(|e| ShieldedSnapshotError::GroveDb(format!("ingest_subtree_sst: {e}")))?;

    // 5. Cross-validate: reload CommitmentTree from the just-ingested data
    //    and check recomputed combined_root matches header. Drift surfaces
    //    BEFORE we touch the parent Merk.
    let subtree_segments = shielded_subtree_segments();
    let subtree_path = SubtreePath::from(subtree_segments.as_slice());

    let local_tx;
    let tx_ref: &drive::grovedb::Transaction = match transaction {
        Some(t) => t,
        None => {
            local_tx = grove.start_transaction();
            &local_tx
        }
    };
    let storage_ctx = grove
        .raw_storage()
        .get_transactional_storage_context(subtree_path, None, tx_ref)
        .unwrap();

    let ct =
        CommitmentTree::<_, DashMemo>::open(header.total_count, header.chunk_power, storage_ctx)
            .value
            .map_err(|e| {
                ShieldedSnapshotError::GroveDb(format!("CommitmentTree::open after ingest: {e}"))
            })?;
    let recomputed = ct
        .compute_current_state_root()
        .map_err(|e| ShieldedSnapshotError::GroveDb(format!("compute_current_state_root: {e}")))?;
    drop(ct);

    if recomputed != header.combined_root {
        return Err(ShieldedSnapshotError::CombinedRootMismatch {
            expected: header.combined_root,
            computed: recomputed,
        });
    }

    // 6. Patch parent Merk leaf.
    let parent_segments = shielded_credit_pool_path_vec();
    let parent_path = SubtreePath::from(parent_segments.as_slice());
    let leaf_key = &[SHIELDED_NOTES_KEY];
    let flags = if header.flags_byte == 0 {
        None
    } else {
        Some(vec![header.flags_byte])
    };

    grove
        .replace_commitment_tree_subtree_root(
            parent_path,
            leaf_key,
            header.total_count,
            header.chunk_power,
            flags,
            header.combined_root,
            transaction,
            &platform_version.drive.grove_version,
        )
        .value
        .map_err(|e| {
            ShieldedSnapshotError::GroveDb(format!("replace_commitment_tree_subtree_root: {e}"))
        })?;

    Ok(ApplyStats {
        total_count: header.total_count,
        combined_root: header.combined_root,
    })
}

/// RAII guard that removes the apply-side tmp SST file regardless of which
/// path through the function we exit on.
struct SstTmpGuard(PathBuf);
impl Drop for SstTmpGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
