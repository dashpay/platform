//! Sdk-bound half of the chained document query surface: the rich →
//! wire encoding. The transport-free query type itself
//! ([`ChainedDocumentQuery`]) lives in `dash-platform-queries`.

use dapi_grpc::platform::v0 as platform_proto;
use dapi_grpc::platform::v0::GetChainedDocumentsRequest;
use dash_platform_queries::documents::chained_document_query::ChainedDocumentQuery;
use dpp::version::TryFromPlatformVersioned;

use crate::Error;

/// Encode a [`ChainedDocumentQuery`] onto the wire.
///
/// The [`Fetch`](crate::platform::Fetch) trampoline for
/// [`drive_proof_verifier::ChainedDocuments`] splits `Query =
/// ChainedDocumentQuery` (rich, what `FromProof` binds to) from
/// `Request = GetChainedDocumentsRequest` (wire); this impl is the
/// rich→wire step.
impl crate::platform::Query<platform_proto::GetChainedDocumentsRequest> for ChainedDocumentQuery {
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<platform_proto::GetChainedDocumentsRequest, Error> {
        GetChainedDocumentsRequest::try_from_platform_versioned(
            self.clone(),
            settings.protocol_version,
        )
        .map_err(Error::from)
    }
}
