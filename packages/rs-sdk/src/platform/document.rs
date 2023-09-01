//! Document manipulation

use dapi_grpc::platform::v0::{GetDocumentsRequest, GetDocumentsResponse};
use drive::query::DriveQuery;
use rs_dapi_client::{DapiRequest, RequestSettings};

use crate::{
    crud::{Listable, ObjectQuery, Readable},
    dapi::DashAPI,
    error::Error,
};
use drive_proof_verifier::proof::from_proof::{Documents, FromProof, Length};

use super::document_query::DocumentQuery;

/// A document
#[derive(Debug, Clone)]
pub struct Document {
    inner: dpp::document::Document,
}

impl From<Document> for dpp::document::Document {
    fn from(doc: Document) -> Self {
        doc.inner
    }
}

impl From<dpp::document::Document> for Document {
    fn from(doc: dpp::document::Document) -> Self {
        Self { inner: doc }
    }
}

impl Length for Document {
    fn count_some(&self) -> usize {
        1
    }
}

#[async_trait::async_trait]
impl<API: DashAPI> Readable<API> for Document {
    type Identifier = DocumentQuery;

    async fn read<Q: ObjectQuery<Self::Identifier>>(api: &API, query: &Q) -> Result<Self, Error> {
        let document_query = query.query()?;
        let request: GetDocumentsRequest = document_query.clone().try_into()?;
        let drive_query: DriveQuery = (&document_query).try_into()?;

        let mut client = api.platform_client().await;
        let response: GetDocumentsResponse = request
            .execute(&mut client, RequestSettings::default())
            .await?;

        let docs =
            Documents::maybe_from_proof(&drive_query, &response, api.quorum_info_provider()?)?
                .ok_or(Error::NotFound)?;

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
impl<API: DashAPI> Listable<API> for Document {
    type Request = DocumentQuery;
    async fn list<Q: ObjectQuery<Self::Request>>(api: &API, query: &Q) -> Result<Vec<Self>, Error> {
        let document_query: DocumentQuery = query.query()?;
        let drive_query: DriveQuery = (&document_query).try_into()?;
        let request: GetDocumentsRequest = document_query.clone().try_into()?;

        let mut client = api.platform_client().await;
        let response: GetDocumentsResponse = request
            .execute(&mut client, RequestSettings::default())
            .await?;

        let docs =
            Documents::maybe_from_proof(&drive_query, &response, api.quorum_info_provider()?)?
                .ok_or(Error::NotFound)?
                .into_iter()
                .map(|doc| doc.into())
                .collect();

        Ok(docs)
    }
}

impl ObjectQuery<GetDocumentsRequest> for DocumentQuery {
    fn query(&self) -> Result<GetDocumentsRequest, Error> {
        <Self as TryInto<GetDocumentsRequest>>::try_into(self.clone()).map_err(|e| e.into())
    }
}
