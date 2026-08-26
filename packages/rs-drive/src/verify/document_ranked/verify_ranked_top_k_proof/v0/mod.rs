use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::drive_document_ranked_query::branches::{
    axis_entries_to_ranked, decompose_branch_paths, merge_branch_pages,
};
use crate::query::{DriveDocumentRankedQuery, RankedPage};
use crate::verify::RootHash;
use dpp::version::PlatformVersion;
use grovedb::operations::proof::VerifiedPathQuery;
use grovedb::GroveDb;
use grovedb::PathQuery;
use grovedb_query::AxisQuery;

impl DriveDocumentRankedQuery<'_> {
    /// v0 of [`Self::verify_ranked_top_k_proof`].
    ///
    /// Rebuilds the proved subtree path with
    /// [`Self::indexed_property_name_tree_path`] and hands the proof to
    /// [`GroveDb::verify_indexed_axis_top_k_paginated`], which is an
    /// associated function — no database handle is involved, so this
    /// compiles and runs in a verifier-only build.
    ///
    /// Three things are checked before the page is returned:
    ///
    /// 1. **The envelope's `(axis, k, offset, descending)` match this
    ///    query.** grovedb does this itself: the values are echoed in
    ///    the proof and compared against the arguments, so a proof
    ///    generated for a different ranking — or a different page of the
    ///    same ranking — is rejected rather than silently reinterpreted.
    /// 2. **The result's axis shape matches the requested axis** — a
    ///    `Count` request must not come back holding `Sum` entries. This
    ///    is belt-and-braces on top of (1) (the tag check already rules
    ///    it out) and exists so a future decoder change that decouples
    ///    the tag from the decoded variant surfaces as an error here
    ///    instead of a mis-typed number reaching the caller.
    /// 3. **At most `k` entries.** Fewer is normal — the index may hold
    ///    fewer groups than were asked for — but more would mean the
    ///    proof committed a longer walk than the request authorized.
    ///
    /// The returned [`RankedPage::skipped`] is grovedb's independently
    /// re-derived skip count, not an echo of the request: it is
    /// recomputed by the verifier from the counted subtree commitments
    /// in the proof bytes. It equals `self.offset` on a full page; when
    /// the walk exhausted the secondary during the skip it is smaller,
    /// `entries` is empty, and the pair is a proof that the ranking
    /// holds exactly `skipped` groups. Deliberately **not** rejected
    /// here — "you paged past the end, and here is how far the end is"
    /// is a useful answer, and the only place that knows whether it is
    /// acceptable is the caller.
    ///
    #[inline(always)]
    pub(super) fn verify_ranked_top_k_proof_v0(
        &self,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, RankedPage), Error> {
        if self.prefix_branches.len() > 1 {
            // `IN`-pinned request: the proof is one grovedb branched
            // envelope. The branch set (and its order) comes from
            // *this* query's own resolution; grovedb binds each branch
            // tail to its branch key through the branching-level
            // multi-key proof, reconstructs one root hash, and echoes
            // `(axis, k, offset, direction)`. The page is then
            // re-derived by the shared merge — the client never trusts
            // a server-side merge.
            let paths = (0..self.prefix_branches.len())
                .map(|branch| self.indexed_property_name_tree_path(branch))
                .collect::<Result<Vec<_>, Error>>()?;
            let (prefix, keys, suffix) = decompose_branch_paths(&paths)?;
            let path_query = PathQuery::new_branched_axis(
                prefix,
                keys.clone(),
                suffix,
                AxisQuery::top_k(
                    self.axis.into(),
                    self.k,
                    self.offset as u64,
                    self.descending,
                ),
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
                    "a branched ranked top-k proof verified to a non-branched shape".to_string(),
                )));
            };
            if branches.len() != keys.len() || branches.iter().map(|(key, _)| key).ne(keys.iter()) {
                return Err(Error::Drive(DriveError::CorruptedDriveState(
                    "a branched ranked top-k proof verified a different branch set than the \
                     request resolved"
                        .to_string(),
                )));
            }
            let per_branch = branches
                .into_iter()
                .map(|(_key, entries)| {
                    // A branch key whose absence the branching-level Merk
                    // proof authenticates contributes an empty page — the
                    // same reading the unproved path gives an absent `IN`
                    // element.
                    let entries = match entries {
                        None => Vec::new(),
                        Some(entries) => axis_entries_to_ranked(self.axis, entries)?,
                    };
                    if entries.len() > self.k as usize {
                        return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                            "a branch of a ranked top-k proof verified to {} entries for \
                             k = {}",
                            entries.len(),
                            self.k
                        ))));
                    }
                    Ok(entries)
                })
                .collect::<Result<Vec<_>, Error>>()?;
            let entries = merge_branch_pages(
                per_branch,
                &self.prefix_branches,
                self.descending,
                self.k as usize,
            )?;
            return Ok((
                root_hash,
                RankedPage {
                    skipped: 0,
                    entries,
                },
            ));
        }
        self.verify_ranked_top_k_proof_v0_branch(0, proof, platform_version)
    }

    /// One branch's verification — the entire pre-`IN` verifier,
    /// parameterized by the prefix branch.
    fn verify_ranked_top_k_proof_v0_branch(
        &self,
        branch: usize,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, RankedPage), Error> {
        let path = self.indexed_property_name_tree_path(branch)?;
        let path_query = PathQuery::new_axis_top_k(
            path,
            self.axis.into(),
            self.k,
            self.offset as u64,
            self.descending,
        );
        let verified =
            GroveDb::verify_path_query(proof, &path_query, &platform_version.drive.grove_version)
                .map_err(|e| Error::GroveDB(Box::new(e)))?;
        let VerifiedPathQuery::AxisEntries {
            root_hash,
            entries,
            skipped,
        } = verified
        else {
            return Err(Error::Drive(DriveError::CorruptedDriveState(
                "a ranked top-k proof verified to a different shape".to_string(),
            )));
        };
        let entries = axis_entries_to_ranked(self.axis, entries)?;
        if entries.len() > self.k as usize {
            return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                "ranked top-k proof verified to {} entries for k = {}",
                entries.len(),
                self.k
            ))));
        }
        // A paginated traversal's proof always attests its skip.
        let skipped = skipped.ok_or_else(|| {
            Error::Drive(DriveError::CorruptedDriveState(
                "a paginated ranked proof carried no skip attestation".to_string(),
            ))
        })?;
        Ok((root_hash, RankedPage { skipped, entries }))
    }
}
