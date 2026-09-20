//! Proof verification of the contract moderation queries.

use crate::error::MapGroveDbError;
use crate::types::contract_moderation::{
    entries_query_from_request, identifier_from_request, lists_from_request,
    ContractModerationEntries, ContractModerationListStatuses,
};
use crate::verify::{supported_grovedb_proof_bytes, verify_tenderdash_proof};
use crate::{ContextProvider, Error, FromProof};
use dapi_grpc::platform::v0::{
    get_contract_moderation_entries_request, get_contract_moderation_status_request,
    GetContractModerationEntriesRequest, GetContractModerationEntriesResponse,
    GetContractModerationStatusRequest, GetContractModerationStatusResponse, Proof,
    ResponseMetadata,
};
use dapi_grpc::platform::VersionedGrpcResponse;
use dpp::dashcore::Network;
use dpp::version::PlatformVersion;
use drive::drive::Drive;

impl FromProof<GetContractModerationStatusRequest> for ContractModerationListStatuses {
    type Request = GetContractModerationStatusRequest;
    type Response = GetContractModerationStatusResponse;

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

        let get_contract_moderation_status_request::Version::V0(v0) =
            request.version.ok_or(Error::EmptyVersion)?;
        let contract_id = identifier_from_request(&v0.contract_id, "contract_id")?;
        let identity_id = identifier_from_request(&v0.identity_id, "identity_id")?;
        let lists = lists_from_request(&v0.lists)?;

        let metadata = response
            .metadata()
            .or(Err(Error::EmptyResponseMetadata))?
            .clone();
        let proof = response.proof_owned().or(Err(Error::NoProofInResult))?;

        let (root_hash, statuses) = Drive::verify_contract_moderation_status(
            supported_grovedb_proof_bytes(&proof, platform_version)?,
            contract_id,
            identity_id,
            &lists,
            platform_version,
        )
        .map_drive_error(&proof, &metadata)?;

        verify_tenderdash_proof(&proof, &metadata, &root_hash, provider, platform_version)?;

        // An absent entry is a status too: the identity is not on that list. Only the lists
        // queried were proved, and the verifier reports only those.
        Ok((Some(statuses), metadata, proof))
    }
}

impl FromProof<GetContractModerationEntriesRequest> for ContractModerationEntries {
    type Request = GetContractModerationEntriesRequest;
    type Response = GetContractModerationEntriesResponse;

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

        let get_contract_moderation_entries_request::Version::V0(v0) =
            request.version.ok_or(Error::EmptyVersion)?;
        let contract_id = identifier_from_request(&v0.contract_id, "contract_id")?;
        let query = entries_query_from_request(
            v0.list,
            v0.start_after.as_deref(),
            v0.limit,
            platform_version,
        )?;

        let metadata = response
            .metadata()
            .or(Err(Error::EmptyResponseMetadata))?
            .clone();
        let proof = response.proof_owned().or(Err(Error::NoProofInResult))?;

        let (root_hash, entries) = Drive::verify_contract_moderation_entries(
            supported_grovedb_proof_bytes(&proof, platform_version)?,
            contract_id,
            &query,
            platform_version,
        )
        .map_drive_error(&proof, &metadata)?;

        verify_tenderdash_proof(&proof, &metadata, &root_hash, provider, platform_version)?;

        // An empty list proves as an empty page, so the page itself is always the answer.
        Ok((Some(ContractModerationEntries(entries)), metadata, proof))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dapi_grpc::platform::v0::get_contract_moderation_entries_request::GetContractModerationEntriesRequestV0;
    use dapi_grpc::platform::v0::get_contract_moderation_entries_response::{
        get_contract_moderation_entries_response_v0::Result as EntriesResult,
        GetContractModerationEntriesResponseV0, Version as EntriesResponseVersion,
    };
    use dapi_grpc::platform::v0::get_contract_moderation_status_request::GetContractModerationStatusRequestV0;
    use dapi_grpc::platform::v0::get_contract_moderation_status_response::{
        get_contract_moderation_status_response_v0::Result as StatusResult,
        GetContractModerationStatusResponseV0, Version as StatusResponseVersion,
    };
    use dash_context_provider::ContextProviderError;
    use dpp::data_contract::TokenConfiguration;
    use dpp::identifier::Identifier;
    use dpp::prelude::{CoreBlockHeight, DataContract};
    use std::sync::Arc;

    /// Context provider that panics if called: every test here fails before the Tenderdash
    /// proof is checked, so the provider must stay unreachable.
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

    fn status_request(
        contract_id: Vec<u8>,
        identity_id: Vec<u8>,
        lists: Vec<i32>,
    ) -> GetContractModerationStatusRequest {
        GetContractModerationStatusRequest {
            version: Some(get_contract_moderation_status_request::Version::V0(
                GetContractModerationStatusRequestV0 {
                    contract_id,
                    identity_id,
                    lists,
                    prove: true,
                },
            )),
        }
    }

    fn status_response(result: Option<StatusResult>) -> GetContractModerationStatusResponse {
        GetContractModerationStatusResponse {
            version: Some(StatusResponseVersion::V0(
                GetContractModerationStatusResponseV0 {
                    result,
                    metadata: Some(ResponseMetadata::default()),
                },
            )),
        }
    }

