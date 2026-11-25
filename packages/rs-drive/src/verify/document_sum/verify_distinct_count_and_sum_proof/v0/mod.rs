use crate::error::Error;
use crate::query::drive_document_average_query::AverageEntry;
use crate::query::drive_document_sum_query::DriveDocumentSumQuery;
use crate::query::WhereOperator;
use crate::verify::RootHash;
use dpp::version::PlatformVersion;
use grovedb::GroveDb;

impl DriveDocumentSumQuery<'_> {
    /// v0 of [`Self::verify_distinct_count_and_sum_proof`].
    ///
    /// Mirror of [`Self::verify_distinct_sum_proof_v0`] but
    /// extracts `count_sum_value_or_default()` from each verified
    /// terminator element. Returns
    /// `(root_hash, Vec<AverageEntry { count, sum }>)`.
    #[inline(always)]
    pub(super) fn verify_distinct_count_and_sum_proof_v0(
        &self,
        proof: &[u8],
        limit: u16,
        left_to_right: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<AverageEntry>), Error> {
        let path_query =
            self.distinct_sum_path_query(Some(limit), left_to_right, platform_version)?;
        let base_path_len = path_query.path.len();
        let has_in_on_prefix = self
            .where_clauses
            .iter()
            .any(|wc| wc.operator == WhereOperator::In);
        let (root_hash, elements) =
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)
                .map_err(|e| Error::GroveDB(Box::new(e)))?;

        let mut out: Vec<AverageEntry> = Vec::with_capacity(elements.len());
        for (path, key, elem) in elements {
            if let Some(e) = elem {
                // At 128 bits, whatever width the type keeps its sums in: for an element holding a
                // 64-bit sum (every element before protocol version 15) this is that sum widened
                // (`crate::query::sum_width`). Edited in place in this shipped generation: it
                // returned the sum at 64 bits and refused a wider one; a verifier runs on the
                // client and decides nothing in a block, and every sum it verified before is
                // returned as the same number.
                let (count, sum) = e.count_big_sum_value_or_default();
                let in_key = if has_in_on_prefix && path.len() > base_path_len {
                    Some(path[base_path_len].clone())
                } else {
                    None
                };
                out.push(AverageEntry {
                    in_key,
                    key,
                    count: Some(count),
                    sum: Some(sum),
                });
            }
        }
        Ok((root_hash, out))
    }
}
