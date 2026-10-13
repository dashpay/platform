//! Proof verification of the contract moderation queries and of the fee pots query.

use crate::error::MapGroveDbError;
use crate::types::contract_moderation::{
    entries_query_from_request, identifier_from_request, lists_from_request,
    removals_query_from_request, team_action_status_from_request, team_actions_query_from_request,
    ContractDocumentRemovals, ContractFeePots, ContractModerationActionCounts,
    ContractModerationEntries, ContractModerationListStatuses, ContractTeamActionSigners,
    ContractTeamActions, CONTRACT_FEE_POTS_QUERIED,
};
use crate::verify::{supported_grovedb_proof_bytes, verify_tenderdash_proof};
use crate::{ContextProvider, Error, FromProof};
use dapi_grpc::platform::v0::{
    get_contract_document_removals_request, get_contract_fee_pots_request,
    get_contract_moderation_action_counts_request, get_contract_moderation_entries_request,
    get_contract_moderation_status_request, get_contract_team_action_signers_request,
    get_contract_team_actions_request, GetContractDocumentRemovalsRequest,
    GetContractDocumentRemovalsResponse, GetContractFeePotsRequest, GetContractFeePotsResponse,
    GetContractModerationActionCountsRequest, GetContractModerationActionCountsResponse,
    GetContractModerationEntriesRequest, GetContractModerationEntriesResponse,
    GetContractModerationStatusRequest, GetContractModerationStatusResponse,
    GetContractTeamActionSignersRequest, GetContractTeamActionSignersResponse,
    GetContractTeamActionsRequest, GetContractTeamActionsResponse, Proof, ResponseMetadata,
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
            supported_grovedb_proof_bytes(&proof)?,
            contract_id,
            identity_id,
            &lists,
            false,
            platform_version,
        )
        .map_drive_error(&proof, &metadata)?;

        verify_tenderdash_proof(&proof, &metadata, &root_hash, provider)?;

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
            supported_grovedb_proof_bytes(&proof)?,
            contract_id,
            &query,
            platform_version,
        )
        .map_drive_error(&proof, &metadata)?;

        verify_tenderdash_proof(&proof, &metadata, &root_hash, provider)?;

        // An empty list proves as an empty page, so the page itself is always the answer.
        Ok((Some(ContractModerationEntries(entries)), metadata, proof))
    }
}

impl FromProof<GetContractFeePotsRequest> for ContractFeePots {
    type Request = GetContractFeePotsRequest;
    type Response = GetContractFeePotsResponse;

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

        let get_contract_fee_pots_request::Version::V0(v0) =
            request.version.ok_or(Error::EmptyVersion)?;
        let contract_id = identifier_from_request(&v0.contract_id, "contract_id")?;

        let metadata = response
            .metadata()
            .or(Err(Error::EmptyResponseMetadata))?
            .clone();
        let proof = response.proof_owned().or(Err(Error::NoProofInResult))?;

        let (root_hash, pots) = Drive::verify_contract_fee_pots(
            supported_grovedb_proof_bytes(&proof)?,
            contract_id,
            &CONTRACT_FEE_POTS_QUERIED,
            false,
            platform_version,
        )
        .map_drive_error(&proof, &metadata)?;

        verify_tenderdash_proof(&proof, &metadata, &root_hash, provider)?;

        // A pot nothing was ever paid into proves as absent and reads as zero credits, so the
        // pots themselves are always the answer. The proof says nothing about the contract: a
        // node refuses the query for a contract it does not hold before it proves anything.
        Ok((Some(pots), metadata, proof))
    }
}

impl FromProof<GetContractDocumentRemovalsRequest> for ContractDocumentRemovals {
    type Request = GetContractDocumentRemovalsRequest;
    type Response = GetContractDocumentRemovalsResponse;

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

        let get_contract_document_removals_request::Version::V0(v0) =
            request.version.ok_or(Error::EmptyVersion)?;
        let contract_id = identifier_from_request(&v0.contract_id, "contract_id")?;
        let query =
            removals_query_from_request(v0.document_type_name, v0.selection, platform_version)?;

