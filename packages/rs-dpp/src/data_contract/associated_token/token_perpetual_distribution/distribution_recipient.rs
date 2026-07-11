use crate::data_contract::associated_token::token_distribution_key::{
    TokenDistributionType, TokenDistributionTypeWithResolvedRecipient,
};
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::PlatformSerialize;
use platform_value::Identifier;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(
    Decode, Encode, PlatformSerialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Default,
)]
#[platform_serialize(unversioned)]
// Custom `Serialize` / `Deserialize` below — `derive(Serialize, Deserialize)`
// can't produce the desired flat wire shape because the `Identity` variant
// wraps `Identifier` (serializes as a base58 string, not a map), so internal
// tagging doesn't apply. The custom impl emits a flat
// `{"$type": ..., "identity": ...}` shape with a synthesized field name (same
// pattern as `ResourceVoteChoice` / `AuthorizedActionTakers`). Bincode
// `Encode` / `Decode` derives are untouched (consensus binary format is
// unaffected).
pub enum TokenDistributionRecipient {
    /// Distribute to the contract Owner
    #[default]
    ContractOwner,
    /// Distribute to a single identity
    Identity(Identifier),
    /// Distribute tokens by participation
    /// This distribution can only happen when choosing epoch based distribution
    EvonodesByParticipation,
}

impl Serialize for TokenDistributionRecipient {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        match self {
            TokenDistributionRecipient::ContractOwner => {
                let mut m = serializer.serialize_map(Some(1))?;
                m.serialize_entry("$type", "contractOwner")?;
                m.end()
            }
            TokenDistributionRecipient::Identity(id) => {
                let mut m = serializer.serialize_map(Some(2))?;
                m.serialize_entry("$type", "identity")?;
                m.serialize_entry("identity", id)?;
                m.end()
            }
            TokenDistributionRecipient::EvonodesByParticipation => {
                let mut m = serializer.serialize_map(Some(1))?;
                m.serialize_entry("$type", "evonodesByParticipation")?;
                m.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for TokenDistributionRecipient {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::{self, MapAccess, Visitor};

        struct V;

        impl<'de> Visitor<'de> for V {
            type Value = TokenDistributionRecipient;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                // Mention the old shape: contract JSON authored before
                // 4.0.0-beta.4 used bare strings / externally-tagged maps, and
                // this message is the only hint users get on ingest failure.
                f.write_str(
                    "TokenDistributionRecipient as a map with a `type` discriminator, \
                     e.g. {\"type\": \"contractOwner\"} or {\"type\": \"identity\", \"identity\": \"<base58>\"} \
                     (the pre-4.0.0-beta.4 shapes \"ContractOwner\" / {\"Identity\": \"<base58>\"} are no longer accepted)",
                )
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut variant: Option<String> = None;
                let mut identity: Option<Identifier> = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "$type" => {
                            if variant.is_some() {
                                return Err(de::Error::duplicate_field("$type"));
                            }
                            variant = Some(map.next_value()?);
                        }
                        "identity" => {
                            if identity.is_some() {
                                return Err(de::Error::duplicate_field("identity"));
                            }
                            identity = Some(map.next_value()?);
                        }
                        _ => {
                            let _: serde::de::IgnoredAny = map.next_value()?;
                        }
                    }
                }

                let variant = variant.ok_or_else(|| de::Error::missing_field("$type"))?;
                match variant.as_str() {
                    "contractOwner" => Ok(TokenDistributionRecipient::ContractOwner),
                    "identity" => {
                        let id = identity.ok_or_else(|| de::Error::missing_field("identity"))?;
                        Ok(TokenDistributionRecipient::Identity(id))
                    }
                    "evonodesByParticipation" => {
                        Ok(TokenDistributionRecipient::EvonodesByParticipation)
                    }
                    other => Err(de::Error::unknown_variant(
                        other,
                        &["contractOwner", "identity", "evonodesByParticipation"],
                    )),
                }
            }
        }

        deserializer.deserialize_map(V)
    }
}

// Manual impls because TokenDistributionRecipient is a flat enum (not versioned V0/V1).
#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for TokenDistributionRecipient {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for TokenDistributionRecipient {}

