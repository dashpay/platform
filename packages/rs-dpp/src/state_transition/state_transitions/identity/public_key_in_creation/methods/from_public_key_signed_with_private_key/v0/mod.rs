use crate::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use crate::identity::IdentityPublicKey;
use crate::serialization::PlatformMessageSignable;
use crate::state_transition::public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Setters;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use crate::{BlsModule, ProtocolError};

impl IdentityPublicKeyInCreation {
    #[inline(always)]
    pub(super) fn from_public_key_signed_with_private_key_v0(
        public_key: IdentityPublicKey,
        state_transition_bytes: &[u8],
        private_key: &[u8],
        bls: &impl BlsModule,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized,
        Self: From<IdentityPublicKey>,
    {
        let key_type = public_key.key_type();
        let mut public_key_with_witness: Self = public_key.into();
        public_key_with_witness.set_signature(
            state_transition_bytes
                .sign_by_private_key(private_key, key_type, bls)?
                .into(),
        );
        Ok(public_key_with_witness)
    }
}
