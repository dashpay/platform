//! GroupStateTransitionInfoStatus wrapper for WASM.
//!
//! This module provides WASM bindings for the GroupStateTransitionInfoStatus enum,
//! which represents group action context for state transitions.

use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use crate::impl_wasm_type_info;
use crate::state_transitions::base::GroupStateTransitionInfoWasm;
use crate::utils::IntoWasm;
use dpp::data_contract::GroupContractPosition;
use dpp::group::{GroupStateTransitionInfo, GroupStateTransitionInfoStatus};
use dpp::prelude::Identifier;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

/// Wrapper for GroupStateTransitionInfoStatus enum.
///
/// This represents the group action context for a state transition:
/// - Proposer: The identity proposing a new group action
/// - OtherSigner: The identity signing/voting on an existing group action
#[derive(Debug, Clone, PartialEq)]
#[wasm_bindgen(js_name = GroupStateTransitionInfoStatus)]
pub struct GroupStateTransitionInfoStatusWasm(GroupStateTransitionInfoStatus);

impl From<GroupStateTransitionInfoStatusWasm> for GroupStateTransitionInfoStatus {
    fn from(status: GroupStateTransitionInfoStatusWasm) -> Self {
        status.0
    }
}

impl From<GroupStateTransitionInfoStatus> for GroupStateTransitionInfoStatusWasm {
    fn from(status: GroupStateTransitionInfoStatus) -> Self {
        GroupStateTransitionInfoStatusWasm(status)
    }
}

#[wasm_bindgen(js_class = GroupStateTransitionInfoStatus)]
impl GroupStateTransitionInfoStatusWasm {
    /// Create a new proposer status for initiating a group action.
    ///
    /// Use this when the identity is proposing a new group action.
    ///
    /// @param groupContractPosition - The position of the group in the contract
    /// @returns GroupStateTransitionInfoStatus for a proposer
    #[wasm_bindgen(js_name = "proposer")]
    pub fn proposer(group_contract_position: u16) -> GroupStateTransitionInfoStatusWasm {
        GroupStateTransitionInfoStatusWasm(
            GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(
                group_contract_position,
            ),
        )
    }

    /// Create a new other signer status for voting on an existing group action.
    ///
    /// Use this when the identity is signing/voting on an action proposed by someone else.
    ///
    /// @param groupContractPosition - The position of the group in the contract
    /// @param actionId - The ID of the action being voted on
    /// @returns GroupStateTransitionInfoStatus for an other signer
    #[wasm_bindgen(js_name = "otherSigner")]
    pub fn other_signer(
        group_contract_position: u16,
        #[wasm_bindgen(unchecked_param_type = "Identifier | Uint8Array | string")]
        action_id: &JsValue,
    ) -> WasmDppResult<GroupStateTransitionInfoStatusWasm> {
        let action_id: Identifier = IdentifierWasm::try_from(action_id)?.into();

        Ok(GroupStateTransitionInfoStatusWasm(
            GroupStateTransitionInfoStatus::GroupStateTransitionInfoOtherSigner(
                GroupStateTransitionInfo {
                    group_contract_position,
                    action_id,
                    action_is_proposer: false,
                },
            ),
        ))
    }

    /// Check if this is a proposer status.
    #[wasm_bindgen(getter = "isProposer")]
    pub fn is_proposer(&self) -> bool {
        matches!(
            self.0,
            GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(_)
        )
    }

    /// Get the group contract position.
    #[wasm_bindgen(getter = "groupContractPosition")]
    pub fn group_contract_position(&self) -> GroupContractPosition {
        match &self.0 {
            GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(pos) => *pos,
            GroupStateTransitionInfoStatus::GroupStateTransitionInfoOtherSigner(info) => {
                info.group_contract_position
            }
        }
    }

    /// Get the action ID (only available for other signer status).
    /// Returns null for proposer status.
    #[wasm_bindgen(getter = "actionId")]
    pub fn action_id(&self) -> Option<IdentifierWasm> {
        match &self.0 {
            GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(_) => None,
            GroupStateTransitionInfoStatus::GroupStateTransitionInfoOtherSigner(info) => {
                Some(info.action_id.into())
            }
        }
    }

    /// Convert to GroupStateTransitionInfo.
    #[wasm_bindgen(js_name = "toInfo")]
    pub fn to_info(&self) -> GroupStateTransitionInfoWasm {
        let info: GroupStateTransitionInfo = self.0.into();
        info.into()
    }
}

impl GroupStateTransitionInfoStatusWasm {
    /// Try to extract a GroupStateTransitionInfoStatus from an options object field.
    pub fn try_from_options(options: &JsValue, field_name: &str) -> WasmDppResult<Self> {
        let field_value = js_sys::Reflect::get(options, &JsValue::from_str(field_name))
            .map_err(|_| WasmDppError::invalid_argument(format!("{} is required", field_name)))?;

        if field_value.is_undefined() || field_value.is_null() {
            return Err(WasmDppError::invalid_argument(format!(
                "'{}' is required",
                field_name
            )));
        }

        // Try to convert using the IntoWasm helper
        field_value
            .to_wasm::<GroupStateTransitionInfoStatusWasm>("GroupStateTransitionInfoStatus")
            .map(|boxed| boxed.clone())
    }

    /// Try to extract an optional GroupStateTransitionInfoStatus from an options object field.
    pub fn try_from_optional_options(
        options: &JsValue,
        field_name: &str,
    ) -> WasmDppResult<Option<Self>> {
        let field_value =
            js_sys::Reflect::get(options, &JsValue::from_str(field_name)).map_err(|_| {
                WasmDppError::invalid_argument(format!("Failed to access {}", field_name))
            })?;

        if field_value.is_undefined() || field_value.is_null() {
            return Ok(None);
        }

        Self::try_from_options(options, field_name).map(Some)
    }
}

impl_wasm_type_info!(GroupStateTransitionInfoStatusWasm, GroupStateTransitionInfoStatus);