    fn entries_request(
        contract_id: Vec<u8>,
        list: i32,
        start_after: Option<Vec<u8>>,
        limit: Option<u32>,
    ) -> GetContractModerationEntriesRequest {
        GetContractModerationEntriesRequest {
            version: Some(get_contract_moderation_entries_request::Version::V0(
                GetContractModerationEntriesRequestV0 {
                    contract_id,
                    list,
                    start_after,
                    limit,
                    prove: true,
                },
            )),
        }
    }

    fn entries_response(result: Option<EntriesResult>) -> GetContractModerationEntriesResponse {
        GetContractModerationEntriesResponse {
            version: Some(EntriesResponseVersion::V0(
                GetContractModerationEntriesResponseV0 {
                    result,
                    metadata: Some(ResponseMetadata::default()),
                },
            )),
        }
    }

    fn status_error(
        request: GetContractModerationStatusRequest,
        response: GetContractModerationStatusResponse,
    ) -> Error {
        <ContractModerationListStatuses as FromProof<_>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            PlatformVersion::latest(),
            &UnreachableProvider,
        )
        .unwrap_err()
    }

    fn entries_error(
        request: GetContractModerationEntriesRequest,
        response: GetContractModerationEntriesResponse,
    ) -> Error {
        <ContractModerationEntries as FromProof<_>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            PlatformVersion::latest(),
            &UnreachableProvider,
        )
        .unwrap_err()
    }

    #[test]
    fn status_should_fail_with_empty_version_when_request_has_no_version() {
        let err = status_error(
            GetContractModerationStatusRequest { version: None },
            status_response(Some(StatusResult::Proof(Proof::default()))),
        );
        assert!(matches!(err, Error::EmptyVersion), "got: {err:?}");
    }

    #[test]
    fn status_should_reject_malformed_requests() {
        let id = vec![1; 32];
        for (request, needle) in [
            (
                status_request(vec![1; 5], id.clone(), vec![1]),
                "contract_id",
            ),
            (
                status_request(id.clone(), vec![1; 5], vec![1]),
                "identity_id",
            ),
            (
                status_request(id.clone(), id.clone(), vec![]),
                "at least one",
            ),
            (status_request(id.clone(), id.clone(), vec![1, 1]), "twice"),
            (
                status_request(id.clone(), id.clone(), vec![9]),
                "not a moderation list",
            ),
            (
                status_request(id.clone(), id.clone(), vec![0]),
                "not a moderation list",
            ),
        ] {
            let err = status_error(
                request,
                status_response(Some(StatusResult::Proof(Proof::default()))),
            );
            assert!(
                matches!(&err, Error::RequestError { error } if error.contains(needle)),
                "{needle}: {err:?}"
            );
        }
    }

    #[test]
    fn status_should_fail_without_proof_when_response_carries_none() {
        let err = status_error(
            status_request(vec![1; 32], vec![2; 32], vec![1]),
            status_response(None),
        );
        assert!(matches!(err, Error::NoProofInResult), "got: {err:?}");
    }

    #[test]
    fn status_should_fail_on_a_proof_that_does_not_verify() {
        let err = status_error(
            status_request(vec![1; 32], vec![2; 32], vec![1, 2]),
            status_response(Some(StatusResult::Proof(Proof::default()))),
        );
        assert!(
            !matches!(err, Error::RequestError { .. } | Error::NoProofInResult),
            "got: {err:?}"
        );
    }

    #[test]
    fn entries_should_fail_with_empty_version_when_request_has_no_version() {
        let err = entries_error(
            GetContractModerationEntriesRequest { version: None },
            entries_response(Some(EntriesResult::Proof(Proof::default()))),
        );
        assert!(matches!(err, Error::EmptyVersion), "got: {err:?}");
    }

    #[test]
    fn entries_should_reject_malformed_requests() {
        let id = vec![1; 32];
        for (request, needle) in [
            (entries_request(vec![1; 5], 1, None, None), "contract_id"),
            (
                entries_request(id.clone(), 9, None, None),
                "not a moderation list",
            ),
            (
                entries_request(id.clone(), 0, None, None),
                "not a moderation list",
            ),
            (
                entries_request(id.clone(), 1, Some(vec![1; 5]), None),
                "start_after",
            ),
            (
                entries_request(id.clone(), 1, None, Some(70_000)),
                "out of bounds",
            ),
        ] {
            let err = entries_error(
                request,
                entries_response(Some(EntriesResult::Proof(Proof::default()))),
            );
            assert!(
                matches!(&err, Error::RequestError { error } if error.contains(needle)),
                "{needle}: {err:?}"
            );
        }
    }

    #[test]
    fn entries_should_fail_without_proof_when_response_carries_none() {
        let err = entries_error(
            entries_request(vec![1; 32], 1, None, None),
            entries_response(None),
        );
        assert!(matches!(err, Error::NoProofInResult), "got: {err:?}");
    }

    #[test]
    fn entries_should_fail_on_a_proof_that_does_not_verify() {
        let err = entries_error(
            entries_request(vec![1; 32], 2, Some(vec![2; 32]), Some(10)),
            entries_response(Some(EntriesResult::Proof(Proof::default()))),
        );
        assert!(
            !matches!(err, Error::RequestError { .. } | Error::NoProofInResult),
            "got: {err:?}"
        );
    }
}
