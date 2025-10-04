use crate::identifier::IdentifierWASM;
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
#[wasm_bindgen(js_name = "GroupWASM")]
pub struct GroupWASM(Group);

impl From<Group> for GroupWASM {
    fn from(group: Group) -> Self {
        GroupWASM(group)
    }
}

impl From<GroupWASM> for Group {
    fn from(group: GroupWASM) -> Self {
        group.0
    }
}

pub fn js_members_to_map(
    js_members: &JsValue,
) -> Result<BTreeMap<Identifier, GroupMemberPower>, JsValue> {
    let members_object = Object::from(js_members.clone());
    let members_keys = Object::keys(&members_object);

    let mut members = BTreeMap::new();

    for key in members_keys.iter() {
        let key_str = key
            .as_string()
            .ok_or_else(|| JsValue::from_str("cannot convert key to string"))?;

        let id_wasm = IdentifierWASM::try_from(key.clone())
            .map_err(|_| JsValue::from_str(&format!("Invalid identifier: {}", key_str)))?;

        let val = Reflect::get(js_members, &key)
            .map_err(|_| JsValue::from_str(&format!("Invalid value at key '{}'", key_str)))?;

        let power: GroupMemberPower = serde_wasm_bindgen::from_value(val)?;

        members.insert(Identifier::from(id_wasm), power);
    }

    Ok(members)
}

#[wasm_bindgen]
impl GroupWASM {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "GroupWASM".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "GroupWASM".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(
        js_members: &JsValue,
        required_power: GroupRequiredPower,
    ) -> Result<GroupWASM, JsValue> {
        let members = js_members_to_map(js_members)?;

        Ok(GroupWASM(Group::V0(GroupV0 {
            members,
            required_power,
        })))
    }

    #[wasm_bindgen(getter = "members")]
    pub fn get_members(&self) -> Result<JsValue, JsValue> {
        let members = self.0.members();

        let js_members = Object::new();

        for (k, v) in members {
            Reflect::set(
                &js_members,
                &JsValue::from(k.to_string(Encoding::Base58)),
                &JsValue::from(v.clone()),
            )?;
        }

        Ok(js_members.into())
    }

    #[wasm_bindgen(getter = "requiredPower")]
    pub fn get_required_power(&self) -> GroupRequiredPower {
        self.0.required_power()
    }

    #[wasm_bindgen(setter = "members")]
    pub fn set_members(&mut self, js_members: &JsValue) -> Result<(), JsValue> {
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
    ) -> Result<(), JsValue> {
        let member = IdentifierWASM::try_from(js_member.clone())?;

        self.0
            .set_member_power(member.into(), member_required_power);

        Ok(())
    }
}
