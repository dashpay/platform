mod v0;
pub use v0::*;
use crate::identity::IdentityPublicKey;
use crate::serialization::ValueConvertible;

impl<'a> ValueConvertible<'a> for IdentityPublicKey {}
