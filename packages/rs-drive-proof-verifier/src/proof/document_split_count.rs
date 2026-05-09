use crate::error::MapGroveDbError;
use crate::verify::verify_tenderdash_proof;
use crate::{ContextProvider, Error, FromProof};
use dapi_grpc::platform::v0::{GetDocumentsCountResponse, Proof, ResponseMetadata};
use dapi_grpc::platform::VersionedGrpcResponse;
use dpp::dashcore::Network;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::document::Document;
use dpp::document::DocumentV0Getters;
use dpp::version::PlatformVersion;
use drive::query::DriveDocumentQuery;
use std::collections::BTreeMap;

/// The split counts of documents matching a query, verified from proof.
/// Maps property value bytes to count.
///
/// The keys are the byte form of each split-property value as produced by
/// [`DocumentTypeBasicMethods::serialize_value_for_key`], so they line up
/// with the keys returned on the no-proof / CountTree path.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DocumentSplitCounts(pub BTreeMap<Vec<u8>, u64>);

/// Reject the generic [`FromProof`] entry point for [`DocumentSplitCounts`].
///
/// Splitting requires the split-property name, which isn't carried by
/// `DriveDocumentQuery`. Earlier versions of this impl silently returned an
/// empty map under proof, which made `prove=true` callers think there were
/// zero documents per group. To stop that footgun, the generic
/// [`FromProof`] now returns an explicit error; SDK-level callers must use
/// [`DocumentSplitCounts::maybe_from_proof_with_split_property`] (or, in
/// `rs-sdk`, the [`Fetch`](dash_sdk::platform::Fetch) impl on
/// `DocumentSplitCountQuery`) which threads the split property through.
impl<'dq, Q> FromProof<Q> for DocumentSplitCounts
where
    Q: TryInto<DriveDocumentQuery<'dq>> + Clone + 'dq,
    Q::Error: std::fmt::Display,
{
    type Request = Q;
    type Response = GetDocumentsCountResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        _request: I,
        _response: O,
        _network: Network,
        _platform_version: &PlatformVersion,
        _provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error>
    where
        Self: 'a,
    {
        Err(Error::RequestError {
            error: "DocumentSplitCounts requires a split-property; call \
                 DocumentSplitCounts::maybe_from_proof_with_split_property \
                 (or use the rs-sdk Fetch impl on DocumentSplitCountQuery)"
                .to_string(),
        })
    }
}

impl DocumentSplitCounts {
    /// Verify a `GetDocumentsCount` proof and aggregate the verified
    /// documents into per-key counts using `split_property` as the grouping
    /// key.
    ///
    /// `Q` is anything that can be turned into a [`DriveDocumentQuery`] —
    /// typically a `DocumentSplitCountQuery` from `rs-sdk` or a
    /// `DriveDocumentQuery` directly.
    ///
    /// Returns `(Some(splits), metadata, proof)` even when no documents
    /// matched (in which case `splits.0` is empty).
    pub fn maybe_from_proof_with_split_property<'dq, 'a, Q, I, O>(
        request: I,
        split_property: &str,
        response: O,
        _network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error>
    where
        Q: TryInto<DriveDocumentQuery<'dq>> + Clone + 'dq,
        Q::Error: std::fmt::Display,
        I: Into<Q>,
        O: Into<GetDocumentsCountResponse>,
        Self: 'a,
    {
        let request: Q = request.into();
        let response: GetDocumentsCountResponse = response.into();

        let drive_query: DriveDocumentQuery<'dq> =
            request
                .clone()
                .try_into()
                .map_err(|e: Q::Error| Error::RequestError {
                    error: e.to_string(),
                })?;

        let proof = response.proof().or(Err(Error::NoProofInResult))?;
        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let (root_hash, documents) = drive_query
            .verify_proof(&proof.grovedb_proof, platform_version)
            .map_drive_error(proof, mtd)?;

        verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

        let aggregated = aggregate_documents_by_property(
            &documents,
            drive_query.document_type,
            split_property,
            platform_version,
        )?;

        Ok((
            Some(DocumentSplitCounts(aggregated)),
            mtd.clone(),
            proof.clone(),
        ))
    }
}

/// Group documents by the byte-encoded value of `split_property` and return
/// the per-key counts. Documents that don't carry the property are skipped
/// (mirroring the server-side CountTree path, which only counts documents
/// whose primary-key tree path includes the property).
fn aggregate_documents_by_property(
    documents: &[Document],
    document_type: dpp::data_contract::document_type::DocumentTypeRef<'_>,
    split_property: &str,
    platform_version: &PlatformVersion,
) -> Result<BTreeMap<Vec<u8>, u64>, Error> {
    let mut counts: BTreeMap<Vec<u8>, u64> = BTreeMap::new();

    for document in documents {
        let value = match document.properties().get(split_property) {
            Some(v) => v,
            None => continue,
        };

        let key = document_type
            .serialize_value_for_key(split_property, value, platform_version)
            .map_err(|e| Error::ResponseDecodeError {
                error: format!(
                    "Failed to serialize split property `{}` for grouping: {}",
                    split_property, e
                ),
            })?;

        *counts.entry(key).or_insert(0) += 1;
    }

    Ok(counts)
}

// Aggregation unit tests live in higher-level crates with full test fixtures:
//   - SDK: packages/rs-sdk/tests/fetch/document_split_count.rs
//   - drive-abci: src/query/document_split_count_query/v0/mod.rs tests
// (drive-proof-verifier's feature surface doesn't expose dpp test helpers)
