use crate::consensus::basic::data_contract::UnknownDocumentCreationRestrictionModeError;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::identity::state_transition::asset_lock_proof::{Decode, Encode};
use crate::ProtocolError;
use std::fmt;
use std::fmt::{Display, Formatter};

#[derive(Debug, PartialEq, Clone, Copy, Eq, Encode, Decode)]
pub enum CreationRestrictionMode {
    NoRestrictions,
    OwnerOnly,
    NoCreationAllowed,
}

impl Display for CreationRestrictionMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            CreationRestrictionMode::NoRestrictions => write!(f, "No Restrictions"),
            CreationRestrictionMode::OwnerOnly => write!(f, "Owner Only"),
            CreationRestrictionMode::NoCreationAllowed => write!(f, "No Creation Allowed"),
        }
    }
}

impl TryFrom<u8> for CreationRestrictionMode {
    type Error = ProtocolError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::NoRestrictions),
            1 => Ok(Self::OwnerOnly),
            2 => Ok(Self::NoCreationAllowed),
            value => Err(ProtocolError::ConsensusError(
                ConsensusError::BasicError(
                    BasicError::UnknownDocumentCreationRestrictionModeError(
                        UnknownDocumentCreationRestrictionModeError::new(vec![0, 1, 2], value),
                    ),
                )
                .into(),
            )),
        }
    }
}
