//! Shielded-pool genesis snapshot — production module.
//!
//! Reduces shielded-pool seeding from a runtime cost (~3h 41m for 500k notes
//! on macOS Docker, ~65 min native) to a one-shot offline bake + at-boot
//! load of the baked records.
//!
//! Two entry points:
//!
//! - [`dump_shielded_subtree`] — runs from a snapshot-bake binary. Dumps the
//!   already-populated shielded subtree of a live GroveDB and writes a
//!   portable snapshot file containing header + dump + checksum.
//! - [`apply_shielded_snapshot`] — runs from drive-abci's `InitChain`. Reads
//!   the snapshot file, validates header + checksum, loads the dump into the
//!   empty shielded subtree, cross-validates the loaded records against the
//!   header's `combined_root`, then writes the parent-Merk
//!   `Element::CommitmentTree` leaf via `GroveDb::replace_subtree_root`.
//!
//! Built on four GroveDB calls behind its `unsafe-dump-load` feature, none of
//! which names a storage engine:
//!
//! 1. `GroveDb::dump_subtree(path, key, out, ..)` — writes the records of
//!    one subtree, the element that hosts it and the root they give, as a
//!    stream with a checksum of its own.
//! 2. `GroveDb::load_subtree(path, key, input, ..)` — writes the records of
//!    one dump into an empty subtree. It is a write of its transaction: it
//!    leaves with a rollback, and it writes no element.
//! 3. `GroveDb::subtree_root_as(path, key, element, ..)` — the root the
//!    records under `key` give when read as the subtree of `element`.
//! 4. `GroveDb::replace_subtree_root(...)` — patch the
//!    parent-Merk `Element::CommitmentTree` leaf with a caller-provided
//!    `combined_root`.
//!
//! The load, the cross-validation and the replacement share one transaction.
//! Given a transaction, an apply commits nothing: the records and the leaf
//! are committed together when the owner of the transaction commits it. An
//! apply that fails after its load leaves the loaded records in that
//! transaction, under the leaf of an empty tree, so its owner discards the
//! transaction and a retry starts a new one. Given no transaction, an apply
//! makes its writes in a transaction of its own, which it commits only where
//! every step before the commit succeeded; an error of the commit is the
//! error of the apply.
//!
//! See `docs/genesis-snapshot-design.md` for the full design + threat model.

#![allow(missing_docs)]

use std::io::{BufWriter, Read, Write};
use std::path::Path;

use dpp::version::PlatformVersion;
use drive::drive::shielded::paths::{shielded_credit_pool_path_vec, SHIELDED_NOTES_KEY};
use drive::grovedb::engine::Engine;
use drive::grovedb::operations::dump_load::DumpLoadError;
use drive::grovedb::{Element, GroveDb, Transaction, TransactionArg};
use drive::grovedb_path::SubtreePath;
use grovedb_commitment_tree::{CommitmentTree, DashMemo};

/// File magic — 8 bytes, matches the `b"DRVSHLD\0"` literal.
const MAGIC: [u8; 8] = *b"DRVSHLD\0";
/// Snapshot file format version. Bump on any breaking header/section change.
/// v2: parent-leaf flags encoded as `[present, byte]` (injective `Option<u8>`)
/// instead of a single byte that collapsed `None` and `Some(0)`.
/// v3: the body is a GroveDB subtree dump (`GroveDb::dump_subtree`) instead of
/// a RocksDB SST file. A v2 file is refused and has to be baked again.
const FORMAT_VERSION: u32 = 3;

