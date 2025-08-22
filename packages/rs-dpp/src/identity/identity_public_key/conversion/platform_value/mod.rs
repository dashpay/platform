mod v0;
use crate::identity::IdentityPublicKey;
use crate::serialization::ValueConvertible;
pub use v0::*;

impl ValueConvertible<'_> for IdentityPublicKey {}
