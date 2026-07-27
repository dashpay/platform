use crate::error::MapGroveDbError;
use crate::types::groups::{GroupActionSigners, GroupActions, Groups};
use crate::verify::verify_tenderdash_proof;
use crate::{ContextProvider, Error, FromProof};
use dapi_grpc::platform::v0::{
    get_group_action_signers_request, get_group_actions_request, get_group_info_request,
    get_group_infos_request, GetGroupActionSignersRequest, GetGroupActionSignersResponse,
    GetGroupActionsRequest, GetGroupActionsResponse, GetGroupInfoRequest, GetGroupInfoResponse,
    GetGroupInfosRequest, GetGroupInfosResponse, Proof, ResponseMetadata,
};
use dapi_grpc::platform::VersionedGrpcResponse;
use dpp::dashcore::Network;
use dpp::data_contract::group::{Group, GroupMemberPower};
use dpp::data_contract::GroupContractPosition;
use dpp::group::group_action::GroupAction;
use dpp::group::group_action_status::GroupActionStatus;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use indexmap::IndexMap;

impl FromProof<GetGroupInfoRequest> for Group {
    type Request = GetGroupInfoRequest;
    type Response = GetGroupInfoResponse;

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

        let (contract_id, group_contract_position) = match request
            .version
            .ok_or(Error::EmptyVersion)?
        {
            get_group_info_request::Version::V0(v0) => {
                let contract_id =
                    Identifier::try_from(v0.contract_id).map_err(|error| Error::RequestError {
                        error: format!("can't convert contract_id to identifier: {error}"),
                    })?;

                let group_contract_position = v0.group_contract_position as GroupContractPosition;

                (contract_id, group_contract_position)
            }
        };

        let metadata = response
            .metadata()
            .or(Err(Error::EmptyResponseMetadata))?
            .clone();

        let proof = response.proof_owned().or(Err(Error::NoProofInResult))?;

        let (root_hash, result) = Drive::verify_group_info(
            &proof.grovedb_proof,
            contract_id,
            group_contract_position,
            false,
            platform_version,
        )
        .map_drive_error(&proof, &metadata)?;

        verify_tenderdash_proof(&proof, &metadata, &root_hash, provider)?;

        Ok((result, metadata, proof))
    }
}

impl FromProof<GetGroupInfosRequest> for Groups {
    type Request = GetGroupInfosRequest;
    type Response = GetGroupInfosResponse;

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

        let (contract_id, start_at_group_contract_position, count) = match request
            .version
            .ok_or(Error::EmptyVersion)?
        {
            get_group_infos_request::Version::V0(v0) => {
                let contract_id =
                    Identifier::try_from(v0.contract_id).map_err(|error| Error::RequestError {
                        error: format!("can't convert contract_id to identifier: {error}"),
                    })?;

                let start_group_contract_position =
                    v0.start_at_group_contract_position.map(|start_position| {
                        (
                            start_position.start_group_contract_position as GroupContractPosition,
                            start_position.start_group_contract_position_included,
                        )
                    });

                let count = v0.count.map(|count| count as u16);

                (contract_id, start_group_contract_position, count)
            }
        };

        let metadata = response
            .metadata()
            .or(Err(Error::EmptyResponseMetadata))?
            .clone();

        let proof = response.proof_owned().or(Err(Error::NoProofInResult))?;

        let (root_hash, result) = Drive::verify_group_infos_in_contract(
            &proof.grovedb_proof,
            contract_id,
            start_at_group_contract_position,
            count,
            false,
            platform_version,
        )
        // Make value optional
        .map(
            |(root_hash, result): (_, IndexMap<GroupContractPosition, Group>)| {
                let optional_value_map = result
                    .into_iter()
                    .map(|(action_id, group_action)| (action_id, Some(group_action)))
                    .collect::<Groups>();
                (root_hash, optional_value_map)
            },
        )
        .map_drive_error(&proof, &metadata)?;

        verify_tenderdash_proof(&proof, &metadata, &root_hash, provider)?;

        Ok((Some(result), metadata, proof))
    }
}

impl FromProof<GetGroupActionsRequest> for GroupActions {
    type Request = GetGroupActionsRequest;
    type Response = GetGroupActionsResponse;

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