/// Max permitted `chunk_power`. BulkAppendTree internally caps at 31; for
/// genesis snapshots we want a tighter sanity bound — anything > 16 would
/// imply a chunk holding ≥ 65 536 cmx values, which is far beyond anything
/// we'd ever ship.
const MAX_CHUNK_POWER: u8 = 16;

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
    /// Header's `combined_root` doesn't match the root the loaded records
    /// give: `computed` is the root GroveDB derives from them, or the one the
    /// commitment tree reads from them. Indicates tampering, truncation, or
    /// version skew.
    CombinedRootMismatch {
        expected: [u8; 32],
        computed: [u8; 32],
    },
    /// The element at the expected parent-leaf path/key is not
    /// `Element::CommitmentTree`. InitChain must build the parent skeleton
    /// before applying the snapshot.
    ParentLeafWrongType,
    /// Bubbled from GroveDB: a dump or a load it refused, or an error of the grove.
    GroveDb(String),
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
    /// Parent-leaf `Element::CommitmentTree` flags (`Option<ElementFlags>`
    /// where `ElementFlags = Vec<u8>`), narrowed to "absent, or exactly one
    /// byte" — the only shapes the shielded leaf carries in practice.
    ///
    /// Encoded as a presence discriminator byte plus the value byte so the
    /// round-trip is injective: `None` and `Some(0)` are distinct on the
    /// wire (a single `flags_byte` collapsed both to `0`, silently rewriting
    /// `Some(vec![0])` as `None` and diverging the parent-Merk serialization
    /// from a live-seeded tree even though `combined_root` still matched).
    /// The dumper rejects an empty or multi-byte flags vec loudly; widening
    /// past one byte requires bumping `FORMAT_VERSION`.
    pub flags: Option<u8>,
    pub combined_root: [u8; 32],
    pub body_len: u64,
}

impl SnapshotHeader {
    /// Wire size of the encoded header (NOT including the dump or the
    /// trailing checksum).
    // flags is encoded as 2 bytes: [present, byte].
    const ENCODED_LEN: usize = 8 + 4 + 8 + 1 + 2 + 32 + 8; // = 63 bytes

    fn write_to<W: Write>(&self, w: &mut W) -> Result<(), std::io::Error> {
        w.write_all(&MAGIC)?;
        w.write_all(&self.format_version.to_be_bytes())?;
        w.write_all(&self.total_count.to_be_bytes())?;
        w.write_all(&[self.chunk_power])?;
        // [present, byte]: present=1 means Some(byte); present=0 means None
        // (byte then ignored). Keeps None distinct from Some(0).
        let (present, byte) = match self.flags {
            Some(b) => (1u8, b),
            None => (0u8, 0u8),
        };
        w.write_all(&[present, byte])?;
        w.write_all(&self.combined_root)?;
        w.write_all(&self.body_len.to_be_bytes())?;
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
        let mut two = [0u8; 2];
        r.read_exact(&mut two)?;
        let flags = match two[0] {
            0 => None,
            1 => Some(two[1]),
            other => {
                return Err(ShieldedSnapshotError::Inconsistent(format!(
                    "invalid flags presence discriminator {other}; expected 0 (None) or 1 (Some)"
                )));
            }
        };
        let mut combined_root = [0u8; 32];
        r.read_exact(&mut combined_root)?;
        r.read_exact(&mut buf8)?;
        let body_len = u64::from_be_bytes(buf8);
        Ok(Self {
            format_version,
            total_count,
            chunk_power,
            flags,
            combined_root,
            body_len,
        })
    }
}

/// Stats returned by [`dump_shielded_subtree`].
#[derive(Debug, Clone)]
pub struct DumpStats {
    pub total_count: u64,
    pub key_count: u64,
    pub body_bytes: u64,
}

/// Stats returned by [`apply_shielded_snapshot`].
#[derive(Debug, Clone)]
pub struct ApplyStats {
    pub total_count: u64,
    pub combined_root: [u8; 32],
}

impl From<DumpLoadError> for ShieldedSnapshotError {
    fn from(e: DumpLoadError) -> Self {
        match e {
            DumpLoadError::Io(e) => Self::Io(e),
            DumpLoadError::Malformed(what) => {
                Self::Inconsistent(format!("snapshot body is not a subtree dump: {what}"))
            }
            other => Self::GroveDb(other.to_string()),
        }
    }
}

