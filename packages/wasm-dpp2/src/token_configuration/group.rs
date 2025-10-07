use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use dpp::data_contract::group::accessors::v0::{GroupV0Getters, GroupV0Setters};
use dpp::data_contract::group::v0::GroupV0;
use dpp::data_contract::group::{Group, GroupMemberPower, GroupRequiredPower};
use dpp::platform_value::string_encoding::Encoding;
use dpp::prelude::Identifier;
use js_sys::Object;
use js_sys::Reflect;
use std::collections::BTreeMap;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Clone, PartialEq, Debug)]
#[wasm_bindgen(js_name = "Group")]
pub struct GroupWasm(Group);

impl From<Group> for GroupWasm {
    fn from(group: Group) -> Self {
        GroupWasm(group)
    }
}

impl From<GroupWasm> for Group {
    fn from(group: GroupWasm) -> Self {
        group.0
    }
}

pub fn js_members_to_map(
    js_members: &JsValue,
) -> WasmDppResult<BTreeMap<Identifier, GroupMemberPower>> {
    let members_object = Object::from(js_members.clone());
    let members_keys = Object::keys(&members_object);

    let mut members = BTreeMap::new();

    for key in members_keys.iter() {
        let key_str = key
            .as_string()
            .ok_or_else(|| WasmDppError::invalid_argument("cannot convert key to string"))?;

        let id_wasm = IdentifierWasm::try_from(key.clone()).map_err(|_| {
            WasmDppError::invalid_argument(format!("Invalid identifier: {}", key_str))
        })?;

        let val = Reflect::get(js_members, &key).map_err(|_| {
            WasmDppError::invalid_argument(format!("Invalid value at key '{}'", key_str))
        })?;

        let power: GroupMemberPower = serde_wasm_bindgen::from_value(val)
            .map_err(|err| WasmDppError::serialization(err.to_string()))?;

        members.insert(Identifier::from(id_wasm), power);
    }

    Ok(members)
}

#[wasm_bindgen(js_class = Group)]
impl GroupWasm {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "Group".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "Group".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(
        js_members: &JsValue,
        required_power: GroupRequiredPower,
    ) -> WasmDppResult<GroupWasm> {
        let members = js_members_to_map(js_members)?;

        Ok(GroupWasm(Group::V0(GroupV0 {
            members,
            required_power,
        })))
    }

    #[wasm_bindgen(getter = "members")]
    pub fn get_members(&self) -> WasmDppResult<JsValue> {
        let members = self.0.members();

        let js_members = Object::new();

        for (k, v) in members {
            Reflect::set(
                &js_members,
                &JsValue::from(k.to_string(Encoding::Base58)),
                &JsValue::from(v.clone()),
            )
            .map_err(|err| WasmDppError::from_js_value(err))?;
        }

        Ok(js_members.into())
    }

    #[wasm_bindgen(getter = "requiredPower")]
    pub fn get_required_power(&self) -> GroupRequiredPower {
        self.0.required_power()
    }

    #[wasm_bindgen(setter = "members")]
    pub fn set_members(&mut self, js_members: &JsValue) -> WasmDppResult<()> {
        let members = js_members_to_map(js_members)?;

        self.0.set_members(members);

        Ok(())
    }

    #[wasm_bindgen(setter = "requiredPower")]
    pub fn set_required_power(&mut self, required_power: GroupRequiredPower) {
        self.0.set_required_power(required_power);
    }

    #[wasm_bindgen(js_name = "setMemberRequiredPower")]
    pub fn set_member_required_power(
        &mut self,
        js_member: &JsValue,
        member_required_power: GroupRequiredPower,
    ) -> WasmDppResult<()> {
        let member = IdentifierWasm::try_from(js_member.clone())?;

        self.0
            .set_member_power(member.into(), member_required_power);

        Ok(())
    }
}