impl TokenDistributionRecipient {
    /// Simple resolve matches the contract owner but does not try to resolve the evonodes
    pub fn simple_resolve_with_distribution_type(
        &self,
        owner_id: Identifier,
        distribution_type: TokenDistributionType,
    ) -> Result<TokenDistributionTypeWithResolvedRecipient, ProtocolError> {
        match distribution_type {
            TokenDistributionType::PreProgrammed => match self {
                TokenDistributionRecipient::ContractOwner => Ok(
                    TokenDistributionTypeWithResolvedRecipient::PreProgrammed(owner_id),
                ),
                TokenDistributionRecipient::Identity(identity) => Ok(
                    TokenDistributionTypeWithResolvedRecipient::PreProgrammed(*identity),
                ),
                TokenDistributionRecipient::EvonodesByParticipation => {
                    Err(ProtocolError::NotSupported(
                        "trying to simple resolve for pre-programmed evonode distribution"
                            .to_string(),
                    ))
                }
            },
            TokenDistributionType::Perpetual => match self {
                TokenDistributionRecipient::ContractOwner => {
                    Ok(TokenDistributionTypeWithResolvedRecipient::Perpetual(
                        TokenDistributionResolvedRecipient::ContractOwnerIdentity(owner_id),
                    ))
                }
                TokenDistributionRecipient::Identity(identity) => {
                    Ok(TokenDistributionTypeWithResolvedRecipient::Perpetual(
                        TokenDistributionResolvedRecipient::Identity(*identity),
                    ))
                }
                TokenDistributionRecipient::EvonodesByParticipation => {
                    Ok(TokenDistributionTypeWithResolvedRecipient::Perpetual(
                        TokenDistributionResolvedRecipient::Evonode(owner_id),
                    ))
                }
            },
        }
    }
}

pub type TokenDistributionWeight = u64;

// #[derive(
//     Serialize,
//     Deserialize,
//     Decode,
//     Encode,
//     Debug,
//     Clone,
//     PartialEq,
//     Eq,
//     PartialOrd,
// )]
// pub struct EpochProposedBlocks {
//     pub block_count: u64,
//     pub total_blocks: u64,
// }

#[derive(Decode, Encode, PlatformSerialize, Debug, Clone, PartialEq, Eq, PartialOrd)]
#[platform_serialize(unversioned)]
// Custom `Serialize` / `Deserialize` below — every variant wraps `Identifier`
// (a base58 string in JSON, not a map), so serde's internal tagging can't
// auto-derive. The custom impl emits a flat `{"$type": ..., "identity": ...}`
// shape (same pattern as `TokenDistributionRecipient` above). Bincode
// `Encode` / `Decode` derives are untouched.
pub enum TokenDistributionResolvedRecipient {
    /// Distribute to a single identity
    ContractOwnerIdentity(Identifier),
    /// Distribute to a single identity
    Identity(Identifier),
    /// A single Evonode recipient that should share the token reward
    Evonode(Identifier),
}

impl Serialize for TokenDistributionResolvedRecipient {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let (variant, id) = match self {
            TokenDistributionResolvedRecipient::ContractOwnerIdentity(id) => {
                ("contractOwnerIdentity", id)
            }
            TokenDistributionResolvedRecipient::Identity(id) => ("identity", id),
            TokenDistributionResolvedRecipient::Evonode(id) => ("evonode", id),
        };
        let mut m = serializer.serialize_map(Some(2))?;
        m.serialize_entry("$type", variant)?;
        m.serialize_entry("identity", id)?;
        m.end()
    }
}

impl<'de> Deserialize<'de> for TokenDistributionResolvedRecipient {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::{self, MapAccess, Visitor};

        struct V;

        impl<'de> Visitor<'de> for V {
            type Value = TokenDistributionResolvedRecipient;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str(
                    "TokenDistributionResolvedRecipient as a map with a `type` discriminator, \
                     e.g. {\"type\": \"identity\", \"identity\": \"<base58>\"} \
                     (the pre-4.0.0-beta.4 externally-tagged {\"Identity\": \"<base58>\"} shape is no longer accepted)",
                )
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut variant: Option<String> = None;
                let mut identity: Option<Identifier> = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "$type" => {
                            if variant.is_some() {
                                return Err(de::Error::duplicate_field("$type"));
                            }
                            variant = Some(map.next_value()?);
                        }
                        "identity" => {
                            if identity.is_some() {
                                return Err(de::Error::duplicate_field("identity"));
                            }
                            identity = Some(map.next_value()?);
                        }
                        _ => {
                            let _: serde::de::IgnoredAny = map.next_value()?;
                        }
                    }
                }

                let variant = variant.ok_or_else(|| de::Error::missing_field("$type"))?;
                let id = identity.ok_or_else(|| de::Error::missing_field("identity"))?;
                match variant.as_str() {
                    "contractOwnerIdentity" => Ok(
                        TokenDistributionResolvedRecipient::ContractOwnerIdentity(id),
                    ),
                    "identity" => Ok(TokenDistributionResolvedRecipient::Identity(id)),
                    "evonode" => Ok(TokenDistributionResolvedRecipient::Evonode(id)),
                    other => Err(de::Error::unknown_variant(
                        other,
                        &["contractOwnerIdentity", "identity", "evonode"],
                    )),
                }
            }
        }

        deserializer.deserialize_map(V)
    }
}

