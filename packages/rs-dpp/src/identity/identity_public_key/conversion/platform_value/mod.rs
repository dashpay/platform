mod v0;
use crate::identity::IdentityPublicKey;
use crate::serialization::ValueConvertible;
pub use v0::*;

impl<'a> ValueConvertible<'a> for IdentityPublicKey {}