        let (contract_id, group_contract_position, status, start_at_action_id, count) =
            match request.version.ok_or(Error::EmptyVersion)? {
                get_group_actions_request::Version::V0(v0) => {
                    let contract_id = Identifier::try_from(v0.contract_id).map_err(|error| {
                        Error::RequestError {
                            error: format!("can't convert contract_id to identifier: {error}"),
                        }
                    })?;

                    let start_at_action_id =
                        v0.start_at_action_id
                            .map(|start_at_action_id| {
                                let start_action_id =
                                    Identifier::try_from(start_at_action_id.start_action_id)
                                        .map_err(|error| Error::RequestError {
                                            error: format!(
                                    "can't convert start_action_id to identifier: {error}"
                                ),
                                        })?;

                                Ok::<_, Error>((
                                    start_action_id,
                                    start_at_action_id.start_action_id_included,
                                ))
                            })
                            .transpose()?;

                    let group_contract_position =
                        v0.group_contract_position as GroupContractPosition;

                    let count = v0.count.map(|count| count as u16);

                    let status = GroupActionStatus::try_from(v0.status).map_err(|error| {
                        Error::RequestError {
                            error: format!("can't convert status to GroupActionStatus: {error}"),
                        }
                    })?;

                    (
                        contract_id,
                        group_contract_position,
                        status,
                        start_at_action_id,
                        count,
                    )
                }
            };

        let metadata = response
            .metadata()
            .or(Err(Error::EmptyResponseMetadata))?
            .clone();

        let proof = response.proof_owned().or(Err(Error::NoProofInResult))?;

        let (root_hash, result) = Drive::verify_action_infos_in_contract(
            &proof.grovedb_proof,
            contract_id,
            group_contract_position,
            status,
            start_at_action_id,
            count,
            false,
            platform_version,
        )
        // Make value optional
        .map(
            |(root_hash, result): (_, IndexMap<Identifier, GroupAction>)| {
                let optional_value_map = result
                    .into_iter()
                    .map(|(action_id, group_action)| (action_id, Some(group_action)))
                    .collect::<GroupActions>();
                (root_hash, optional_value_map)
            },
        )
        .map_drive_error(&proof, &metadata)?;

        verify_tenderdash_proof(&proof, &metadata, &root_hash, provider)?;

        Ok((Some(result), metadata, proof))
    }
}

impl FromProof<GetGroupActionSignersRequest> for GroupActionSigners {
    type Request = GetGroupActionSignersRequest;
    type Response = GetGroupActionSignersResponse;

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

        let (contract_id, group_contract_position, status, action_id) = match request
            .version
            .ok_or(Error::EmptyVersion)?
        {
            get_group_action_signers_request::Version::V0(v0) => {
                let contract_id =
                    Identifier::try_from(v0.contract_id).map_err(|error| Error::RequestError {
                        error: format!("can't convert contract_id to identifier: {error}"),
                    })?;

                let action_id =
                    Identifier::try_from(v0.action_id).map_err(|error| Error::RequestError {
                        error: format!("can't convert action_id to identifier: {error}"),
                    })?;

                let group_contract_position = v0.group_contract_position as GroupContractPosition;

                let status = GroupActionStatus::try_from(v0.status).map_err(|error| {
                    Error::RequestError {
                        error: format!("can't convert status to GroupActionStatus: {error}"),
                    }
                })?;

                (contract_id, group_contract_position, status, action_id)
            }
        };

        let metadata = response
            .metadata()
            .or(Err(Error::EmptyResponseMetadata))?
            .clone();

        let proof = response.proof_owned().or(Err(Error::NoProofInResult))?;

        let (root_hash, result) = Drive::verify_action_signers(
            &proof.grovedb_proof,
            contract_id,
            group_contract_position,
            status,
            action_id,
            false,
            platform_version,
        )
        // Make value optional
        .map(
            |(root_hash, result): (_, IndexMap<Identifier, GroupMemberPower>)| {
                let optional_value_map = result
                    .into_iter()
                    .map(|(action_id, group_action)| (action_id, Some(group_action)))
                    .collect::<GroupActionSigners>();
                (root_hash, optional_value_map)
            },
        )
        .map_drive_error(&proof, &metadata)?;

        verify_tenderdash_proof(&proof, &metadata, &root_hash, provider)?;

