use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::drive_document_ranked_query::branches::{
    decode_branch_proofs, merge_branch_pages,
};
use crate::query::{DriveDocumentHavingQuery, RankedAxis, RankedEntry, RankedEntryValue};
use crate::verify::RootHash;
use dpp::version::PlatformVersion;
use grovedb::operations::proof::indexed_axis::AxisEntries;
use grovedb::GroveDb;

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
            // `IN`-pinned request: same branch-container discipline as
            // the ranked verifier — count from this query's own
            // resolution, one root hash across branches, page re-derived
            // by the shared merge.
            let branch_proofs = decode_branch_proofs(proof, self.prefix_branches.len())?;
            let mut root_hash: Option<RootHash> = None;
            let mut per_branch = Vec::with_capacity(branch_proofs.len());
            for (branch, branch_proof) in branch_proofs.iter().enumerate() {
                let (branch_root, entries) =
                    self.verify_having_range_proof_v0_branch(branch, branch_proof)?;
                match root_hash {
                    None => root_hash = Some(branch_root),
                    Some(existing) if existing == branch_root => {}
                    Some(_) => {
                        return Err(Error::Drive(DriveError::CorruptedDriveState(
                            "branch proofs attest different root hashes: every branch of \
                             one response must be proved against one platform state"
                                .to_string(),
                        )));
                    }
                }
                per_branch.push(entries);
            }
            let entries = merge_branch_pages(
                per_branch,
                &self.prefix_branches,
                self.descending,
                self.limit as usize,
            )?;
            let root_hash = root_hash.ok_or_else(|| {
                Error::Drive(DriveError::CorruptedDriveState(
                    "branch container verified to zero branches".to_string(),
                ))
            })?;
            return Ok((root_hash, entries));
        }
        self.verify_having_range_proof_v0_branch(0, proof)
    }

    /// One branch's verification — the entire pre-`IN` verifier,
    /// parameterized by the prefix branch.
    fn verify_having_range_proof_v0_branch(
        &self,
        branch: usize,
        proof: &[u8],
    ) -> Result<(RootHash, Vec<RankedEntry>), Error> {
        let path = self.indexed_property_name_tree_path(branch)?;
        let path_refs: Vec<&[u8]> = path.iter().map(|segment| segment.as_slice()).collect();
        let secondary_query = self.bounds.merk_query(self.descending);

        let result = match self.bounds.axis() {
            RankedAxis::Count => GroveDb::verify_indexed_count_query(
                proof,
                path_refs.as_slice(),
                secondary_query,
                Some(self.limit),
                &platform_version.drive.grove_version,
            ),
            RankedAxis::Sum => GroveDb::verify_indexed_sum_query(
                proof,
                path_refs.as_slice(),
                secondary_query,
                Some(self.limit),
                &platform_version.drive.grove_version,
            ),
            RankedAxis::Avg => GroveDb::verify_indexed_avg_query(
                proof,
                path_refs.as_slice(),
                secondary_query,
                Some(self.limit),
                &platform_version.drive.grove_version,
            ),
        }
        .map_err(|e| Error::GroveDB(Box::new(e)))?;

        let entries = match (self.bounds.axis(), result.entries) {
            (RankedAxis::Count, AxisEntries::Count(entries)) => entries
                .into_iter()
                .map(|entry| entry.key_pair())
                .map(|(count, key)| RankedEntry {
                    in_key: None,
                    key,
                    value: RankedEntryValue::Count(count),
                })
                .collect::<Vec<_>>(),
            (RankedAxis::Sum, AxisEntries::Sum(entries)) => entries
                .into_iter()
                .map(|entry| entry.key_pair())
                .map(|(sum, key)| RankedEntry {
                    in_key: None,
                    key,
                    value: RankedEntryValue::Sum(sum),
                })
                .collect::<Vec<_>>(),
            (RankedAxis::Avg, AxisEntries::Avg(entries)) => entries
                .into_iter()
                .map(|entry| entry.key_pair())
                .map(|(avg, key)| RankedEntry {
                    in_key: None,
                    key,
                    value: RankedEntryValue::AvgFixedPoint(avg),
                })
                .collect::<Vec<_>>(),
            (axis, other) => {
                return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                    "having range proof for the {axis:?} axis verified to {} entries of a \
                     different axis shape",
                    other.len()
                ))));
            }
        };

        if entries.len() > self.limit as usize {
            return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                "having range proof for the {:?} axis verified to {} entries for limit = {}",
                self.bounds.axis(),
                entries.len(),
                self.limit
            ))));
        }

        Ok((result.root_hash, entries))
    }
}