/// The parent-leaf values a snapshot header carries, read from the element
/// that hosts the shielded subtree.
fn header_values_of(element: &Element) -> Result<(u64, u8, Option<u8>), ShieldedSnapshotError> {
    let (total_count, chunk_power, flags) = match element {
        Element::CommitmentTree(tc, cp, f) => (*tc, *cp, f),
        _ => return Err(ShieldedSnapshotError::ParentLeafWrongType),
    };
    if chunk_power > MAX_CHUNK_POWER {
        return Err(ShieldedSnapshotError::ChunkPowerTooLarge {
            got: chunk_power,
            max: MAX_CHUNK_POWER,
        });
    }
    // The header encodes the parent-leaf flags as `Option<u8>` — absent, or
    // exactly one byte — preserving `None` vs `Some(0)` so a snapshot-booted
    // devnet reconstructs the identical parent-Merk leaf. Fail loudly on any
    // other shape (empty or multi-byte vec) rather than lossily folding it.
    let flags = match flags.as_deref() {
        None => None,
        Some([b]) => Some(*b),
        Some(v) => {
            return Err(ShieldedSnapshotError::Inconsistent(format!(
                "parent-leaf flags has {} bytes; snapshot format encodes None or exactly 1 byte — bump FORMAT_VERSION and widen the header",
                v.len()
            )));
        }
    };
    Ok((total_count, chunk_power, flags))
}

/// Dump the live shielded subtree into a snapshot file at `out_path`.
///
/// The header's `total_count`/`chunk_power`/`flags` come from the
/// `Element::CommitmentTree` leaf the dump carries, and `combined_root` is
/// the root GroveDB computed from the dumped records — that root is what the
/// apply side cross-validates against. With no transaction the dump reads one
/// committed state.
pub fn dump_shielded_subtree<'db>(
    grove: &'db GroveDb,
    transaction: TransactionArg<'db, '_>,
    out_path: &Path,
    platform_version: &PlatformVersion,
) -> Result<DumpStats, ShieldedSnapshotError> {
    let parent_segments = shielded_credit_pool_path_vec();
    let parent_path = SubtreePath::from(parent_segments.as_slice());

    // 1. Dump the subtree to a temporary file of this call's own, alongside
    //    the output path: the header needs the dump's length and root before
    //    the body follows it. Dropping `body_tmp` tries to remove the file,
    //    and an error of that removal is not reported.
    let tmp_dir = match out_path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    let body_tmp = tempfile::NamedTempFile::new_in(tmp_dir)?;
    let dumped = grove.dump_subtree(
        parent_path,
        &[SHIELDED_NOTES_KEY],
        BufWriter::new(body_tmp.as_file()),
        transaction,
        &platform_version.drive.grove_version,
    )?;
    let (total_count, chunk_power, flags) = header_values_of(&dumped.element)?;
    let body_bytes_on_disk = body_tmp.as_file().metadata()?.len();
    if body_bytes_on_disk != dumped.bytes {
        return Err(ShieldedSnapshotError::Inconsistent(format!(
            "the dump states {} bytes and its file holds {body_bytes_on_disk}",
            dumped.bytes
        )));
    }

    // 2. Compose the output file: header || dump || blake3 checksum.
    let header = SnapshotHeader {
        format_version: FORMAT_VERSION,
        total_count,
        chunk_power,
        flags,
        combined_root: dumped.root,
        body_len: body_bytes_on_disk,
    };

    let mut hasher = blake3::Hasher::new();
    let mut out = std::fs::File::create(out_path)?;
    let mut header_buf = Vec::with_capacity(SnapshotHeader::ENCODED_LEN);
    header.write_to(&mut header_buf)?;
    hasher.update(&header_buf);
    out.write_all(&header_buf)?;

    // Stream the dump in chunks to avoid loading the full
    // (potentially-huge) body into RAM.
    let mut body_file = body_tmp.reopen()?;
    let mut buf = vec![0u8; 256 * 1024];
    loop {
        let n = body_file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        out.write_all(&buf[..n])?;
    }

    let checksum = hasher.finalize();
    out.write_all(checksum.as_bytes())?;
    out.sync_all()?;

    Ok(DumpStats {
        total_count,
        key_count: dumped.data_records
            + dumped.aux_records
            + dumped.root_key_records
            + dumped.meta_records,
        body_bytes: body_bytes_on_disk,
    })
}

