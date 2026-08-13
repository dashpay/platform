//! Branch mechanics for `IN`-pinned ranked / having-range queries —
//! shared by the executors (walk one secondary per branch, merge) and
//! the verifiers (verify one proof per branch, re-merge), so both sides
//! implement one comparator and one proof-container layout.
//!
//! ## Why a merge needs no proof of its own
//!
//! Each branch's walk is independently proved complete against the same
//! root hash, and the merged page is a *deterministic function* of the
//! branch pages: any union entry that precedes a returned entry in the
//! merge order is preceded, within its own branch, by fewer than `limit`
//! entries — so it is in its branch's returned page. The union's first
//! `limit` entries are therefore contained in the union of the branch
//! pages, and re-merging verified branch pages reconstructs a complete,
//! correctly ordered page. (This is the same argument for both surfaces:
//! "first `limit` in merge order restricted to the branch" is top-k for
//! ranked and the in-bound range prefix for having.)
//!
//! ## The merge order
//!
//! `(aggregate in walk direction, branch segment ascending, group key in
//! walk direction)`. The middle term is the canonical branch tie-break:
//! the **encoded** segment bytes of the branch's `IN` position, fixed
//! ascending regardless of walk direction, independent of the caller's
//! element order — which also makes `null` (the empty segment) sort
//! first deterministically.

use super::{RankedEntry, RankedEntryValue};
use crate::error::drive::DriveError;
use crate::error::Error;
use std::cmp::Ordering;

/// Version byte of the branch-proof container. Bumped only with a
/// method-version bump on the prove/verify pair — the container is part
/// of the prover/verifier agreement, not a transport detail.
const BRANCH_PROOF_CONTAINER_VERSION: u8 = 1;

/// The position (index into a branch's segment list) at which the
/// branches differ — the `IN` pin's position in the covering index's
/// leading properties. `None` when there is a single branch (no `IN`),
/// or when branches are degenerate duplicates (the encoder rejects
/// those, so it is unreachable off the resolver path).
pub fn varying_position(branches: &[Vec<Vec<u8>>]) -> Option<usize> {
    let first = branches.first()?;
    for other in &branches[1..] {
        for (position, (a, b)) in first.iter().zip(other.iter()).enumerate() {
            if a != b {
                return Some(position);
            }
        }
    }
    None
}

/// The `in_key` for one branch: the encoded segment at the varying
/// position. `None` for single-branch queries — entries of an un-branched
/// response carry no discriminator, keeping the shape byte-identical to
/// the pre-`IN` surface.
pub fn branch_in_key(branches: &[Vec<Vec<u8>>], branch: usize) -> Option<Vec<u8>> {
    if branches.len() < 2 {
        return None;
    }
    let position = varying_position(branches)?;
    branches.get(branch)?.get(position).cloned()
}

/// Compare two entries' aggregates on the same axis. The executors and
/// verifiers only ever merge entries of one axis, so a variant mismatch
/// is corrupted state, reported rather than ordered.
fn aggregate_cmp(a: &RankedEntryValue, b: &RankedEntryValue) -> Result<Ordering, Error> {
    match (a, b) {
        (RankedEntryValue::Count(a), RankedEntryValue::Count(b)) => Ok(a.cmp(b)),
        (RankedEntryValue::Sum(a), RankedEntryValue::Sum(b)) => Ok(a.cmp(b)),
        (RankedEntryValue::AvgFixedPoint(a), RankedEntryValue::AvgFixedPoint(b)) => Ok(a.cmp(b)),
        _ => Err(Error::Drive(DriveError::CorruptedDriveState(
            "branch merge compared entries from different aggregate axes".to_string(),
        ))),
    }
}

