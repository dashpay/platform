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
    /// Rebuilds the proved subtree path with
    /// [`Self::indexed_property_name_tree_path`] and the secondary query
    /// with
    /// [`AxisRangeBounds::merk_query`](crate::query::drive_document_having_query::AxisRangeBounds::merk_query),
    /// then hands the proof to the matching
    /// `GroveDb::verify_indexed_*_query` — an associated function, no
    /// database handle, so this compiles and runs in a verifier-only
    /// build.
    ///
    /// Three things are checked before the entries are returned:
    ///
    /// 1. **The envelope matches this query.** grovedb re-checks the
    ///    proof against the reconstructed Merk query (the encoded bounds
    ///    and walk direction) and the expected limit, so a proof
    ///    generated for a different bound — or a different direction, or
    ///    a different limit — is rejected rather than silently
    ///    reinterpreted. Completeness rides on the same check: a Merk
    ///    range proof commits its boundaries, so an in-range group the
    ///    prover omitted fails reconstruction.
    /// 2. **The result's axis shape matches the requested axis** — the
    ///    same belt-and-braces check the ranked verifier does.
    /// 3. **At most `limit` entries.** Fewer is normal — fewer groups
    ///    may match the bound — but more would mean the proof committed
    ///    a longer walk than the request authorized.
    ///
    /// No `platform_version` argument: the parent dispatcher already
    /// consumed it to select this version, and verification derives
    /// everything else from the proof bytes plus the query.
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
