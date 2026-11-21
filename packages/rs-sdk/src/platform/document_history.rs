//! Fetching a document's history from a network at either protocol version.
//!
//! Protocol 15 serves only history request V1 (revision and cursor filters, lifecycle
//! proofs) and protocol 14 only V0 (a start time). So an SDK that would send V1 first asks
//! whether the network is proven below 15, with the protocol gate's own probe, and sends
//! V0 when it is: the request the 5.0 SDK sent, with V1's inclusive start time. Its answer
//! has the same shape, without `lifecycle`, which the V0 proof does not carry.
//! [`fetch_document_history_with_lifecycle`] never falls back, for callers that need it.

use super::fetch::fetch_proved;
use super::{DocumentHistoryQuery, Fetch, ProtocolRequirement, Query};
use crate::{Error, Sdk};
use dapi_grpc::platform::v0::get_document_history_request::get_document_history_request_v1::Filter;
use dapi_grpc::platform::v0::get_document_history_request::{GetDocumentHistoryRequestV0, Version};
use dapi_grpc::platform::v0::{GetDocumentHistoryRequest, Proof, ResponseMetadata};
use drive::query::document_history_drive_query::{
    DocumentHistoryDriveQuery, DocumentHistoryFilter,
};
use drive_proof_verifier::types::{DocumentHistory, DocumentHistoryProofInfo};
use rs_dapi_client::transport::TransportError;
use rs_dapi_client::{DapiClientError, RequestSettings};

/// What needs protocol 15 in a history request, when the network is below it.
const REVISION_AND_CURSOR_FILTERS: &str = "revision and cursor history filters";

#[async_trait::async_trait]
impl Fetch for DocumentHistory {
    type Query = GetDocumentHistoryRequest;
    type Request = GetDocumentHistoryRequest;

    async fn fetch_with_metadata_and_proof<Q: Query<GetDocumentHistoryRequest>>(
        sdk: &Sdk,
        query: Q,
        request_settings: Option<RequestSettings>,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error> {
        fetch_history(sdk, query, request_settings).await
    }
}

#[async_trait::async_trait]
impl Fetch for DocumentHistoryProofInfo {
    type Query = GetDocumentHistoryRequest;
    type Request = GetDocumentHistoryRequest;