// Manual impls because TokenDistributionResolvedRecipient is a flat enum (not versioned V0/V1).
#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for TokenDistributionResolvedRecipient {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for TokenDistributionResolvedRecipient {}

impl From<TokenDistributionResolvedRecipient> for TokenDistributionRecipient {
    fn from(value: TokenDistributionResolvedRecipient) -> Self {
        match value {
            TokenDistributionResolvedRecipient::ContractOwnerIdentity(_) => {
                TokenDistributionRecipient::ContractOwner
            }
            TokenDistributionResolvedRecipient::Identity(identifier) => {
                TokenDistributionRecipient::Identity(identifier)
            }
            TokenDistributionResolvedRecipient::Evonode(_) => {
                TokenDistributionRecipient::EvonodesByParticipation
            }
        }
    }
}

impl From<&TokenDistributionResolvedRecipient> for TokenDistributionRecipient {
    fn from(value: &TokenDistributionResolvedRecipient) -> Self {
        match value {
            TokenDistributionResolvedRecipient::ContractOwnerIdentity(_) => {
                TokenDistributionRecipient::ContractOwner
            }
            TokenDistributionResolvedRecipient::Identity(identifier) => {
                TokenDistributionRecipient::Identity(*identifier)
            }
            TokenDistributionResolvedRecipient::Evonode(_) => {
                TokenDistributionRecipient::EvonodesByParticipation
            }
        }
    }
}

impl fmt::Display for TokenDistributionRecipient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenDistributionRecipient::ContractOwner => {
                write!(f, "ContractOwner")
            }
            TokenDistributionRecipient::Identity(identifier) => {
                write!(f, "Identity({})", identifier)
            }
            TokenDistributionRecipient::EvonodesByParticipation => {
                write!(f, "EvonodesByParticipation")
            }
        }
    }
}

