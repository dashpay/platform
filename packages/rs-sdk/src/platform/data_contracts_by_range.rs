//! Paginated enumeration of every data contract on Platform (`getDataContractsByRange`).
//!
//! Pages are ordered by ascending contract id. Fetch the first page with
//! [`DataContractsByRangeQuery::first_page`], then keep passing the last id of each page as
//! [`DataContractsByRangeStart::After`] until a page comes back shorter than the limit (or
//! empty).

use crate::platform::{Fetch, Identifier, Query};
use crate::Error;
use dapi_grpc::platform::v0::get_data_contracts_by_range_request::get_data_contracts_by_range_request_v0::Start;
use dapi_grpc::platform::v0::get_data_contracts_by_range_request::GetDataContractsByRangeRequestV0;
use dapi_grpc::platform::v0::{
    get_data_contracts_by_range_request, GetDataContractsByRangeRequest,
};
pub use drive_proof_verifier::types::data_contracts_by_range::DataContractsByRange;

/// Where a page of the contract enumeration starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataContractsByRangeStart {
    /// Start after this contract id (exclusive). Pass the last id of the previous page.
    After(Identifier),
    /// Start at this contract id (inclusive).
    At(Identifier),
}

/// Query for one page of the contract enumeration, in ascending contract id order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DataContractsByRangeQuery {
    /// Where the page starts. `None` starts from the first contract.
    pub start: Option<DataContractsByRangeStart>,
    /// Maximum number of contracts in the page, `1..=100`. `None` means the maximum.
    pub limit: Option<u32>,
    /// Return contract ids only: every page value is `None` and the proof is much smaller.
    pub ids_only: bool,
}

impl DataContractsByRangeQuery {
    /// The first page of the enumeration with the maximum page size.
    pub fn first_page() -> Self {
        Self::default()
    }

    /// The query for the page following `page`, keeping this query's limit and mode.
    ///
    /// Returns `None` when `page` is empty, which means the enumeration is exhausted. A
    /// page shorter than the limit is also the last one; callers that track the limit can
    /// stop there and save the final empty round trip.
    pub fn next_page(&self, page: &DataContractsByRange) -> Option<Self> {
        page.next_start_after().map(|last_id| Self {
            start: Some(DataContractsByRangeStart::After(last_id)),
            limit: self.limit,
            ids_only: self.ids_only,
        })
    }
}

impl Query<GetDataContractsByRangeRequest> for DataContractsByRangeQuery {
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<GetDataContractsByRangeRequest, Error> {
        let prove = settings.prove;
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }

        let start = self.start.map(|start| match start {
            DataContractsByRangeStart::After(id) => Start::StartAfter(id.to_vec()),
            DataContractsByRangeStart::At(id) => Start::StartAt(id.to_vec()),
        });

        Ok(GetDataContractsByRangeRequest {
            version: Some(get_data_contracts_by_range_request::Version::V0(
                GetDataContractsByRangeRequestV0 {
                    limit: self.limit,
                    start,
                    ids_only: self.ids_only,
                    prove,
                },
            )),
        })
    }
}

impl Fetch for DataContractsByRange {
    type Query = GetDataContractsByRangeRequest;
    type Request = GetDataContractsByRangeRequest;
}
