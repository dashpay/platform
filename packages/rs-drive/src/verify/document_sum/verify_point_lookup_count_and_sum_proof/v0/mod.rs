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
            let (count, sum) = match elem {
                Some(e) => {
                    let (c, s) = e.count_sum_value_or_default();
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
