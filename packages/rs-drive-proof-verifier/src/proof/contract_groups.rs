//! Proof verification of the contract group queries.

use crate::error::MapGroveDbError;
use crate::types::contract_groups::{
    members_limit_from_request, members_query_from_request, ContractGroupInfo,
    ContractGroupMembersPage, ContractGroupMembershipsForContract,
};
use crate::verify::{supported_grovedb_proof_bytes, verify_tenderdash_proof};
use crate::{ContextProvider, Error, FromProof};
use dapi_grpc::platform::v0::{
    get_contract_group_info_request, get_contract_group_members_request,
    get_contract_groups_for_contract_request, GetContractGroupInfoRequest,
    GetContractGroupInfoResponse, GetContractGroupMembersRequest, GetContractGroupMembersResponse,
    GetContractGroupsForContractRequest, GetContractGroupsForContractResponse, Proof,
    ResponseMetadata,
};
use dapi_grpc::platform::VersionedGrpcResponse;
use dpp::dashcore::Network;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use drive::drive::Drive;

fn identifier_from_request(bytes: Vec<u8>, what: &str) -> Result<Identifier, Error> {
    Identifier::from_bytes(&bytes).map_err(|_| Error::RequestError {
        error: format!(
            "{what} must be a 32 byte identifier, got {} bytes",
            bytes.len()
        ),
    })
}

impl FromProof<GetContractGroupInfoRequest> for ContractGroupInfo {
    type Request = GetContractGroupInfoRequest;
    type Response = GetContractGroupInfoResponse;

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

        let get_contract_group_info_request::Version::V0(v0) =
            request.version.ok_or(Error::EmptyVersion)?;
        let contract_group_id = identifier_from_request(v0.contract_group_id, "contract_group_id")?;

        let metadata = response
            .metadata()
            .or(Err(Error::EmptyResponseMetadata))?
            .clone();
        let proof = response.proof_owned().or(Err(Error::NoProofInResult))?;

        let (root_hash, info) = Drive::verify_contract_group_info(
            supported_grovedb_proof_bytes(&proof, platform_version)?,
            contract_group_id,
            platform_version,
        )
        .map_drive_error(&proof, &metadata)?;

        verify_tenderdash_proof(&proof, &metadata, &root_hash, provider, platform_version)?;

        Ok((info, metadata, proof))
    }
}

impl FromProof<GetContractGroupMembersRequest> for ContractGroupMembersPage {
    type Request = GetContractGroupMembersRequest;
    type Response = GetContractGroupMembersResponse;

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

        let get_contract_group_members_request::Version::V0(v0) =
            request.version.ok_or(Error::EmptyVersion)?;
        let contract_group_id = identifier_from_request(v0.contract_group_id, "contract_group_id")?;
        let query = members_query_from_request(v0.members)?;
        let limit = members_limit_from_request(v0.limit, platform_version)?;

        let metadata = response
            .metadata()
            .or(Err(Error::EmptyResponseMetadata))?
            .clone();
        let proof = response.proof_owned().or(Err(Error::NoProofInResult))?;

        let (root_hash, page) = Drive::verify_contract_group_members(
            supported_grovedb_proof_bytes(&proof, platform_version)?,
            contract_group_id,
            &query,
            limit,
            platform_version,
        )
        .map_drive_error(&proof, &metadata)?;

        verify_tenderdash_proof(&proof, &metadata, &root_hash, provider, platform_version)?;

        // An absent group and a group with no members of that kind both prove as an empty
        // page, so the page itself is always the answer.
        Ok((Some(page), metadata, proof))
    }
}

impl FromProof<GetContractGroupsForContractRequest> for ContractGroupMembershipsForContract {
    type Request = GetContractGroupsForContractRequest;
    type Response = GetContractGroupsForContractResponse;

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

        let get_contract_groups_for_contract_request::Version::V0(v0) =
            request.version.ok_or(Error::EmptyVersion)?;
        let contract_id = identifier_from_request(v0.contract_id, "contract_id")?;

        let metadata = response
            .metadata()
            .or(Err(Error::EmptyResponseMetadata))?
            .clone();
        let proof = response.proof_owned().or(Err(Error::NoProofInResult))?;

        let (root_hash, memberships) = Drive::verify_contract_group_memberships_for_contract(
            supported_grovedb_proof_bytes(&proof, platform_version)?,
            contract_id,
            platform_version,
        )
        .map_drive_error(&proof, &metadata)?;

        verify_tenderdash_proof(&proof, &metadata, &root_hash, provider, platform_version)?;