        let metadata = response
            .metadata()
            .or(Err(Error::EmptyResponseMetadata))?
            .clone();
        let proof = response.proof_owned().or(Err(Error::NoProofInResult))?;

        let (root_hash, removals) = Drive::verify_contract_document_removals(
            supported_grovedb_proof_bytes(&proof)?,
            contract_id,
            &query,
            false,
            platform_version,
        )
        .map_drive_error(&proof, &metadata)?;

        verify_tenderdash_proof(&proof, &metadata, &root_hash, provider)?;

        // A document type nobody moderated proves as no records at all, and an id with no
        // record is proved absent rather than missing, so the records are always the answer.
        Ok((Some(ContractDocumentRemovals(removals)), metadata, proof))
    }
}

impl FromProof<GetContractTeamActionsRequest> for ContractTeamActions {
    type Request = GetContractTeamActionsRequest;
    type Response = GetContractTeamActionsResponse;

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

        let get_contract_team_actions_request::Version::V0(v0) =
            request.version.ok_or(Error::EmptyVersion)?;
        let contract_id = identifier_from_request(&v0.contract_id, "contract_id")?;
        let query = team_actions_query_from_request(
            v0.status,
            v0.start_at_action_id,
            v0.count,
            platform_version,
        )?;

        let metadata = response
            .metadata()
            .or(Err(Error::EmptyResponseMetadata))?
            .clone();
        let proof = response.proof_owned().or(Err(Error::NoProofInResult))?;

        let (root_hash, actions) = Drive::verify_contract_team_actions(
            supported_grovedb_proof_bytes(&proof)?,
            contract_id,
            &query,
            false,
            platform_version,
        )
        .map_drive_error(&proof, &metadata)?;

        verify_tenderdash_proof(&proof, &metadata, &root_hash, provider)?;

        // A page past the last action proves as no actions at all, so the actions are always
        // the answer.
        Ok((Some(ContractTeamActions(actions)), metadata, proof))
    }
}

impl FromProof<GetContractTeamActionSignersRequest> for ContractTeamActionSigners {
    type Request = GetContractTeamActionSignersRequest;
    type Response = GetContractTeamActionSignersResponse;

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

        let get_contract_team_action_signers_request::Version::V0(v0) =
            request.version.ok_or(Error::EmptyVersion)?;
        let contract_id = identifier_from_request(&v0.contract_id, "contract_id")?;
        let action_id = identifier_from_request(&v0.action_id, "action_id")?;
        let status = team_action_status_from_request(v0.status)?;

        let metadata = response
            .metadata()
            .or(Err(Error::EmptyResponseMetadata))?
            .clone();
        let proof = response.proof_owned().or(Err(Error::NoProofInResult))?;

        let (root_hash, signers) = Drive::verify_contract_team_action_signers(
            supported_grovedb_proof_bytes(&proof)?,
            contract_id,
            status,
            action_id,
            false,
            platform_version,
        )
        .map_drive_error(&proof, &metadata)?;

        verify_tenderdash_proof(&proof, &metadata, &root_hash, provider)?;

        // An action that is not there with that status proves as no approvals, so the approvals
        // are always the answer.
        Ok((Some(ContractTeamActionSigners(signers)), metadata, proof))
    }
}

impl FromProof<GetContractModerationActionCountsRequest> for ContractModerationActionCounts {
    type Request = GetContractModerationActionCountsRequest;
    type Response = GetContractModerationActionCountsResponse;

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

        let get_contract_moderation_action_counts_request::Version::V0(v0) =
            request.version.ok_or(Error::EmptyVersion)?;
        let contract_id = identifier_from_request(&v0.contract_id, "contract_id")?;

        let metadata = response
            .metadata()
            .or(Err(Error::EmptyResponseMetadata))?
            .clone();
        let proof = response.proof_owned().or(Err(Error::NoProofInResult))?;