    async fn fetch_with_metadata_and_proof<Q: Query<GetDocumentHistoryRequest>>(
        sdk: &Sdk,
        query: Q,
        request_settings: Option<RequestSettings>,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error> {
        fetch_history(sdk, query, request_settings).await
    }
}

/// Fetch a page of a document's history with its authenticated lifecycle: request V1
/// only, never the V0 fallback of [`DocumentHistory::fetch`]. On a network below protocol
/// 15 it is refused with [`Error::ProtocolVersionNotActive`], so the answer always carries
/// `lifecycle`. It takes a [`DocumentHistoryQuery`], which always encodes to V1.
pub async fn fetch_document_history_with_lifecycle(
    sdk: &Sdk,
    query: DocumentHistoryQuery,
    request_settings: Option<RequestSettings>,
) -> Result<Option<DocumentHistory>, Error> {
    fetch_proved::<DocumentHistory, _>(sdk, query, request_settings)
        .await
        .map(|(history, _, _)| history)
}

/// Fetch `query` as request V0 when it encodes to V1 and the network is proven below the
/// version that serves V1 ([`Sdk::network_proven_below_with_settings`]), asked under the
/// caller's `request_settings` as the gate's own probe is.
///
/// Otherwise the request is unchanged, and the protocol gate decides whether V1 is sent: a
/// network proven at 15 serves it, and an SDK that cannot learn the network's version, or
/// is pinned below 15 on a network at 15, refuses it as before. Both versions are encoded
/// from `query` after the version check, as every fetch is.
///
/// When a node refuses V0 as a query version it no longer serves, the network may have
/// reached 15 since it was proven below it. The probe hold is dropped, the network is
/// asked once more, and V1 goes to the gate when it has reached 15. Any other failure is
/// returned as it is, after the caller's usual retries.
async fn fetch_history<T, Q>(
    sdk: &Sdk,
    query: Q,
    request_settings: Option<RequestSettings>,
) -> Result<(Option<T>, ResponseMetadata, Proof), Error>
where
    T: Fetch<Query = GetDocumentHistoryRequest> + Send,
    Q: Query<GetDocumentHistoryRequest>,
{
    let settings = sdk.query_settings();
    let request = query.query(&settings)?;
    let Some(requirement) = ProtocolRequirement::of_request(&request)
        .filter(|requirement| settings.network_protocol_basis() < requirement.protocol_version)
    else {
        return fetch_proved::<T, _>(sdk, query, request_settings).await;
    };
    let proven_below = sdk
        .network_proven_below_with_settings(requirement.protocol_version, request_settings)
        .await
        .map_err(|error| {
            sdk.protocol_version_not_active(
                requirement.protocol_version,
                requirement.method,
                None,
                Some(error),
            )
        })?;
    if !proven_below {
        return fetch_proved::<T, _>(sdk, query, request_settings).await;
    }
    // Asking the network for its version may have changed the version requests are
    // encoded with.
    let request = query.query(&sdk.query_settings())?;
    let legacy = history_request_below_protocol_15(request)?.ok_or_else(|| {
        sdk.protocol_version_not_active(
            requirement.protocol_version,
            requirement.method,
            Some(REVISION_AND_CURSOR_FILTERS),
            None,
        )
    })?;
    match fetch_proved::<T, _>(sdk, legacy, request_settings).await {
        Err(error) if refuses_history_query_version(&error) => {
            sdk.forget_probe_hold();
            if !sdk
                .network_reached_past_hold(requirement.protocol_version, request_settings)
                .await
            {
                return Err(error);
            }
            tracing::debug!(%error, "history request V0 refused on a network now at protocol 15; sending V1");
            fetch_proved::<T, _>(sdk, query, request_settings).await
        }
        answer => answer,
    }
}

/// The start of the status message Drive refuses a query version it does not serve with,
/// for history (`QueryError::UnsupportedQueryVersion`).
const HISTORY_QUERY_VERSION_REFUSAL: &str = "unsupported version for query: document_history";

/// Whether `error` is a node refusing the history request's version, as a node at protocol
/// 15 refuses V0.
fn refuses_history_query_version(error: &Error) -> bool {
    let transport = match error {
        Error::DapiClientError(DapiClientError::Transport(transport)) => transport,
        Error::DapiClientError(DapiClientError::NoAvailableAddressesToRetry(transport)) => {
            transport.as_ref()
        }
        _ => return false,
    };
    matches!(transport, TransportError::Grpc(status)
        if status.message().starts_with(HISTORY_QUERY_VERSION_REFUSAL))
}

/// The V0 form of a V1 history request, as [`DocumentHistory::fetch`] sends it to a network
/// below protocol 15: a start-time filter, the only one V0 has, one millisecond earlier.
/// `Ok(None)` for a revision or cursor filter, and an error for a request V1 itself
/// refuses (Drive's history query validation). A request that is not V1 is returned as
/// it is.
///
/// A caller checking a [`DocumentHistoryProofInfo`] it did not fetch derives the request it
/// expects with this, and passes it to [`DocumentHistoryProofInfo::verify`].
pub fn history_request_below_protocol_15(
    request: GetDocumentHistoryRequest,
) -> Result<Option<GetDocumentHistoryRequest>, Error> {
    let Some(Version::V1(v1)) = request.version else {
        return Ok(Some(request));
    };
    let Some(Filter::StartAtMs(start_at_ms)) = v1.filter else {
        return Ok(None);
    };
    let identifier = |bytes: &[u8]| {
        <[u8; 32]>::try_from(bytes)
            .map_err(|_| Error::InvalidRequest("history identifiers are 32 bytes".to_string()))
    };
    // Validated as V1 first: the V0 bound encodes differently, so an out-of-range time
    // would select another page instead of being refused.
    DocumentHistoryDriveQuery {
        contract_id: identifier(&v1.data_contract_id)?,
        document_type_name: v1.document_type_name.clone(),
        document_id: identifier(&v1.document_id)?,
        filter: DocumentHistoryFilter::StartAtTime(start_at_ms),
        limit: v1
            .limit
            .map(u16::try_from)
            .transpose()
            .map_err(|_| Error::InvalidRequest("history limit out of bounds".to_string()))?,
    }
    .validate()?;
    Ok(Some(
        GetDocumentHistoryRequestV0 {
            data_contract_id: v1.data_contract_id,
            document_type_name: v1.document_type_name,
            document_id: v1.document_id,
            limit: v1.limit,
            offset: None,
            // V1 starts at its time and V0 after it (Drive's `start_after_ms`), so V0 starts
            // one millisecond earlier. No revision is written at time 0, so 0 stays 0.
            start_at_ms: start_at_ms.saturating_sub(1),
            prove: v1.prove,
        }
        .into(),
    ))
}

#[cfg(all(test, feature = "mocks"))]
mod tests {
    use super::*;
    use crate::SdkBuilder;
    use dapi_grpc::platform::v0::get_document_history_request::GetDocumentHistoryRequestV1;
    use dapi_grpc::tonic::{Code, Status};
    use dpp::dashcore::Network;
    use dpp::document::{Document, DocumentV0};
    use dpp::version::PlatformVersion;
    use drive_proof_verifier::types::DocumentHistoryEntry;