        // A contract in no group proves as empty memberships, so they are always the answer.
        Ok((Some(memberships), metadata, proof))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dapi_grpc::platform::v0::get_contract_group_info_request::GetContractGroupInfoRequestV0;
    use dapi_grpc::platform::v0::get_contract_group_info_response::{
        get_contract_group_info_response_v0::Result as InfoResult, GetContractGroupInfoResponseV0,
        Version as InfoResponseVersion,
    };
    use dapi_grpc::platform::v0::get_contract_group_members_request::{
        ContractMembersQuery, GetContractGroupMembersRequestV0,
    };
    use dapi_grpc::platform::v0::get_contract_group_members_response::{
        get_contract_group_members_response_v0::Result as MembersResult,
        GetContractGroupMembersResponseV0, Version as MembersResponseVersion,
    };
    use dapi_grpc::platform::v0::get_contract_group_members_request::get_contract_group_members_request_v0::Members;
    use dash_context_provider::ContextProviderError;
    use dpp::data_contract::TokenConfiguration;
    use dpp::prelude::{CoreBlockHeight, DataContract};
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

    fn info_request(contract_group_id: Vec<u8>) -> GetContractGroupInfoRequest {
        GetContractGroupInfoRequest {
            version: Some(get_contract_group_info_request::Version::V0(
                GetContractGroupInfoRequestV0 {
                    contract_group_id,
                    prove: true,
                },
            )),
        }
    }

    fn info_response(result: Option<InfoResult>) -> GetContractGroupInfoResponse {
        GetContractGroupInfoResponse {
            version: Some(InfoResponseVersion::V0(GetContractGroupInfoResponseV0 {
                result,
                metadata: Some(ResponseMetadata::default()),
            })),
        }
    }

    fn members_request(
        members: Option<Members>,
        limit: Option<u32>,
    ) -> GetContractGroupMembersRequest {
        GetContractGroupMembersRequest {
            version: Some(get_contract_group_members_request::Version::V0(
                GetContractGroupMembersRequestV0 {
                    contract_group_id: vec![1; 32],
                    members,
                    limit,
                    prove: true,
                },
            )),
        }
    }

    fn members_response_with_proof() -> GetContractGroupMembersResponse {
        GetContractGroupMembersResponse {
            version: Some(MembersResponseVersion::V0(
                GetContractGroupMembersResponseV0 {
                    result: Some(MembersResult::Proof(Proof::default())),
                    metadata: Some(ResponseMetadata::default()),
                },
            )),
        }
    }

    #[test]
    fn info_should_fail_with_empty_version_when_request_has_no_version() {
        let err = <ContractGroupInfo as FromProof<_>>::maybe_from_proof(
            GetContractGroupInfoRequest { version: None },
            info_response(Some(InfoResult::Proof(Proof::default()))),
            Network::Testnet,
            PlatformVersion::latest(),
            &UnreachableProvider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::EmptyVersion), "got: {err:?}");
    }

    #[test]
    fn info_should_reject_id_that_is_not_32_bytes() {
        let err = <ContractGroupInfo as FromProof<_>>::maybe_from_proof(
            info_request(vec![1; 5]),
            info_response(Some(InfoResult::Proof(Proof::default()))),
            Network::Testnet,
            PlatformVersion::latest(),
            &UnreachableProvider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::RequestError { .. }), "got: {err:?}");
    }

    #[test]
    fn info_should_fail_without_proof_when_response_carries_none() {
        let err = <ContractGroupInfo as FromProof<_>>::maybe_from_proof(
            info_request(vec![1; 32]),
            info_response(None),
            Network::Testnet,
            PlatformVersion::latest(),
            &UnreachableProvider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::NoProofInResult), "got: {err:?}");
    }

    #[test]
    fn members_should_reject_a_request_without_a_kind() {
        let err = <ContractGroupMembersPage as FromProof<_>>::maybe_from_proof(
            members_request(None, None),
            members_response_with_proof(),
            Network::Testnet,
            PlatformVersion::latest(),
            &UnreachableProvider,
        )
        .unwrap_err();
        assert!(
            matches!(&err, Error::RequestError { error } if error.contains("members")),
            "got: {err:?}"
        );
    }

    #[test]
    fn members_should_reject_out_of_bounds_limits() {
        let platform_version = PlatformVersion::latest();
        let too_many = platform_version.drive_abci.query.max_returned_elements as u32 + 1;
        for limit in [0, too_many] {
            let err = <ContractGroupMembersPage as FromProof<_>>::maybe_from_proof(
                members_request(
                    Some(Members::Contracts(ContractMembersQuery {
                        start_after: None,
                    })),
                    Some(limit),
                ),
                members_response_with_proof(),
                Network::Testnet,
                platform_version,
                &UnreachableProvider,
            )
            .unwrap_err();
            assert!(
                matches!(&err, Error::RequestError { error } if error.contains("limit")),
                "limit {limit}: {err:?}"
            );
        }
    }

    #[test]
    fn members_should_reject_a_cursor_that_is_not_an_identifier() {
        let err = <ContractGroupMembersPage as FromProof<_>>::maybe_from_proof(
            members_request(
                Some(Members::Contracts(ContractMembersQuery {
                    start_after: Some(vec![1; 3]),
                })),
                None,
            ),
            members_response_with_proof(),
            Network::Testnet,
            PlatformVersion::latest(),
            &UnreachableProvider,
        )
        .unwrap_err();
        assert!(
            matches!(&err, Error::RequestError { error } if error.contains("start_after")),
            "got: {err:?}"
        );
    }
}
