use crate::enums::token::action_goal::ActionGoalWasm;
use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use crate::impl_try_from_js_value;
use crate::impl_wasm_type_info;
use crate::tokens::configuration::action_taker::ActionTakerWasm;
use crate::tokens::configuration::authorized_action_takers::AuthorizedActionTakersWasm;
use crate::tokens::configuration::group::GroupWasm;
use crate::utils::{
    try_from_options, try_from_options_optional_with, try_from_options_with, try_to_u16,
};
use dpp::data_contract::GroupContractPosition;
use dpp::data_contract::change_control_rules::ChangeControlRules;
use dpp::data_contract::change_control_rules::v0::ChangeControlRulesV0;
use dpp::data_contract::group::Group;
use dpp::prelude::Identifier;
use js_sys::Object;
use serde::Deserialize;
use std::collections::BTreeMap;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChangeControlRulesOptions {
    #[serde(default)]
    is_changing_authorized_action_takers_to_no_one_allowed: bool,
    #[serde(default)]
    is_changing_admin_action_takers_to_no_one_allowed: bool,
    #[serde(default)]
    is_self_changing_admin_action_takers_allowed: bool,
}

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
export interface ChangeControlRulesOptions {
    authorizedToMakeChange: AuthorizedActionTakers;
    adminActionTakers: AuthorizedActionTakers;
    isChangingAuthorizedActionTakersToNoOneAllowed?: boolean;
    isChangingAdminActionTakersToNoOneAllowed?: boolean;
    isSelfChangingAdminActionTakersAllowed?: boolean;
}

export interface CanChangeAdminActionTakersOptions {
    adminActionTakers: AuthorizedActionTakers;
    contractOwnerId: IdentifierLike;
    mainGroup?: number;
    groups: Record<number, Group>;
    actionTaker: ActionTaker;
    goal: ActionGoalLike;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "ChangeControlRulesOptions")]
    pub type ChangeControlRulesOptionsJs;

    #[wasm_bindgen(typescript_type = "CanChangeAdminActionTakersOptions")]
    pub type CanChangeAdminActionTakersOptionsJs;
}

#[derive(Clone, Debug, PartialEq)]
#[wasm_bindgen(js_name = "ChangeControlRules")]
pub struct ChangeControlRulesWasm(ChangeControlRules);

impl From<ChangeControlRules> for ChangeControlRulesWasm {
    fn from(value: ChangeControlRules) -> Self {
        Self(value)
    }
}

impl From<ChangeControlRulesWasm> for ChangeControlRules {
    fn from(value: ChangeControlRulesWasm) -> Self {
        value.0
    }
}

#[wasm_bindgen(js_class = ChangeControlRules)]
impl ChangeControlRulesWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(options: ChangeControlRulesOptionsJs) -> WasmDppResult<Self> {
        // Extract AuthorizedActionTakers objects which need special handling
        let authorized_to_make_change: AuthorizedActionTakersWasm =
            try_from_options(&options, "authorizedToMakeChange")?;

        let admin_action_takers: AuthorizedActionTakersWasm =
            try_from_options(&options, "adminActionTakers")?;

        // Extract boolean options with serde (they have defaults)
        let opts: ChangeControlRulesOptions = serde_wasm_bindgen::from_value(options.into())
            .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;

