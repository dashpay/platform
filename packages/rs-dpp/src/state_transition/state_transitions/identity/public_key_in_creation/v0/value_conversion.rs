use crate::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0;
use crate::state_transition::StateTransitionValueConvert;

impl StateTransitionValueConvert<'_> for IdentityPublicKeyInCreationV0 {
    // this might be faster (todo: check)
    // fn from_value_map(mut value_map: BTreeMap<String, Value>) -> Result<Self, ProtocolError> where Self: Sized {
    //         Ok(Self {
    //             id: value_map
    //                 .get_integer("id")
    //                 .map_err(ProtocolError::ValueError)?,
    //             purpose: value_map
    //                 .get_integer::<u8>("purpose")
    //                 .map_err(ProtocolError::ValueError)?
    //                 .try_into()?,
    //             security_level: value_map
    //                 .get_integer::<u8>("securityLevel")
    //                 .map_err(ProtocolError::ValueError)?
    //                 .try_into()?,
    //             key_type: value_map
    //                 .get_integer::<u8>("keyType")
    //                 .map_err(ProtocolError::ValueError)?
    //                 .try_into()?,
    //             data: value_map
    //                 .remove_binary_data("data")
    //                 .map_err(ProtocolError::ValueError)?,
    //             read_only: value_map
    //                 .get_bool("readOnly")
    //                 .map_err(ProtocolError::ValueError)?,
    //             signature: value_map
    //                 .remove_binary_data("signature")
    //                 .map_err(ProtocolError::ValueError)?,
    //         })
    // }
}