/// Implements `Display` for `TokenDistributionResolvedRecipient`
impl fmt::Display for TokenDistributionResolvedRecipient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenDistributionResolvedRecipient::ContractOwnerIdentity(id) => {
                write!(f, "ContractOwnerIdentity({})", id)
            }
            TokenDistributionResolvedRecipient::Identity(id) => {
                write!(f, "Identity({})", id)
            }
            TokenDistributionResolvedRecipient::Evonode(id) => {
                write!(f, "Evonode({})", id)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::associated_token::token_distribution_key::{
        TokenDistributionType, TokenDistributionTypeWithResolvedRecipient,
    };
    use platform_value::Identifier;

    mod construction {
        use super::*;

        #[test]
        fn contract_owner_default() {
            let recipient = TokenDistributionRecipient::default();
            assert!(matches!(
                recipient,
                TokenDistributionRecipient::ContractOwner
            ));
        }

        #[test]
        fn identity_recipient() {
            let id = Identifier::new([1u8; 32]);
            let recipient = TokenDistributionRecipient::Identity(id);
            match recipient {
                TokenDistributionRecipient::Identity(stored_id) => assert_eq!(stored_id, id),
                _ => panic!("Expected Identity variant"),
            }
        }

        #[test]
        fn evonodes_by_participation() {
            let recipient = TokenDistributionRecipient::EvonodesByParticipation;
            assert!(matches!(
                recipient,
                TokenDistributionRecipient::EvonodesByParticipation
            ));
        }
    }

    mod simple_resolve_pre_programmed {
        use super::*;

        #[test]
        fn contract_owner_resolves_to_owner_id() {
            let owner_id = Identifier::new([10u8; 32]);
            let recipient = TokenDistributionRecipient::ContractOwner;
            let result = recipient
                .simple_resolve_with_distribution_type(
                    owner_id,
                    TokenDistributionType::PreProgrammed,
                )
                .expect("should resolve");
            match result {
                TokenDistributionTypeWithResolvedRecipient::PreProgrammed(id) => {
                    assert_eq!(id, owner_id)
                }
                _ => panic!("Expected PreProgrammed variant"),
            }
        }

        #[test]
        fn identity_resolves_to_given_id() {
            let owner_id = Identifier::new([10u8; 32]);
            let identity_id = Identifier::new([20u8; 32]);
            let recipient = TokenDistributionRecipient::Identity(identity_id);
            let result = recipient
                .simple_resolve_with_distribution_type(
                    owner_id,
                    TokenDistributionType::PreProgrammed,
                )
                .expect("should resolve");
            match result {
                TokenDistributionTypeWithResolvedRecipient::PreProgrammed(id) => {
                    assert_eq!(id, identity_id)
                }
                _ => panic!("Expected PreProgrammed variant"),
            }
        }

        #[test]
        fn evonodes_not_supported_for_pre_programmed() {
            let owner_id = Identifier::new([10u8; 32]);
            let recipient = TokenDistributionRecipient::EvonodesByParticipation;
            let result = recipient.simple_resolve_with_distribution_type(
                owner_id,
                TokenDistributionType::PreProgrammed,
            );
            assert!(result.is_err());
            match result.unwrap_err() {
                ProtocolError::NotSupported(_) => {} // expected
                other => panic!("Expected NotSupported error, got: {:?}", other),
            }
        }
    }

    mod simple_resolve_perpetual {
        use super::*;

        #[test]
        fn contract_owner_resolves_to_contract_owner_identity() {
            let owner_id = Identifier::new([30u8; 32]);
            let recipient = TokenDistributionRecipient::ContractOwner;
            let result = recipient
                .simple_resolve_with_distribution_type(owner_id, TokenDistributionType::Perpetual)
                .expect("should resolve");
            match result {
                TokenDistributionTypeWithResolvedRecipient::Perpetual(
                    TokenDistributionResolvedRecipient::ContractOwnerIdentity(id),
                ) => assert_eq!(id, owner_id),
                _ => panic!("Expected Perpetual(ContractOwnerIdentity) variant"),
            }
        }

        #[test]
        fn identity_resolves_to_identity() {
            let owner_id = Identifier::new([30u8; 32]);
            let identity_id = Identifier::new([40u8; 32]);
            let recipient = TokenDistributionRecipient::Identity(identity_id);
            let result = recipient
                .simple_resolve_with_distribution_type(owner_id, TokenDistributionType::Perpetual)
                .expect("should resolve");
            match result {
                TokenDistributionTypeWithResolvedRecipient::Perpetual(
                    TokenDistributionResolvedRecipient::Identity(id),
                ) => assert_eq!(id, identity_id),
                _ => panic!("Expected Perpetual(Identity) variant"),
            }
        }

        #[test]
        fn evonodes_resolves_to_evonode_with_owner_id() {
            let owner_id = Identifier::new([50u8; 32]);
            let recipient = TokenDistributionRecipient::EvonodesByParticipation;
            let result = recipient
                .simple_resolve_with_distribution_type(owner_id, TokenDistributionType::Perpetual)
                .expect("should resolve");
            match result {
                TokenDistributionTypeWithResolvedRecipient::Perpetual(
                    TokenDistributionResolvedRecipient::Evonode(id),
                ) => assert_eq!(id, owner_id),
                _ => panic!("Expected Perpetual(Evonode) variant"),
            }
        }
    }

    mod resolved_to_unresolved_conversion {
        use super::*;

        #[test]
        fn contract_owner_identity_to_contract_owner() {
            let id = Identifier::new([60u8; 32]);
            let resolved = TokenDistributionResolvedRecipient::ContractOwnerIdentity(id);
            let unresolved: TokenDistributionRecipient = resolved.into();
            assert!(matches!(
                unresolved,
                TokenDistributionRecipient::ContractOwner
            ));
        }

        #[test]
        fn identity_to_identity() {
            let id = Identifier::new([70u8; 32]);
            let resolved = TokenDistributionResolvedRecipient::Identity(id);
            let unresolved: TokenDistributionRecipient = resolved.into();
            match unresolved {
                TokenDistributionRecipient::Identity(stored_id) => assert_eq!(stored_id, id),
                _ => panic!("Expected Identity variant"),
            }
        }

        #[test]
        fn evonode_to_evonodes_by_participation() {
            let id = Identifier::new([80u8; 32]);
            let resolved = TokenDistributionResolvedRecipient::Evonode(id);
            let unresolved: TokenDistributionRecipient = resolved.into();
            assert!(matches!(
                unresolved,
                TokenDistributionRecipient::EvonodesByParticipation
            ));
        }

        #[test]
        fn from_ref_contract_owner_identity() {
            let id = Identifier::new([90u8; 32]);
            let resolved = TokenDistributionResolvedRecipient::ContractOwnerIdentity(id);
            let unresolved: TokenDistributionRecipient = (&resolved).into();
            assert!(matches!(
                unresolved,
                TokenDistributionRecipient::ContractOwner
            ));
        }

        #[test]
        fn from_ref_identity_preserves_id() {
            let id = Identifier::new([0xA0; 32]);
            let resolved = TokenDistributionResolvedRecipient::Identity(id);
            let unresolved: TokenDistributionRecipient = (&resolved).into();
            match unresolved {
                TokenDistributionRecipient::Identity(stored_id) => assert_eq!(stored_id, id),
                _ => panic!("Expected Identity variant"),
            }
        }

        #[test]
        fn from_ref_evonode() {
            let id = Identifier::new([0xB0; 32]);
            let resolved = TokenDistributionResolvedRecipient::Evonode(id);
            let unresolved: TokenDistributionRecipient = (&resolved).into();
            assert!(matches!(
                unresolved,
                TokenDistributionRecipient::EvonodesByParticipation
            ));
        }
    }

    mod display {
        use super::*;

        #[test]
        fn contract_owner_display() {
            let recipient = TokenDistributionRecipient::ContractOwner;
            let s = format!("{}", recipient);
            assert_eq!(s, "ContractOwner");
        }

        #[test]
        fn identity_display() {
            let id = Identifier::new([0xCC; 32]);
            let recipient = TokenDistributionRecipient::Identity(id);
            let s = format!("{}", recipient);
            assert!(s.starts_with("Identity("));
        }

        #[test]
        fn evonodes_display() {
            let recipient = TokenDistributionRecipient::EvonodesByParticipation;
            let s = format!("{}", recipient);
            assert_eq!(s, "EvonodesByParticipation");
        }

        #[test]
        fn resolved_contract_owner_display() {
            let id = Identifier::new([0xDD; 32]);
            let resolved = TokenDistributionResolvedRecipient::ContractOwnerIdentity(id);
            let s = format!("{}", resolved);
            assert!(s.starts_with("ContractOwnerIdentity("));
        }

        #[test]
        fn resolved_identity_display() {
            let id = Identifier::new([0xEE; 32]);
            let resolved = TokenDistributionResolvedRecipient::Identity(id);
            let s = format!("{}", resolved);
            assert!(s.starts_with("Identity("));
        }

        #[test]
        fn resolved_evonode_display() {
            let id = Identifier::new([0xFF; 32]);
            let resolved = TokenDistributionResolvedRecipient::Evonode(id);
            let s = format!("{}", resolved);
            assert!(s.starts_with("Evonode("));
        }
    }

    mod equality {
        use super::*;

        #[test]
        fn same_contract_owner_equal() {
            let a = TokenDistributionRecipient::ContractOwner;
            let b = TokenDistributionRecipient::ContractOwner;
            assert_eq!(a, b);
        }

        #[test]
        fn same_identity_equal() {
            let id = Identifier::new([1u8; 32]);
            let a = TokenDistributionRecipient::Identity(id);
            let b = TokenDistributionRecipient::Identity(id);
            assert_eq!(a, b);
        }

        #[test]
        fn different_identity_ids_not_equal() {
            let a = TokenDistributionRecipient::Identity(Identifier::new([1u8; 32]));
            let b = TokenDistributionRecipient::Identity(Identifier::new([2u8; 32]));
            assert_ne!(a, b);
        }

        #[test]
        fn different_variants_not_equal() {
            let a = TokenDistributionRecipient::ContractOwner;
            let b = TokenDistributionRecipient::EvonodesByParticipation;
            assert_ne!(a, b);
        }

        #[test]
        fn clone_preserves_equality() {
            let id = Identifier::new([3u8; 32]);
            let original = TokenDistributionRecipient::Identity(id);
            let cloned = original;
            assert_eq!(original, cloned);
        }
    }
}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests {
    use super::*;
    use crate::serialization::{JsonConvertible, ValueConvertible};
    use platform_value::{platform_value, Value};
    use serde_json::json;

    fn id() -> Identifier {
        Identifier::from([0x42u8; 32])
    }

    const ID_B58: &str = "5TeWSsjg2gbxCyWVniXeCmwM7UtHTCK7svzJr5xYJzHf";

    /// Per-variant wire-shape coverage — the custom Serialize/Deserialize pair
    /// must stay in sync when variants are added.
    #[test]
    fn recipient_json_round_trip_with_full_wire_shape_all_variants() {
        let cases = vec![
            (
                TokenDistributionRecipient::ContractOwner,
                json!({"$type": "contractOwner"}),
            ),
            (
                TokenDistributionRecipient::Identity(id()),
                json!({"$type": "identity", "identity": ID_B58}),
            ),
            (
                TokenDistributionRecipient::EvonodesByParticipation,
                json!({"$type": "evonodesByParticipation"}),
            ),
        ];
        for (original, expected) in cases {
            let json_v = original.to_json().expect("to_json");
            assert_eq!(json_v, expected, "json wire shape for {original}");
            let recovered = TokenDistributionRecipient::from_json(json_v).expect("from_json");
            assert_eq!(original, recovered);
        }
    }

    #[test]
    fn recipient_value_round_trip_with_full_wire_shape_all_variants() {
        // `identity` round-trips as the typed `Value::Identifier` variant.
        let cases = vec![
            (
                TokenDistributionRecipient::ContractOwner,
                platform_value!({"$type": "contractOwner"}),
            ),
            (
                TokenDistributionRecipient::Identity(id()),
                Value::Map(vec![
                    (
                        Value::Text("$type".to_string()),
                        Value::Text("identity".to_string()),
                    ),
                    (
                        Value::Text("identity".to_string()),
                        Value::Identifier([0x42; 32]),
                    ),
                ]),
            ),
            (
                TokenDistributionRecipient::EvonodesByParticipation,
                platform_value!({"$type": "evonodesByParticipation"}),
            ),
        ];
        for (original, expected) in cases {
            let value = original.to_object().expect("to_object");
            assert_eq!(value, expected, "value wire shape for {original}");
            let recovered = TokenDistributionRecipient::from_object(value).expect("from_object");
            assert_eq!(original, recovered);
        }
    }

    #[test]
    fn resolved_recipient_json_round_trip_with_full_wire_shape_all_variants() {
        let cases = vec![
            (
                TokenDistributionResolvedRecipient::ContractOwnerIdentity(id()),
                json!({"$type": "contractOwnerIdentity", "identity": ID_B58}),
            ),
            (
                TokenDistributionResolvedRecipient::Identity(id()),
                json!({"$type": "identity", "identity": ID_B58}),
            ),
            (
                TokenDistributionResolvedRecipient::Evonode(id()),
                json!({"$type": "evonode", "identity": ID_B58}),
            ),
        ];
        for (original, expected) in cases {
            let json_v = original.to_json().expect("to_json");
            assert_eq!(json_v, expected, "json wire shape for {original}");
            let recovered =
                TokenDistributionResolvedRecipient::from_json(json_v).expect("from_json");
            assert_eq!(original, recovered);
        }
    }

    #[test]
    fn resolved_recipient_value_round_trip_all_variants() {
        let cases = vec![
            (
                TokenDistributionResolvedRecipient::ContractOwnerIdentity(id()),
                "contractOwnerIdentity",
            ),
            (
                TokenDistributionResolvedRecipient::Identity(id()),
                "identity",
            ),
            (TokenDistributionResolvedRecipient::Evonode(id()), "evonode"),
        ];
        for (original, expected_tag) in cases {
            let value = original.to_object().expect("to_object");
            let expected = Value::Map(vec![
                (
                    Value::Text("$type".to_string()),
                    Value::Text(expected_tag.to_string()),
                ),
                (
                    Value::Text("identity".to_string()),
                    Value::Identifier([0x42; 32]),
                ),
            ]);
            assert_eq!(value, expected, "value wire shape for {original}");
            let recovered =
                TokenDistributionResolvedRecipient::from_object(value).expect("from_object");
            assert_eq!(original, recovered);
        }
    }
}
