//! The current versions of data contracts (`getDataContractsLatestVersions`).
//!
//! The cheap way to check that contracts held locally are still current. From protocol
//! version 14 every contract carries a four-byte version item in state, and a query without
//! [`DataContractsLatestVersionsQuery::include_contracts`] is answered from it: the unproved
//! fetch ([`FetchUnproved::fetch_unproved`] on [`DataContractsLatestVersions`]) returns one
//! integer per contract, and the proved fetch ([`FetchMany::fetch_many`] on
//! [`DataContractLatestVersion`]) verifies a proof of the items, a few hundred bytes of hash
//! path per contract. With `include_contracts`, and on earlier protocol versions, the proof
//! is the multi-contract proof `getDataContracts` returns, so it costs as much as fetching
//! the contracts. Either way the contracts themselves come back only when
//! `include_contracts` is set.

use crate::platform::{FetchMany, FetchUnproved, Identifier, Query, QuerySettings};
use crate::Error;
use dapi_grpc::platform::v0::get_data_contracts_latest_versions_request::GetDataContractsLatestVersionsRequestV0;
use dapi_grpc::platform::v0::{
    get_data_contracts_latest_versions_request, GetDataContractsLatestVersionsRequest,
};
pub use drive_proof_verifier::types::data_contracts_latest_versions::{
    DataContractLatestVersion, DataContractsLatestVersions,
};

/// Query for the current versions of data contracts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataContractsLatestVersionsQuery {
    /// The contracts to look up, at least one and at most `max_returned_elements`.
    pub ids: Vec<Identifier>,
    /// Also return the contracts. Off by default: the point of the query is to learn the
    /// versions without transferring the contracts.
    pub include_contracts: bool,
}

impl DataContractsLatestVersionsQuery {
    /// The versions of `ids`, without the contracts.
    pub fn new(ids: Vec<Identifier>) -> Self {
        Self {
            ids,
            include_contracts: false,
        }
    }

    /// The versions of `ids` together with the contracts.
    pub fn with_contracts(ids: Vec<Identifier>) -> Self {
        Self {
            ids,
            include_contracts: true,
        }
    }
}

impl From<Vec<Identifier>> for DataContractsLatestVersionsQuery {
    fn from(ids: Vec<Identifier>) -> Self {
        Self::new(ids)
    }
}

impl Query<GetDataContractsLatestVersionsRequest> for DataContractsLatestVersionsQuery {
    fn query(
        &self,
        settings: &QuerySettings<'_>,
    ) -> Result<GetDataContractsLatestVersionsRequest, Error> {
        Ok(GetDataContractsLatestVersionsRequest {
            version: Some(get_data_contracts_latest_versions_request::Version::V0(
                GetDataContractsLatestVersionsRequestV0 {
                    ids: self.ids.iter().map(|id| id.to_vec()).collect(),
                    include_contracts: self.include_contracts,
                    prove: settings.prove,
                },
            )),
        })
    }
}

impl Query<GetDataContractsLatestVersionsRequest> for Vec<Identifier> {
    fn query(
        &self,
        settings: &QuerySettings<'_>,
    ) -> Result<GetDataContractsLatestVersionsRequest, Error> {
        DataContractsLatestVersionsQuery::new(self.clone()).query(settings)
    }
}

impl FetchMany<Identifier, DataContractsLatestVersions> for DataContractLatestVersion {
    type Query = GetDataContractsLatestVersionsRequest;
    type Request = GetDataContractsLatestVersionsRequest;
}

impl FetchUnproved for DataContractsLatestVersions {
    type Request = GetDataContractsLatestVersionsRequest;
}
