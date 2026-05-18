//! Point-lookup executor for sum queries. Parallels count's
//! `execute_point_lookup.rs`.
//!
//! Two entry points: `execute_no_proof` (sum the matched value-trees
//! via `grove_get_path_query`) and `execute_point_lookup_sum_with_proof`
//! (proof bytes via `grove.get_proved_path_query`). Verifier side:
//! `GroveDb::verify_query` + `sum_value_or_default()` extraction on
//! each verified SumTree element.

use super::{DriveDocumentSumQuery, SumEntry};
use crate::drive::Drive;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::query_result_type::{QueryResultElement, QueryResultType};
use grovedb::TransactionArg;
use grovedb_costs::CostContext;

impl DriveDocumentSumQuery<'_> {
    /// Executes the sum query without generating a proof.
    ///
    /// Returns the total sum as a single `SumEntry` with empty `key`
    /// (the unified-sum Total shape).
    ///
    /// Mirror of count's `execute_no_proof` — runs through the same
    /// [`Self::point_lookup_sum_path_query`] builder the prove path
    /// uses, then runs `grove.query` to fetch the matched SumTree
    /// elements and sums their `sum_value_or_default()`.
    pub fn execute_no_proof(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<SumEntry>, Error> {
        let drive_version = &platform_version.drive;
        let path_query = self.point_lookup_sum_path_query(platform_version)?;
        let mut drive_operations = vec![];
        let (results, _) = drive.grove_get_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryElementResultType,
            &mut drive_operations,
            drive_version,
        )?;
        // Sum across emitted SumTree elements:
        // - Equal-only: 0 or 1 element (0 when the branch is absent).
        // - In at any position: one element per In branch that has at
        //   least one doc; missing branches contribute 0.
        //
        // Use `checked_add` so an overflowed aggregate fails
        // deterministically with a typed query error rather than
        // panicking (debug) or silently wrapping (release). The
        // iterator `.sum::<i64>()` form would do `i64::Add` which has
        // neither property in a stable consensus-level contract.
        let sum: i64 = results
            .elements
            .iter()
            .map(|e| match e {
                QueryResultElement::ElementResultItem(elem) => elem.sum_value_or_default(),
                _ => 0,
            })
            .try_fold(0i64, |acc, v| acc.checked_add(v))
            .ok_or_else(|| {
                Error::Query(QuerySyntaxError::Unsupported(
                    "point-lookup sum overflowed i64 when summing per-In branches. \
                     Narrow the query (smaller In set) or use multiple queries and \
                     combine client-side."
                        .to_string(),
                ))
            })?;
        Ok(vec![SumEntry {
            in_key: None,
            key: vec![],
            sum: Some(sum),
        }])
    }

    /// Generates a grovedb proof of the SumTree elements covering a
    /// fully-covered Equal/`In` sum query against a `summable: "<x>"`
    /// index. Mirrors count's `execute_point_lookup_count_with_proof`.
    pub fn execute_point_lookup_sum_with_proof(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let drive_version = &platform_version.drive;
        let path_query = self.point_lookup_sum_path_query(platform_version)?;
        let CostContext { value, cost: _ } = drive.grove.get_proved_path_query(
            &path_query,
            None,
            transaction,
            &drive_version.grove_version,
        );
        let proof = value.map_err(|e| Error::GroveDB(Box::new(e)))?;
        Ok(proof)
    }
}
