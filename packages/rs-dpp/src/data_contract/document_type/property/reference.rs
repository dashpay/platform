use crate::ProtocolError;
use bincode_derive::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use serde::Serialize;

#[derive(Debug, PartialEq, Clone, Serialize)]
pub struct DocumentPropertyReference {
    pub target: DocumentPropertyReferenceTarget,
    pub must_exist: bool,
}

#[derive(
    Debug, PartialEq, Eq, Clone, Serialize, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum DocumentPropertyReferenceTarget {
    IdentityReferenceTarget,
    ContractReferenceTarget,
}

impl std::fmt::Display for DocumentPropertyReferenceTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DocumentPropertyReferenceTarget::IdentityReferenceTarget => {
                write!(f, "identity reference target")
            }
            DocumentPropertyReferenceTarget::ContractReferenceTarget => {
                write!(f, "contract reference target")
            }
        }
    }
}
