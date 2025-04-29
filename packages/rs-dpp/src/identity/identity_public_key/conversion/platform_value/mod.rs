mod v0;
use crate::identity::identity_public_key::IdentityPublicKey;
use crate::serialization::ValueConvertible;
pub use v0::*;

impl ValueConvertible<'_> for IdentityPublicKey {}
