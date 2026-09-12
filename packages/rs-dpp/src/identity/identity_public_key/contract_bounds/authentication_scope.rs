//! Immutable application delegation. Budgets are deliberately not part of V0.
use crate::identifier::Identifier;
use crate::identity::TimestampMillis;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
#[cfg(feature = "json-conversion")]
use crate::serialization::{json_safe_fields, JsonConvertible};
use crate::ProtocolError;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

pub const MAX_SCOPE_BYTES: usize = 2048;
pub const MAX_SCOPE_CONTRACTS: usize = 16;
pub const MAX_SCOPE_DOCUMENT_TYPES: usize = 16;

/// Stable wire bits. Unknown bits are rejected, never ignored.
pub mod permissions {
    pub const DOCUMENT_CREATE: u32 = 1 << 0;
    pub const DOCUMENT_REPLACE: u32 = 1 << 1;
    pub const DOCUMENT_DELETE: u32 = 1 << 2;
    pub const DOCUMENT_TRANSFER: u32 = 1 << 3;
    pub const DOCUMENT_UPDATE_PRICE: u32 = 1 << 4;
    pub const DOCUMENT_PURCHASE: u32 = 1 << 5;
    pub const DOCUMENT_TOKEN_PAYMENT: u32 = 1 << 6;
    pub const TOKEN_BURN: u32 = 1 << 7;
    pub const TOKEN_MINT: u32 = 1 << 8;
    pub const TOKEN_TRANSFER: u32 = 1 << 9;
    pub const TOKEN_FREEZE: u32 = 1 << 10;
    pub const TOKEN_UNFREEZE: u32 = 1 << 11;
    pub const TOKEN_DESTROY_FROZEN_FUNDS: u32 = 1 << 12;
    pub const TOKEN_CLAIM: u32 = 1 << 13;
    pub const TOKEN_EMERGENCY_ACTION: u32 = 1 << 14;
    pub const TOKEN_CONFIG_UPDATE: u32 = 1 << 15;
    pub const TOKEN_DIRECT_PURCHASE: u32 = 1 << 16;
    pub const TOKEN_SET_PRICE: u32 = 1 << 17;
    pub const ALL: u32 = (1 << 18) - 1;
}

#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Encode, Decode, Serialize, Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub struct ContractScope {
    pub id: Identifier,
    /// None grants all document types; Some(empty) is invalid.
    pub document_types: Option<Vec<String>>,
}

#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Encode, Decode, Serialize, Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticationScopeV0 {
    pub contracts: Vec<ContractScope>,
    pub permissions: u32,
    pub expires_at: Option<TimestampMillis>,
}

#[cfg_attr(feature = "json-conversion", derive(JsonConvertible))]
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Encode, Decode, Serialize, Deserialize,
)]
#[serde(tag = "$formatVersion")]
pub enum AuthenticationScope {
    #[serde(rename = "0")]
    V0(AuthenticationScopeV0),
}

impl AuthenticationScope {
    pub fn v0(&self) -> &AuthenticationScopeV0 {
        match self {
            Self::V0(scope) => scope,
        }
    }

    pub fn contracts(&self) -> &[ContractScope] {
        &self.v0().contracts
    }
    pub fn expires_at(&self) -> Option<TimestampMillis> {
        self.v0().expires_at
    }
    pub fn allows(&self, permission: u32) -> bool {
        self.v0().permissions & permission == permission
    }
    pub fn is_expired(&self, time_ms: TimestampMillis) -> bool {
        self.expires_at().is_some_and(|expiry| time_ms >= expiry)
    }
    pub fn allows_contract(&self, id: &Identifier) -> bool {
        self.contracts().iter().any(|scope| scope.id == id)
    }
    pub fn allows_document(&self, id: &Identifier, name: &str) -> bool {
        self.contracts().iter().any(|scope| {
            scope.id == id
                && scope
                    .document_types
                    .as_ref()
                    .is_none_or(|names| names.iter().any(|n| n == name))
        })
    }