    fn query(filter: DocumentHistoryFilter) -> DocumentHistoryQuery {
        DocumentHistoryQuery {
            data_contract_id: [1; 32].into(),
            document_type_name: "note".into(),
            document_id: [2; 32].into(),
            filter,
            limit: Some(5),
        }
    }

    /// A mock SDK pinned at protocol 14, on a network a verified answer showed at 14.
    fn sdk_at_protocol_14() -> Sdk {
        let sdk = SdkBuilder::new_mock()
            .with_network(Network::Devnet)
            .with_version(PlatformVersion::get(14).unwrap())
            .build()
            .unwrap();
        sdk.mock_verified_protocol_version(14).unwrap();
        sdk
    }

    #[tokio::test]
    async fn should_fetch_history_with_the_v0_request_below_protocol_15() {
        let mut sdk = sdk_at_protocol_14();
        let v0 = GetDocumentHistoryRequest::from(GetDocumentHistoryRequestV0 {
            data_contract_id: vec![1; 32],
            document_type_name: "note".into(),
            document_id: vec![2; 32],
            limit: Some(5),
            offset: None,
            start_at_ms: 0,
            prove: true,
        });
        let expected = DocumentHistory {
            entries: vec![DocumentHistoryEntry {
                time_ms: 2000,
                revision: 1,
                document: Document::V0(DocumentV0 {
                    revision: Some(1),
                    ..Default::default()
                }),
            }],
            lifecycle: None,
        };
        // Only the V0 request is expected: a V1 request would find no expectation.
        sdk.mock()
            .expect_fetch(v0, Some(expected.clone()))
            .await
            .unwrap();

        let history = DocumentHistory::fetch(&sdk, query(DocumentHistoryFilter::StartAtTime(0)))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(history, expected);
    }

    #[tokio::test]
    async fn should_refuse_a_revision_filter_below_protocol_15_without_sending_it() {
        let sdk = sdk_at_protocol_14();
        for filter in [
            DocumentHistoryFilter::Revision(1),
            DocumentHistoryFilter::StartAtRevision(1),
            DocumentHistoryFilter::StartAfter {
                time_ms: 2000,
                revision: 1,
            },
        ] {
            // A single revision is asked with limit one.
            let query = DocumentHistoryQuery {
                limit: Some(1),
                ..query(filter)
            };
            let v1 = query.query(&sdk.query_settings()).unwrap();
            let error = DocumentHistory::fetch(&sdk, query).await.unwrap_err();
            assert_eq!(sdk.mock_dapi().lock().await.request_count(&v1), 0);
            assert!(
                matches!(
                    error,
                    Error::ProtocolVersionNotActive {
                        method: "get_document_history",
                        required: 15,
                        feature: Some(REVISION_AND_CURSOR_FILTERS),
                        network: 14,
                        pinned: true,
                        cause: None,
                    }
                ),
                "{error:?}"
            );
        }
    }

