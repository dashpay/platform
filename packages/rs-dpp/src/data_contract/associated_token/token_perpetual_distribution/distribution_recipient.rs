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
    Serialize,
    Deserialize,
    Decode,
    Encode,
    PlatformSerialize,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Default,
)]
#[platform_serialize(unversioned)]
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

#[derive(
    Serialize,
    Deserialize,
    Decode,
    Encode,
    PlatformSerialize,
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
)]
#[platform_serialize(unversioned)]
pub enum TokenDistributionResolvedRecipient {
    /// Distribute to a single identity
    ContractOwnerIdentity(Identifier),
    /// Distribute to a single identity
    Identity(Identifier),
    /// A single Evonode recipient that should share the token reward
    Evonode(Identifier),
}

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