        let (root_hash, counts) = Drive::verify_contract_moderation_action_counts(
            supported_grovedb_proof_bytes(&proof)?,
            contract_id,
            false,
            platform_version,
        )
        .map_drive_error(&proof, &metadata)?;

        verify_tenderdash_proof(&proof, &metadata, &root_hash, provider)?;

        // No member acted since the last payout proves as no counts, so the counts are always
        // the answer.
        Ok((
            Some(ContractModerationActionCounts(counts)),
            metadata,
            proof,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dapi_grpc::platform::v0::get_contract_document_removals_request::get_contract_document_removals_request_v0::Selection;
    use dapi_grpc::platform::v0::get_contract_document_removals_request::{
        DocumentIds, GetContractDocumentRemovalsRequestV0, Page,
    };
    use dapi_grpc::platform::v0::get_contract_document_removals_response::{
        get_contract_document_removals_response_v0::Result as RemovalsResult,
        GetContractDocumentRemovalsResponseV0, Version as RemovalsResponseVersion,
    };
    use dapi_grpc::platform::v0::get_contract_fee_pots_request::GetContractFeePotsRequestV0;
    use dapi_grpc::platform::v0::get_contract_fee_pots_response::{
        get_contract_fee_pots_response_v0::Result as FeePotsResult, GetContractFeePotsResponseV0,
        Version as FeePotsResponseVersion,
    };
    use dapi_grpc::platform::v0::get_contract_moderation_action_counts_request::GetContractModerationActionCountsRequestV0;
    use dapi_grpc::platform::v0::get_contract_moderation_action_counts_response::{
        get_contract_moderation_action_counts_response_v0::Result as CountsResult,
        GetContractModerationActionCountsResponseV0, Version as CountsResponseVersion,
    };
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
    use dapi_grpc::platform::v0::get_contract_team_action_signers_request::GetContractTeamActionSignersRequestV0;
    use dapi_grpc::platform::v0::get_contract_team_action_signers_response::{
        get_contract_team_action_signers_response_v0::Result as SignersResult,
        GetContractTeamActionSignersResponseV0, Version as SignersResponseVersion,
    };
    use dapi_grpc::platform::v0::get_contract_team_actions_request::{
        ActionStatus, GetContractTeamActionsRequestV0,
    };
    use dapi_grpc::platform::v0::get_contract_team_actions_response::{
        get_contract_team_actions_response_v0::Result as TeamActionsResult,
        GetContractTeamActionsResponseV0, Version as TeamActionsResponseVersion,
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

    fn fee_pots_request(contract_id: Vec<u8>) -> GetContractFeePotsRequest {
        GetContractFeePotsRequest {
            version: Some(get_contract_fee_pots_request::Version::V0(
                GetContractFeePotsRequestV0 {
                    contract_id,
                    prove: true,
                },
            )),
        }
    }

    fn fee_pots_response(result: Option<FeePotsResult>) -> GetContractFeePotsResponse {
        GetContractFeePotsResponse {
            version: Some(FeePotsResponseVersion::V0(GetContractFeePotsResponseV0 {
                result,
                metadata: Some(ResponseMetadata::default()),
            })),
        }
    }

    fn fee_pots_error(
        request: GetContractFeePotsRequest,
        response: GetContractFeePotsResponse,
    ) -> Error {
        <ContractFeePots as FromProof<_>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            PlatformVersion::latest(),
            &UnreachableProvider,
        )
        .unwrap_err()
    }

    #[test]
    fn fee_pots_should_fail_with_empty_version_when_request_has_no_version() {
        let err = fee_pots_error(
            GetContractFeePotsRequest { version: None },
            fee_pots_response(Some(FeePotsResult::Proof(Proof::default()))),
        );
        assert!(matches!(err, Error::EmptyVersion), "got: {err:?}");
    }

    #[test]
    fn fee_pots_should_reject_a_malformed_contract_id() {
        let err = fee_pots_error(
            fee_pots_request(vec![1; 5]),
            fee_pots_response(Some(FeePotsResult::Proof(Proof::default()))),
        );
        assert!(
            matches!(&err, Error::RequestError { error } if error.contains("contract_id")),
            "got: {err:?}"
        );
    }

    #[test]
    fn fee_pots_should_fail_without_proof_when_response_carries_none() {
        let err = fee_pots_error(fee_pots_request(vec![1; 32]), fee_pots_response(None));
        assert!(matches!(err, Error::NoProofInResult), "got: {err:?}");
    }

    #[test]
    fn fee_pots_should_fail_on_a_proof_that_does_not_verify() {
        let err = fee_pots_error(
            fee_pots_request(vec![1; 32]),
            fee_pots_response(Some(FeePotsResult::Proof(Proof::default()))),
        );
        assert!(
            !matches!(err, Error::RequestError { .. } | Error::NoProofInResult),
            "got: {err:?}"
        );
    }

    fn removals_request(
        contract_id: Vec<u8>,
        document_type_name: &str,
        selection: Option<Selection>,
    ) -> GetContractDocumentRemovalsRequest {
        GetContractDocumentRemovalsRequest {
            version: Some(get_contract_document_removals_request::Version::V0(
                GetContractDocumentRemovalsRequestV0 {
                    contract_id,
                    document_type_name: document_type_name.to_string(),
                    selection,
                    prove: true,
                },
            )),
        }
    }

    fn removals_response(result: Option<RemovalsResult>) -> GetContractDocumentRemovalsResponse {
        GetContractDocumentRemovalsResponse {
            version: Some(RemovalsResponseVersion::V0(
                GetContractDocumentRemovalsResponseV0 {
                    result,
                    metadata: Some(ResponseMetadata::default()),
                },
            )),
        }
    }

    fn removals_error(
        request: GetContractDocumentRemovalsRequest,
        response: GetContractDocumentRemovalsResponse,
    ) -> Error {
        <ContractDocumentRemovals as FromProof<_>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            PlatformVersion::latest(),
            &UnreachableProvider,
        )
        .unwrap_err()
    }

    fn removals_page(start_after: Option<Vec<u8>>, limit: Option<u32>) -> Option<Selection> {
        Some(Selection::Page(Page { start_after, limit }))
    }

    #[test]
    fn removals_should_fail_with_empty_version_when_request_has_no_version() {
        let err = removals_error(
            GetContractDocumentRemovalsRequest { version: None },
            removals_response(Some(RemovalsResult::Proof(Proof::default()))),
        );
        assert!(matches!(err, Error::EmptyVersion), "got: {err:?}");
    }

    #[test]
    fn removals_should_reject_malformed_requests() {
        let id = vec![1; 32];
        for (request, needle) in [
            (
                removals_request(vec![1; 5], "post", removals_page(None, None)),
                "contract_id",
            ),
            (
                removals_request(id.clone(), "post", None),
                "either document_ids or page must be set",
            ),
            (
                removals_request(
                    id.clone(),
                    "post",
                    Some(Selection::DocumentIds(DocumentIds {
                        document_ids: vec![],
                    })),
                ),
                "must name between 1 and",
            ),
            (
                removals_request(
                    id.clone(),
                    "post",
                    Some(Selection::DocumentIds(DocumentIds {
                        document_ids: vec![id.clone(), id.clone()],
                    })),
                ),
                "name a document id twice",
            ),
            (
                removals_request(id.clone(), "post", removals_page(Some(vec![1; 5]), None)),
                "start_after",
            ),
            (
                removals_request(id.clone(), "post", removals_page(None, Some(70_000))),
                "limit must be between 1 and",
            ),
        ] {
            let err = removals_error(
                request,
                removals_response(Some(RemovalsResult::Proof(Proof::default()))),
            );
            assert!(
                matches!(&err, Error::RequestError { error } if error.contains(needle)),
                "{needle}: {err:?}"
            );
        }
    }

    #[test]
    fn removals_should_fail_without_proof_when_response_carries_none() {
        let err = removals_error(
            removals_request(vec![1; 32], "post", removals_page(None, None)),
            removals_response(None),
        );
        assert!(matches!(err, Error::NoProofInResult), "got: {err:?}");
    }

    #[test]
    fn removals_should_fail_on_a_proof_that_does_not_verify() {
        let err = removals_error(
            removals_request(
                vec![1; 32],
                "post",
                Some(Selection::DocumentIds(DocumentIds {
                    document_ids: vec![vec![2; 32]],
                })),
            ),
            removals_response(Some(RemovalsResult::Proof(Proof::default()))),
        );
        assert!(
            !matches!(err, Error::RequestError { .. } | Error::NoProofInResult),
            "got: {err:?}"
        );
    }

    fn team_actions_request(
        contract_id: Vec<u8>,
        status: i32,
        count: Option<u32>,
    ) -> GetContractTeamActionsRequest {
        GetContractTeamActionsRequest {
            version: Some(get_contract_team_actions_request::Version::V0(
                GetContractTeamActionsRequestV0 {
                    contract_id,
                    status,
                    start_at_action_id: None,
                    count,
                    prove: true,
                },
            )),
        }
    }

    fn team_actions_response(result: Option<TeamActionsResult>) -> GetContractTeamActionsResponse {
        GetContractTeamActionsResponse {
            version: Some(TeamActionsResponseVersion::V0(
                GetContractTeamActionsResponseV0 {
                    result,
                    metadata: Some(ResponseMetadata::default()),
                },
            )),
        }
    }

    fn team_actions_error(
        request: GetContractTeamActionsRequest,
        response: GetContractTeamActionsResponse,
    ) -> Error {
        <ContractTeamActions as FromProof<_>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            PlatformVersion::latest(),
            &UnreachableProvider,
        )
        .unwrap_err()
    }

