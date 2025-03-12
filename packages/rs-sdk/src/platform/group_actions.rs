use crate::platform::{Fetch, FetchMany, Identifier, Query};
use crate::Error;
use dapi_grpc::platform::v0::get_group_action_signers_request::GetGroupActionSignersRequestV0;
use dapi_grpc::platform::v0::get_group_actions_request::{
    GetGroupActionsRequestV0, StartAtActionId,
};
use dapi_grpc::platform::v0::get_group_info_request::GetGroupInfoRequestV0;
use dapi_grpc::platform::v0::get_group_infos_request::{
    GetGroupInfosRequestV0, StartAtGroupContractPosition,
};
use dapi_grpc::platform::v0::{
    get_group_action_signers_request, get_group_actions_request, get_group_info_request,
    get_group_infos_request, GetGroupActionSignersRequest, GetGroupActionsRequest,
    GetGroupInfoRequest, GetGroupInfosRequest,
};
use dpp::data_contract::group::{Group, GroupMemberPower};
use dpp::data_contract::GroupContractPosition;
use dpp::group::group_action::GroupAction;
use dpp::group::group_action_status::GroupActionStatus;
use dpp::prelude::StartAtIncluded;
use drive_proof_verifier::types::groups::{GroupActionSigners, GroupActions, Groups};

#[derive(Debug, Clone)]
/// Query to fetch data contract group
pub struct GroupQuery {
    /// Data contract ID
    pub contract_id: Identifier,
    /// Group contract position in the data contract definition
    pub group_contract_position: GroupContractPosition,
}

impl Query<GetGroupInfoRequest> for GroupQuery {
    fn query(self, prove: bool) -> Result<GetGroupInfoRequest, Error> {
        let request = GetGroupInfoRequest {
            version: Some(get_group_info_request::Version::V0(GetGroupInfoRequestV0 {
                contract_id: self.contract_id.to_vec(),
                group_contract_position: self.group_contract_position as u32,
                prove,
            })),
        };

        Ok(request)
    }
}

impl Fetch for Group {
    type Request = GetGroupInfoRequest;
}

#[derive(Debug, Clone)]
/// Query to fetch multiple data contract groups
pub struct GroupInfosQuery {
    /// Data contract ID
    pub contract_id: Identifier,
    /// Optional start group contract position to fetch groups from
    /// If not provided, the first group will be fetched
    /// If provided, the group at the provided position will be included in the result if `StartAtIncluded` is `true`
    pub start_group_contract_position: Option<(GroupContractPosition, StartAtIncluded)>,
    /// An optional limit of groups to fetch
    pub limit: Option<u16>,
}

impl Query<GetGroupInfosRequest> for GroupInfosQuery {
    fn query(self, prove: bool) -> Result<GetGroupInfosRequest, Error> {
        let request = GetGroupInfosRequest {
            version: Some(get_group_infos_request::Version::V0(
                GetGroupInfosRequestV0 {
                    contract_id: self.contract_id.to_vec(),
                    start_at_group_contract_position: self.start_group_contract_position.map(
                        |(position, included)| StartAtGroupContractPosition {
                            start_group_contract_position: position as u32,
                            start_group_contract_position_included: included,
                        },
                    ),
                    count: self.limit.map(|limit| limit as u32),
                    prove,
                },
            )),
        };

        Ok(request)
    }
}

impl FetchMany<GroupContractPosition, Groups> for Group {
    type Request = GetGroupInfosRequest;
}

#[derive(Debug, Clone)]
/// Query to fetch available group actions on specific data contract
pub struct GroupActionsQuery {
    /// Data contract ID
    pub contract_id: Identifier,
    /// Group contract position in the data contract definition
    pub group_contract_position: GroupContractPosition,
    /// Group action status
    pub status: GroupActionStatus,
    /// Optional start action ID to fetch actions from
    /// If not provided, the first action will be fetched
    /// If provided, the action at the provided position will be included in the result if `StartAtIncluded` is `true`
    pub start_at_action_id: Option<(Identifier, StartAtIncluded)>,
    /// An optional limit of actions to fetch
    pub limit: Option<u16>,
}

impl Query<GetGroupActionsRequest> for GroupActionsQuery {
    fn query(self, prove: bool) -> Result<GetGroupActionsRequest, Error> {
        let request = GetGroupActionsRequest {
            version: Some(get_group_actions_request::Version::V0(
                GetGroupActionsRequestV0 {
                    contract_id: self.contract_id.to_vec(),
                    group_contract_position: self.group_contract_position as u32,
                    status: self.status as i32,
                    start_at_action_id: self.start_at_action_id.map(|(id, included)| {
                        StartAtActionId {
                            start_action_id: id.to_vec(),
                            start_action_id_included: included,
                        }
                    }),
                    count: self.limit.map(|limit| limit as u32),
                    prove,
                },
            )),
        };

        Ok(request)
    }
}

impl FetchMany<Identifier, GroupActions> for GroupAction {
    type Request = GetGroupActionsRequest;
}

#[derive(Debug, Clone)]
/// Query to fetch available signatures of specific data contract and group action
pub struct GroupActionSignersQuery {
    /// Data contract ID
    pub contract_id: Identifier,
    /// Group contract position in the data contract definition
    pub group_contract_position: GroupContractPosition,
    /// Group action status
    pub status: GroupActionStatus,
    /// Group action ID
    pub action_id: Identifier,
}

impl Query<GetGroupActionSignersRequest> for GroupActionSignersQuery {
    fn query(self, prove: bool) -> Result<GetGroupActionSignersRequest, Error> {
        let request = GetGroupActionSignersRequest {
            version: Some(get_group_action_signers_request::Version::V0(
                GetGroupActionSignersRequestV0 {
                    contract_id: self.contract_id.to_vec(),
                    group_contract_position: self.group_contract_position as u32,
                    status: self.status as i32,
                    action_id: self.action_id.to_vec(),
                    prove,
                },
            )),
        };

        Ok(request)
    }
}

impl FetchMany<Identifier, GroupActionSigners> for GroupMemberPower {
    type Request = GetGroupActionSignersRequest;
}
