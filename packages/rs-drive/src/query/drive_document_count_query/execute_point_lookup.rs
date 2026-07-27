//! Equal/In point-lookup execution paths for the count query.
//!
//! No-proof and proof executors for fully-covered Equal/`In` queries
//! against a `countable: true` index. Both sides share the same
//! [`DriveDocumentCountQuery::point_lookup_count_path_query`] builder,
//! so the proof bytes the server signs and the path query the verifier
//! reconstructs (and the no-proof read this file performs) all see
//! the exact same shape — there's only one source of truth for which
//! `CountTree` elements compose the answer.
//!
//! Range-mode executors live in
//! [`super::execute_range_count`](super::execute_range_count); this
//! file is the Equal/In half of the dispatch surface.
//!
//! Whole module is gated `feature = "server"` via the parent's
//! `pub mod execute_point_lookup;` declaration.

use super::{DriveDocumentCountQuery, SplitCountEntry};
use crate::drive::Drive;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::query_result_type::{QueryResultElement, QueryResultType};
use grovedb::TransactionArg;
use grovedb_costs::CostContext;

impl DriveDocumentCountQuery<'_> {
    /// Executes the count query without generating a proof.
    ///
    /// Returns the total count as a single `SplitCountEntry` with
    /// empty `key` (the unified-count Total shape).
    ///
    /// Implementation goes through the same
    /// [`Self::point_lookup_count_path_query`] builder the prove
    /// path uses, then runs `grove.query` to fetch the matched
    /// `CountTree` elements and sums their `count_value_or_default()`
    /// values. The builder handles all three structural cases
    /// (Equal-only fully covered, In at any index position, In with
    /// trailing Equals via `set_subquery_path`) — there's no need
    /// for a separate recursive walker on the no-proof side.
    pub fn execute_no_proof(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<SplitCountEntry>, Error> {
        let drive_version = &platform_version.drive;
        let path_query = self.point_lookup_count_path_query(platform_version)?;
        // `grove_get_path_query` requires a `drive_operations` sink for
        // cost accounting; the no-proof executor doesn't propagate fees
        // upward (callers that need cost are the per-mode dispatchers
        // in `drive_dispatcher.rs`, which wrap this for fee calculation),
        // so we use a local vec and discard.
        let mut drive_operations = vec![];
        let (results, _) = drive.grove_get_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryElementResultType,
            &mut drive_operations,
            drive_version,
        )?;
        // Sum across emitted CountTree elements:
        // - Equal-only: 0 or 1 element (0 when the branch is absent).
        // - In at any position: one element per In branch that has at
        //   least one doc; missing branches contribute 0 by virtue of
        //   being absent from the result set.
        // `count_value_or_default()` returns the `CountTree`'s count
        // for `Element::CountTree` / `Element::SumTree` and 1 for
        // `Element::Reference` (the unique-index-with-all-non-null
        // case — see `Element::count_value_or_default` for the per-
        // variant contract).
        let count: u64 = results
            .elements
            .iter()
            .map(|e| match e {
                QueryResultElement::ElementResultItem(elem) => elem.count_value_or_default(),
                // `QueryElementResultType` only emits `ElementResultItem`;
                // the other variants belong to `QueryKeyElementPairResultType`
                // / `QueryPathKeyElementTrioResultType` which we don't
                // request. Defensive 0 keeps the executor total-correct
                // even if grovedb's emission shape ever broadens.
                _ => 0,
            })
            .sum();
        Ok(vec![SplitCountEntry {
            in_key: None,
            key: vec![],
            // Point-lookup executor summed verified CountTree
            // counts to produce this; the count is explicit, hence
            // `Some(_)` (possibly `Some(0)` if every covered branch
            // was empty or absent).
            count: Some(count),
        }])
    }

    /// Generates a grovedb proof of the CountTree elements covering a
    /// fully-covered Equal/`In` count query against a `countable: true`
    /// index. Returns the raw proof bytes; the SDK-side
    /// [`Self::verify_point_lookup_count_proof`] walks the proof and
    /// extracts `count_value_or_default()` from each verified CountTree
    /// element.
    ///
    /// Builds the path query via
    /// [`Self::point_lookup_count_path_query`] (shared with the
    /// verifier AND with [`Self::execute_no_proof`] above, so all three
    /// sites see byte-identical path queries). Errors surface from the
    /// builder when the query shape isn't supported — partial
    /// coverage, more than one In, etc. — see that builder's docstring
    /// for the exhaustive contract.
    ///
    /// Proof size is O(k × log n) where k is the number of covered
    /// (Equal/In) branches and n is the tree depth: one merk path
    /// proof per CountTree element, not per matching document.
    /// Avoids the materialize-and-count alternative used by the
    /// regular document-query path, which scales with the number
    /// of matching docs and is capped at `u16::MAX`.
    pub fn execute_point_lookup_count_with_proof(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let drive_version = &platform_version.drive;
        let path_query = self.point_lookup_count_path_query(platform_version)?;
        // Destructure the `CostContext` explicitly rather than calling
        // `.unwrap()` on it: `CostContext::unwrap` is infallible (it just
        // drops the cost field), but the visual pattern collides with
        // `Option/Result::unwrap` and makes review noisier. Cost is
        // discarded here because the per-mode dispatcher in
        // `drive_dispatcher` wraps these executors with its own fee
        // accounting.
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