/// Apply a snapshot file produced by [`dump_shielded_subtree`] into a fresh
/// GroveDB. Intended to be called from `InitChain` AFTER the parent tree
/// skeleton (`/platform/shielded_pool/`) has been built but BEFORE the
/// shielded subtree would otherwise be populated by the runtime seeder.
///
/// Everything the apply writes is a write of `transaction`: it is committed
/// with that transaction and leaves with its rollback, so an `InitChain`
/// that is retried after a rolled-back attempt finds the subtree empty again.
/// After an error the transaction may hold a part of the apply's writes and
/// is rolled back by the caller. With no transaction the apply runs in one of
/// its own and commits it at the end.
///
/// A shielded subtree that already holds records is refused: the snapshot is
/// only ever applied to an empty pool.
pub fn apply_shielded_snapshot<'db>(
    grove: &'db GroveDb,
    transaction: TransactionArg<'db, '_>,
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
    let body_end = usize::try_from(header.body_len)
        .ok()
        .and_then(|len| body_start.checked_add(len))
        .filter(|end| end.checked_add(32) == Some(bytes.len()))
        .ok_or_else(|| {
            ShieldedSnapshotError::Inconsistent(format!(
                "header says body_len={}, which is not what a file of {} bytes holds",
                header.body_len,
                bytes.len()
            ))
        })?;
    let body = &bytes[body_start..body_end];

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

    // 3. Load, cross-validate and patch the parent leaf in one transaction.
    match transaction {
        Some(transaction) => {
            apply_in_transaction(grove, transaction, &header, body, platform_version)
        }
        None => {
            let transaction = grove.start_transaction();
            let stats = apply_in_transaction(grove, &transaction, &header, body, platform_version)?;
            grove
                .commit_transaction(transaction)
                .value
                .map_err(|e| ShieldedSnapshotError::GroveDb(format!("commit: {e}")))?;
            Ok(stats)
        }
    }
}

