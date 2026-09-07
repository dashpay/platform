//! [`Fetch`] bindings for the document aggregate views.
//!
//! The `FromProof` decoding for these types moved to the transport-free
//! `dash-platform-queries` crate together with [`DocumentQuery`]; the
//! [`Fetch`] trait is Sdk-bound, so its impls stay here.

use crate::platform::documents::document_query::DocumentQuery;
use crate::platform::Fetch;
use dapi_grpc::platform::v0::GetDocumentsRequest;
use drive_proof_verifier::{
    DocumentAverage, DocumentCount, DocumentHavingEntries, DocumentRankedEntries,
    DocumentSplitAverages, DocumentSplitCounts, DocumentSplitSums, DocumentSum,
};

impl Fetch for DocumentCount {
    type Query = DocumentQuery;
    type Request = GetDocumentsRequest;
}

impl Fetch for DocumentSum {
    type Query = DocumentQuery;
    type Request = GetDocumentsRequest;
}

impl Fetch for DocumentAverage {
    type Query = DocumentQuery;
    type Request = GetDocumentsRequest;
}

impl Fetch for DocumentSplitCounts {
    type Query = DocumentQuery;
    type Request = GetDocumentsRequest;
}

impl Fetch for DocumentSplitSums {
    type Query = DocumentQuery;
    type Request = GetDocumentsRequest;
}

impl Fetch for DocumentSplitAverages {
    type Query = DocumentQuery;
    type Request = GetDocumentsRequest;
}

impl Fetch for DocumentRankedEntries {
    type Query = DocumentQuery;
    type Request = GetDocumentsRequest;
}

impl Fetch for DocumentHavingEntries {
    type Query = DocumentQuery;
    type Request = GetDocumentsRequest;
}

impl Fetch for drive_proof_verifier::ChainedDocuments {
    type Query = dash_platform_queries::documents::chained_document_query::ChainedDocumentQuery;
    type Request = dapi_grpc::platform::v0::GetDocumentsRequest;
}
