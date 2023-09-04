//! Document manipulation

use dapi_grpc::platform::v0::{GetDocumentsRequest, GetDocumentsResponse};
use drive::query::DriveQuery;
use rs_dapi_client::{DapiRequest, RequestSettings};

use crate::{
    crud::{Listable, Readable, SdkQuery},
    dapi::DashAPI,
    error::Error,
};
use drive_proof_verifier::proof::from_proof::{Documents, FromProof, Length};

use super::document_query::SdkDocumentQuery;

/// A document
#[derive(Debug, Clone)]
pub struct SdkDocument {
    inner: dpp::document::Document,
}

impl From<SdkDocument> for dpp::document::Document {
    fn from(doc: SdkDocument) -> Self {
        doc.inner
    }
}

impl From<dpp::document::Document> for SdkDocument {
    fn from(doc: dpp::document::Document) -> Self {
        Self { inner: doc }
    }
}

impl Length for SdkDocument {
    fn count_some(&self) -> usize {
        1
    }
}
#[async_trait::async_trait]
impl<API: DashAPI> Readable<API> for SdkDocument {
    type Identifier = SdkDocumentQuery;

    async fn read<Q: SdkQuery<Self::Identifier>>(api: &API, query: &Q) -> Result<Self, Error> {
        if let Some(docs) = Self::list(api, query).await? {
            if docs.len() == 1 {
                return Ok(docs.first().unwrap().clone().into());
            }
            return Err(Error::TooManyElementsReceived {
                expected: 1,
                got: docs.len(),
            });
        }
        Err(Error::NotFound("document not found".to_string()))
    }
}

#[async_trait::async_trait]
impl<API: DashAPI> Listable<API> for SdkDocument {
    type Request = SdkDocumentQuery;
    async fn list<Q: SdkQuery<Self::Request>>(
        api: &API,
        query: &Q,
    ) -> Result<Option<Vec<Self>>, Error> {
        let document_query: SdkDocumentQuery = query.query()?;
        let drive_query: DriveQuery = (&document_query).try_into()?;
        let request: GetDocumentsRequest = document_query.clone().try_into()?;

        let mut client = api.platform_client().await;
        let response: GetDocumentsResponse = request
            .execute(&mut client, RequestSettings::default())
            .await?;

        tracing::trace!(request=?document_query, response=?response, "list documents");

        match Documents::maybe_from_proof(&drive_query, &response, api.quorum_info_provider()?)? {
            Some(documents) => Ok(Some(documents.into_iter().map(|doc| doc.into()).collect())),
            None => Ok(None),
        }
    }
}

impl SdkQuery<GetDocumentsRequest> for SdkDocumentQuery {
    fn query(&self) -> Result<GetDocumentsRequest, Error> {
        <Self as TryInto<GetDocumentsRequest>>::try_into(self.clone()).map_err(|e| e.into())
    }
}

#[cfg(test)]
mod tests {
    use dpp::document::{Document, DocumentV0};

    #[test]
    fn test_try_from() {
        let src = Document::V0(DocumentV0::default());
        let sdk_doc = super::SdkDocument::try_from(src.clone()).expect("try from document");
        assert_eq!(sdk_doc.inner, src);

        let doc = super::SdkDocument::try_from(sdk_doc).expect("try from sdk document");
        assert_eq!(doc.inner, src);
    }
}