/// The three writes and reads of an apply, all through `transaction`.
fn apply_in_transaction<'db>(
    grove: &'db GroveDb,
    transaction: &Transaction<'db>,
    header: &SnapshotHeader,
    body: &[u8],
    platform_version: &PlatformVersion,
) -> Result<ApplyStats, ShieldedSnapshotError> {
    let grove_version = &platform_version.drive.grove_version;
    let parent_segments = shielded_credit_pool_path_vec();
    let parent_path = || SubtreePath::from(parent_segments.as_slice());
    let leaf_key = &[SHIELDED_NOTES_KEY];

    // a. Load the dump into the subtree. The load refuses a subtree that
    //    holds any record, and bytes that follow the dump are not part of it.
    let mut input = body;
    let loaded = grove.load_subtree(
        parent_path(),
        leaf_key,
        &mut input,
        Some(transaction),
        grove_version,
    )?;
    if !input.is_empty() {
        return Err(ShieldedSnapshotError::Inconsistent(format!(
            "{} bytes follow the dump inside the snapshot body",
            input.len()
        )));
    }

    // b. The header and the dump must describe the same leaf.
    let expected_element = Element::new_commitment_tree(
        header.total_count,
        header.chunk_power,
        header.flags.map(|b| vec![b]),
    );
    if loaded.element != expected_element {
        return Err(ShieldedSnapshotError::Inconsistent(
            "the header's parent leaf is not the one the dump carries".to_string(),
        ));
    }

    if loaded.root != header.combined_root {
        return Err(ShieldedSnapshotError::Inconsistent(format!(
            "the dump states the root {} and the header {}",
            hex::encode(loaded.root),
            hex::encode(header.combined_root)
        )));
    }

    // c. Cross-validate: the root the loaded records give must be the
    //    header's, by GroveDB's derivation from the records and by the
    //    commitment tree's ordinary versioned read of them. Both are checked
    //    before the parent Merk is changed.
    let derived = grove
        .subtree_root_as(
            parent_path(),
            leaf_key,
            &loaded.element,
            Some(transaction),
            grove_version,
        )
        .map_err(|e| ShieldedSnapshotError::GroveDb(format!("subtree_root_as: {e}")))?;
    let subtree_segments: Vec<&[u8]> = parent_segments
        .iter()
        .map(Vec::as_slice)
        .chain(std::iter::once(leaf_key.as_slice()))
        .collect();
    let read = {
        let records = grove
            .engine()
            .subtree(
                SubtreePath::from(subtree_segments.as_slice()),
                None,
                transaction,
            )
            .unwrap();
        CommitmentTree::<_, DashMemo>::open(
            header.total_count,
            header.chunk_power,
            records,
            grove_version,
        )
        .value
        .map_err(|e| ShieldedSnapshotError::GroveDb(format!("CommitmentTree::open: {e}")))?
        .compute_current_state_root(grove_version)
        .map_err(|e| ShieldedSnapshotError::GroveDb(format!("compute_current_state_root: {e}")))?
    };
    for computed in [derived, read] {
        if computed != header.combined_root {
            return Err(ShieldedSnapshotError::CombinedRootMismatch {
                expected: header.combined_root,
                computed,
            });
        }
    }

    // d. Patch parent Merk leaf.
    grove
        .replace_subtree_root(
            parent_path(),
            leaf_key,
            loaded.element,
            header.combined_root,
            Some(transaction),
            grove_version,
        )
        .value
        .map_err(|e| ShieldedSnapshotError::GroveDb(format!("replace_subtree_root: {e}")))?;

    Ok(ApplyStats {
        total_count: header.total_count,
        combined_root: header.combined_root,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Encode a header with the given parent-leaf flags, decode it back, and
    /// return the decoded `flags` so a test can assert the round-trip.
    fn roundtrip_flags(flags: Option<u8>) -> Option<u8> {
        let header = SnapshotHeader {
            format_version: FORMAT_VERSION,
            total_count: 7,
            chunk_power: 11,
            flags,
            combined_root: [0xAB; 32],
            body_len: 123,
        };
        let mut buf = Vec::with_capacity(SnapshotHeader::ENCODED_LEN);
        header.write_to(&mut buf).expect("write_to");
        assert_eq!(
            buf.len(),
            SnapshotHeader::ENCODED_LEN,
            "encoded header length must match ENCODED_LEN"
        );
        let mut cursor = std::io::Cursor::new(buf);
        SnapshotHeader::read_from(&mut cursor)
            .expect("read_from")
            .flags
    }

    /// The flags round-trip must be injective: `None` and `Some(0)` are
    /// distinct on the wire. A prior single-byte encoding collapsed both to
    /// `0`/`None`, silently rewriting `Some(vec![0])` as `None` on apply.
    #[test]
    fn flags_roundtrip_is_injective() {
        assert_eq!(roundtrip_flags(None), None, "None must stay None");
        assert_eq!(
            roundtrip_flags(Some(0)),
            Some(0),
            "Some(0) must stay Some(0), not collapse to None"
        );
        assert_eq!(roundtrip_flags(Some(0xAB)), Some(0xAB));
        // The two cases that previously aliased are now distinguishable.
        assert_ne!(roundtrip_flags(None), roundtrip_flags(Some(0)));
    }

    /// An invalid presence discriminator (anything but 0 or 1) is rejected.
    #[test]
    fn flags_invalid_presence_discriminator_rejected() {
        let header = SnapshotHeader {
            format_version: FORMAT_VERSION,
            total_count: 0,
            chunk_power: 0,
            flags: None,
            combined_root: [0u8; 32],
            body_len: 0,
        };
        let mut buf = Vec::new();
        header.write_to(&mut buf).expect("write_to");
        // Corrupt the presence byte (immediately after MAGIC + version +
        // total_count + chunk_power = 8 + 4 + 8 + 1 = offset 21).
        buf[21] = 2;
        let mut cursor = std::io::Cursor::new(buf);
        let err = SnapshotHeader::read_from(&mut cursor).unwrap_err();
        assert!(
            matches!(err, ShieldedSnapshotError::Inconsistent(_)),
            "expected Inconsistent for bad presence discriminator, got {err:?}"
        );
    }
}
