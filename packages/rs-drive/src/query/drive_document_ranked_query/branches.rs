//! Branch mechanics for `IN`-pinned ranked / having-range queries —
//! shared by the executors (walk one secondary per branch, merge) and
//! the verifiers (verify one branched envelope, re-merge), so both
//! sides implement one comparator and one grove-path decomposition.
//!
//! ## Why a merge needs no proof of its own
//!
//! Each branch's walk is proved complete inside one grovedb **branched
//! envelope** (shared ancestor layers once, one multi-key proof at the
//! branching level, one secondary proof per branch, one root hash),
//! and the merged page is a *deterministic function* of the branch
//! pages: any union entry that precedes a returned entry in the merge
//! order is preceded, within its own branch, by fewer than `limit`
//! entries — so it is in its branch's returned page. The union's first
//! `limit` entries are therefore contained in the union of the branch
//! pages, and re-merging verified branch pages reconstructs a
//! complete, correctly ordered page. (This is the same argument for
//! both surfaces: "first `limit` in merge order restricted to the
//! branch" is top-k for ranked and the in-bound range prefix for
//! having.)
//!
//! ## The merge order
//!
//! `(aggregate in walk direction, branch segment ascending, group key in
//! walk direction)`. The middle term is the canonical branch tie-break:
//! the **encoded** segment bytes of the branch's `IN` position, fixed
//! ascending regardless of walk direction, independent of the caller's
//! element order — which also makes `null` (the empty segment) sort
//! first deterministically.

use super::{RankedAxis, RankedEntry, RankedEntryValue};
use crate::error::drive::DriveError;
use crate::error::Error;
use grovedb::operations::proof::indexed_axis::AxisEntries;
#[cfg(feature = "server")]
use grovedb::{GroveDb, TransactionArg};
#[cfg(feature = "server")]
use grovedb_costs::CostContext;
#[cfg(feature = "server")]
use grovedb_version::version::GroveVersion;
use std::cmp::Ordering;

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

/// The `(shared prefix, branch keys, shared suffix)` decomposition of a
/// branch path set — the triple `PathQuery::new_branched_axis` takes.
pub type BranchPathDecomposition = (Vec<Vec<u8>>, Vec<Vec<u8>>, Vec<Vec<u8>>);

/// Decompose per-branch grove paths into the `(shared prefix, branch
/// keys, shared suffix)` triple grovedb's branched proof primitives
/// take. The paths differ at exactly one segment position by
/// construction (one `IN` pin); anything else is an internal
/// resolution error.
pub fn decompose_branch_paths(paths: &[Vec<Vec<u8>>]) -> Result<BranchPathDecomposition, Error> {
    let first = paths.first().ok_or_else(|| {
        Error::Drive(DriveError::CorruptedDriveState(
            "branch decomposition over zero paths".to_string(),
        ))
    })?;
    let mut varying: Option<usize> = None;
    for other in &paths[1..] {
        if other.len() != first.len() {
            return Err(Error::Drive(DriveError::CorruptedDriveState(
                "branch paths of different lengths".to_string(),
            )));
        }
        for (position, (a, b)) in first.iter().zip(other.iter()).enumerate() {
            if a != b {
                match varying {
                    None => varying = Some(position),
                    Some(existing) if existing == position => {}
                    Some(_) => {
                        return Err(Error::Drive(DriveError::CorruptedDriveState(
                            "branch paths differ at more than one segment".to_string(),
                        )));
                    }
                }
            }
        }
    }
    let position = varying.ok_or_else(|| {
        Error::Drive(DriveError::CorruptedDriveState(
            "branch paths are identical; the encoder rejects duplicate branches".to_string(),
        ))
    })?;
    let prefix = first[..position].to_vec();
    let keys = paths
        .iter()
        .map(|path| path[position].clone())
        .collect::<Vec<_>>();
    let suffix = first[position + 1..].to_vec();
    Ok((prefix, keys, suffix))
}

/// Whether any level of one branch's path below the shared prefix is
/// absent — the branch key itself, or any shared-suffix segment under it.
///
/// grovedb's branched reader and prover treat a missing segment at ANY
/// depth as the authenticated absence of the whole branch, so the
/// unproved executors must agree: an `IN` element whose value tree exists
/// (because some other pin value was written under it) but whose deeper
/// pinned path was not is an absent branch contributing an empty page —
/// not an error that discards the other branches' results.
///
/// Levels are probed top-down, so each parent is known present before
/// its child is read; the shared prefix itself (the contract and
/// document-type levels) is the caller's responsibility and is not
/// probed here.
#[cfg(feature = "server")]
pub(crate) fn branch_subpath_is_absent(
    grove: &GroveDb,
    full_path: &[Vec<u8>],
    shared_prefix_len: usize,
    transaction: TransactionArg,
    grove_version: &GroveVersion,
) -> Result<bool, Error> {
    for depth in shared_prefix_len..full_path.len() {
        let parent: Vec<&[u8]> = full_path[..depth].iter().map(|s| s.as_slice()).collect();
        let CostContext { value, cost: _ } = grove.get_raw_optional(
            parent.as_slice().into(),
            &full_path[depth],
            transaction,
            grove_version,
        );
        if value.map_err(|e| Error::GroveDB(Box::new(e)))?.is_none() {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Translate one branch's verified [`AxisEntries`] into drive entries
/// on the requested axis — the same mapping the single-path verifiers
/// perform, shared here so both surfaces' branched verifiers agree.
pub fn axis_entries_to_ranked(
    axis: RankedAxis,
    entries: AxisEntries,
) -> Result<Vec<RankedEntry>, Error> {
    match (axis, entries) {
        (RankedAxis::Count, AxisEntries::Count(entries)) => Ok(entries
            .into_iter()
            .map(|entry| entry.key_pair())
            .map(|(count, key)| RankedEntry {
                in_key: None,
                key,
                value: RankedEntryValue::Count(count),
            })
            .collect()),
        (RankedAxis::Sum, AxisEntries::Sum(entries)) => Ok(entries
            .into_iter()
            .map(|entry| entry.key_pair())
            .map(|(sum, key)| RankedEntry {
                in_key: None,
                key,
                value: RankedEntryValue::Sum(sum),
            })
            .collect()),
        (RankedAxis::Avg, AxisEntries::Avg(entries)) => Ok(entries
            .into_iter()
            .map(|entry| entry.key_pair())
            .map(|(avg, key)| RankedEntry {
                in_key: None,
                key,
                value: RankedEntryValue::AvgFixedPoint(avg),
            })
            .collect()),
        (axis, other) => Err(Error::Drive(DriveError::CorruptedDriveState(format!(
            "a branch of a {axis:?} proof verified to {} entries of a different axis shape",
            other.len()
        )))),
    }
}
