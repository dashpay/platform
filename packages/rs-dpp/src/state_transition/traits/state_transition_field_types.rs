use serde::Serialize;
use crate::serialization_traits::{PlatformSerializable, Signable};

/// The trait contains methods related to conversion of StateTransition into different formats
pub trait StateTransitionFieldTypes: Serialize + Signable + PlatformSerializable {
    // TODO remove this as it is not necessary and can be hardcoded
    fn signature_property_paths() -> Vec<&'static str>;
    fn identifiers_property_paths() -> Vec<&'static str>;
    fn binary_property_paths() -> Vec<&'static str>;
}