    fn signers_request(
        contract_id: Vec<u8>,
        status: i32,
        action_id: Vec<u8>,
    ) -> GetContractTeamActionSignersRequest {
        GetContractTeamActionSignersRequest {
            version: Some(get_contract_team_action_signers_request::Version::V0(
                GetContractTeamActionSignersRequestV0 {
                    contract_id,
                    status,
                    action_id,
                    prove: true,
                },
            )),
        }
    }

    fn signers_response(result: Option<SignersResult>) -> GetContractTeamActionSignersResponse {
        GetContractTeamActionSignersResponse {
            version: Some(SignersResponseVersion::V0(
                GetContractTeamActionSignersResponseV0 {
                    result,
                    metadata: Some(ResponseMetadata::default()),
                },
            )),
        }
    }

    fn signers_error(
        request: GetContractTeamActionSignersRequest,
        response: GetContractTeamActionSignersResponse,
    ) -> Error {
        <ContractTeamActionSigners as FromProof<_>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            PlatformVersion::latest(),
            &UnreachableProvider,
        )
        .unwrap_err()
    }

    #[test]
    fn should_fail_team_action_queries_with_empty_version_when_request_has_no_version() {
        let err = team_actions_error(
            GetContractTeamActionsRequest { version: None },
            team_actions_response(Some(TeamActionsResult::Proof(Proof::default()))),
        );
        assert!(matches!(err, Error::EmptyVersion), "got: {err:?}");
        let err = signers_error(
            GetContractTeamActionSignersRequest { version: None },
            signers_response(Some(SignersResult::Proof(Proof::default()))),
        );
        assert!(matches!(err, Error::EmptyVersion), "got: {err:?}");
    }

    #[test]
    fn should_reject_malformed_team_action_requests() {
        let id = vec![1; 32];
        let active = ActionStatus::Active as i32;
        for (request, needle) in [
            (
                team_actions_request(vec![1; 5], active, None),
                "contract_id",
            ),
            (
                team_actions_request(id.clone(), 7, None),
                "is not an action status",
            ),
            (
                team_actions_request(id.clone(), active, Some(70_000)),
                "limit must be between 1 and",
            ),
        ] {
            let err = team_actions_error(
                request,
                team_actions_response(Some(TeamActionsResult::Proof(Proof::default()))),
            );
            assert!(
                matches!(&err, Error::RequestError { error } if error.contains(needle)),
                "{needle}: {err:?}"
            );
        }
        for (request, needle) in [
            (
                signers_request(vec![1; 5], active, id.clone()),
                "contract_id",
            ),
            (signers_request(id.clone(), active, vec![1; 5]), "action_id"),
            (
                signers_request(id.clone(), 7, id.clone()),
                "is not an action status",
            ),
        ] {
            let err = signers_error(
                request,
                signers_response(Some(SignersResult::Proof(Proof::default()))),
            );
            assert!(
                matches!(&err, Error::RequestError { error } if error.contains(needle)),
                "{needle}: {err:?}"
            );
        }
    }

    #[test]
    fn should_fail_team_action_queries_without_proof_when_response_carries_none() {
        let active = ActionStatus::Active as i32;
        let err = team_actions_error(
            team_actions_request(vec![1; 32], active, None),
            team_actions_response(None),
        );
        assert!(matches!(err, Error::NoProofInResult), "got: {err:?}");
        let err = signers_error(
            signers_request(vec![1; 32], active, vec![2; 32]),
            signers_response(None),
        );
        assert!(matches!(err, Error::NoProofInResult), "got: {err:?}");
    }

    #[test]
    fn should_fail_on_team_action_proofs_that_do_not_verify() {
        let active = ActionStatus::Active as i32;
        let err = team_actions_error(
            team_actions_request(vec![1; 32], active, None),
            team_actions_response(Some(TeamActionsResult::Proof(Proof::default()))),
        );
        assert!(
            !matches!(err, Error::RequestError { .. } | Error::NoProofInResult),
            "got: {err:?}"
        );
        let err = signers_error(
            signers_request(vec![1; 32], active, vec![2; 32]),
            signers_response(Some(SignersResult::Proof(Proof::default()))),
        );
        assert!(
            !matches!(err, Error::RequestError { .. } | Error::NoProofInResult),
            "got: {err:?}"
        );
    }

    fn counts_request(contract_id: Vec<u8>) -> GetContractModerationActionCountsRequest {
        GetContractModerationActionCountsRequest {
            version: Some(get_contract_moderation_action_counts_request::Version::V0(
                GetContractModerationActionCountsRequestV0 {
                    contract_id,
                    prove: true,
                },
            )),
        }
    }

    fn counts_response(result: Option<CountsResult>) -> GetContractModerationActionCountsResponse {
        GetContractModerationActionCountsResponse {
            version: Some(CountsResponseVersion::V0(
                GetContractModerationActionCountsResponseV0 {
                    result,
                    metadata: Some(ResponseMetadata::default()),
                },
            )),
        }
    }

    fn counts_error(
        request: GetContractModerationActionCountsRequest,
        response: GetContractModerationActionCountsResponse,
    ) -> Error {
        <ContractModerationActionCounts as FromProof<_>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            PlatformVersion::latest(),
            &UnreachableProvider,
        )
        .unwrap_err()
    }

    #[test]
    fn should_check_the_request_and_the_proof_of_moderation_action_counts() {
        let proof = || counts_response(Some(CountsResult::Proof(Proof::default())));
        let err = counts_error(
            GetContractModerationActionCountsRequest { version: None },
            proof(),
        );
        assert!(matches!(err, Error::EmptyVersion), "got: {err:?}");

        let err = counts_error(counts_request(vec![1; 5]), proof());
        assert!(
            matches!(&err, Error::RequestError { error } if error.contains("contract_id")),
            "got: {err:?}"
        );

        let err = counts_error(counts_request(vec![1; 32]), counts_response(None));
        assert!(matches!(err, Error::NoProofInResult), "got: {err:?}");

        // A proof that does not verify fails in the proof check, past the request's
        let err = counts_error(counts_request(vec![1; 32]), proof());
        assert!(
            !matches!(err, Error::RequestError { .. } | Error::NoProofInResult),
            "got: {err:?}"
        );
    }
}