        Ok((Some(result), metadata, proof))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dapi_grpc::platform::v0::get_group_action_signers_request::{
        GetGroupActionSignersRequestV0, Version as SignersReqVersion,
    };
    use dapi_grpc::platform::v0::get_group_actions_request::{
        GetGroupActionsRequestV0, StartAtActionId, Version as ActionsReqVersion,
    };
    use dapi_grpc::platform::v0::get_group_actions_response::{
        get_group_actions_response_v0::Result as ActionsRespResult, GetGroupActionsResponseV0,
        Version as ActionsRespVersion,
    };
    use dapi_grpc::platform::v0::get_group_info_request::{
        GetGroupInfoRequestV0, Version as InfoReqVersion,
    };
    use dapi_grpc::platform::v0::get_group_info_response::{
        get_group_info_response_v0::Result as InfoRespResult, GetGroupInfoResponseV0,
        Version as InfoRespVersion,
    };
    use dapi_grpc::platform::v0::get_group_infos_request::{
        GetGroupInfosRequestV0, StartAtGroupContractPosition, Version as InfosReqVersion,
    };
    use dapi_grpc::platform::v0::get_group_infos_response::{
        get_group_infos_response_v0::Result as InfosRespResult, GetGroupInfosResponseV0,
        Version as InfosRespVersion,
    };
    use dash_context_provider::ContextProviderError;
    use dpp::data_contract::TokenConfiguration;
    use dpp::prelude::{CoreBlockHeight, DataContract};
    use std::sync::Arc;

    struct UnreachableProvider;

