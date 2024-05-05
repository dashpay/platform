use crate::consensus::basic::data_contract::UnknownTransferableTypeError;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;

/// We made this enum because in the future we might have a case where documents are sometimes
/// transferable

#[derive(Clone, Copy, Debug, PartialEq, Default)]
#[repr(u8)]
pub enum Transferable {
    #[default]
    Never = 0,
    Always = 1,
}

impl Transferable {
    pub fn is_transferable(&self) -> bool {
        match self {
            Transferable::Never => false,
            Transferable::Always => true,
        }
    }
}

impl TryFrom<u8> for Transferable {
    type Error = ProtocolError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Never),
            1 => Ok(Self::Always),
            value => Err(ProtocolError::ConsensusError(
                ConsensusError::BasicError(BasicError::UnknownTransferableTypeError(
                    UnknownTransferableTypeError::new(vec![0, 1], value),
                ))
                .into(),
            )),
        }
    }
}
