//! Document manipulation

use dapi_grpc::platform::v0::{
    get_documents_request::Start, GetDocumentsRequest, GetDocumentsResponse,
};
use dpp::{
    platform_value::platform_value,
    prelude::{DataContract, Identifier},
};
use drive::query::{DriveQuery, OrderClause, WhereClause, WhereOperator};
use rs_dapi_client::{DapiRequest, RequestSettings};

use crate::{
    crud::{ObjectQuery, Readable},
    dapi::DAPI,
    error::Error,
};
use drive_proof_verifier::proof::from_proof::{Documents, FromProof};

use super::document_query::DocumentQuery;

/// A document
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
/// `DocumentIdentifier` represents a unique identifier for a Document.
pub struct DocumentIdentifier {
    /// `data_contract` is the DataContract associated with the Document.
    pub data_contract: DataContract,

    /// `document_type_name` is the name of the document type.
    pub document_type_name: String,

    /// `id` is the unique Identifier for the Document.
    pub id: Identifier,
}

impl ObjectQuery<super::document_query::DocumentQuery> for DocumentIdentifier {
    fn query(&self) -> Result<super::document_query::DocumentQuery, Error> {
        let where_clauses = vec![WhereClause {
            field: "id".to_string(),
            operator: WhereOperator::Equal,
            value: platform_value!(self.id),
        }];

        // Order clause
        let order_by_clauses = vec![OrderClause {
            ascending: true,
            field: "id".to_string(),
        }];

        Ok(DocumentQuery {
            data_contract: self.data_contract.clone(),
            document_type_name: self.document_type_name.clone(),
            where_clauses,
            order_by_clauses,
            start: Some(Start::StartAt(self.id.to_vec())),
            limit: 1,
        })
    }
}

#[async_trait::async_trait]
impl<A: DAPI> Readable<A, DocumentQuery, DocumentIdentifier> for Document {
    async fn read(api: &A, query: &DocumentIdentifier) -> Result<Self, Error> {
        let document_query = query.query()?;
        let request: GetDocumentsRequest = (&document_query).try_into()?;
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
