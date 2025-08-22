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