/// Merge per-branch pages into the final page: tag each entry with its
/// branch's `in_key`, order by the merge comparator, cut at `limit`.
///
/// `per_branch` must be indexed identically to `branches` (the resolver
/// produces both in canonical branch order). Branch pages arrive sorted
/// by the walk; the merged set is small (≤ branches × limit), so a
/// plain total sort is used instead of a k-way heap — simpler to keep
/// byte-identical between server and verifier.
pub fn merge_branch_pages(
    per_branch: Vec<Vec<RankedEntry>>,
    branches: &[Vec<Vec<u8>>],
    descending: bool,
    limit: usize,
) -> Result<Vec<RankedEntry>, Error> {
    let mut merged: Vec<RankedEntry> = Vec::with_capacity(per_branch.iter().map(Vec::len).sum());
    for (branch, entries) in per_branch.into_iter().enumerate() {
        let in_key = branch_in_key(branches, branch);
        merged.extend(entries.into_iter().map(|mut entry| {
            entry.in_key = in_key.clone();
            entry
        }));
    }
    let mut comparison_error: Option<Error> = None;
    merged.sort_by(|a, b| {
        let aggregate = match aggregate_cmp(&a.value, &b.value) {
            Ok(ordering) => ordering,
            Err(e) => {
                comparison_error.get_or_insert(e);
                Ordering::Equal
            }
        };
        let directional = |ordering: Ordering| {
            if descending {
                ordering.reverse()
            } else {
                ordering
            }
        };
        directional(aggregate)
            .then_with(|| a.in_key.cmp(&b.in_key))
            .then_with(|| directional(a.key.cmp(&b.key)))
    });
    if let Some(e) = comparison_error {
        return Err(e);
    }
    merged.truncate(limit);
    Ok(merged)
}

/// Frame per-branch grovedb proofs into the single opaque byte string
/// the wire's `Proof` carries: version byte, `u16` branch count, then
/// each proof length-prefixed with a `u32` (all big-endian). Used only
/// when there are two or more branches — a single-branch proof stays
/// the raw grovedb envelope, byte-identical to the pre-`IN` surface.
pub fn encode_branch_proofs(proofs: &[Vec<u8>]) -> Vec<u8> {
    let mut out = Vec::with_capacity(3 + proofs.iter().map(|p| 4 + p.len()).sum::<usize>());
    out.push(BRANCH_PROOF_CONTAINER_VERSION);
    out.extend_from_slice(&(proofs.len() as u16).to_be_bytes());
    for proof in proofs {
        out.extend_from_slice(&(proof.len() as u32).to_be_bytes());
        out.extend_from_slice(proof);
    }
    out
}

/// Parse the branch-proof container, requiring exactly
/// `expected_branches` proofs — the verifier derives that count from
/// its own resolution of the request, so a server cannot drop or
/// duplicate a branch without the container failing to parse. Trailing
/// bytes are rejected: an envelope is exactly its declared content.
pub fn decode_branch_proofs(bytes: &[u8], expected_branches: usize) -> Result<Vec<Vec<u8>>, Error> {
    let malformed = |what: &str| {
        Error::Drive(DriveError::CorruptedDriveState(format!(
            "branch-proof container: {what}"
        )))
    };
    let (&version, mut rest) = bytes.split_first().ok_or_else(|| malformed("empty"))?;
    if version != BRANCH_PROOF_CONTAINER_VERSION {
        return Err(malformed(&format!(
            "unknown container version {version}; expected {BRANCH_PROOF_CONTAINER_VERSION}"
        )));
    }
    if rest.len() < 2 {
        return Err(malformed("truncated branch count"));
    }
    let (count_bytes, tail) = rest.split_at(2);
    let count = u16::from_be_bytes([count_bytes[0], count_bytes[1]]) as usize;
    rest = tail;
    if count != expected_branches {
        return Err(malformed(&format!(
            "container carries {count} branch proofs; the request resolves to \
             {expected_branches} branches"
        )));
    }
    let mut proofs = Vec::with_capacity(count);
    for _ in 0..count {
        if rest.len() < 4 {
            return Err(malformed("truncated proof length"));
        }
        let (len_bytes, tail) = rest.split_at(4);
        let len =
            u32::from_be_bytes([len_bytes[0], len_bytes[1], len_bytes[2], len_bytes[3]]) as usize;
        if tail.len() < len {
            return Err(malformed("truncated proof body"));
        }
        let (proof, tail) = tail.split_at(len);
        proofs.push(proof.to_vec());
        rest = tail;
    }
    if !rest.is_empty() {
        return Err(malformed("trailing bytes after the declared proofs"));
    }
    Ok(proofs)
}