        Ok(ChangeControlRulesWasm(ChangeControlRules::V0(
            ChangeControlRulesV0 {
                authorized_to_make_change: authorized_to_make_change.into(),
                admin_action_takers: admin_action_takers.into(),
                changing_authorized_action_takers_to_no_one_allowed: opts
                    .is_changing_authorized_action_takers_to_no_one_allowed,
                changing_admin_action_takers_to_no_one_allowed: opts
                    .is_changing_admin_action_takers_to_no_one_allowed,
                self_changing_admin_action_takers_allowed: opts
                    .is_self_changing_admin_action_takers_allowed,
            },
        )))
    }

    #[wasm_bindgen(getter = "authorizedToMakeChange")]
    pub fn authorized_to_make_change(&self) -> AuthorizedActionTakersWasm {
        (*self.0.authorized_to_make_change_action_takers()).into()
    }

    #[wasm_bindgen(getter = "adminActionTakers")]
    pub fn admin_action_takers(&self) -> AuthorizedActionTakersWasm {
        (*self.0.admin_action_takers()).into()
    }

    #[wasm_bindgen(getter = "isChangingAuthorizedActionTakersToNoOneAllowed")]
    pub fn is_changing_authorized_action_takers_to_no_one_allowed(&self) -> bool {
        match self.0.clone() {
            ChangeControlRules::V0(v0) => v0.changing_authorized_action_takers_to_no_one_allowed,
        }
    }

    #[wasm_bindgen(getter = "isChangingAdminActionTakersToNoOneAllowed")]
    pub fn is_changing_admin_action_takers_to_no_one_allowed(&self) -> bool {
        match self.0.clone() {
            ChangeControlRules::V0(v0) => v0.changing_admin_action_takers_to_no_one_allowed,
        }
    }

    #[wasm_bindgen(getter = "isSelfChangingAdminActionTakersAllowed")]
    pub fn is_self_changing_admin_action_takers_allowed(&self) -> bool {
        match self.0.clone() {
            ChangeControlRules::V0(v0) => v0.self_changing_admin_action_takers_allowed,
        }
    }

    #[wasm_bindgen(setter = "authorizedToMakeChange")]
    pub fn set_authorized_to_make_change(
        &mut self,
        #[wasm_bindgen(js_name = "authorizedToMakeChange")]
        authorized_to_make_change: &AuthorizedActionTakersWasm,
    ) {
        self.0
            .set_authorized_to_make_change_action_takers(authorized_to_make_change.clone().into());
    }

    #[wasm_bindgen(setter = "adminActionTakers")]
    pub fn set_admin_action_takers(
        &mut self,
        #[wasm_bindgen(js_name = "adminActionTakers")]
        admin_action_takers: &AuthorizedActionTakersWasm,
    ) {
        self.0
            .set_admin_action_takers(admin_action_takers.clone().into());
    }

    #[wasm_bindgen(setter = "isChangingAuthorizedActionTakersToNoOneAllowed")]
    pub fn set_is_changing_authorized_action_takers_to_no_one_allowed(
        &mut self,
        #[wasm_bindgen(js_name = "isChangingAuthorizedActionTakersToNoOneAllowed")]
        is_changing_authorized_action_takers_to_no_one_allowed: bool,
    ) {
        let v0 = match self.0.clone() {
            ChangeControlRules::V0(mut v0) => {
                v0.changing_authorized_action_takers_to_no_one_allowed =
                    is_changing_authorized_action_takers_to_no_one_allowed;
                v0
            }
        };

        self.0 = ChangeControlRules::V0(v0);
    }

    #[wasm_bindgen(setter = "isChangingAdminActionTakersToNoOneAllowed")]
    pub fn set_is_changing_admin_action_takers_to_no_one_allowed(
        &mut self,
        #[wasm_bindgen(js_name = "isChangingAdminActionTakersToNoOneAllowed")]
        is_changing_admin_action_takers_to_no_one_allowed: bool,
    ) {
        let v0 = match self.0.clone() {
            ChangeControlRules::V0(mut v0) => {
                v0.changing_admin_action_takers_to_no_one_allowed =
                    is_changing_admin_action_takers_to_no_one_allowed;
                v0
            }
        };

        self.0 = ChangeControlRules::V0(v0)
    }

    #[wasm_bindgen(setter = "isSelfChangingAdminActionTakersAllowed")]
    pub fn set_is_self_changing_admin_action_takers_allowed(
        &mut self,
        #[wasm_bindgen(js_name = "isSelfChangingAdminActionTakersAllowed")]
        is_self_changing_admin_action_takers_allowed: bool,
    ) {
        let v0 = match self.0.clone() {
            ChangeControlRules::V0(mut v0) => {
                v0.self_changing_admin_action_takers_allowed =
                    is_self_changing_admin_action_takers_allowed;
                v0
            }
        };

        self.0 = ChangeControlRules::V0(v0);
    }

    #[wasm_bindgen(js_name = "canChangeAdminActionTakers")]
    pub fn can_change_admin_action_takers(
        &self,
        options: CanChangeAdminActionTakersOptionsJs,
    ) -> WasmDppResult<bool> {
        let admin_action_takers: AuthorizedActionTakersWasm =
            try_from_options(&options, "adminActionTakers")?;

        let contract_owner_id: Identifier =
            try_from_options::<IdentifierWasm>(&options, "contractOwnerId")?.into();

        let main_group: Option<GroupContractPosition> =
            try_from_options_optional_with(&options, "mainGroup", |v| try_to_u16(v, "mainGroup"))?;

        let action_taker: ActionTakerWasm = try_from_options(&options, "actionTaker")?;

        let goal = try_from_options_with(&options, "goal", |v| ActionGoalWasm::try_from(v.clone()))?;

        // Extract groups - need Object for Object::keys
        let groups_value: JsValue =
            try_from_options_with(&options, "groups", |v| Ok::<_, WasmDppError>(v.clone()))?;
        let groups_object = Object::from(groups_value);
        let groups_keys = Object::keys(&groups_object);

        let mut groups: BTreeMap<GroupContractPosition, Group> = BTreeMap::new();

        for key in groups_keys.iter() {
            let contract_position = try_to_u16(&key, "group contract position")?;

            let group: GroupWasm = try_from_options(
                &groups_object.clone().into(),
                &contract_position.to_string(),
            )?;

            groups.insert(contract_position, group.into());
        }

        Ok(self.0.can_change_admin_action_takers(
            &admin_action_takers.into(),
            &contract_owner_id,
            main_group,
            &groups,
            &action_taker.into(),
            goal.into(),
        ))
    }
}

impl_try_from_js_value!(ChangeControlRulesWasm, "ChangeControlRules");
impl_wasm_type_info!(ChangeControlRulesWasm, ChangeControlRules);
