use crate::ProtocolError;
use std::convert::TryFrom;

pub mod converter;

/// Credits type
pub type Credits = u64;

/// Signed Credits type is used for internal computations and total credits
/// balance verification
pub type SignedCredits = i64;

/// Maximum value of credits
pub const MAX_CREDITS: Credits = SignedCredits::MAX as Credits;

/// Trait for signed and unsigned credits
pub trait Creditable {
    /// Convert unsigned credit to singed
    fn to_signed(&self) -> Result<SignedCredits, ProtocolError>;
    /// Convert singed credit to unsigned
    fn to_unsigned(&self) -> Credits;
}

impl Creditable for Credits {
    fn to_signed(&self) -> Result<SignedCredits, ProtocolError> {
        SignedCredits::try_from(*self).map_err(|e| {
            ProtocolError::Overflow(
                format!("credits are too big to convert to signed value: {e}").as_str(),
            )
        })
    }

    fn to_unsigned(&self) -> Credits {
        *self
    }
}

impl Creditable for SignedCredits {
    fn to_signed(&self) -> Result<SignedCredits, ProtocolError> {
        Ok(*self)
    }

    fn to_unsigned(&self) -> Credits {
        self.unsigned_abs()
    }
}