    impl ContextProvider for UnreachableProvider {
        fn get_data_contract(
            &self,
            _id: &Identifier,
            _pv: &PlatformVersion,
        ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
            panic!("should not be called")
        }
        fn get_token_configuration(
            &self,
            _id: &Identifier,
        ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
            panic!("should not be called")
        }
        fn get_quorum_public_key(
            &self,
            _qt: u32,
            _qh: [u8; 32],
            _h: u32,
        ) -> Result<[u8; 48], ContextProviderError> {
            panic!("should not be called")
        }
        fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
            panic!("should not be called")
        }
    }

    fn pv() -> &'static PlatformVersion {
        PlatformVersion::latest()
    }

    // -------- GetGroupInfoRequest / Group --------

    #[test]
    fn group_info_empty_version_on_request_missing() {
        let request = GetGroupInfoRequest { version: None };
        let response = GetGroupInfoResponse {
            version: Some(InfoRespVersion::V0(GetGroupInfoResponseV0 {
                result: Some(InfoRespResult::Proof(Proof::default())),
                metadata: Some(ResponseMetadata::default()),
            })),
        };
        let err = <Group as FromProof<_>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            pv(),
            &UnreachableProvider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::EmptyVersion), "got: {err:?}");
    }

    #[test]
    fn group_info_request_error_on_bad_contract_id() {
        let request = GetGroupInfoRequest {
            version: Some(InfoReqVersion::V0(GetGroupInfoRequestV0 {
                contract_id: vec![0u8; 7], // wrong length
                group_contract_position: 0,
                prove: true,
            })),
        };
        let response = GetGroupInfoResponse::default();
        let err = <Group as FromProof<_>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            pv(),
            &UnreachableProvider,
        )
        .unwrap_err();
        match err {
            Error::RequestError { error } => assert!(error.contains("contract_id"), "got: {error}"),
            other => panic!("expected RequestError, got: {other:?}"),
        }
    }

    #[test]
    fn group_info_empty_response_metadata() {
        let request = GetGroupInfoRequest {
            version: Some(InfoReqVersion::V0(GetGroupInfoRequestV0 {
                contract_id: vec![0u8; 32],
                group_contract_position: 0,
                prove: true,
            })),
        };
        let response = GetGroupInfoResponse { version: None };
        let err = <Group as FromProof<_>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            pv(),
            &UnreachableProvider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::EmptyResponseMetadata), "got: {err:?}");
    }

    #[test]
    fn group_info_no_proof_when_result_missing() {
        let request = GetGroupInfoRequest {
            version: Some(InfoReqVersion::V0(GetGroupInfoRequestV0 {
                contract_id: vec![0u8; 32],
                group_contract_position: 0,
                prove: true,
            })),
        };
        let response = GetGroupInfoResponse {
            version: Some(InfoRespVersion::V0(GetGroupInfoResponseV0 {
                result: None,
                metadata: Some(ResponseMetadata::default()),
            })),
        };
        let err = <Group as FromProof<_>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            pv(),
            &UnreachableProvider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::NoProofInResult), "got: {err:?}");
    }

    // -------- GetGroupInfosRequest / Groups --------

    #[test]
    fn group_infos_empty_version_on_request_missing() {
        let request = GetGroupInfosRequest { version: None };
        let response = GetGroupInfosResponse::default();
        let err = <Groups as FromProof<_>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            pv(),
            &UnreachableProvider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::EmptyVersion), "got: {err:?}");
    }

    #[test]
    fn group_infos_request_error_on_bad_contract_id() {
        let request = GetGroupInfosRequest {
            version: Some(InfosReqVersion::V0(GetGroupInfosRequestV0 {
                contract_id: vec![0u8; 12], // wrong length
                start_at_group_contract_position: Some(StartAtGroupContractPosition {
                    start_group_contract_position: 0,
                    start_group_contract_position_included: true,
                }),
                count: Some(10),
                prove: true,
            })),
        };
        let response = GetGroupInfosResponse::default();
        let err = <Groups as FromProof<_>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            pv(),
            &UnreachableProvider,
        )
        .unwrap_err();
        match err {
            Error::RequestError { error } => assert!(error.contains("contract_id"), "got: {error}"),
            other => panic!("expected RequestError, got: {other:?}"),
        }
    }

    #[test]
    fn group_infos_empty_response_metadata() {
        let request = GetGroupInfosRequest {
            version: Some(InfosReqVersion::V0(GetGroupInfosRequestV0 {
                contract_id: vec![0u8; 32],
                start_at_group_contract_position: None,
                count: None,
                prove: true,
            })),
        };
        let response = GetGroupInfosResponse {
            version: Some(InfosRespVersion::V0(GetGroupInfosResponseV0 {
                result: Some(InfosRespResult::Proof(Proof::default())),
                metadata: None,
            })),
        };
        let err = <Groups as FromProof<_>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            pv(),
            &UnreachableProvider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::EmptyResponseMetadata), "got: {err:?}");
    }

    // -------- GetGroupActionsRequest / GroupActions --------

    #[test]
    fn group_actions_empty_version_on_request_missing() {
        let request = GetGroupActionsRequest { version: None };
        let response = GetGroupActionsResponse::default();
        let err = <GroupActions as FromProof<_>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            pv(),
            &UnreachableProvider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::EmptyVersion), "got: {err:?}");
    }

    #[test]
    fn group_actions_request_error_on_bad_contract_id() {
        let request = GetGroupActionsRequest {
            version: Some(ActionsReqVersion::V0(GetGroupActionsRequestV0 {
                contract_id: vec![0u8; 3],
                group_contract_position: 0,
                status: 0,
                start_at_action_id: None,
                count: None,
                prove: true,
            })),
        };
        let response = GetGroupActionsResponse::default();
        let err = <GroupActions as FromProof<_>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            pv(),
            &UnreachableProvider,
        )
        .unwrap_err();
        match err {
            Error::RequestError { error } => assert!(error.contains("contract_id"), "got: {error}"),
            other => panic!("expected RequestError, got: {other:?}"),
        }
    }

    #[test]
    fn group_actions_request_error_on_bad_start_action_id() {
        let request = GetGroupActionsRequest {
            version: Some(ActionsReqVersion::V0(GetGroupActionsRequestV0 {
                contract_id: vec![0u8; 32],
                group_contract_position: 0,
                status: 0,
                start_at_action_id: Some(StartAtActionId {
                    start_action_id: vec![0u8; 9], // wrong length
                    start_action_id_included: true,
                }),
                count: None,
                prove: true,
            })),
        };
        let response = GetGroupActionsResponse::default();
        let err = <GroupActions as FromProof<_>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            pv(),
            &UnreachableProvider,
        )
        .unwrap_err();
        match err {
            Error::RequestError { error } => {
                assert!(error.contains("start_action_id"), "got: {error}")
            }
            other => panic!("expected RequestError, got: {other:?}"),
        }
    }

    #[test]
    fn group_actions_request_error_on_bad_status() {
        let request = GetGroupActionsRequest {
            version: Some(ActionsReqVersion::V0(GetGroupActionsRequestV0 {
                contract_id: vec![0u8; 32],
                group_contract_position: 0,
                status: 999, // invalid status
                start_at_action_id: None,
                count: None,
                prove: true,
            })),
        };
        let response = GetGroupActionsResponse::default();
        let err = <GroupActions as FromProof<_>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            pv(),
            &UnreachableProvider,
        )
        .unwrap_err();
        match err {
            Error::RequestError { error } => {
                assert!(error.contains("GroupActionStatus"), "got: {error}")
            }
            other => panic!("expected RequestError, got: {other:?}"),
        }
    }

    #[test]
    fn group_actions_empty_response_metadata() {
        let request = GetGroupActionsRequest {
            version: Some(ActionsReqVersion::V0(GetGroupActionsRequestV0 {
                contract_id: vec![0u8; 32],
                group_contract_position: 0,
                status: 0,
                start_at_action_id: None,
                count: None,
                prove: true,
            })),
        };
        let response = GetGroupActionsResponse {
            version: Some(ActionsRespVersion::V0(GetGroupActionsResponseV0 {
                result: Some(ActionsRespResult::Proof(Proof::default())),
                metadata: None,
            })),
        };
        let err = <GroupActions as FromProof<_>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            pv(),
            &UnreachableProvider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::EmptyResponseMetadata), "got: {err:?}");
    }

    // -------- GetGroupActionSignersRequest / GroupActionSigners --------

    #[test]
    fn group_action_signers_empty_version_on_request_missing() {
        let request = GetGroupActionSignersRequest { version: None };
        let response = GetGroupActionSignersResponse::default();
        let err = <GroupActionSigners as FromProof<_>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            pv(),
            &UnreachableProvider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::EmptyVersion), "got: {err:?}");
    }

    #[test]
    fn group_action_signers_request_error_on_bad_contract_id() {
        let request = GetGroupActionSignersRequest {
            version: Some(SignersReqVersion::V0(GetGroupActionSignersRequestV0 {
                contract_id: vec![0u8; 9], // bad
                group_contract_position: 0,
                status: 0,
                action_id: vec![1u8; 32],
                prove: true,
            })),
        };
        let response = GetGroupActionSignersResponse::default();
        let err = <GroupActionSigners as FromProof<_>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            pv(),
            &UnreachableProvider,
        )
        .unwrap_err();
        match err {
            Error::RequestError { error } => assert!(error.contains("contract_id"), "got: {error}"),
            other => panic!("expected RequestError, got: {other:?}"),
        }
    }

    #[test]
    fn group_action_signers_request_error_on_bad_action_id() {
        let request = GetGroupActionSignersRequest {
            version: Some(SignersReqVersion::V0(GetGroupActionSignersRequestV0 {
                contract_id: vec![0u8; 32],
                group_contract_position: 0,
                status: 0,
                action_id: vec![1u8; 3], // bad
                prove: true,
            })),
        };
        let response = GetGroupActionSignersResponse::default();
        let err = <GroupActionSigners as FromProof<_>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            pv(),
            &UnreachableProvider,
        )
        .unwrap_err();
        match err {
            Error::RequestError { error } => assert!(error.contains("action_id"), "got: {error}"),
            other => panic!("expected RequestError, got: {other:?}"),
        }
    }

    #[test]
    fn group_action_signers_request_error_on_bad_status() {
        let request = GetGroupActionSignersRequest {
            version: Some(SignersReqVersion::V0(GetGroupActionSignersRequestV0 {
                contract_id: vec![0u8; 32],
                group_contract_position: 0,
                status: 42, // invalid
                action_id: vec![1u8; 32],
                prove: true,
            })),
        };
        let response = GetGroupActionSignersResponse::default();
        let err = <GroupActionSigners as FromProof<_>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            pv(),
            &UnreachableProvider,
        )
        .unwrap_err();
        match err {
            Error::RequestError { error } => {
                assert!(error.contains("GroupActionStatus"), "got: {error}")
            }
            other => panic!("expected RequestError, got: {other:?}"),
        }
    }
}
