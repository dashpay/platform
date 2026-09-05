//! Sdk-bound half of the composite document query surface: the rich →
//! wire encoding. The transport-free query type itself
//! ([`CompositeDocumentQuery`]) lives in `dash-platform-queries`.

use dapi_grpc::platform::v0 as platform_proto;
use dapi_grpc::platform::v0::GetDocumentsRequest;
use dash_platform_queries::documents::composite_document_query::CompositeDocumentQuery;
use dpp::version::TryFromPlatformVersioned;

use crate::Error;

/// Encode a [`CompositeDocumentQuery`] onto the wire.
///
/// The [`Fetch`](crate::platform::Fetch) trampoline for
/// [`drive_proof_verifier::CompositeDocuments`] splits `Query =
/// CompositeDocumentQuery` (rich, what `FromProof` binds to) from
/// `Request = GetDocumentsRequest` (wire); this impl is the rich→wire
/// step.
impl crate::platform::Query<platform_proto::GetDocumentsRequest> for CompositeDocumentQuery {
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<platform_proto::GetDocumentsRequest, Error> {
        GetDocumentsRequest::try_from_platform_versioned(self.clone(), settings.protocol_version)
            .map_err(Error::from)
    }
}

// `CompositeDocumentQuery` does not implement `TransportRequest` (the
// wire form is `GetDocumentsRequest`), so the blanket `Query<T> for T`
// does not apply — provide the identity impl explicitly, same as
// `DocumentQuery`'s, so the fetch trampoline can use it both as the
// user-supplied `Q` and as the rich `Self::Query`.
impl crate::platform::Query<CompositeDocumentQuery> for CompositeDocumentQuery {
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<CompositeDocumentQuery, Error> {
        if !settings.prove {
            tracing::warn!(request= ?self, "sending query without proof, ensure data is trusted");
        }
        Ok(self.clone())
    }
}
