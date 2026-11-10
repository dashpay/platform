//! Proof executor for every proved sum mode
//! ([`super::super::DocumentSumMode::PointLookupProof`], `RangeProof`,
//! `RangeDistinctProof` and `RangeAggregateCarrierProof`) and every proved
//! average: it proves the path query the request's [`SumProofPlan`] lowers
//! to. A client lowers the same plan before sending, and the verify method
//! its plan names rebuilds the path query with the same builders.

use super::super::SumProofPlan;
use crate::drive::Drive;
use crate::error::Error;
use crate::query::WhereClause;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
use grovedb_costs::CostContext;

impl Drive {
    /// Proof of what `plan` reads ([`SumProofPlan::path_query`]):
    ///
    /// - the primary-key sum tree: one merk path proof;
    /// - a point lookup: the matched branches' sum-bearing elements;
    /// - a range: grovedb's `AggregateSumOnRange`, or with `with_counts`
    ///   its `AggregateCountAndSumOnRange` over a PCPS
    ///   (`ProvableCountProvableSumTree`) index;
    /// - a distinct range: one element per distinct in-range value, at most
    ///   the plan's limit of them;
    /// - a carrier: one aggregate per outer `In` value or outer-range match,
    ///   with `with_counts` a `(count, sum)` pair each.
    ///
    /// `with_counts` is whether the request is an average.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn execute_document_sum_proof(
        &self,
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        document_type_name: String,
        where_clauses: Vec<WhereClause>,
        sum_property: String,
        plan: SumProofPlan,
        with_counts: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let path_query = plan.path_query(
            contract_id,
            document_type,
            document_type_name,
            where_clauses,
            sum_property,
            with_counts,
            platform_version,
        )?;
        let CostContext { value, cost: _ } = self.grove.get_proved_path_query(
            &path_query,
            None,
            transaction,
            &platform_version.drive.grove_version,
        );
        value.map_err(|e| Error::GroveDB(Box::new(e)))
    }
}
