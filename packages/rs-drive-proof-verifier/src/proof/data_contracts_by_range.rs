use crate::error::MapGroveDbError;
use crate::types::data_contracts_by_range::DataContractsByRange;
use crate::verify::{supported_grovedb_proof_bytes, verify_tenderdash_proof};
use crate::{ContextProvider, Error, FromProof};
use dapi_grpc::platform::v0::get_data_contracts_by_range_request::get_data_contracts_by_range_request_v0::Start;
use dapi_grpc::platform::v0::{
    get_data_contracts_by_range_request, GetDataContractsByRangeRequest,
    GetDataContractsResponse, Proof, ResponseMetadata,
};
use dapi_grpc::platform::VersionedGrpcResponse;
use dpp::dashcore::Network;
use dpp::version::PlatformVersion;
use drive::drive::Drive;

/// The page a `getDataContractsByRange` request asked for, in the verifier's terms.
struct PageParameters {
    start_at: Option<([u8; 32], bool)>,
    limit: u16,
    ids_only: bool,
}

/// Reads the page parameters back from the request, applying the same defaults and bounds
/// as the server so that the verifier rebuilds the exact query that was proved: an omitted
/// limit is the protocol page cap (`max_returned_elements`), which is also the upper bound.
fn page_parameters(
    request: GetDataContractsByRangeRequest,
    platform_version: &PlatformVersion,
) -> Result<PageParameters, Error> {
    let get_data_contracts_by_range_request::Version::V0(v0) =
        request.version.ok_or(Error::EmptyVersion)?;

    let max_returned_elements = platform_version.drive_abci.query.max_returned_elements;
    let limit = match v0.limit {
        None => max_returned_elements,
        Some(requested) => match u16::try_from(requested) {
            Ok(limit) if (1..=max_returned_elements).contains(&limit) => limit,
            _ => {
                return Err(Error::RequestError {
                    error: format!(
                        "limit {requested} is out of bounds, it must be between 1 and {max_returned_elements}"
                    ),
                })
            }
        },
    };

    let start_at = match v0.start {
        None => None,
        Some(Start::StartAfter(cursor)) => {
            Some((contract_id_from_cursor(cursor, "start_after")?, false))
        }
        Some(Start::StartAt(cursor)) => Some((contract_id_from_cursor(cursor, "start_at")?, true)),
    };

    Ok(PageParameters {
        start_at,
        limit,
        ids_only: v0.ids_only,
    })
}

fn contract_id_from_cursor(cursor: Vec<u8>, field: &str) -> Result<[u8; 32], Error> {
    cursor
        .try_into()
        .map_err(|cursor: Vec<u8>| Error::RequestError {
            error: format!(
                "{field} must be a 32 byte contract id, got {} bytes",
                cursor.len()
            ),
        })
}

impl FromProof<GetDataContractsByRangeRequest> for DataContractsByRange {
    type Request = GetDataContractsByRangeRequest;
    type Response = GetDataContractsResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        _network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error>
    where
        Self: Sized + 'a,
    {
        let request: Self::Request = request.into();
        let response: Self::Response = response.into();

        let PageParameters {
            start_at,
            limit,
            ids_only,
        } = page_parameters(request, platform_version)?;

        let metadata = response
            .metadata()
            .or(Err(Error::EmptyResponseMetadata))?
            .clone();
        let proof = response.proof_owned().or(Err(Error::NoProofInResult))?;

        let (root_hash, page) = Drive::verify_contracts_by_range(
            supported_grovedb_proof_bytes(&proof, platform_version)?,
            start_at,
            limit,
            ids_only,
            platform_version,
        )
        .map_drive_error(&proof, &metadata)?;

        verify_tenderdash_proof(&proof, &metadata, &root_hash, provider, platform_version)?;

        // An empty page is a proved result in its own right: no contract exists at or after
        // the cursor. It is never `None`.
        let page: DataContractsByRange = page.into_iter().collect();

        Ok((Some(page), metadata, proof))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dapi_grpc::platform::v0::get_data_contracts_by_range_request::{
        GetDataContractsByRangeRequestV0, Version as ReqVersion,
    };
    use dapi_grpc::platform::v0::get_data_contracts_response::{
        get_data_contracts_response_v0::Result as RespResult, GetDataContractsResponseV0,
        Version as RespVersion,
    };
    use dash_context_provider::ContextProviderError;
    use dpp::data_contract::TokenConfiguration;
    use dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
    use std::sync::Arc;

    /// Context provider that panics if called: every test here fails on the request before
    /// proof verification, so the provider must stay unreachable.
    struct UnreachableProvider;

    impl ContextProvider for UnreachableProvider {
        fn get_data_contract(
            &self,
            _id: &Identifier,
            _pv: &PlatformVersion,
        ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
            panic!("context provider should not be called")
        }

        fn get_token_configuration(
            &self,
            _id: &Identifier,
        ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
            panic!("context provider should not be called")
        }

        fn get_quorum_public_key(
            &self,
            _qt: u32,
            _qh: [u8; 32],
            _h: u32,
        ) -> Result<[u8; 48], ContextProviderError> {
            panic!("context provider should not be called")
        }

        fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
            panic!("context provider should not be called")
        }
    }

    fn pv() -> &'static PlatformVersion {
        PlatformVersion::latest()
    }

    fn response_with_proof_and_metadata() -> GetDataContractsResponse {
        GetDataContractsResponse {
            version: Some(RespVersion::V0(GetDataContractsResponseV0 {
                result: Some(RespResult::Proof(Proof::default())),
                metadata: Some(ResponseMetadata::default()),
            })),
        }
    }

    fn request(v0: GetDataContractsByRangeRequestV0) -> GetDataContractsByRangeRequest {
        GetDataContractsByRangeRequest {
            version: Some(ReqVersion::V0(v0)),
        }
    }

    fn page_v0(limit: Option<u32>, start: Option<Start>) -> GetDataContractsByRangeRequestV0 {
        GetDataContractsByRangeRequestV0 {
            limit,
            start,
            ids_only: false,
            prove: true,
        }
    }

    fn from_proof_error(request: GetDataContractsByRangeRequest) -> Error {
        <DataContractsByRange as FromProof<_>>::maybe_from_proof(
            request,
            response_with_proof_and_metadata(),
            Network::Testnet,
            pv(),
            &UnreachableProvider,
        )
        .unwrap_err()
    }

    #[test]
    fn should_fail_with_empty_version_when_request_has_no_version() {
        let err = from_proof_error(GetDataContractsByRangeRequest { version: None });
        assert!(matches!(err, Error::EmptyVersion), "got: {err:?}");
    }

    #[test]
    fn should_reject_start_after_that_is_not_a_contract_id() {
        let err = from_proof_error(request(page_v0(
            None,
            Some(Start::StartAfter(vec![0u8; 8])),
        )));
        assert!(
            matches!(err, Error::RequestError { ref error } if error.contains("start_after")),
            "got: {err:?}"
        );
    }

    #[test]
    fn should_reject_zero_limit() {
        let err = from_proof_error(request(page_v0(Some(0), None)));
        assert!(matches!(err, Error::RequestError { .. }), "got: {err:?}");
    }

    #[test]
    fn should_reject_limit_above_the_page_cap() {
        let above_cap = pv().drive_abci.query.max_returned_elements as u32 + 1;
        let err = from_proof_error(request(page_v0(Some(above_cap), None)));
        assert!(matches!(err, Error::RequestError { .. }), "got: {err:?}");
    }
}
