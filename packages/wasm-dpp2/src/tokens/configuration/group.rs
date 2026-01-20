use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::{IdentifierLikeJs, IdentifierWasm};
use crate::impl_wasm_type_info;
use crate::serialization;
use crate::utils::JsValueExt;
use dpp::data_contract::group::accessors::v0::{GroupV0Getters, GroupV0Setters};
use dpp::data_contract::group::v0::GroupV0;
use dpp::data_contract::group::{Group, GroupMemberPower, GroupRequiredPower};
use dpp::platform_value::string_encoding::Encoding;
use dpp::prelude::Identifier;
use js_sys::{Object, Reflect};
use std::collections::BTreeMap;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue};

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &'static str = r#"
/**
 * Group members mapping: identifier (base58) -> power.
 */
export interface GroupMembers {
    [identifierBase58: string]: number;
}

/**
 * Group serialized as a plain object.
 */
export interface GroupObject {
    $formatVersion: string;
    members: GroupMembers;
    requiredPower: number;
}

/**
 * Group serialized as JSON.
 */
export interface GroupJSON {
    $formatVersion: string;
    members: GroupMembers;
    requiredPower: number;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "GroupMembers")]
    pub type GroupMembersJs;

    #[wasm_bindgen(typescript_type = "GroupObject")]
    pub type GroupObjectJs;

    #[wasm_bindgen(typescript_type = "GroupJSON")]
    pub type GroupJSONJs;
}

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

pub fn members_to_map(
    members_object: &Object,
) -> WasmDppResult<BTreeMap<Identifier, GroupMemberPower>> {
    let members_keys = Object::keys(&members_object);

    let mut members = BTreeMap::new();

    for key in members_keys.iter() {
        let key_str = key
            .as_string()
            .ok_or_else(|| WasmDppError::invalid_argument("cannot convert key to string"))?;

        let identifier: Identifier = IdentifierWasm::try_from(key.clone())
            .map_err(|_| {
                WasmDppError::invalid_argument(format!("Invalid identifier: {}", key_str))
            })?
            .into();

        let val = Reflect::get(members_object, &key).map_err(|_| {
            WasmDppError::invalid_argument(format!("Invalid value at key '{}'", key_str))
        })?;

        let power: GroupMemberPower = serde_wasm_bindgen::from_value(val)
            .map_err(|err| WasmDppError::serialization(err.to_string()))?;

        members.insert(identifier, power);
    }

    Ok(members)
}

#[wasm_bindgen(js_class = Group)]
impl GroupWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        members: &Object,
        required_power: GroupRequiredPower,
    ) -> WasmDppResult<GroupWasm> {
        let members = members_to_map(members)?;

        Ok(GroupWasm(Group::V0(GroupV0 {
            members,
            required_power,
        })))
    }

    #[wasm_bindgen(getter = "members")]
    pub fn get_members(&self) -> WasmDppResult<GroupMembersJs> {
        let members = self.0.members();

        let js_members = Object::new();

        for (k, v) in members {
            Reflect::set(
                &js_members,
                &JsValue::from(k.to_string(Encoding::Base58)),
                &JsValue::from(*v),
            )
            .map_err(|err| {
                let message = err.error_message();
                WasmDppError::generic(format!(
                    "unable to write group member '{}' into JS object: {}",
                    k.to_string(Encoding::Base58),
                    message
                ))
            })?;
        }

        Ok(js_members.unchecked_into())
    }

    #[wasm_bindgen(getter = "requiredPower")]
    pub fn get_required_power(&self) -> GroupRequiredPower {
        self.0.required_power()
    }

    #[wasm_bindgen(setter = "members")]
    pub fn set_members(&mut self, members: &Object) -> WasmDppResult<()> {
        let members = members_to_map(members)?;

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
        member: IdentifierLikeJs,
        member_required_power: GroupRequiredPower,
    ) -> WasmDppResult<()> {
        self.0.set_member_power(member.try_into()?, member_required_power);
        Ok(())
    }

    #[wasm_bindgen(js_name = "toJSON")]
    pub fn to_json(&self) -> WasmDppResult<GroupJSONJs> {
        serialization::to_json(&self.0).map(JsCast::unchecked_into)
    }

    #[wasm_bindgen(js_name = "fromJSON")]
    pub fn from_json(object: GroupJSONJs) -> WasmDppResult<GroupWasm> {
        serialization::from_json(object.into()).map(GroupWasm)
    }

    #[wasm_bindgen(js_name = "toObject")]
    pub fn to_object(&self) -> WasmDppResult<GroupObjectJs> {
        // Use toJSON for serialization because it handles BTreeMap<Identifier, u32>
        // correctly (Identifier becomes base58 string in human-readable mode).
        // This ensures all fields are automatically included when new versions are added.
        serialization::to_json(&self.0).map(JsCast::unchecked_into)
    }

    #[wasm_bindgen(js_name = "fromObject")]
    pub fn from_object(value: GroupObjectJs) -> WasmDppResult<GroupWasm> {
        serialization::from_object(value.into()).map(GroupWasm)
    }
}

impl_wasm_type_info!(GroupWasm, Group);
