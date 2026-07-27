use crate::error::Error;
use crate::query::drive_document_sum_query::{DriveDocumentSumQuery, SumEntry};
use crate::query::WhereOperator;
use crate::verify::RootHash;
use dpp::version::PlatformVersion;
use grovedb::GroveDb;

impl DriveDocumentSumQuery<'_> {
    /// v0 of [`Self::verify_point_lookup_sum_proof`].
    ///
    /// Mirror of count's
    /// [`super::super::super::document_count::verify_point_lookup_count_proof::v0`]
    /// — same path-query rebuild + element walk, but extracts
    /// `sum_value_or_default()` from the verified SumTree element
    /// instead of `count_value_or_default()`.
    ///
    /// For Equal-only covered queries the entry's `key` stays
    /// empty; for In-bearing shapes the In value sits either at
    /// `path[base_path_len]` (In-with-trailing-Equals shape) or in
    /// `grove_key` (In-on-terminator shape) — see count's analog
    /// docstring for the descent-vs-direct discriminator.
    #[inline(always)]
    pub(super) fn verify_point_lookup_sum_proof_v0(
        &self,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<SumEntry>), Error> {
        let path_query = self.point_lookup_sum_path_query(platform_version)?;
        let base_path_len = path_query.path.len();
        let has_in_clause = self
            .where_clauses
            .iter()
            .any(|wc| wc.operator == WhereOperator::In);
        let (root_hash, elements) =
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)
                .map_err(|e| Error::GroveDB(Box::new(e)))?;

        let mut out: Vec<SumEntry> = Vec::with_capacity(elements.len());
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
            let sum = elem.map(|e| e.sum_value_or_default());
            out.push(SumEntry {
                in_key: None,
                key,
                sum,
            });
        }
        Ok((root_hash, out))
    }
}
