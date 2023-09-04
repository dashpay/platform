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
        let document_query = query.query()?;
        let request: GetDocumentsRequest = document_query.clone().try_into()?;
        let drive_query: DriveQuery = (&document_query).try_into()?;

        let mut client = api.platform_client().await;
        let response: GetDocumentsResponse = request
            .execute(&mut client, RequestSettings::default())
            .await?;

        let docs =
            Documents::maybe_from_proof(&drive_query, &response, api.quorum_info_provider()?)?
                .ok_or(Error::NotFound("document not found".to_string()))?;

        if docs.len() == 1 {
            Ok(docs
                .into_iter()
                .next()
                .expect("first element must exist")
                .into())
        } else {
            Err(Error::TooManyElementsReceived {
                expected: 1,
                got: docs.len(),
            })
        }
    }
}

#[async_trait::async_trait]
impl<API: DashAPI> Listable<API> for SdkDocument {
    type Request = SdkDocumentQuery;
    async fn list<Q: SdkQuery<Self::Request>>(api: &API, query: &Q) -> Result<Vec<Self>, Error> {
        let document_query: SdkDocumentQuery = query.query()?;
        let drive_query: DriveQuery = (&document_query).try_into()?;
        let request: GetDocumentsRequest = document_query.clone().try_into()?;

        let mut client = api.platform_client().await;
        let response: GetDocumentsResponse = request
            .execute(&mut client, RequestSettings::default())
            .await?;

        let docs =
            Documents::maybe_from_proof(&drive_query, &response, api.quorum_info_provider()?)?
                .ok_or(Error::NotFound("no document found".to_string()))?
                .into_iter()
                .map(|doc| doc.into())
                .collect();

        Ok(docs)
    }
}

impl SdkQuery<GetDocumentsRequest> for SdkDocumentQuery {
    fn query(&self) -> Result<GetDocumentsRequest, Error> {
        <Self as TryInto<GetDocumentsRequest>>::try_into(self.clone()).map_err(|e| e.into())
    }
}
