use crate::error::WasmSdkError;
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
use js_sys::{Array, BigInt, Map, Number, Reflect};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use wasm_dpp2::group::GroupActionWasm;
use wasm_dpp2::identifier::IdentifierWasm;
use wasm_dpp2::tokens::GroupWasm;
use wasm_dpp2::utils::JsValueExt;

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

#[wasm_bindgen]
impl WasmSdk {
    #[wasm_bindgen(js_name = "getGroupInfo")]
    pub async fn get_group_info(
        &self,
        data_contract_id: &str,
        group_contract_position: u32,
    ) -> Result<Option<GroupWasm>, WasmSdkError> {
        // Parse data contract ID
        let contract_id = Identifier::from_string(
            data_contract_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid contract ID: {}", e)))?;

        // Create group query
        let query = GroupQuery {
            contract_id,
            group_contract_position: group_contract_position as GroupContractPosition,
        };

        // Fetch the group
        let group = Group::fetch(self.as_ref(), query).await?;

        Ok(group.map(Into::into))
    }

    #[wasm_bindgen(js_name = "getGroupMembers")]
    pub async fn get_group_members(
        &self,
        data_contract_id: &str,
        group_contract_position: u32,
        member_ids: Option<Vec<String>>,
        start_at: Option<String>,
        limit: Option<u32>,
    ) -> Result<Map, WasmSdkError> {
        // Parse data contract ID
        let contract_id = Identifier::from_string(
            data_contract_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid contract ID: {}", e)))?;

        // Create group query
        let query = GroupQuery {
            contract_id,
            group_contract_position: group_contract_position as GroupContractPosition,
        };

        // Fetch the group
        let group = Group::fetch(self.as_ref(), query).await?;

        if let Some(group) = group {
            let members = collect_group_members_map(&group, &member_ids, &start_at, limit)?;
            return Ok(members);
        };

        Ok(Map::new())
    }

    #[wasm_bindgen(js_name = "getIdentityGroups")]
    pub async fn get_identity_groups(
        &self,
        identity_id: &str,
        member_data_contracts: Option<Vec<String>>,
        owner_data_contracts: Option<Vec<String>>,
        moderator_data_contracts: Option<Vec<String>>,
    ) -> Result<Array, WasmSdkError> {
        // Parse identity ID
        let id = Identifier::from_string(
            identity_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", e)))?;

        let groups_array = Array::new();

        // Check member data contracts
        if let Some(contracts) = member_data_contracts {
            for contract_id_str in contracts {
                let contract_id = Identifier::from_string(
                    &contract_id_str,
                    dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
                )
                .map_err(|e| {
                    WasmSdkError::invalid_argument(format!(
                        "Invalid contract ID '{}': {}",
                        contract_id_str, e
                    ))
                })?;

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
                        if let Ok(power) = group.member_power(id) {
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

    #[wasm_bindgen(js_name = "getGroupInfos")]
    pub async fn get_group_infos(
        &self,
        contract_id: &str,
        start_at_info: JsValue,
        count: Option<u32>,
    ) -> Result<Map, WasmSdkError> {
        // Parse contract ID
        let contract_id = Identifier::from_string(
            contract_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid contract ID: {}", e)))?;

        // Parse start at info if provided
        let start_group_contract_position = if !start_at_info.is_null()
            && !start_at_info.is_undefined()
        {
            let position_value = Reflect::get(&start_at_info, &JsValue::from_str("position"))
                .map_err(|err| WasmSdkError::invalid_argument(err.error_message()))?;
            let position = position_value
                .as_f64()
                .ok_or_else(|| WasmSdkError::invalid_argument("Invalid start position"))?
                as GroupContractPosition;

            let included = Reflect::get(&start_at_info, &JsValue::from_str("included"))
                .ok()
                .and_then(|value| value.as_bool())
                .unwrap_or(false);

            Some((position, included))
        } else {
            None
        };

        // Create query
        let query = GroupInfosQuery {
            contract_id,
            start_group_contract_position,
            limit: count.map(|c| c as u16),
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

    #[wasm_bindgen(js_name = "getGroupActions")]
    pub async fn get_group_actions(
        &self,
        contract_id: &str,
        group_contract_position: u32,
        status: &str,
        start_at_info: JsValue,
        count: Option<u32>,
    ) -> Result<Map, WasmSdkError> {
        // Parse contract ID
        let contract_id = Identifier::from_string(
            contract_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid contract ID: {}", e)))?;

        // Parse status
        let status = match status {
            "ACTIVE" => GroupActionStatus::ActionActive,
            "CLOSED" => GroupActionStatus::ActionClosed,
            _ => {
                return Err(WasmSdkError::invalid_argument(format!(
                    "Invalid status: {}. Must be ACTIVE or CLOSED",
                    status
                )))
            }
        };

        // Parse start action ID if provided
        let start_at_action_id = if !start_at_info.is_null() && !start_at_info.is_undefined() {
            let action_id_value = Reflect::get(&start_at_info, &JsValue::from_str("actionId"))
                .map_err(|err| WasmSdkError::invalid_argument(err.error_message()))?;
            let action_id_str = action_id_value
                .as_string()
                .ok_or_else(|| WasmSdkError::invalid_argument("Invalid action ID"))?;
            let included = Reflect::get(&start_at_info, &JsValue::from_str("included"))
                .ok()
                .and_then(|value| value.as_bool())
                .unwrap_or(false);
            let identifier = Identifier::from_string(
                &action_id_str,
                dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
            )
            .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid action ID: {}", e)))?;
            Some((identifier, included))
        } else {
            None
        };

        // Create query
        let query = GroupActionsQuery {
            contract_id,
            group_contract_position: group_contract_position as GroupContractPosition,
            status,
            start_at_action_id,
            limit: count.map(|c| c as u16),
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

    #[wasm_bindgen(js_name = "getGroupActionSigners")]
    pub async fn get_group_action_signers(
        &self,
        contract_id: &str,
        group_contract_position: u32,
        status: &str,
        action_id: &str,
    ) -> Result<Map, WasmSdkError> {
        // Parse contract ID
        let contract_id = Identifier::from_string(
            contract_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid contract ID: {}", e)))?;

        // Parse action ID
        let action_id = Identifier::from_string(
            action_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid action ID: {}", e)))?;

        // Parse status
        let status = match status {
            "ACTIVE" => GroupActionStatus::ActionActive,
            "CLOSED" => GroupActionStatus::ActionClosed,
            _ => {
                return Err(WasmSdkError::invalid_argument(format!(
                    "Invalid status: {}. Must be ACTIVE or CLOSED",
                    status
                )))
            }
        };

        // Create query
        let query = GroupActionSignersQuery {
            contract_id,
            group_contract_position: group_contract_position as GroupContractPosition,
            status,
            action_id,
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

    #[wasm_bindgen(js_name = "getGroupsDataContracts")]
    pub async fn get_groups_data_contracts(
        &self,
        data_contract_ids: Vec<String>,
    ) -> Result<Map, WasmSdkError> {
        let contracts_map = Map::new();

        for contract_id_str in data_contract_ids {
            let contract_id = Identifier::from_string(
                &contract_id_str,
                dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
            )
            .map_err(|e| {
                WasmSdkError::invalid_argument(format!(
                    "Invalid contract ID '{}': {}",
                    contract_id_str, e
                ))
            })?;

            let contract_key = JsValue::from(IdentifierWasm::from(contract_id.clone()));

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

    #[wasm_bindgen(js_name = "getGroupInfoWithProofInfo")]
    pub async fn get_group_info_with_proof_info(
        &self,
        data_contract_id: &str,
        group_contract_position: u32,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        // Parse data contract ID
        let contract_id = Identifier::from_string(
            data_contract_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid contract ID: {}", e)))?;

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
        );

        Ok(response)
    }

    #[wasm_bindgen(js_name = "getGroupInfosWithProofInfo")]
    pub async fn get_group_infos_with_proof_info(
        &self,
        contract_id: &str,
        start_at_info: JsValue,
        count: Option<u32>,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        // Parse contract ID
        let contract_id = Identifier::from_string(
            contract_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid contract ID: {}", e)))?;

        // Parse start at info if provided
        let start_group_contract_position = if !start_at_info.is_null()
            && !start_at_info.is_undefined()
        {
            let position_value = Reflect::get(&start_at_info, &JsValue::from_str("position"))
                .map_err(|err| WasmSdkError::invalid_argument(err.error_message()))?;
            let position = position_value
                .as_f64()
                .ok_or_else(|| WasmSdkError::invalid_argument("Invalid start position"))?
                as GroupContractPosition;

            let included = Reflect::get(&start_at_info, &JsValue::from_str("included"))
                .ok()
                .and_then(|value| value.as_bool())
                .unwrap_or(false);

            Some((position, included))
        } else {
            None
        };

        // Create query
        let query = GroupInfosQuery {
            contract_id,
            start_group_contract_position,
            limit: count.map(|c| c as u16),
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

        let response = ProofMetadataResponseWasm::from_sdk_parts(infos_map, metadata, proof);

        Ok(response)
    }

    // Additional proof info versions for remaining group queries

    #[wasm_bindgen(js_name = "getGroupMembersWithProofInfo")]
    pub async fn get_group_members_with_proof_info(
        &self,
        data_contract_id: &str,
        group_contract_position: u32,
        member_ids: Option<Vec<String>>,
        start_at: Option<String>,
        limit: Option<u32>,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        // Parse data contract ID
        let contract_id = Identifier::from_string(
            data_contract_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid contract ID: {}", e)))?;

        // Create group query
        let query = GroupQuery {
            contract_id,
            group_contract_position: group_contract_position as GroupContractPosition,
        };

        // Fetch the group with proof
        let (group_result, metadata, proof) =
            Group::fetch_with_metadata_and_proof(self.as_ref(), query, None).await?;

        let data = match group_result {
            Some(group) => collect_group_members_map(&group, &member_ids, &start_at, limit)?.into(),
            None => JsValue::UNDEFINED,
        };

        let response = ProofMetadataResponseWasm::from_sdk_parts(data, metadata, proof);

        Ok(response)
    }

    #[wasm_bindgen(js_name = "getIdentityGroupsWithProofInfo")]
    pub async fn get_identity_groups_with_proof_info(
        &self,
        identity_id: &str,
        member_data_contracts: Option<Vec<String>>,
        owner_data_contracts: Option<Vec<String>>,
        moderator_data_contracts: Option<Vec<String>>,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        // Parse identity ID
        let id = Identifier::from_string(
            identity_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", e)))?;

        let groups_array = Array::new();
        let mut combined_metadata: Option<dash_sdk::platform::proto::ResponseMetadata> = None;
        let mut combined_proof: Option<dash_sdk::platform::proto::Proof> = None;

        // Check member data contracts
        if let Some(contracts) = member_data_contracts {
            for contract_id_str in contracts {
                let contract_id = Identifier::from_string(
                    &contract_id_str,
                    dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
                )
                .map_err(|e| {
                    WasmSdkError::invalid_argument(format!(
                        "Invalid contract ID '{}': {}",
                        contract_id_str, e
                    ))
                })?;

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
                        if let Ok(power) = group.member_power(id) {
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
        let response = ProofMetadataResponseWasm::from_sdk_parts(groups_array, metadata, proof);

        Ok(response)
    }

    #[wasm_bindgen(js_name = "getGroupActionsWithProofInfo")]
    pub async fn get_group_actions_with_proof_info(
        &self,
        contract_id: &str,
        group_contract_position: u32,
        status: &str,
        start_at_info: JsValue,
        count: Option<u32>,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        // Parse contract ID
        let contract_id = Identifier::from_string(
            contract_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid contract ID: {}", e)))?;

        // Parse status
        let status = match status {
            "ACTIVE" => GroupActionStatus::ActionActive,
            "CLOSED" => GroupActionStatus::ActionClosed,
            _ => {
                return Err(WasmSdkError::invalid_argument(format!(
                    "Invalid status: {}. Must be ACTIVE or CLOSED",
                    status
                )))
            }
        };

        // Parse start action ID if provided
        let start_at_action_id = if !start_at_info.is_null() && !start_at_info.is_undefined() {
            let action_id_value = Reflect::get(&start_at_info, &JsValue::from_str("actionId"))
                .map_err(|err| WasmSdkError::invalid_argument(err.error_message()))?;
            let action_id_str = action_id_value
                .as_string()
                .ok_or_else(|| WasmSdkError::invalid_argument("Invalid action ID"))?;
            let included = Reflect::get(&start_at_info, &JsValue::from_str("included"))
                .ok()
                .and_then(|value| value.as_bool())
                .unwrap_or(false);
            let identifier = Identifier::from_string(
                &action_id_str,
                dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
            )
            .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid action ID: {}", e)))?;
            Some((identifier, included))
        } else {
            None
        };

        // Create query
        let query = GroupActionsQuery {
            contract_id,
            group_contract_position: group_contract_position as GroupContractPosition,
            status,
            start_at_action_id,
            limit: count.map(|c| c as u16),
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

        let response = ProofMetadataResponseWasm::from_sdk_parts(actions_map, metadata, proof);

        Ok(response)
    }

    #[wasm_bindgen(js_name = "getGroupActionSignersWithProofInfo")]
    pub async fn get_group_action_signers_with_proof_info(
        &self,
        contract_id: &str,
        group_contract_position: u32,
        status: &str,
        action_id: &str,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        // Parse contract ID
        let contract_id = Identifier::from_string(
            contract_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid contract ID: {}", e)))?;

        // Parse action ID
        let action_id = Identifier::from_string(
            action_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid action ID: {}", e)))?;

        // Parse status
        let status = match status {
            "ACTIVE" => GroupActionStatus::ActionActive,
            "CLOSED" => GroupActionStatus::ActionClosed,
            _ => {
                return Err(WasmSdkError::invalid_argument(format!(
                    "Invalid status: {}. Must be ACTIVE or CLOSED",
                    status
                )))
            }
        };

        // Create query
        let query = GroupActionSignersQuery {
            contract_id,
            group_contract_position: group_contract_position as GroupContractPosition,
            status,
            action_id,
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

        let response = ProofMetadataResponseWasm::from_sdk_parts(signers_map, metadata, proof);

        Ok(response)
    }

    #[wasm_bindgen(js_name = "getGroupsDataContractsWithProofInfo")]
    pub async fn get_groups_data_contracts_with_proof_info(
        &self,
        data_contract_ids: Vec<String>,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        let contracts_map = Map::new();
        let mut combined_metadata: Option<dash_sdk::platform::proto::ResponseMetadata> = None;
        let mut combined_proof: Option<dash_sdk::platform::proto::Proof> = None;

        for contract_id_str in data_contract_ids {
            let contract_id = Identifier::from_string(
                &contract_id_str,
                dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
            )
            .map_err(|e| {
                WasmSdkError::invalid_argument(format!(
                    "Invalid contract ID '{}': {}",
                    contract_id_str, e
                ))
            })?;
            let contract_key = JsValue::from(IdentifierWasm::from(contract_id.clone()));

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
        let response = ProofMetadataResponseWasm::from_sdk_parts(contracts_map, metadata, proof);

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
    member_ids: &Option<Vec<String>>,
    start_at: &Option<String>,
    limit: Option<u32>,
) -> Result<Map, WasmSdkError> {
    let members_map = Map::new();

    if let Some(requested_ids) = member_ids {
        for id_str in requested_ids {
            let identifier = Identifier::from_string(
                id_str,
                dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
            )
            .map_err(|e| {
                WasmSdkError::invalid_argument(format!("Invalid member identity ID: {}", e))
            })?;

            if let Ok(power) = group.member_power(identifier) {
                insert_member(&members_map, identifier, power)?;
            }
        }
    } else {
        let mut start_identifier = None;
        if let Some(start_id) = start_at {
            let identifier = Identifier::from_string(
                start_id,
                dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
            )
            .map_err(|e| {
                WasmSdkError::invalid_argument(format!("Invalid start identity ID: {}", e))
            })?;
            start_identifier = Some(identifier);
        }

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
