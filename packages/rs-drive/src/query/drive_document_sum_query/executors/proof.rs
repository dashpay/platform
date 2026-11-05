//! Proof executor for every proved sum mode
//! ([`super::super::DocumentSumMode::PointLookupProof`], `RangeProof`,
//! `RangeDistinctProof` and `RangeAggregateCarrierProof`) and every proved
//! average: it proves what the request's [`SumProofPlan`] reads. The SDK's
//! verifiers rebuild the path query from the same plan.

use super::super::{DriveDocumentSumQuery, SumProofPlan};
use crate::drive::Drive;
use crate::error::Error;
use crate::query::WhereClause;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Proof of what `plan` reads:
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
    /// `with_counts` is whether the request is an average, whose range and
    /// carrier proofs read each element's count beside its sum; its point
    /// lookups and distinct walks read the count-sum-bearing elements the
    /// index's terminators already hold.
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
        // The index-reading arms each move the query parts into the query
        // they prove.
        let sum_query = |index| DriveDocumentSumQuery {
            document_type,
            contract_id,
            document_type_name,
            index,
            where_clauses,
            sum_property,
        };
        match plan {
            SumProofPlan::PrimaryKeySumTree => {
                let path_query = DriveDocumentSumQuery::primary_key_sum_path_query(
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
            SumProofPlan::PointLookup(index) => sum_query(index)
                .execute_point_lookup_sum_with_proof(self, transaction, platform_version),
            SumProofPlan::Range(index) if with_counts => sum_query(index)
                .execute_aggregate_count_and_sum_with_proof(self, transaction, platform_version),
            SumProofPlan::Range(index) => sum_query(index).execute_aggregate_sum_with_proof(
                self,
                transaction,
                platform_version,
            ),
            SumProofPlan::RangeDistinct {
                index,
                limit,
                left_to_right,
            } => sum_query(index).execute_distinct_sum_with_proof(
                self,
                limit,
                left_to_right,
                transaction,
                platform_version,
            ),
            SumProofPlan::RangeAggregateCarrier {
                index,
                limit,
                left_to_right,
            } if with_counts => sum_query(index)
                .execute_carrier_aggregate_count_and_sum_with_proof(
                    self,
                    limit,
                    left_to_right,
                    transaction,
                    platform_version,
                ),
            SumProofPlan::RangeAggregateCarrier {
                index,
                limit,
                left_to_right,
            } => sum_query(index).execute_carrier_aggregate_sum_with_proof(
                self,
                limit,
                left_to_right,
                transaction,
                platform_version,
            ),
        }
    }
}
