use crate::error::WasmSdkError;
use crate::queries::utils::{
    convert_optional_limit, deserialize_required_query, identifiers_from_js,
};
use crate::queries::ProofMetadataResponseWasm;
use crate::sdk::WasmSdk;
use dash_sdk::dpp::data_contract::group::accessors::v0::GroupV0Getters;
use dash_sdk::dpp::data_contract::group::Group;
use dash_sdk::dpp::data_contract::group::GroupMemberPower;
use dash_sdk::dpp::data_contract::GroupContractPosition;
use dash_sdk::dpp::group::group_action::GroupAction;
use dash_sdk::dpp::group::group_action_status::GroupActionStatus;
use dash_sdk::platform::group_actions::{
    GroupActionSignersQuery, GroupActionsQuery, GroupInfosQuery, GroupQuery,
};
use dash_sdk::platform::{Fetch, FetchMany, Identifier};
use js_sys::{Array, BigInt, Map, Number};
use serde::Deserialize;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use wasm_dpp2::group::GroupActionWasm;
use wasm_dpp2::identifier::IdentifierWasm;
use wasm_dpp2::tokens::GroupWasm;

// Proof info functions are now included below

#[wasm_bindgen(js_name = "IdentityGroupInfo")]
pub struct IdentityGroupInfoWasm {
    data_contract_id: String,
    group_contract_position: u32,
    role: String,
    power: Option<GroupMemberPower>,
}

impl IdentityGroupInfoWasm {
    fn new(
        data_contract_id: String,
        group_contract_position: u32,
        role: String,
        power: Option<GroupMemberPower>,
    ) -> Self {
        IdentityGroupInfoWasm {
            data_contract_id,
            group_contract_position,
            role,
            power,
        }
    }
}

#[wasm_bindgen(js_class = IdentityGroupInfo)]
impl IdentityGroupInfoWasm {
    #[wasm_bindgen(getter = "dataContractId")]
    pub fn data_contract_id(&self) -> String {
        self.data_contract_id.clone()
    }

    #[wasm_bindgen(getter = "groupContractPosition")]
    pub fn group_contract_position(&self) -> u32 {
        self.group_contract_position
    }

    #[wasm_bindgen(getter = "role")]
    pub fn role(&self) -> String {
        self.role.clone()
    }

    #[wasm_bindgen(getter = "power")]
    pub fn power(&self) -> Option<BigInt> {
        self.power.map(|value| BigInt::from(value as u64))
    }
}

#[wasm_bindgen(typescript_custom_section)]
const GROUP_ACTIONS_QUERY_TS: &'static str = r#"
/**
 * Group action status filter.
 */
export type GroupActionStatusFilter = 'ACTIVE' | 'CLOSED';

/**
 * Cursor describing where to resume fetching group actions.
 */
export interface GroupActionsStartAt {
  /**
   * Group action identifier.
   */
  actionId: IdentifierLike

  /**
   * Include the `actionId` entry in the result set.
   * @default false
   */
  included?: boolean;
}

/**
 * Query parameters for retrieving group actions.
 */
export interface GroupActionsQuery {
  /**
   * Data contract identifier.
   */
  dataContractId: IdentifierLike

  /**
   * Position of the group within the contract.
   */
  groupContractPosition: number;

  /**
   * Filter actions by status.
   */
  status: GroupActionStatusFilter;

  /**
   * Cursor describing where to resume from.
   * @default undefined
   */
  startAt?: GroupActionsStartAt;

  /**
   * Maximum number of actions to return.
   * @default undefined
   */
  limit?: number;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "GroupActionsQuery")]
    pub type GroupActionsQueryJs;
}

#[wasm_bindgen(typescript_custom_section)]
const GROUP_ACTION_SIGNERS_QUERY_TS: &'static str = r#"
/**
 * Query parameters for retrieving signers of a group action.
 */
export interface GroupActionSignersQuery {
  /**
   * Data contract identifier.
   */
  dataContractId: IdentifierLike

  /**
   * Position of the group within the contract.
   */
  groupContractPosition: number;

  /**
   * Action status filter.
   */
  status: GroupActionStatusFilter;

