use crate::consensus::basic::data_contract::UnknownDocumentCreationRestrictionModeError;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use std::fmt;
use std::fmt::{Display, Formatter};

#[derive(Debug, PartialEq, Clone, Copy, Eq, Encode, Decode)]
pub enum CreationRestrictionMode {
    NoRestrictions,
    OwnerOnly,
    NoCreationAllowed,
    AnyGroupMember,
}

impl Display for CreationRestrictionMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            CreationRestrictionMode::NoRestrictions => write!(f, "No Restrictions"),
            CreationRestrictionMode::OwnerOnly => write!(f, "Owner Only"),
            CreationRestrictionMode::NoCreationAllowed => write!(f, "No Creation Allowed"),
            CreationRestrictionMode::AnyGroupMember => write!(f, "Any Group Member"),
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
            3 => Ok(Self::AnyGroupMember),
            value => Err(ProtocolError::ConsensusError(
                ConsensusError::BasicError(
                    BasicError::UnknownDocumentCreationRestrictionModeError(
                        UnknownDocumentCreationRestrictionModeError::new(vec![0, 1, 2, 3], value),
                    ),
                )
                .into(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::basic::BasicError;
    use crate::consensus::ConsensusError;
    use assert_matches::assert_matches;

    #[test]
    fn should_parse_any_group_member_mode() {
        let mode = CreationRestrictionMode::try_from(3).expect("mode 3 should be valid");
        assert_eq!(mode, CreationRestrictionMode::AnyGroupMember);
    }

    #[test]
    fn should_include_new_mode_in_unknown_error_allowed_values() {
        let result = CreationRestrictionMode::try_from(9);

        assert_matches!(
            result,
            Err(ProtocolError::ConsensusError(boxed)) => {
                assert_matches!(
                    boxed.as_ref(),
                    ConsensusError::BasicError(
                        BasicError::UnknownDocumentCreationRestrictionModeError(err)
                    ) if err.allowed_values() == vec![0, 1, 2, 3]
                )
            }
        );
    }
}
