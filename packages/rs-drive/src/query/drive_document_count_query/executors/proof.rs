//! Proof executor for every proved count mode
//! ([`super::super::DocumentCountMode::PointLookupProof`], `RangeProof`,
//! `RangeDistinctProof` and `RangeAggregateCarrierProof`): it proves what the
//! request's [`CountProofPlan`] reads. The SDK's verifier rebuilds the path
//! query from the same plan.

use super::super::super::conditions::WhereClause;
use super::super::{CountProofPlan, DriveDocumentCountQuery};
use crate::drive::Drive;
use crate::error::Error;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Proof of what `plan` reads:
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
    ///
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
        // The index-reading arms each move the query parts into the query
        // they prove.
        let count_query = |index| DriveDocumentCountQuery {
            document_type,
            contract_id,
            document_type_name,
            index,
            where_clauses,
        };
        match plan {
            CountProofPlan::PrimaryKeyCountTree => {
                let path_query = DriveDocumentCountQuery::primary_key_count_tree_path_query(
                    contract_id,
                    document_type.name(),
                );
                self.grove
                    .get_proved_path_query(
                        &path_query,
                        None,
                        transaction,
                        &platform_version.drive.grove_version,
                    )
                    .unwrap()
                    .map_err(|e| Error::GroveDB(Box::new(e)))
            }
            CountProofPlan::PointLookup(index) => count_query(index)
                .execute_point_lookup_count_with_proof(self, transaction, platform_version),
            CountProofPlan::Range(index) => count_query(index).execute_aggregate_count_with_proof(
                self,
                transaction,
                platform_version,
            ),
            CountProofPlan::RangeDistinct {
                index,
                limit,
                left_to_right,
            } => count_query(index).execute_distinct_count_with_proof(
                self,
                limit,
                left_to_right,
                transaction,
                platform_version,
            ),
            CountProofPlan::RangeAggregateCarrier {
                index,
                limit,
                left_to_right,
            } => count_query(index).execute_carrier_aggregate_count_with_proof(
                self,
                limit,
                left_to_right,
                transaction,
                platform_version,
            ),
        }
    }
}
