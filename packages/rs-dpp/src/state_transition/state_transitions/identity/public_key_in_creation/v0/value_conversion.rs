use std::collections::BTreeMap;
use platform_value::Value;
use crate::ProtocolError;
use crate::state_transition::ValueConvert;
use crate::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0;

impl ValueConvert for IdentityPublicKeyInCreationV0 {
    fn from_object(raw_object: Value) -> Result<Self, ProtocolError> where Self: Sized {
        todo!()
    }

    fn from_value_map(raw_data_contract_create_transition: BTreeMap<String, Value>) -> Result<Self, ProtocolError> where Self: Sized {
        todo!()
    }

    fn clean_value(value: &mut Value) -> Result<(), ProtocolError> {
        todo!()
    }
}
