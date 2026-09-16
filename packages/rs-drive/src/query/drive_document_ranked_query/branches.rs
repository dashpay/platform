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
use grovedb::{
    query_result_type::QueryResultType, AxisKeys, GroveDb, PathQuery, PathQueryRun, TransactionArg,
};
#[cfg(feature = "server")]
use grovedb_costs::CostContext;
#[cfg(feature = "server")]
use grovedb_query::AxisQuery;
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

/// Translate one branch's keys-only [`AxisKeys`] page into drive entries
/// on the requested axis — the unproved twin of [`axis_entries_to_ranked`],
/// used by the single-snapshot branched reads.
#[cfg(feature = "server")]
pub(crate) fn axis_keys_to_ranked(
    axis: RankedAxis,
    keys: AxisKeys,
) -> Result<Vec<RankedEntry>, Error> {
    match (axis, keys) {
        (RankedAxis::Count, AxisKeys::Count(pairs)) => Ok(pairs
            .into_iter()
            .map(|(count, key)| RankedEntry {
                in_key: None,
                key,
                value: RankedEntryValue::Count(count),
            })
            .collect()),
        (RankedAxis::Sum, AxisKeys::Sum(pairs)) => Ok(pairs
            .into_iter()
            .map(|(sum, key)| RankedEntry {
                in_key: None,
                key,
                value: RankedEntryValue::Sum(sum),
            })
            .collect()),
        (RankedAxis::Avg, AxisKeys::Avg(pairs)) => Ok(pairs
            .into_iter()
            .map(|(avg, key)| RankedEntry {
                in_key: None,
                key,
                value: RankedEntryValue::AvgFixedPoint(avg),
            })
            .collect()),
        (axis, _) => Err(Error::Drive(DriveError::CorruptedDriveState(format!(
            "a branch of a {axis:?} read returned keys of a different axis shape"
        )))),
    }
}

/// The entire unproved branched-read sequence, shared by the ranked and
/// having-range executors so the read/prove contract has exactly ONE
/// implementation: decompose the branch paths, run one branched
/// keys-only grovedb call, validate the run shape and the branch-set
/// identity, translate and cap each branch's page, and merge with the
/// shared comparator.
///
/// The union is served from **one committed state**: the call always
/// runs under a grovedb snapshot read transaction taken here, so every
/// per-branch absence probe and axis walk inside grovedb's branched arm
/// reads the same RocksDB snapshot and a block commit landing mid-read
/// cannot mix committed states into the merged page.
///
/// A caller-supplied transaction is **rejected**, mirroring the
/// branched provers: an ordinary grovedb transaction reads the latest
/// committed state on every operation, so forwarding it would reopen
/// the exact tear the snapshot closes — and there is no way to tell an
/// ordinary transaction from a snapshot-pinned one at this boundary.
/// Transactional callers read per prefix element (each single-branch
/// read is one grovedb operation and honors the transaction exactly),
/// or commit first.
///
/// `page_cap` is the surface's page bound (`k` for ranked, `limit` for
/// having-range): each branch may return at most that many entries and
/// the merged union is cut at it. `surface` names the caller in the
/// corrupted-state messages.
#[cfg(feature = "server")]
#[allow(clippy::too_many_arguments)]
pub(crate) fn read_branched_union(
    grove: &GroveDb,
    surface: &'static str,
    prefix_branches: &[Vec<Vec<u8>>],
    paths: &[Vec<Vec<u8>>],
    axis: RankedAxis,
    axis_query: AxisQuery,
    page_cap: usize,
    descending: bool,
    transaction: TransactionArg,
    grove_version: &GroveVersion,
) -> Result<Vec<RankedEntry>, Error> {
    if transaction.is_some() {
        return Err(Error::Drive(DriveError::NotSupported(
            "an IN-pinned (branched) unproved read under a caller transaction is not \
             supported: an ordinary grovedb transaction reads the latest committed state on \
             every operation, so a concurrent commit could tear the union across branches — \
             read per prefix element under the transaction, or pass no transaction (the read \
             then runs under an internal snapshot)",
        )));
    }
    let (prefix, keys, suffix) = decompose_branch_paths(paths)?;
    let path_query =
        PathQuery::new_branched_axis(prefix, keys.clone(), suffix, axis_query.keys_only());
    let snapshot_transaction = grove.start_snapshot_read_transaction();
    // Test-only seam: lets a regression test land a commit deterministically
    // INSIDE the window — after the snapshot is taken, before the read runs —
    // proving the automatic snapshot selection on the production `None` path.
    #[cfg(test)]
    test_hooks::AFTER_BRANCHED_SNAPSHOT.with(|hook| {
        if let Some(hook) = hook.borrow_mut().as_mut() {
            hook();
        }
    });
    let CostContext { value, cost: _ } = grove.run_path_query(
        &path_query,
        true,
        true,
        true,
        QueryResultType::QueryKeyElementPairResultType,
        Some(&snapshot_transaction),
        grove_version,
    );
    let run = value.map_err(|e| Error::GroveDB(Box::new(e)))?;
    let PathQueryRun::BranchedAxisKeys(branches) = run else {
        return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
            "a branched keys-only {surface} read returned a non-branched shape"
        ))));
    };
    if branches.len() != keys.len() || branches.iter().map(|(key, _)| key).ne(keys.iter()) {
        return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
            "a branched {surface} read returned a different branch set than the request resolved"
        ))));
    }
    let per_branch = branches
        .into_iter()
        .map(|(_key, page)| {
            let entries = match page {
                None => Vec::new(),
                Some(page) => axis_keys_to_ranked(axis, page)?,
            };
            if entries.len() > page_cap {
                return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                    "a branch of a {surface} read returned {} entries for a page cap of \
                     {page_cap}",
                    entries.len(),
                ))));
            }
            Ok(entries)
        })
        .collect::<Result<Vec<_>, Error>>()?;
    merge_branch_pages(per_branch, prefix_branches, descending, page_cap)
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

/// Test-only seam for [`read_branched_union`]: invoked after the
/// internal snapshot transaction is taken and before the branched call
/// runs, so a test can land a commit deterministically inside the
/// window and prove the production `None` path's automatic snapshot
/// selection end-to-end. Last in the file: clippy's
/// `items_after_test_module` forbids items after a `#[cfg(test)]`
/// module.
#[cfg(test)]
pub(crate) mod test_hooks {
    use std::cell::RefCell;
    thread_local! {
        pub(crate) static AFTER_BRANCHED_SNAPSHOT: RefCell<Option<Box<dyn FnMut()>>> =
            const { RefCell::new(None) };
    }
}
