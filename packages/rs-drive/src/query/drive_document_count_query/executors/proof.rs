//! Proof executor for every proved count mode
//! ([`super::super::DocumentCountMode::PointLookupProof`], `RangeProof`,
//! `RangeDistinctProof` and `RangeAggregateCarrierProof`): it proves the
//! path query the request's [`CountProofPlan`] lowers to. A client lowers
//! the same plan before sending, and the verify method its plan names
//! rebuilds the path query with the same builders.

use super::super::super::conditions::WhereClause;
use super::super::CountProofPlan;
use crate::drive::Drive;
use crate::error::Error;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
use grovedb_costs::CostContext;

impl Drive {
    /// Proof of what `plan` reads ([`CountProofPlan::path_query`]):
    ///
    /// - the primary-key count tree: one merk path proof, O(log n) bytes;
    /// - a point lookup: the matched branches' count trees, whose
    ///   `count_value` the verifier extracts with no document
    ///   materialization;
    /// - a range: grovedb's `AggregateCountOnRange`, one `u64`;
    /// - a distinct range: one `KVCount` per distinct in-range value, at most
    ///   the plan's limit of them;
    /// - a carrier: one aggregate per outer `In` value or outer-range match
    ///   (grovedb's carrier-subquery composition).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn execute_document_count_proof(
        &self,
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        document_type_name: String,
        where_clauses: Vec<WhereClause>,
        plan: CountProofPlan,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let path_query = plan.path_query(
            contract_id,
            document_type,
            document_type_name,
            where_clauses,
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