    /// A raw V1 request bypasses the typed query's checks. One Drive would refuse is refused
    /// before it is translated, since its V0 form would select another page, and nothing
    /// is sent.
    #[tokio::test]
    async fn should_refuse_a_raw_v1_start_time_v1_refuses_without_sending_v0() {
        let sdk = sdk_at_protocol_14();
        let raw = GetDocumentHistoryRequest::from(GetDocumentHistoryRequestV1 {
            data_contract_id: vec![1; 32],
            document_type_name: "note".into(),
            document_id: vec![2; 32],
            limit: Some(5),
            prove: true,
            filter: Some(Filter::StartAtMs((1 << 63) + 1)),
        });
        let v0 = GetDocumentHistoryRequest::from(GetDocumentHistoryRequestV0 {
            data_contract_id: vec![1; 32],
            document_type_name: "note".into(),
            document_id: vec![2; 32],
            limit: Some(5),
            offset: None,
            start_at_ms: 1 << 63,
            prove: true,
        });
        let error = DocumentHistory::fetch(&sdk, raw).await.unwrap_err();
        assert!(matches!(error, Error::Drive(_)), "{error:?}");
        assert_eq!(sdk.mock_dapi().lock().await.request_count(&v0), 0);
    }

    #[test]
    fn should_tell_a_history_version_refusal_from_other_errors() {
        let status = |code: Code, message: &str| {
            Error::DapiClientError(DapiClientError::Transport(TransportError::Grpc(
                Status::new(code, message),
            )))
        };
        let refusal = "unsupported version for query: document_history, currently supporting \
                       versions 1 to 1 on platform protocol 15, given 0";
        assert!(refuses_history_query_version(&status(
            Code::Unknown,
            refusal
        )));
        assert!(refuses_history_query_version(&Error::DapiClientError(
            DapiClientError::NoAvailableAddressesToRetry(Box::new(TransportError::Grpc(
                Status::unknown(refusal)
            )))
        )));
        assert!(!refuses_history_query_version(&status(
            Code::Unknown,
            "unsupported version for query: identity_keys_remaining_budgets, currently \
             supporting versions 0 to 0 on platform protocol 14, given 1"
        )));
        assert!(!refuses_history_query_version(&status(
            Code::NotFound,
            "document not found"
        )));
        assert!(!refuses_history_query_version(&Error::Generic(
            refusal.to_string()
        )));
    }

    #[test]
    fn should_start_v0_at_zero_for_a_start_time_of_zero() {
        let settings_source = SdkBuilder::new_mock().build().unwrap();
        let v1 = query(DocumentHistoryFilter::StartAtTime(0))
            .query(&settings_source.query_settings())
            .unwrap();
        let Some(Version::V0(v0)) = history_request_below_protocol_15(v1)
            .unwrap()
            .unwrap()
            .version
        else {
            panic!("start-time requests become V0");
        };
        assert_eq!(v0.start_at_ms, 0);
    }

    #[test]
    fn should_keep_every_field_of_a_start_time_request_in_v0() {
        let settings_source = SdkBuilder::new_mock()
            .with_version(PlatformVersion::latest())
            .build()
            .unwrap();
        let v1 = query(DocumentHistoryFilter::StartAtTime(1234))
            .query(&settings_source.query_settings())
            .unwrap();
        assert!(matches!(v1.version, Some(Version::V1(_))));
        let Some(Version::V0(v0)) = history_request_below_protocol_15(v1)
            .unwrap()
            .unwrap()
            .version
        else {
            panic!("start-time requests become V0");
        };
        assert_eq!(
            v0,
            GetDocumentHistoryRequestV0 {
                data_contract_id: vec![1; 32],
                document_type_name: "note".into(),
                document_id: vec![2; 32],
                limit: Some(5),
                offset: None,
                // V0 starts after its time, V1 at it.
                start_at_ms: 1233,
                prove: true,
            }
        );
    }
}
