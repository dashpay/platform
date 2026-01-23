use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::{IdentifierLikeJs, IdentifierWasm};
use crate::impl_from_for_extern_type;
use crate::impl_wasm_type_info;
use crate::serialization;
use crate::utils::{try_to_array, try_to_map};
use dpp::data_contract::group::accessors::v0::{GroupV0Getters, GroupV0Setters};
use dpp::data_contract::group::v0::GroupV0;
use dpp::data_contract::group::{Group, GroupMemberPower, GroupRequiredPower};
use dpp::prelude::Identifier;
use js_sys::{Map, Reflect};
use std::collections::BTreeMap;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
/**
 * Group members mapping: base58 Identifier string -> power.
 */
export type GroupMembersMap = Map<string, number>;

/**
 * Group serialized as a plain object.
 */
export interface GroupObject {
    $formatVersion: string;
    members: Record<string, number>;
    requiredPower: number;
}

/**
 * Group serialized as JSON.
 */
export interface GroupJSON {
    $formatVersion: string;
    members: Record<string, number>;
    requiredPower: number;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "GroupMembersMap")]
    pub type GroupMembersMapJs;

    #[wasm_bindgen(typescript_type = "GroupObject")]
    pub type GroupObjectJs;

    #[wasm_bindgen(typescript_type = "GroupJSON")]
    pub type GroupJSONJs;
}

impl_from_for_extern_type!(GroupMembersMapJs, Map);

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

pub fn members_map_to_btree(
    members_map: &Map,
) -> WasmDppResult<BTreeMap<Identifier, GroupMemberPower>> {
    let mut members = BTreeMap::new();

    // Iterate using entries()
    let entries = members_map.entries();
    loop {
        let next = entries.next().map_err(|e| {
            WasmDppError::invalid_argument(format!("Failed to iterate map entries: {:?}", e))
        })?;

        let done = Reflect::get(&next, &JsValue::from_str("done"))
            .map_err(|_| WasmDppError::invalid_argument("Failed to get 'done' property"))?
            .as_bool()
            .unwrap_or(true);

        if done {
            break;
        }

        let entry_value = Reflect::get(&next, &JsValue::from_str("value"))
            .map_err(|_| WasmDppError::invalid_argument("Failed to get 'value' property"))?;

        let entry_array = try_to_array(entry_value, "map entry")?;
        let key = entry_array.get(0);
        let value = entry_array.get(1);

        let identifier: Identifier = IdentifierWasm::try_from(key)
            .map_err(|e| WasmDppError::invalid_argument(format!("Invalid identifier key: {}", e)))?
            .into();

        let power: GroupMemberPower = serde_wasm_bindgen::from_value(value)
            .map_err(|err| WasmDppError::serialization(err.to_string()))?;

        members.insert(identifier, power);
    }

    Ok(members)
}

#[wasm_bindgen(js_class = Group)]
impl GroupWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        members: GroupMembersMapJs,
        #[wasm_bindgen(js_name = "requiredPower")] required_power: GroupRequiredPower,
    ) -> WasmDppResult<GroupWasm> {
        let members = members_map_to_btree(&try_to_map(members.into(), "members")?)?;

        Ok(GroupWasm(Group::V0(GroupV0 {
            members,
            required_power,
        })))
    }

    #[wasm_bindgen(getter = "members")]
    pub fn members(&self) -> WasmDppResult<GroupMembersMapJs> {
        let members = self.0.members();

        let js_map = Map::new();

        for (k, v) in members {
            let identifier_wasm = IdentifierWasm::from(*k);
            js_map.set(&identifier_wasm.to_base58().into(), &JsValue::from(*v));
        }

        Ok(js_map.into())
    }

    #[wasm_bindgen(getter = "requiredPower")]
    pub fn required_power(&self) -> GroupRequiredPower {
        self.0.required_power()
    }

    #[wasm_bindgen(setter = "members")]
    pub fn set_members(&mut self, members: GroupMembersMapJs) -> WasmDppResult<()> {
        let members = members_map_to_btree(&try_to_map(members.into(), "members")?)?;

        self.0.set_members(members);

        Ok(())
    }

    #[wasm_bindgen(setter = "requiredPower")]
    pub fn set_required_power(&mut self, #[wasm_bindgen(js_name = "requiredPower")] required_power: GroupRequiredPower) {
        self.0.set_required_power(required_power);
    }

    #[wasm_bindgen(js_name = "setMemberRequiredPower")]
    pub fn set_member_required_power(
        &mut self,
        member: IdentifierLikeJs,
        #[wasm_bindgen(js_name = "memberRequiredPower")] member_required_power: GroupRequiredPower,
    ) -> WasmDppResult<()> {
        self.0
            .set_member_power(member.try_into()?, member_required_power);
        Ok(())
    }

    #[wasm_bindgen(js_name = "toJSON")]
    pub fn to_json(&self) -> WasmDppResult<GroupJSONJs> {
        serialization::to_json(&self.0).map(Into::into)
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
        serialization::to_json(&self.0).map(Into::into)
    }

    #[wasm_bindgen(js_name = "fromObject")]
    pub fn from_object(value: GroupObjectJs) -> WasmDppResult<GroupWasm> {
        serialization::from_object(value.into()).map(GroupWasm)
    }
}

impl_wasm_type_info!(GroupWasm, Group);
