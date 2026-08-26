use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::drive_document_ranked_query::branches::{
    axis_entries_to_ranked, decompose_branch_paths, merge_branch_pages,
};
use crate::query::{DriveDocumentHavingQuery, RankedEntry};
use crate::verify::RootHash;
use dpp::version::PlatformVersion;
use grovedb::operations::proof::VerifiedPathQuery;
use grovedb::GroveDb;
use grovedb::PathQuery;
use grovedb_query::AxisQuery;

impl DriveDocumentHavingQuery<'_> {
    /// v0 of [`Self::verify_having_range_proof`].
    ///
    /// Rebuilds the same `PathQuery` the prover built — the subtree
    /// path via [`Self::indexed_property_name_tree_path`], the
    /// `Bounded` traversal from the request's inclusive bounds
    /// ([`AxisRangeBounds::inclusive_bounds_i128`](crate::query::drive_document_having_query::AxisRangeBounds::inclusive_bounds_i128)),
    /// limit and direction — and hands the proof to
    /// [`GroveDb::verify_path_query`], an associated function: no
    /// database handle, so this compiles and runs in a verifier-only
    /// build.
    ///
    /// Three things are checked before the entries are returned:
    ///
    /// 1. **The proof covers this query's own traversal.** Binding is by
    ///    RECONSTRUCTION, not echo comparison: the verifier re-executes
    ///    the proof against the bounds, direction and limit it rebuilt
    ///    from the request, so a proof for a different bound or
    ///    direction fails to cover it. The limit binds as a CAP under
    ///    re-execution — an exhausted-walk proof is a complete answer
    ///    under any admitting cap (sound, and pinned by the
    ///    limit-tamper test), while a proof truncated by a smaller limit
    ///    fails a larger cap for missing coverage of the rest of the
    ///    bound. Completeness rides on the Merk range proof itself: its
    ///    boundary commitments show no in-range group was omitted.
    /// 2. **The result's axis shape matches the requested axis** — the
    ///    same belt-and-braces check the ranked verifier does.
    /// 3. **At most `limit` entries.** Fewer is normal — fewer groups
    ///    may match the bound — but more would mean the proof committed
    ///    a longer walk than the request authorized.
    #[inline(always)]
    pub(super) fn verify_having_range_proof_v0(
        &self,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<RankedEntry>), Error> {
        if self.prefix_branches.len() > 1 {
            // `IN`-pinned request: one grovedb branched envelope — same
            // discipline as the ranked verifier (branch set from this
            // query's own resolution, tails bound to keys by the
            // branching-level proof, one root hash, page re-derived by
            // the shared merge).
            let paths = (0..self.prefix_branches.len())
                .map(|branch| self.indexed_property_name_tree_path(branch))
                .collect::<Result<Vec<_>, Error>>()?;
            let (prefix, keys, suffix) = decompose_branch_paths(&paths)?;
            let axis = self.bounds.axis();
            let (lo, hi) = self.bounds.inclusive_bounds_i128();
            let path_query = PathQuery::new_branched_axis(
                prefix,
                keys.clone(),
                suffix,
                AxisQuery::bounded(axis.into(), lo, hi, self.limit, self.descending),
            );
            let verified = GroveDb::verify_path_query(
                proof,
                &path_query,
                &platform_version.drive.grove_version,
            )
            .map_err(|e| Error::GroveDB(Box::new(e)))?;
            let VerifiedPathQuery::BranchedAxisEntries {
                root_hash,
                branches,
            } = verified
            else {
                return Err(Error::Drive(DriveError::CorruptedDriveState(
                    "a branched having range proof verified to a non-branched shape".to_string(),
                )));
            };
            if branches.len() != keys.len() || branches.iter().map(|(key, _)| key).ne(keys.iter()) {
                return Err(Error::Drive(DriveError::CorruptedDriveState(
                    "a branched having range proof verified a different branch set than the \
                     request resolved"
                        .to_string(),
                )));
            }
            let per_branch = branches
                .into_iter()
                .map(|(_key, entries)| {
                    // Authenticated absence of a branch key = an empty page,
                    // the same reading the unproved path gives an absent
                    // `IN` element.
                    let entries = match entries {
                        None => Vec::new(),
                        Some(entries) => axis_entries_to_ranked(axis, entries)?,
                    };
                    if entries.len() > self.limit as usize {
                        return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                            "a branch of a having range proof verified to {} entries for \
                             limit = {}",
                            entries.len(),
                            self.limit
                        ))));
                    }
                    Ok(entries)
                })
                .collect::<Result<Vec<_>, Error>>()?;
            let entries = merge_branch_pages(
                per_branch,
                &self.prefix_branches,
                self.descending,
                self.limit as usize,
            )?;
            return Ok((root_hash, entries));
        }
        self.verify_having_range_proof_v0_branch(0, proof, platform_version)
    }

    /// One branch's verification — the entire pre-`IN` verifier,
    /// parameterized by the prefix branch.
    fn verify_having_range_proof_v0_branch(
        &self,
        branch: usize,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<RankedEntry>), Error> {
        let path = self.indexed_property_name_tree_path(branch)?;
        let axis = self.bounds.axis();
        let (lo, hi) = self.bounds.inclusive_bounds_i128();
        let path_query =
            PathQuery::new_axis_bounded(path, axis.into(), lo, hi, self.limit, self.descending);
        let verified =
            GroveDb::verify_path_query(proof, &path_query, &platform_version.drive.grove_version)
                .map_err(|e| Error::GroveDB(Box::new(e)))?;
        let VerifiedPathQuery::AxisEntries {
            root_hash,
            entries,
            skipped: _,
        } = verified
        else {
            return Err(Error::Drive(DriveError::CorruptedDriveState(
                "a having range proof verified to a different shape".to_string(),
            )));
        };
        let entries = axis_entries_to_ranked(axis, entries)?;
        if entries.len() > self.limit as usize {
            return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                "having range proof verified to {} entries for limit = {}",
                entries.len(),
                self.limit
            ))));
        }
        Ok((root_hash, entries))
    }
}
