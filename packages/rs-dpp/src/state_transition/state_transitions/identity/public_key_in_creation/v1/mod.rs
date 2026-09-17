mod types;
mod version;

use crate::errors::ProtocolError;
use crate::fee::Credits;
use crate::identity::contract_bounds::ContractBounds;
use crate::identity::identity_public_key::v1::IdentityPublicKeyV1;
use crate::identity::{IdentityPublicKey, KeyID, KeyType, Purpose, SecurityLevel, TimestampMillis};
#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
use crate::state_transition::public_key_in_creation::accessors::{
    IdentityPublicKeyInCreationV0Getters, IdentityPublicKeyInCreationV0Setters,
    IdentityPublicKeyInCreationV1Getters,
};
use crate::state_transition::public_key_in_creation::methods::IdentityPublicKeyInCreationMethodsV0;
use crate::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::PlatformSignable;
use platform_value::BinaryData;
use serde::{Deserialize, Serialize};

/// A public key in creation that may carry usage limits, from protocol version 14.
///
/// The fields up to `data` are the `IdentityPublicKeyInCreationV0` fields in the same order.
/// `budget` and `expires_at` are part of the signable bytes: the identity signs the limits it
/// grants.
#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[derive(
    Default,
    Debug,
    Serialize,
    Deserialize,
    Encode,
    Decode,
    PlatformSignable,
    Clone,
    PartialEq,
    Eq,
    DecodeUntrusted,
)]
#[serde(rename_all = "camelCase")]
pub struct IdentityPublicKeyInCreationV1 {
    pub id: KeyID,
    #[serde(rename = "type")]
    pub key_type: KeyType,
    pub purpose: Purpose,
    pub security_level: SecurityLevel,
    pub contract_bounds: Option<ContractBounds>,
    pub read_only: bool,
    pub data: BinaryData,
    /// The total credits that state transitions signed with this key may take from the identity
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget: Option<Credits>,
    /// The block time, in milliseconds, from which the key can no longer sign
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<TimestampMillis>,
    /// The signature is needed for ECDSA_SECP256K1 Key type and BLS12_381 Key type
    #[platform_signable(exclude_from_sig_hash)]
    pub signature: BinaryData,
}

impl IdentityPublicKeyInCreationV1 {
    /// Adds usage limits to a V0 key in creation. The signature is kept, but it no longer
    /// covers the key: sign again after calling this.
    pub fn from_v0_with_limits(
        key: IdentityPublicKeyInCreationV0,
        budget: Option<Credits>,
        expires_at: Option<TimestampMillis>,
    ) -> Self {
        let IdentityPublicKeyInCreationV0 {
            id,
            key_type,
            purpose,
            security_level,
            contract_bounds,
            read_only,
            data,
            signature,
        } = key;
        IdentityPublicKeyInCreationV1 {
            id,
            key_type,
            purpose,
            security_level,
            contract_bounds,
            read_only,
            data,
            budget,
            expires_at,
            signature,
        }
    }
}

impl IdentityPublicKeyInCreationV0Getters for IdentityPublicKeyInCreationV1 {
    fn id(&self) -> KeyID {
        self.id
    }

    fn key_type(&self) -> KeyType {
        self.key_type
    }

    fn purpose(&self) -> Purpose {
        self.purpose
    }

    fn security_level(&self) -> SecurityLevel {
        self.security_level
    }

    fn read_only(&self) -> bool {
        self.read_only
    }

    fn data(&self) -> &BinaryData {
        &self.data
    }

    fn signature(&self) -> &BinaryData {
        &self.signature
    }

    fn contract_bounds(&self) -> Option<&ContractBounds> {
        self.contract_bounds.as_ref()
    }
}

impl IdentityPublicKeyInCreationV1Getters for IdentityPublicKeyInCreationV1 {
    fn budget(&self) -> Option<Credits> {
        self.budget
    }

    fn expires_at(&self) -> Option<TimestampMillis> {
        self.expires_at
    }
}

