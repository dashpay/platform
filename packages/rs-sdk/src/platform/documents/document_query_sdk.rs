//! Sdk-bound surface of [`DocumentQuery`].
//!
//! [`DocumentQuery`] itself is transport-free and lives in
//! `dash-platform-queries`; this module holds the pieces that need an
//! [`Sdk`]: the contract-fetching constructor and the rich→wire
//! [`Query`](crate::platform::Query) encoding step.

use crate::platform::documents::document_query::DocumentQuery;
use crate::platform::Fetch;
use crate::{error::Error, sdk::Sdk};
use dapi_grpc::platform::v0 as platform_proto;
use dapi_grpc::platform::v0::GetDocumentsRequest;
use dpp::prelude::{DataContract, Identifier};
use dpp::version::TryFromPlatformVersioned;

/// Sdk-bound extension methods for [`DocumentQuery`].
///
/// Kept as an extension trait because [`DocumentQuery`] is defined in the
/// transport-free `dash-platform-queries` crate, so its Sdk-dependent
/// constructor cannot be an inherent method there. Bring this trait into
/// scope to keep calling `DocumentQuery::new_with_data_contract_id(...)`.
#[allow(async_fn_in_trait)]
pub trait DocumentQuerySdk: Sized {
    /// Create new document query for provided document type name and data contract ID.
    ///
    /// Note that this method will fetch data contract first.
    async fn new_with_data_contract_id(
        api: &Sdk,
        data_contract_id: Identifier,
        document_type_name: &str,
    ) -> Result<Self, Error>;
}

impl DocumentQuerySdk for DocumentQuery {
    async fn new_with_data_contract_id(
        api: &Sdk,
        data_contract_id: Identifier,
        document_type_name: &str,
    ) -> Result<Self, Error> {
        let data_contract =
            DataContract::fetch(api, data_contract_id)
                .await?
                .ok_or(Error::MissingDependency(
                    "DataContract".to_string(),
                    format!("data contract {} not found", data_contract_id),
                ))?;

        Self::new(data_contract, document_type_name).map_err(Error::from)
    }
}

/// Encode a [`DocumentQuery`] onto the wire using the SDK's
/// currently-known [`dpp::version::PlatformVersion`] for V0 vs V1 dispatch.
///
/// The [`Fetch`] / [`FetchMany`](crate::platform::FetchMany) trampolines for
/// [`dpp::document::Document`] (and the document aggregate views) split
/// `Fetch::Query = DocumentQuery` (rich, what `FromProof` binds to) from
/// `Fetch::Request = GetDocumentsRequest` (wire); this impl is the
/// rich→wire step the trampoline invokes via
/// `Query::query(&rich, &sdk.query_settings())`.
impl crate::platform::Query<platform_proto::GetDocumentsRequest> for DocumentQuery {
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<platform_proto::GetDocumentsRequest, Error> {
        GetDocumentsRequest::try_from_platform_versioned(self.clone(), settings.protocol_version)
            .map_err(Error::from)
    }
}