    /// Validate before fetching any referenced contracts.
    pub fn validate(&self) -> Result<(), ProtocolError> {
        let invalid =
            |reason: &str| ProtocolError::InvalidKeyContractBoundsError(reason.to_owned());
        let scope = self.v0();
        if scope.contracts.is_empty() || scope.contracts.len() > MAX_SCOPE_CONTRACTS {
            return Err(invalid("scope must contain between 1 and 16 contracts"));
        }
        if scope.permissions == 0 || scope.permissions & !permissions::ALL != 0 {
            return Err(invalid("scope must have a nonempty, known permission mask"));
        }
        if !scope
            .contracts
            .windows(2)
            .all(|pair| pair[0].id < pair[1].id)
        {
            return Err(invalid("scope contract IDs must be sorted and unique"));
        }
        for contract in &scope.contracts {
            if let Some(names) = &contract.document_types {
                if names.is_empty()
                    || names.len() > MAX_SCOPE_DOCUMENT_TYPES
                    || names
                        .iter()
                        .any(|name| name.is_empty() || name.len() > MAX_SCOPE_BYTES)
                    || !names.windows(2).all(|pair| pair[0] < pair[1])
                {
                    return Err(invalid("scope document types must be nonempty, sorted, unique and at most 16 per contract"));
                }
            }
        }
        let bytes = bincode::encode_to_vec(self, bincode::config::standard())
            .map_err(|e| ProtocolError::EncodingError(e.to_string()))?;
        if bytes.len() > MAX_SCOPE_BYTES {
            return Err(invalid("encoded authentication scope exceeds 2048 bytes"));
        }
        Ok(())
    }

    /// Canonical native persistence / shielded preimage representation.
    pub fn to_bytes(&self) -> Result<Vec<u8>, ProtocolError> {
        self.validate()?;
        bincode::encode_to_vec(self, bincode::config::standard())
            .map_err(|e| ProtocolError::EncodingError(e.to_string()))
    }
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ProtocolError> {
        if bytes.len() > MAX_SCOPE_BYTES {
            return Err(ProtocolError::DecodingError(
                "scope exceeds 2048 bytes".into(),
            ));
        }
        let (scope, consumed): (Self, usize) = bincode::decode_from_slice(
            bytes,
            bincode::config::standard().with_limit::<{ MAX_SCOPE_BYTES * 8 }>(),
        )
        .map_err(|e| ProtocolError::DecodingError(e.to_string()))?;
        if consumed != bytes.len() {
            return Err(ProtocolError::DecodingError("trailing scope bytes".into()));
        }
        scope.validate()?;
        Ok(scope)
    }