impl IdentityPublicKeyInCreationV0Setters for IdentityPublicKeyInCreationV1 {
    fn set_signature(&mut self, signature: BinaryData) {
        self.signature = signature
    }

    fn set_data(&mut self, data: BinaryData) {
        self.data = data
    }

    fn set_id(&mut self, id: KeyID) {
        self.id = id
    }

    fn set_type(&mut self, key_type: KeyType) {
        self.key_type = key_type;
    }

    fn set_security_level(&mut self, security_level: SecurityLevel) {
        self.security_level = security_level;
    }

    fn set_contract_bounds(&mut self, contract_bounds: Option<ContractBounds>) {
        self.contract_bounds = contract_bounds;
    }

    fn set_purpose(&mut self, purpose: Purpose) {
        self.purpose = purpose;
    }

    fn set_read_only(&mut self, read_only: bool) {
        self.read_only = read_only;
    }
}

impl IdentityPublicKeyInCreationMethodsV0 for IdentityPublicKeyInCreationV1 {
    fn into_identity_public_key(self) -> IdentityPublicKey {
        self.into()
    }
}

impl From<IdentityPublicKeyInCreationV1> for IdentityPublicKey {
    fn from(val: IdentityPublicKeyInCreationV1) -> Self {
        IdentityPublicKeyV1 {
            id: val.id,
            purpose: val.purpose,
            security_level: val.security_level,
            contract_bounds: val.contract_bounds,
            key_type: val.key_type,
            read_only: val.read_only,
            data: val.data,
            disabled_at: None,
            budget: val.budget,
            expires_at: val.expires_at,
        }
        .into()
    }
}

impl From<&IdentityPublicKeyInCreationV1> for IdentityPublicKey {
    fn from(val: &IdentityPublicKeyInCreationV1) -> Self {
        val.clone().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use crate::identity::identity_public_key::accessors::v1::IdentityPublicKeyGettersV1;
    use crate::serialization::Signable;
    use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;

    fn key_in_creation(budget: Option<Credits>) -> IdentityPublicKeyInCreationV1 {
        IdentityPublicKeyInCreationV1 {
            id: 4,
            key_type: KeyType::ECDSA_SECP256K1,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            read_only: false,
            data: BinaryData::new(vec![2; 33]),
            budget,
            expires_at: Some(9_000),
            signature: BinaryData::new(vec![1; 65]),
        }
    }

    #[test]
    fn should_carry_the_limits_into_the_identity_public_key_and_back() {
        let in_creation: IdentityPublicKeyInCreation = key_in_creation(Some(77)).into();
        let key: IdentityPublicKey = (&in_creation).into();
        assert!(matches!(key, IdentityPublicKey::V1(_)));
        assert_eq!(key.id(), 4);
        assert_eq!(key.budget(), Some(77));
        assert_eq!(key.expires_at(), Some(9_000));
        assert_eq!(key.disabled_at(), None);

        let back: IdentityPublicKeyInCreation = (&key).into();
        assert_eq!(back.budget(), Some(77));
        assert_eq!(back.expires_at(), Some(9_000));
        // The signature is not part of the stored key.
        assert!(back.signature().is_empty());
    }

    #[test]
    fn should_sign_over_the_limits() {
        // The identity signs the limits it grants: changing one changes the signable bytes,
        // while the key's own signature never enters them.
        let a: IdentityPublicKeyInCreation = key_in_creation(Some(77)).into();
        let b: IdentityPublicKeyInCreation = key_in_creation(Some(78)).into();
        assert_ne!(
            a.signable_bytes().expect("signable bytes"),
            b.signable_bytes().expect("signable bytes")
        );

        let mut resigned = key_in_creation(Some(77));
        resigned.signature = BinaryData::new(vec![9; 65]);
        let resigned: IdentityPublicKeyInCreation = resigned.into();
        assert_eq!(
            a.signable_bytes().expect("signable bytes"),
            resigned.signable_bytes().expect("signable bytes")
        );
    }
}
