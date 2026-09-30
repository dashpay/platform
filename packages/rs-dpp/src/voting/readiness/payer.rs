use bincode::{Decode, DecodeUntrusted, Encode};
use platform_value::Identifier;
use std::fmt;

/// The party that funded a compilation readiness round and receives its unused remainder when
/// the round retires (replacement, cancellation or activation).
///
/// Only an identity can pay today. The contract bucket variant that the typed storage flags
/// work introduces is appended when it lands; the enum is positional on the wire, so variants
/// are only ever added at the end.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub enum ReadinessPayer {
    /// An identity funded the round; the refund is an identity balance credit.
    Identity(Identifier),
}

impl ReadinessPayer {
    /// The identity that receives the refund, when the payer is an identity.
    pub fn identity_id(&self) -> Option<Identifier> {
        match self {
            ReadinessPayer::Identity(identity_id) => Some(*identity_id),
        }
    }
}

impl fmt::Display for ReadinessPayer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReadinessPayer::Identity(identity_id) => write!(f, "Identity({})", identity_id),
        }
    }
}