  /**
   * Group action identifier.
   */
  actionId: IdentifierLike
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "GroupActionSignersQuery")]
    pub type GroupActionSignersQueryJs;
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GroupActionsQueryInput {
    data_contract_id: IdentifierWasm,
    group_contract_position: u32,
    status: String,
    #[serde(default)]
    start_at: Option<GroupActionsStartAtInput>,
    #[serde(default)]
    limit: Option<u32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GroupActionsStartAtInput {
    action_id: IdentifierWasm,
    #[serde(default)]
    included: Option<bool>,
}

struct GroupActionsQueryParsed {
    contract_id: Identifier,
    group_contract_position: GroupContractPosition,
    status: GroupActionStatus,
    start_at: Option<(Identifier, bool)>,
    limit: Option<u16>,
}

fn parse_group_action_status(status: &str) -> Result<GroupActionStatus, WasmSdkError> {
    match status {
        "ACTIVE" => Ok(GroupActionStatus::ActionActive),
        "CLOSED" => Ok(GroupActionStatus::ActionClosed),
        _ => Err(WasmSdkError::invalid_argument(format!(
            "Invalid status: {}. Must be ACTIVE or CLOSED",
            status
        ))),
    }
}

fn parse_group_actions_query(
    query: GroupActionsQueryJs,
) -> Result<GroupActionsQueryParsed, WasmSdkError> {
    let input: GroupActionsQueryInput =
        deserialize_required_query(query, "Query object is required", "group actions query")?;

    let GroupActionsQueryInput {
        data_contract_id,
        group_contract_position,
        status,
        start_at,
        limit,
    } = input;

    let contract_id: Identifier = data_contract_id.into();

    let group_contract_position: GroupContractPosition =
        group_contract_position.try_into().map_err(|_| {
            WasmSdkError::invalid_argument(format!(
                "groupContractPosition {} exceeds maximum of {}",
                group_contract_position,
                u16::MAX,
            ))
        })?;

    let status = parse_group_action_status(&status)?;

    let start_at = if let Some(cursor) = start_at {
        let action_id: Identifier = cursor.action_id.into();
        let included = cursor.included.unwrap_or(false);
        Some((action_id, included))
    } else {
        None
    };

    let limit = convert_optional_limit(limit, "limit")?;

    Ok(GroupActionsQueryParsed {
        contract_id,
        group_contract_position,
        status,
        start_at,
        limit,
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GroupActionSignersQueryInput {
    data_contract_id: IdentifierWasm,
    group_contract_position: u32,
    status: String,
    action_id: IdentifierWasm,
}

struct GroupActionSignersQueryParsed {
    contract_id: Identifier,
    group_contract_position: GroupContractPosition,
    status: GroupActionStatus,
    action_id: Identifier,
}

fn parse_group_action_signers_query(
    query: GroupActionSignersQueryJs,
) -> Result<GroupActionSignersQueryParsed, WasmSdkError> {
    let input: GroupActionSignersQueryInput = deserialize_required_query(
        query,
        "Query object is required",
        "group action signers query",
    )?;
    let GroupActionSignersQueryInput {
        data_contract_id,
        group_contract_position,
        status,
        action_id,
    } = input;
    let contract_id: Identifier = data_contract_id.into();
    let group_contract_position: GroupContractPosition =
        group_contract_position.try_into().map_err(|_| {
            WasmSdkError::invalid_argument(format!(
                "groupContractPosition {} exceeds maximum of {}",
                group_contract_position,
                u16::MAX,
            ))
        })?;
    let status = parse_group_action_status(&status)?;
    let action_id: Identifier = action_id.into();
    Ok(GroupActionSignersQueryParsed {
        contract_id,
        group_contract_position,
        status,
        action_id,
    })
}

#[wasm_bindgen(typescript_custom_section)]
const GROUP_INFOS_QUERY_TS: &'static str = r#"
/**
 * Cursor describing where to resume fetching group infos.
 */
export interface GroupInfosStartAt {
  /**
   * Group contract position.
   */
  position: number;

  /**
   * Include the entry at `position`.
   * @default false
   */
  included?: boolean;
}

/**
 * Query parameters for retrieving group infos.
 */
export interface GroupInfosQuery {
  /**
   * Data contract identifier.
   */
  dataContractId: IdentifierLike

  /**
   * Cursor describing where to resume from.
   * @default undefined
   */
  startAt?: GroupInfosStartAt;

  /**
   * Maximum number of groups to return.
   * @default undefined
   */
  limit?: number;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "GroupInfosQuery")]
    pub type GroupInfosQueryJs;
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GroupInfosQueryInput {
    data_contract_id: IdentifierWasm,
    #[serde(default)]
    start_at: Option<GroupInfosStartAtInput>,
    #[serde(default)]
    limit: Option<u32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GroupInfosStartAtInput {
    position: u32,
    #[serde(default)]
    included: Option<bool>,
}

struct GroupInfosQueryParsed {
    contract_id: Identifier,
    start_at: Option<(GroupContractPosition, bool)>,
    limit: Option<u16>,
}

fn parse_group_infos_query(
    query: GroupInfosQueryJs,
) -> Result<GroupInfosQueryParsed, WasmSdkError> {
    let input: GroupInfosQueryInput =
        deserialize_required_query(query, "Query object is required", "group infos query")?;

    let GroupInfosQueryInput {
        data_contract_id,
        start_at,
        limit,
    } = input;

    let contract_id: Identifier = data_contract_id.into();

    let start_at = if let Some(cursor) = start_at {
        let position = cursor.position as GroupContractPosition;
        let included = cursor.included.unwrap_or(false);
        Some((position, included))
    } else {
        None
    };

    let limit = convert_optional_limit(limit, "limit")?;

    Ok(GroupInfosQueryParsed {
        contract_id,
        start_at,
        limit,
    })
}

#[wasm_bindgen(typescript_custom_section)]
const GROUP_MEMBERS_QUERY_TS: &'static str = r#"
/**
 * Query parameters for retrieving group members.
 */
export interface GroupMembersQuery {
  /**
   * Data contract identifier.
   */
  dataContractId: IdentifierLike

  /**
   * Group position inside the contract.
   */
  groupContractPosition: number;

  /**
   * Optional list of member IDs to retrieve. When provided, pagination options are ignored.
   * @default undefined
   */
  memberIds?: Array<Identifier | Uint8Array | string>;

  /**
   * Member identifier to resume from.
   * @default undefined
   */
  startAtMemberId?: IdentifierLike

  /**
   * Maximum number of members to return when not requesting specific IDs.
   * @default undefined
   */
  limit?: number;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "GroupMembersQuery")]
    pub type GroupMembersQueryJs;
}

#[wasm_bindgen(typescript_custom_section)]
const IDENTITY_GROUPS_QUERY_TS: &'static str = r#"
/**
 * Query parameters for retrieving groups that an identity participates in.
 */
export interface IdentityGroupsQuery {
  /**
   * Identity identifier.
   */
  identityId: IdentifierLike

  /**
   * Data contracts where the identity participates as a member.
   * @default undefined
   */
  memberDataContracts?: Array<Identifier | Uint8Array | string>;

  /**
   * Data contracts where the identity participates as an owner.
   * (Currently not implemented server-side.)
   * @default undefined
   */
  ownerDataContracts?: Array<Identifier | Uint8Array | string>;

  /**
   * Data contracts where the identity participates as a moderator.
   * (Currently not implemented server-side.)
   * @default undefined
   */
  moderatorDataContracts?: Array<Identifier | Uint8Array | string>;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IdentityGroupsQuery")]
    pub type IdentityGroupsQueryJs;
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GroupMembersQueryInput {
    data_contract_id: IdentifierWasm,
    group_contract_position: u32,
    #[serde(default)]
    member_ids: Option<Vec<IdentifierWasm>>,
    #[serde(default)]
    start_at_member_id: Option<IdentifierWasm>,
    #[serde(default)]
    limit: Option<u32>,
}

struct GroupMembersQueryParsed {
    contract_id: Identifier,
    group_contract_position: GroupContractPosition,
    member_ids: Option<Vec<Identifier>>,
    start_at_member_id: Option<Identifier>,
    limit: Option<u16>,
}

fn parse_group_members_query(
    query: GroupMembersQueryJs,
) -> Result<GroupMembersQueryParsed, WasmSdkError> {
    let input: GroupMembersQueryInput =
        deserialize_required_query(query, "Query object is required", "group members query")?;

    let GroupMembersQueryInput {
        data_contract_id,
        group_contract_position,
        member_ids: raw_member_ids,
        start_at_member_id: raw_start_at,
        limit,
    } = input;

    let contract_id: Identifier = data_contract_id.into();

    let limit = convert_optional_limit(limit, "limit")?;

    let member_ids = raw_member_ids.map(|ids| ids.into_iter().map(Identifier::from).collect());

    let start_at_member_id = raw_start_at.map(Identifier::from);

    Ok(GroupMembersQueryParsed {
        contract_id,
        group_contract_position: group_contract_position as GroupContractPosition,
        member_ids,
        start_at_member_id,
        limit,
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdentityGroupsQueryInput {
    identity_id: IdentifierWasm,
    #[serde(default)]
    member_data_contracts: Option<Vec<IdentifierWasm>>,
    #[serde(default)]
    owner_data_contracts: Option<Vec<IdentifierWasm>>,
    #[serde(default)]
    moderator_data_contracts: Option<Vec<IdentifierWasm>>,
}

struct IdentityGroupsQueryParsed {
    identity_id: Identifier,
    member_data_contracts: Option<Vec<Identifier>>,
    owner_data_contracts: Option<Vec<Identifier>>,
    moderator_data_contracts: Option<Vec<Identifier>>,
}

fn parse_identity_groups_query(
    query: IdentityGroupsQueryJs,
) -> Result<IdentityGroupsQueryParsed, WasmSdkError> {
    let input: IdentityGroupsQueryInput =
        deserialize_required_query(query, "Query object is required", "identity groups query")?;

    let IdentityGroupsQueryInput {
        identity_id: identity_js,
        member_data_contracts: member_contracts,
        owner_data_contracts: owner_contracts,
        moderator_data_contracts: moderator_contracts,
    } = input;

    let identity_id: Identifier = identity_js.into();

    let member_data_contracts =
        member_contracts.map(|values| values.into_iter().map(Identifier::from).collect());

    let owner_data_contracts =
        owner_contracts.map(|values| values.into_iter().map(Identifier::from).collect());

    let moderator_data_contracts =
        moderator_contracts.map(|values| values.into_iter().map(Identifier::from).collect());

    Ok(IdentityGroupsQueryParsed {
        identity_id,
        member_data_contracts,
        owner_data_contracts,
        moderator_data_contracts,
    })
}

#[wasm_bindgen]
impl WasmSdk {
    #[wasm_bindgen(js_name = "getGroupInfo")]
    pub async fn get_group_info(
        &self,
        #[wasm_bindgen(js_name = "dataContractId")]
        #[wasm_bindgen(unchecked_param_type = "Identifier | Uint8Array | string")]
        data_contract_id: JsValue,
        #[wasm_bindgen(js_name = "groupContractPosition")] group_contract_position: u32,
    ) -> Result<Option<GroupWasm>, WasmSdkError> {
        // Parse data contract ID
        let contract_id: Identifier = IdentifierWasm::try_from(&data_contract_id)
            .map_err(|err| WasmSdkError::invalid_argument(format!("Invalid contract ID: {}", err)))?
            .into();

        // Create group query
        let query = GroupQuery {
            contract_id,
            group_contract_position: group_contract_position as GroupContractPosition,
        };

        // Fetch the group
        let group = Group::fetch(self.as_ref(), query).await?;

        Ok(group.map(Into::into))
    }

    #[wasm_bindgen(
        js_name = "getGroupMembers",
        unchecked_return_type = "Map<Identifier, bigint>"
    )]
    pub async fn get_group_members(&self, query: GroupMembersQueryJs) -> Result<Map, WasmSdkError> {
        let params = parse_group_members_query(query)?;

        let GroupMembersQueryParsed {
            contract_id,
            group_contract_position,
            member_ids,
            start_at_member_id,
            limit,
        } = params;

        let group_query = GroupQuery {
            contract_id,
            group_contract_position,
        };

        // Fetch the group
        let group = Group::fetch(self.as_ref(), group_query).await?;

        if let Some(group) = group {
            let members =
                collect_group_members_map(&group, &member_ids, &start_at_member_id, limit)?;
            return Ok(members);
        }

        Ok(Map::new())
    }

    #[wasm_bindgen(
        js_name = "getIdentityGroups",
        unchecked_return_type = "Array<IdentityGroupInfo>"
    )]
    pub async fn get_identity_groups(
        &self,
        query: IdentityGroupsQueryJs,
    ) -> Result<Array, WasmSdkError> {
        let IdentityGroupsQueryParsed {
            identity_id,
            member_data_contracts,
            owner_data_contracts,
            moderator_data_contracts,
        } = parse_identity_groups_query(query)?;

        let groups_array = Array::new();

        // Check member data contracts
        if let Some(contracts) = member_data_contracts {
            for contract_id in contracts {
                let contract_id_str = IdentifierWasm::from(contract_id).to_base58();
                // Fetch all groups for this contract
                let query = GroupInfosQuery {
                    contract_id,
                    start_group_contract_position: None,
                    limit: None,
                };

                let groups_result = Group::fetch_many(self.as_ref(), query).await?;

                // Check each group for the identity
                for (position, group_opt) in groups_result {
                    if let Some(group) = group_opt {
                        if let Ok(power) = group.member_power(identity_id) {
                            let entry = IdentityGroupInfoWasm::new(
                                contract_id_str.clone(),
                                position as u32,
                                "member".to_string(),
                                Some(power),
                            );
                            groups_array.push(&JsValue::from(entry));
                        }
                    }
                }
            }
        }

        // Note: Owner and moderator roles would require additional contract queries
        // which are not yet implemented in the SDK. For now, return a warning.
        if owner_data_contracts.is_some() || moderator_data_contracts.is_some() {
            tracing::warn!(
                target = "wasm_sdk",
                "Owner/moderator role queries are not yet implemented"
            );
        }

        Ok(groups_array)
    }

    #[wasm_bindgen(
        js_name = "getGroupInfos",
        unchecked_return_type = "Map<number, Group | undefined>"
    )]
    pub async fn get_group_infos(&self, query: GroupInfosQueryJs) -> Result<Map, WasmSdkError> {
        let params = parse_group_infos_query(query)?;

        // Create query
        let query = GroupInfosQuery {
            contract_id: params.contract_id,
            start_group_contract_position: params.start_at,
            limit: params.limit,
        };

        // Fetch groups
        let groups_result = Group::fetch_many(self.as_ref(), query).await?;

        // Convert result to response format
        let infos_map = Map::new();
        for (position, group_opt) in groups_result {
            let key = Number::from(position as u32);
            let value = JsValue::from(group_opt.map(GroupWasm::from));
            infos_map.set(&key.into(), &value);
        }

        Ok(infos_map)
    }

    #[wasm_bindgen(
        js_name = "getGroupActions",
        unchecked_return_type = "Map<Identifier, GroupAction | undefined>"
    )]
    pub async fn get_group_actions(&self, query: GroupActionsQueryJs) -> Result<Map, WasmSdkError> {
        let params = parse_group_actions_query(query)?;

        // Create query
        let query = GroupActionsQuery {
            contract_id: params.contract_id,
            group_contract_position: params.group_contract_position,
            status: params.status,
            start_at_action_id: params.start_at,
            limit: params.limit,
        };

        // Fetch actions
        let actions_result = GroupAction::fetch_many(self.as_ref(), query).await?;

        let actions_map = Map::new();
        for (action_id, action_opt) in actions_result {
            let key = JsValue::from(IdentifierWasm::from(action_id));
            let value = JsValue::from(action_opt.map(GroupActionWasm::from));
            actions_map.set(&key, &value);
        }

        Ok(actions_map)
    }

    #[wasm_bindgen(
        js_name = "getGroupActionSigners",
        unchecked_return_type = "Map<Identifier, bigint>"
    )]
    pub async fn get_group_action_signers(
        &self,
        query: GroupActionSignersQueryJs,
    ) -> Result<Map, WasmSdkError> {
        let params = parse_group_action_signers_query(query)?;

        // Create query
        let query = GroupActionSignersQuery {
            contract_id: params.contract_id,
            group_contract_position: params.group_contract_position,
            status: params.status,
            action_id: params.action_id,
        };

        // Fetch signers
        let signers_result = GroupMemberPower::fetch_many(self.as_ref(), query).await?;

        let signers_map = Map::new();
        for (signer_id, power_opt) in signers_result {
            if let Some(power) = power_opt {
                let key = JsValue::from(IdentifierWasm::from(signer_id));
                let value = JsValue::from(BigInt::from(power as u64));
                signers_map.set(&key, &value);
            }
        }

        Ok(signers_map)
    }

    #[wasm_bindgen(
        js_name = "getGroupsDataContracts",
        unchecked_return_type = "Map<Identifier, Map<number, Group | undefined>>"
    )]
    pub async fn get_groups_data_contracts(
        &self,
        #[wasm_bindgen(js_name = "dataContractIds")]
        #[wasm_bindgen(unchecked_param_type = "Array<Identifier | Uint8Array | string>")]
        data_contract_ids: Vec<JsValue>,
    ) -> Result<Map, WasmSdkError> {
        let contracts_map = Map::new();

        for contract_js in data_contract_ids {
            let contract_id: Identifier = IdentifierWasm::try_from(&contract_js)
                .map_err(|err| {
                    WasmSdkError::invalid_argument(format!("Invalid contract ID: {}", err))
                })?
                .into();

            let contract_key = JsValue::from(IdentifierWasm::from(contract_id));

            // Fetch all groups for this contract
            let query = GroupInfosQuery {
                contract_id,
                start_group_contract_position: None,
                limit: None,
            };

            let groups_result = Group::fetch_many(self.as_ref(), query).await?;

            let groups_map = Map::new();

            for (position, group_opt) in groups_result {
                let key = Number::from(position as u32);
                let value = JsValue::from(group_opt.map(GroupWasm::from));
                groups_map.set(&key.into(), &value);
            }

            contracts_map.set(&contract_key, &JsValue::from(groups_map));
        }

        Ok(contracts_map)
    }

    // Proof versions for group queries

    #[wasm_bindgen(
        js_name = "getGroupInfoWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<Group | undefined>"
    )]
    pub async fn get_group_info_with_proof_info(
        &self,
        #[wasm_bindgen(js_name = "dataContractId")]
        #[wasm_bindgen(unchecked_param_type = "Identifier | Uint8Array | string")]
        data_contract_id: JsValue,
        #[wasm_bindgen(js_name = "groupContractPosition")] group_contract_position: u32,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        // Parse data contract ID
        let contract_id: Identifier = IdentifierWasm::try_from(&data_contract_id)
            .map_err(|err| WasmSdkError::invalid_argument(format!("Invalid contract ID: {}", err)))?
            .into();

        // Create group query
        let query = GroupQuery {
            contract_id,
            group_contract_position: group_contract_position as GroupContractPosition,
        };

        // Fetch group with proof
        let (group_result, metadata, proof) =
            Group::fetch_with_metadata_and_proof(self.as_ref(), query, None).await?;

        let response = ProofMetadataResponseWasm::from_sdk_parts(
            group_result.map(GroupWasm::from),
            metadata,
            proof,
        )?;

        Ok(response)
    }

    #[wasm_bindgen(
        js_name = "getGroupInfosWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<Map<number, Group | undefined>>"
    )]
    pub async fn get_group_infos_with_proof_info(
        &self,
        query: GroupInfosQueryJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        let params = parse_group_infos_query(query)?;

        // Create query
        let query = GroupInfosQuery {
            contract_id: params.contract_id,
            start_group_contract_position: params.start_at,
            limit: params.limit,
        };

        // Fetch groups with proof
        let (groups_result, metadata, proof) =
            Group::fetch_many_with_metadata_and_proof(self.as_ref(), query, None).await?;

        let infos_map = Map::new();
        for (position, group_opt) in groups_result {
            let key = Number::from(position as u32);
            let value = JsValue::from(group_opt.map(GroupWasm::from));
            infos_map.set(&key.into(), &value);
        }

        let response = ProofMetadataResponseWasm::from_sdk_parts(infos_map, metadata, proof)?;

        Ok(response)
    }

    // Additional proof info versions for remaining group queries

    #[wasm_bindgen(
        js_name = "getGroupMembersWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<Map<Identifier, bigint>>"
    )]
    pub async fn get_group_members_with_proof_info(
        &self,
        query: GroupMembersQueryJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        let params = parse_group_members_query(query)?;

        let GroupMembersQueryParsed {
            contract_id,
            group_contract_position,
            member_ids,
            start_at_member_id,
            limit,
        } = params;

        let group_query = GroupQuery {
            contract_id,
            group_contract_position,
        };

        // Fetch the group with proof
        let (group_result, metadata, proof) =
            Group::fetch_with_metadata_and_proof(self.as_ref(), group_query, None).await?;

        let data = match group_result {
            Some(group) => {
                collect_group_members_map(&group, &member_ids, &start_at_member_id, limit)?.into()
            }
            None => JsValue::UNDEFINED,
        };

        let response = ProofMetadataResponseWasm::from_sdk_parts(data, metadata, proof)?;

        Ok(response)
    }

    #[wasm_bindgen(
        js_name = "getIdentityGroupsWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<Array<IdentityGroupInfo>>"
    )]
    pub async fn get_identity_groups_with_proof_info(
        &self,
        query: IdentityGroupsQueryJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        let IdentityGroupsQueryParsed {
            identity_id,
            member_data_contracts,
            owner_data_contracts,
            moderator_data_contracts,
        } = parse_identity_groups_query(query)?;

        let groups_array = Array::new();
        let mut combined_metadata: Option<dash_sdk::platform::proto::ResponseMetadata> = None;
        let mut combined_proof: Option<dash_sdk::platform::proto::Proof> = None;

        // Check member data contracts
        if let Some(contracts) = member_data_contracts {
            for contract_id in contracts {
                let contract_id_str = IdentifierWasm::from(contract_id).to_base58();
                // Fetch all groups for this contract with proof
                let query = GroupInfosQuery {
                    contract_id,
                    start_group_contract_position: None,
                    limit: None,
                };

                let (groups_result, metadata, proof) =
                    Group::fetch_many_with_metadata_and_proof(self.as_ref(), query, None).await?;

                // Store first metadata and proof
                if combined_metadata.is_none() {
                    combined_metadata = Some(metadata);
                    combined_proof = Some(proof);
                }

                // Check each group for the identity
                for (position, group_opt) in groups_result {
                    if let Some(group) = group_opt {
                        if let Ok(power) = group.member_power(identity_id) {
                            let entry = IdentityGroupInfoWasm::new(
                                contract_id_str.clone(),
                                position as u32,
                                "member".to_string(),
                                Some(power),
                            );
                            groups_array.push(&JsValue::from(entry));
                        }
                    }
                }
            }
        }

        // Note: Owner and moderator roles would require additional contract queries
        // which are not yet implemented in the SDK. For now, return a warning.
        if owner_data_contracts.is_some() || moderator_data_contracts.is_some() {
            tracing::warn!(
                target = "wasm_sdk",
                "Owner/moderator role queries are not yet implemented"
            );
        }

        let metadata = combined_metadata.unwrap_or_default();
        let proof = combined_proof.unwrap_or_default();
        let response = ProofMetadataResponseWasm::from_sdk_parts(groups_array, metadata, proof)?;

        Ok(response)
    }

    #[wasm_bindgen(
        js_name = "getGroupActionsWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<Map<Identifier, GroupAction | undefined>>"
    )]
    pub async fn get_group_actions_with_proof_info(
        &self,
        query: GroupActionsQueryJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        let params = parse_group_actions_query(query)?;

        // Create query
        let query = GroupActionsQuery {
            contract_id: params.contract_id,
            group_contract_position: params.group_contract_position,
            status: params.status,
            start_at_action_id: params.start_at,
            limit: params.limit,
        };

        // Fetch actions with proof
        let (actions_result, metadata, proof) =
            GroupAction::fetch_many_with_metadata_and_proof(self.as_ref(), query, None).await?;

        let actions_map = Map::new();
        for (action_id, action_opt) in actions_result {
            let key = JsValue::from(IdentifierWasm::from(action_id));
            let value = JsValue::from(action_opt.map(GroupActionWasm::from));
            actions_map.set(&key, &value);
        }

        let response = ProofMetadataResponseWasm::from_sdk_parts(actions_map, metadata, proof)?;

        Ok(response)
    }

    #[wasm_bindgen(
        js_name = "getGroupActionSignersWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<Map<Identifier, bigint>>"
    )]
    pub async fn get_group_action_signers_with_proof_info(
        &self,
        query: GroupActionSignersQueryJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        let params = parse_group_action_signers_query(query)?;
        let query = GroupActionSignersQuery {
            contract_id: params.contract_id,
            group_contract_position: params.group_contract_position,
            status: params.status,
            action_id: params.action_id,
        };

        // Fetch signers with proof
        let (signers_result, metadata, proof) =
            GroupMemberPower::fetch_many_with_metadata_and_proof(self.as_ref(), query, None)
                .await?;

        let signers_map = Map::new();
        for (signer_id, power_opt) in signers_result {
            if let Some(power) = power_opt {
                let key = JsValue::from(IdentifierWasm::from(signer_id));
                let value = JsValue::from(BigInt::from(power as u64));
                signers_map.set(&key, &value);
            }
        }

        let response = ProofMetadataResponseWasm::from_sdk_parts(signers_map, metadata, proof)?;

        Ok(response)
    }

    #[wasm_bindgen(
        js_name = "getGroupsDataContractsWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<Map<Identifier, Map<number, Group | undefined>>>"
    )]
    pub async fn get_groups_data_contracts_with_proof_info(
        &self,
        #[wasm_bindgen(js_name = "dataContractIds")]
        #[wasm_bindgen(unchecked_param_type = "Array<Identifier | Uint8Array | string>")]
        data_contract_ids: Vec<JsValue>,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        let contracts_map = Map::new();
        let mut combined_metadata: Option<dash_sdk::platform::proto::ResponseMetadata> = None;
        let mut combined_proof: Option<dash_sdk::platform::proto::Proof> = None;

        let contract_identifiers = identifiers_from_js(data_contract_ids, "contract ID")?;

        for contract_id in contract_identifiers {
            let contract_key = JsValue::from(IdentifierWasm::from(contract_id));

            // Fetch all groups for this contract with proof
            let query = GroupInfosQuery {
                contract_id,
                start_group_contract_position: None,
                limit: None,
            };

            let (groups_result, metadata, proof) =
                Group::fetch_many_with_metadata_and_proof(self.as_ref(), query, None).await?;

            if combined_metadata.is_none() {
                combined_metadata = Some(metadata.clone());
                combined_proof = Some(proof.clone());
            }

            let groups_map = Map::new();
            for (position, group_opt) in groups_result {
                let key = Number::from(position as u32);
                let value = JsValue::from(group_opt.map(GroupWasm::from));
                groups_map.set(&key.into(), &value);
            }

            contracts_map.set(&contract_key, &JsValue::from(groups_map));
        }

        let metadata = combined_metadata.unwrap_or_default();
        let proof = combined_proof.unwrap_or_default();
        let response = ProofMetadataResponseWasm::from_sdk_parts(contracts_map, metadata, proof)?;

        Ok(response)
    }
}

fn insert_member(
    map: &Map,
    identifier: Identifier,
    power: GroupMemberPower,
) -> Result<(), WasmSdkError> {
    let key = JsValue::from(IdentifierWasm::from(identifier));
    let value = JsValue::from(BigInt::from(power as u64));
    map.set(&key, &value);
    Ok(())
}

fn collect_group_members_map(
    group: &Group,
    member_ids: &Option<Vec<Identifier>>,
    start_at: &Option<Identifier>,
    limit: Option<u16>,
) -> Result<Map, WasmSdkError> {
    let members_map = Map::new();

    if let Some(requested_ids) = member_ids {
        for identifier in requested_ids {
            if let Ok(power) = group.member_power(*identifier) {
                insert_member(&members_map, *identifier, power)?;
            }
        }
    } else {
        let start_identifier = *start_at;

        let mut added = 0usize;
        for (identifier, power) in group.members().iter() {
            if let Some(start_id) = start_identifier {
                if *identifier <= start_id {
                    continue;
                }
            }

            insert_member(&members_map, *identifier, *power)?;
            added += 1;

            if let Some(lim) = limit {
                if added >= lim as usize {
                    break;
                }
            }
        }
    }

    Ok(members_map)
}
