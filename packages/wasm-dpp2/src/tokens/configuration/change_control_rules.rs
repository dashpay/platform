use crate::enums::token::action_goal::ActionGoalWasm;
use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierLikeJs;
use crate::impl_wasm_type_info;
use crate::tokens::configuration::action_taker::ActionTakerWasm;
use crate::tokens::configuration::authorized_action_takers::AuthorizedActionTakersWasm;
use crate::tokens::configuration::group::GroupWasm;
use crate::utils::{IntoWasm, JsValueExt, try_from_options_with};
use dpp::data_contract::GroupContractPosition;
use dpp::data_contract::change_control_rules::ChangeControlRules;
use dpp::data_contract::change_control_rules::v0::ChangeControlRulesV0;
use dpp::data_contract::group::Group;
use dpp::prelude::Identifier;
use js_sys::{Object, Reflect};
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
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "ChangeControlRulesOptions")]
    pub type ChangeControlRulesOptionsJs;
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
        let options: JsValue = options.into();
        let object = Object::from(options.clone());

        // Extract AuthorizedActionTakers objects which need special handling
        let authorized_to_make_change: AuthorizedActionTakersWasm =
            try_from_options_with(&object, "authorizedToMakeChange", |v| {
                v.to_wasm::<AuthorizedActionTakersWasm>("AuthorizedActionTakers")
                    .map(|r| r.clone())
            })?;

        let admin_action_takers: AuthorizedActionTakersWasm =
            try_from_options_with(&object, "adminActionTakers", |v| {
                v.to_wasm::<AuthorizedActionTakersWasm>("AuthorizedActionTakers")
                    .map(|r| r.clone())
            })?;

        // Extract boolean options with serde (they have defaults)
        let opts: ChangeControlRulesOptions = serde_wasm_bindgen::from_value(options)
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
        #[wasm_bindgen(js_name = "adminActionTakers")]
        admin_action_takers: &AuthorizedActionTakersWasm,
        #[wasm_bindgen(js_name = "contractOwnerId")] contract_owner_id: IdentifierLikeJs,
        #[wasm_bindgen(js_name = "mainGroup")] main_group: Option<GroupContractPosition>,
        #[wasm_bindgen(js_name = "groupsValue")] groups_value: &JsValue,
        #[wasm_bindgen(js_name = "actionTaker")] action_taker: &ActionTakerWasm,
        goal: &JsValue,
    ) -> WasmDppResult<bool> {
        let contract_owner_id: Identifier = contract_owner_id.try_into()?;
        let goal = ActionGoalWasm::try_from(goal.clone())?;

        let groups_object = Object::from(groups_value.clone());
        let groups_keys = Object::keys(&groups_object);

        let mut groups: BTreeMap<GroupContractPosition, Group> = BTreeMap::new();

        for key in groups_keys.iter() {
            let key_str = key.as_string().ok_or_else(|| {
                WasmDppError::invalid_argument("Cannot read group contract position")
            })?;

            let contract_position = key_str.parse::<GroupContractPosition>().map_err(|err| {
                WasmDppError::invalid_argument(format!(
                    "Invalid group contract position '{}': {}",
                    key_str, err
                ))
            })?;

            let group_value = Reflect::get(groups_value, &key).map_err(|err| {
                let message = err.error_message();
                WasmDppError::invalid_argument(format!(
                    "unable to read group at contract position '{}': {}",
                    key_str, message
                ))
            })?;

            let group = group_value.to_wasm::<GroupWasm>("Group")?.clone();

            groups.insert(contract_position, group.into());
        }

        Ok(self.0.can_change_admin_action_takers(
            &admin_action_takers.clone().into(),
            &contract_owner_id,
            main_group,
            &groups,
            &action_taker.clone().into(),
            goal.clone().into(),
        ))
    }
}

impl_wasm_type_info!(ChangeControlRulesWasm, ChangeControlRules);
