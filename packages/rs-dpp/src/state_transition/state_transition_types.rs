use serde_repr::{Deserialize_repr, Serialize_repr};

#[repr(u8)]
#[derive(Serialize_repr, Deserialize_repr, PartialEq, Eq, Clone, Copy, Debug)]
pub enum StateTransitionType {
    DataContractCreate = 0,
    DocumentsBatch = 1,
    IdentityCreate = 2,
    IdentityTopUp = 3,
    DataContractUpdate = 4,
}


impl std::default::Default for StateTransitionType {
    fn default() -> Self {
         StateTransitionType::DataContractCreate
    }
}