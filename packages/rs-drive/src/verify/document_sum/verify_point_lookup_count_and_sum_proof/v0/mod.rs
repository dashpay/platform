use crate::error::Error;
use crate::query::drive_document_average_query::AverageEntry;
use crate::query::drive_document_sum_query::DriveDocumentSumQuery;
use crate::query::WhereOperator;
use crate::verify::RootHash;
use dpp::version::PlatformVersion;
use grovedb::GroveDb;

impl DriveDocumentSumQuery<'_> {
    /// v0 of [`Self::verify_point_lookup_count_and_sum_proof`].
    ///
    /// Mirror of [`Self::verify_point_lookup_sum_proof_v0`] but
    /// extracts `count_sum_value_or_default()` (returns
    /// `(u64, i64)`) from each verified count-sum-bearing element
    /// instead of `sum_value_or_default()` (returns `i64`).
    ///
    /// In-value extraction follows the same descent-vs-direct
    /// discriminator as the sum-only and count-only point-lookup
    /// verifiers — see count's
    /// `verify_point_lookup_count_proof_v0` for the full
    /// layout-shape docstring.
    #[inline(always)]
    pub(super) fn verify_point_lookup_count_and_sum_proof_v0(
        &self,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<AverageEntry>), Error> {
        let path_query = self.point_lookup_sum_path_query(platform_version)?;
        let base_path_len = path_query.path.len();
        let has_in_clause = self
            .where_clauses
            .iter()
            .any(|wc| wc.operator == WhereOperator::In);
        let (root_hash, elements) =
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)
                .map_err(|e| Error::GroveDB(Box::new(e)))?;

        let mut out: Vec<AverageEntry> = Vec::with_capacity(elements.len());
        for (path, grove_key, elem) in elements {
            let key = if has_in_clause {
                if path.len() > base_path_len {
                    path[base_path_len].clone()
                } else {
                    grove_key
                }
            } else {
                Vec::new()
            };
            // The proof returns a tree element as stored, wrapper included,
            // while the unproven read unwraps it: a `summableOffCountIndex`
            // index's last-property tree, read whole when its other
            // properties are pinned, sits `NonCounted`-wrapped under a
            // countable index's value tree, and its count is the groups. So
            // the decode looks through the wrapper. Edited in place in this
            // shipped generation: no element an average reads before
            // protocol version 14 is wrapped, so those decode as before
            // (`should_verify_an_average_point_proof_unchanged_at_protocol_version_13`).
            let (count, sum) = match elem {
                Some(e) => {
                    let (c, s) = e.underlying().count_sum_value_or_default();
                    (Some(c), Some(s))
                }
                None => (None, None),
            };
            out.push(AverageEntry {
                in_key: None,
                key,
                count,
                sum,
            });
        }
        Ok((root_hash, out))
    }
}