    #[cfg(feature = "state-transitions")]
    pub fn allows_transition(
        &self,
        transition: crate::state_transition::batch_transition::batched_transition::BatchedTransitionRef<'_>,
    ) -> bool {
        use crate::state_transition::batch_transition::batched_transition::document_transition::DocumentTransitionV0Methods;
        use crate::state_transition::batch_transition::batched_transition::token_transition::TokenTransitionV0Methods;
        use crate::state_transition::batch_transition::batched_transition::{
            BatchedTransitionRef, DocumentTransition, TokenTransition,
        };
        use permissions::*;
        let permission = match transition {
            BatchedTransitionRef::Document(doc) => {
                if !self.allows_document(&doc.data_contract_id(), doc.document_type_name()) {
                    return false;
                }
                match doc {
                    DocumentTransition::Create(_) => DOCUMENT_CREATE,
                    DocumentTransition::Replace(_) => DOCUMENT_REPLACE,
                    DocumentTransition::Delete(_) | DocumentTransition::IndexOnlyDelete(_) => {
                        DOCUMENT_DELETE
                    }
                    DocumentTransition::Transfer(_) => DOCUMENT_TRANSFER,
                    DocumentTransition::UpdatePrice(_) => DOCUMENT_UPDATE_PRICE,
                    DocumentTransition::Purchase(_) => DOCUMENT_PURCHASE,
                }
            }
            BatchedTransitionRef::Token(token) => {
                if !self.allows_contract(&token.data_contract_id()) {
                    return false;
                }
                match token {
                    TokenTransition::Burn(_) => TOKEN_BURN,
                    TokenTransition::Mint(_) => TOKEN_MINT,
                    TokenTransition::Transfer(_) => TOKEN_TRANSFER,
                    TokenTransition::Freeze(_) => TOKEN_FREEZE,
                    TokenTransition::Unfreeze(_) => TOKEN_UNFREEZE,
                    TokenTransition::DestroyFrozenFunds(_) => TOKEN_DESTROY_FROZEN_FUNDS,
                    TokenTransition::Claim(_) => TOKEN_CLAIM,
                    TokenTransition::EmergencyAction(_) => TOKEN_EMERGENCY_ACTION,
                    TokenTransition::ConfigUpdate(_) => TOKEN_CONFIG_UPDATE,
                    TokenTransition::DirectPurchase(_) => TOKEN_DIRECT_PURCHASE,
                    TokenTransition::SetPriceForDirectPurchase(_) => TOKEN_SET_PRICE,
                }
            }
        };
        self.allows(permission)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> AuthenticationScope {
        AuthenticationScope::V0(AuthenticationScopeV0 {
            contracts: vec![ContractScope {
                id: Identifier::from([1; 32]),
                document_types: Some(vec!["post".into()]),
            }],
            permissions: permissions::DOCUMENT_CREATE | permissions::DOCUMENT_TOKEN_PAYMENT,
            expires_at: Some(100),
        })
    }
    #[test]
    fn should_decode_stored_keys_with_large_scopes() {
        use crate::identity::contract_bounds::ContractBounds;
        use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
        use crate::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
        use crate::serialization::{PlatformDeserializable, PlatformSerializable};

        for contract_count in [8, 16] {
            let scope = AuthenticationScope::V0(AuthenticationScopeV0 {
                contracts: (0..contract_count)
                    .map(|id| ContractScope {
                        id: Identifier::from([id; 32]),
                        document_types: Some((0..16).map(|n| format!("t{n:02}")).collect()),
                    })
                    .collect(),
                permissions: permissions::ALL,
                expires_at: Some(u64::MAX),
            });
            scope.validate().unwrap();
            let key: IdentityPublicKey = IdentityPublicKeyV0 {
                id: u32::MAX,
                key_type: KeyType::ECDSA_SECP256K1,
                purpose: Purpose::AUTHENTICATION,
                security_level: SecurityLevel::HIGH,
                data: vec![2; 33].into(),
                read_only: false,
                disabled_at: Some(u64::MAX),
                contract_bounds: Some(ContractBounds::Scoped(scope)),
            }
            .into();
            let bytes = key.serialize_to_bytes().unwrap();
            assert_eq!(
                IdentityPublicKey::deserialize_from_bytes(&bytes).unwrap(),
                key
            );
        }
    }

    #[test]
    fn should_preserve_scope_and_reject_trailing_bytes() {
        let scope = fixture();
        let mut bytes = scope.to_bytes().unwrap();
        assert_eq!(AuthenticationScope::from_bytes(&bytes).unwrap(), scope);
        bytes.push(0);
        assert!(AuthenticationScope::from_bytes(&bytes).is_err());
    }
    #[test]
    fn should_restrict_contracts_types_actions_and_expiry() {
        let scope = fixture();
        assert!(scope.allows_document(&Identifier::from([1; 32]), "post"));
        assert!(!scope.allows_document(&Identifier::from([2; 32]), "post"));
        assert!(!scope.allows_document(&Identifier::from([1; 32]), "profile"));
        assert!(!scope.allows(permissions::TOKEN_TRANSFER));
        assert!(!scope.is_expired(99));
        assert!(scope.is_expired(100));
    }
    #[test]
    fn should_reject_empty_types_unknown_bits_and_duplicate_contracts() {
        let AuthenticationScope::V0(original) = fixture();
        let mut scope = original.clone();
        scope.contracts[0].document_types = Some(vec![]);
        assert!(AuthenticationScope::V0(scope).validate().is_err());
        let mut scope = original.clone();
        scope.permissions |= 1 << 31;
        assert!(AuthenticationScope::V0(scope).validate().is_err());
        let mut scope = original;
        scope.contracts.push(scope.contracts[0].clone());
        assert!(AuthenticationScope::V0(scope).validate().is_err());
    }
    #[test]
    fn should_bound_scope_size_and_distinguish_unrestricted_types() {
        let AuthenticationScope::V0(mut scope) = fixture();
        scope.contracts[0].document_types = None;
        let unrestricted = AuthenticationScope::V0(scope.clone());
        assert!(unrestricted.validate().is_ok());
        assert!(unrestricted.allows_document(&scope.contracts[0].id, "anything"));
        scope.contracts[0].document_types = Some(vec!["a".repeat(MAX_SCOPE_BYTES)]);
        assert!(AuthenticationScope::V0(scope).validate().is_err());
        assert!(AuthenticationScope::from_bytes(&vec![0; MAX_SCOPE_BYTES + 1]).is_err());

        let AuthenticationScope::V0(mut scope) = fixture();
        scope.contracts = (0..16)
            .map(|id| ContractScope {
                id: Identifier::from([id; 32]),
                document_types: None,
            })
            .collect();
        assert!(AuthenticationScope::V0(scope.clone()).validate().is_ok());
        scope.contracts.push(ContractScope {
            id: Identifier::from([16; 32]),
            document_types: None,
        });
        assert!(AuthenticationScope::V0(scope.clone()).validate().is_err());
        scope.contracts.clear();
        assert!(AuthenticationScope::V0(scope).validate().is_err());

        let AuthenticationScope::V0(mut scope) = fixture();
        let names = (0..16).map(|n| format!("type{n:02}")).collect::<Vec<_>>();
        scope.contracts[0].document_types = Some(names.clone());
        assert!(AuthenticationScope::V0(scope.clone()).validate().is_ok());
        scope.contracts[0]
            .document_types
            .as_mut()
            .unwrap()
            .push("type16".into());
        assert!(AuthenticationScope::V0(scope.clone()).validate().is_err());
        scope.contracts[0].document_types = Some(names);
        scope.permissions = 0;
        assert!(AuthenticationScope::V0(scope).validate().is_err());
    }

    #[cfg(feature = "state-transitions")]
    #[test]
    fn should_require_each_token_permission_independently_and_reject_foreign_contracts() {
        use crate::state_transition::batch_transition::batched_transition::{
            token_transfer_transition::TokenTransferTransitionV0, BatchedTransitionRef,
            TokenTransition,
        };
        use permissions::*;

        let cases = [
            (TokenTransition::Burn(Default::default()), TOKEN_BURN),
            (TokenTransition::Mint(Default::default()), TOKEN_MINT),
            (
                TokenTransition::Transfer(TokenTransferTransitionV0::default().into()),
                TOKEN_TRANSFER,
            ),
            (TokenTransition::Freeze(Default::default()), TOKEN_FREEZE),
            (
                TokenTransition::Unfreeze(Default::default()),
                TOKEN_UNFREEZE,
            ),
            (
                TokenTransition::DestroyFrozenFunds(Default::default()),
                TOKEN_DESTROY_FROZEN_FUNDS,
            ),
            (TokenTransition::Claim(Default::default()), TOKEN_CLAIM),
            (
                TokenTransition::EmergencyAction(Default::default()),
                TOKEN_EMERGENCY_ACTION,
            ),
            (
                TokenTransition::ConfigUpdate(Default::default()),
                TOKEN_CONFIG_UPDATE,
            ),
            (
                TokenTransition::DirectPurchase(Default::default()),
                TOKEN_DIRECT_PURCHASE,
            ),
            (
                TokenTransition::SetPriceForDirectPurchase(Default::default()),
                TOKEN_SET_PRICE,
            ),
        ];
        // Only the operation and contract are relevant to the authorization policy;
        // amounts, recipients and other action fields are validated separately.
        for (transition, required) in cases {
            let member = BatchedTransitionRef::Token(&transition);
            let mut scope = AuthenticationScopeV0 {
                contracts: vec![ContractScope {
                    id: Identifier::from([0; 32]),
                    document_types: None,
                }],
                permissions: required,
                expires_at: None,
            };
            assert!(AuthenticationScope::V0(scope.clone()).allows_transition(member));
            scope.permissions = ALL & !required;
            assert!(!AuthenticationScope::V0(scope.clone()).allows_transition(member),
                "all other permissions, including document token fees, must not authorize {transition:?}");
            scope.permissions = ALL;
            scope.contracts[0].id = Identifier::from([1; 32]);
            assert!(
                !AuthenticationScope::V0(scope).allows_transition(member),
                "no permission may escape its contract"
            );
        }
    }

    #[cfg(all(feature = "json-conversion", feature = "value-conversion"))]
    #[test]
    fn should_round_trip_scoped_bounds_in_json_and_platform_value() {
        use super::super::ContractBounds;
        let bounds = ContractBounds::Scoped(fixture());
        let json = bounds.to_json().unwrap();
        assert_eq!(ContractBounds::from_json(json).unwrap(), bounds);
        let value = bounds.to_object().unwrap();
        assert_eq!(ContractBounds::from_object(value).unwrap(), bounds);
    }

    #[cfg(feature = "state-transitions")]
    #[test]
    fn should_reject_scoped_keys_before_activation_and_scoped_master_keys() {
        use super::super::ContractBounds;
        use crate::identity::{KeyType, Purpose, SecurityLevel};
        use crate::state_transition::public_key_in_creation::{
            v0::IdentityPublicKeyInCreationV0, IdentityPublicKeyInCreation,
        };
        let mut key = IdentityPublicKeyInCreationV0 {
            id: 2,
            key_type: KeyType::ECDSA_HASH160,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            data: vec![1; 20].into(),
            read_only: false,
            signature: Default::default(),
            contract_bounds: Some(ContractBounds::Scoped(fixture())),
        };
        let old = crate::version::PlatformVersion::get(13).unwrap();
        let new = crate::version::PlatformVersion::latest();
        assert!(
            !IdentityPublicKeyInCreation::validate_identity_public_keys_structure(
                &[key.clone().into()],
                false,
                old
            )
            .unwrap()
            .is_valid()
        );
        assert!(
            IdentityPublicKeyInCreation::validate_identity_public_keys_structure(
                &[key.clone().into()],
                false,
                new
            )
            .unwrap()
            .is_valid()
        );
        key.security_level = SecurityLevel::MASTER;
        assert!(
            !IdentityPublicKeyInCreation::validate_identity_public_keys_structure(
                &[key.into()],
                false,
                new
            )
            .unwrap()
            .is_valid()
        );
    }

    #[cfg(feature = "state-transitions")]
    #[test]
    fn should_bind_every_scope_field_in_versioned_shielded_preimages() {
        use super::super::ContractBounds;
        use crate::address_funds::PlatformAddress;
        use crate::identity::{KeyType, Purpose, SecurityLevel};
        use crate::shielded::{
            identity_create_from_shielded_extra_sighash_data as versioned,
            identity_create_from_shielded_extra_sighash_data_v0 as old,
            identity_create_from_shielded_extra_sighash_data_v1 as new,
        };
        use crate::state_transition::public_key_in_creation::{
            v0::IdentityPublicKeyInCreationV0, IdentityPublicKeyInCreation,
        };
        let mut key = IdentityPublicKeyInCreationV0 {
            id: 2,
            key_type: KeyType::ECDSA_HASH160,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            data: vec![1; 20].into(),
            read_only: false,
            signature: Default::default(),
            contract_bounds: None,
        };
        let fallback = PlatformAddress::P2pkh([2; 20]);
        for address in [fallback, PlatformAddress::P2sh([3; 20])] {
            for bounds in [
                None,
                Some(ContractBounds::SingleContract {
                    id: Identifier::from([4; 32]),
                }),
                Some(ContractBounds::SingleContractDocumentType {
                    id: Identifier::from([4; 32]),
                    document_type_name: "legacy".into(),
                }),
            ] {
                key.contract_bounds = bounds;
                let legacy: IdentityPublicKeyInCreation = key.clone().into();
                let keys = std::slice::from_ref(&legacy);
                let frozen = old(&[1; 32], 1, &address, keys).unwrap();
                assert_eq!(frozen, new(&[1; 32], 1, &address, keys).unwrap());
                for protocol in [13, 14] {
                    assert_eq!(
                        frozen,
                        versioned(
                            &[1; 32],
                            1,
                            &address,
                            keys,
                            crate::version::PlatformVersion::get(protocol).unwrap()
                        )
                        .unwrap(),
                        "legacy key preimage must remain stable under protocol {protocol}"
                    );
                }
            }
        }
        key.contract_bounds = Some(ContractBounds::Scoped(fixture()));
        assert!(old(&[1; 32], 1, &fallback, &[key.clone().into()]).is_err());
        let original = new(&[1; 32], 1, &fallback, &[key.clone().into()]).unwrap();
        for field in ["contract", "types", "permissions", "expiry"] {
            let mut changed = key.clone();
            let Some(ContractBounds::Scoped(AuthenticationScope::V0(ref mut scope))) =
                changed.contract_bounds
            else {
                unreachable!()
            };
            match field {
                "contract" => scope.contracts[0].id = Identifier::from([2; 32]),
                "types" => scope.contracts[0].document_types = None,
                "permissions" => scope.permissions |= permissions::DOCUMENT_DELETE,
                "expiry" => scope.expires_at = Some(101),
                _ => unreachable!(),
            }
            assert_ne!(
                original,
                new(&[1; 32], 1, &fallback, &[changed.into()]).unwrap(),
                "must bind {field}"
            );
        }
    }
}
