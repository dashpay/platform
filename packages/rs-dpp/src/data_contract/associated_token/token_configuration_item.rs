use crate::balances::credits::TokenAmount;
use crate::data_contract::associated_token::token_configuration_convention::TokenConfigurationConvention;
use crate::data_contract::associated_token::token_marketplace_rules::v0::TokenTradeMode;
use crate::data_contract::associated_token::token_perpetual_distribution::TokenPerpetualDistribution;
use crate::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
use crate::data_contract::GroupContractPosition;
use crate::ProtocolError;
use bincode::Encode;
use platform_serialization::de::Decode;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(
    Debug,
    Clone,
    Default,
    PartialOrd,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserialize,
    PartialEq,
    Eq,
)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(
        into = "TokenConfigurationChangeItemRepr",
        from = "TokenConfigurationChangeItemRepr"
    )
)]
pub enum TokenConfigurationChangeItem {
    #[default]
    TokenConfigurationNoChange,
    Conventions(TokenConfigurationConvention),
    ConventionsControlGroup(AuthorizedActionTakers),
    ConventionsAdminGroup(AuthorizedActionTakers),
    MaxSupply(Option<TokenAmount>),
    MaxSupplyControlGroup(AuthorizedActionTakers),
    MaxSupplyAdminGroup(AuthorizedActionTakers),
    PerpetualDistribution(Option<TokenPerpetualDistribution>),
    PerpetualDistributionControlGroup(AuthorizedActionTakers),
    PerpetualDistributionAdminGroup(AuthorizedActionTakers),
    NewTokensDestinationIdentity(Option<Identifier>),
    NewTokensDestinationIdentityControlGroup(AuthorizedActionTakers),
    NewTokensDestinationIdentityAdminGroup(AuthorizedActionTakers),
    MintingAllowChoosingDestination(bool),
    MintingAllowChoosingDestinationControlGroup(AuthorizedActionTakers),
    MintingAllowChoosingDestinationAdminGroup(AuthorizedActionTakers),
    ManualMinting(AuthorizedActionTakers),
    ManualMintingAdminGroup(AuthorizedActionTakers),
    ManualBurning(AuthorizedActionTakers),
    ManualBurningAdminGroup(AuthorizedActionTakers),
    Freeze(AuthorizedActionTakers),
    FreezeAdminGroup(AuthorizedActionTakers),
    Unfreeze(AuthorizedActionTakers),
    UnfreezeAdminGroup(AuthorizedActionTakers),
    DestroyFrozenFunds(AuthorizedActionTakers),
    DestroyFrozenFundsAdminGroup(AuthorizedActionTakers),
    EmergencyAction(AuthorizedActionTakers),
    EmergencyActionAdminGroup(AuthorizedActionTakers),
    MarketplaceTradeMode(TokenTradeMode),
    MarketplaceTradeModeControlGroup(AuthorizedActionTakers),
    MarketplaceTradeModeAdminGroup(AuthorizedActionTakers),
    MainControlGroup(Option<GroupContractPosition>),
}

// Internal-`$type` serde shape via a struct-variant Repr (the outer enum mixes
// AuthorizedActionTakers/struct variants with primitive/Option/unit variants
// that serde cannot auto-internal-tag). Unit -> `{"$type":"..."}`; data ->
// `{"$type":"...","value":...}`. `MaxSupply` gains the json_safe protection it
// previously lacked (Option<u64> above MAX_SAFE_INTEGER -> string in HR JSON;
// Content-safe — never emits u128). `MainControlGroup` is u16 (always JS-safe).
// The macro keeps the Repr and both `From` impls in lockstep with one variant
// list — avoiding the "update Serialize AND Deserialize" maintenance trap.
#[cfg(feature = "serde-conversion")]
macro_rules! token_configuration_change_item_repr {
    (
        unit: $unit:ident,
        $( $variant:ident : $ty:ty $(, with = $with:literal)? );* $(;)?
    ) => {
        #[derive(Serialize, Deserialize)]
        #[serde(tag = "$type", rename_all = "camelCase")]
        enum TokenConfigurationChangeItemRepr {
            $unit,
            $( $variant {
                $(#[serde(with = $with)])?
                value: $ty,
            } ),*
        }

        impl From<TokenConfigurationChangeItem> for TokenConfigurationChangeItemRepr {
            fn from(item: TokenConfigurationChangeItem) -> Self {
                match item {
                    TokenConfigurationChangeItem::$unit => Self::$unit,
                    $( TokenConfigurationChangeItem::$variant(value) => Self::$variant { value }, )*
                }
            }
        }

        impl From<TokenConfigurationChangeItemRepr> for TokenConfigurationChangeItem {
            fn from(repr: TokenConfigurationChangeItemRepr) -> Self {
                match repr {
                    TokenConfigurationChangeItemRepr::$unit => Self::$unit,
                    $( TokenConfigurationChangeItemRepr::$variant { value } => Self::$variant(value), )*
                }
            }
        }
    };
}

#[cfg(feature = "serde-conversion")]
token_configuration_change_item_repr! {
    unit: TokenConfigurationNoChange,
    Conventions: TokenConfigurationConvention;
    ConventionsControlGroup: AuthorizedActionTakers;
    ConventionsAdminGroup: AuthorizedActionTakers;
    MaxSupply: Option<TokenAmount>, with = "crate::serialization::json_safe_option_u64";
    MaxSupplyControlGroup: AuthorizedActionTakers;
    MaxSupplyAdminGroup: AuthorizedActionTakers;
    PerpetualDistribution: Option<TokenPerpetualDistribution>;
    PerpetualDistributionControlGroup: AuthorizedActionTakers;
    PerpetualDistributionAdminGroup: AuthorizedActionTakers;
    NewTokensDestinationIdentity: Option<Identifier>;
    NewTokensDestinationIdentityControlGroup: AuthorizedActionTakers;
    NewTokensDestinationIdentityAdminGroup: AuthorizedActionTakers;
    MintingAllowChoosingDestination: bool;
    MintingAllowChoosingDestinationControlGroup: AuthorizedActionTakers;
    MintingAllowChoosingDestinationAdminGroup: AuthorizedActionTakers;
    ManualMinting: AuthorizedActionTakers;
    ManualMintingAdminGroup: AuthorizedActionTakers;
    ManualBurning: AuthorizedActionTakers;
    ManualBurningAdminGroup: AuthorizedActionTakers;
    Freeze: AuthorizedActionTakers;
    FreezeAdminGroup: AuthorizedActionTakers;
    Unfreeze: AuthorizedActionTakers;
    UnfreezeAdminGroup: AuthorizedActionTakers;
    DestroyFrozenFunds: AuthorizedActionTakers;
    DestroyFrozenFundsAdminGroup: AuthorizedActionTakers;
    EmergencyAction: AuthorizedActionTakers;
    EmergencyActionAdminGroup: AuthorizedActionTakers;
    MarketplaceTradeMode: TokenTradeMode;
    MarketplaceTradeModeControlGroup: AuthorizedActionTakers;
    MarketplaceTradeModeAdminGroup: AuthorizedActionTakers;
    MainControlGroup: Option<GroupContractPosition>;
}

impl TokenConfigurationChangeItem {
    pub fn payload_serialization(&self) -> Result<Option<Vec<u8>>, ProtocolError> {
        Ok(match self {
            TokenConfigurationChangeItem::TokenConfigurationNoChange => None,
            TokenConfigurationChangeItem::Conventions(convention) => Some(
                bincode::encode_to_vec(convention, bincode::config::standard())
                    .map_err(|e| ProtocolError::EncodingError(e.to_string()))?,
            ),
            TokenConfigurationChangeItem::ConventionsControlGroup(a)
            | TokenConfigurationChangeItem::ConventionsAdminGroup(a)
            | TokenConfigurationChangeItem::MaxSupplyControlGroup(a)
            | TokenConfigurationChangeItem::MaxSupplyAdminGroup(a)
            | TokenConfigurationChangeItem::PerpetualDistributionControlGroup(a)
            | TokenConfigurationChangeItem::PerpetualDistributionAdminGroup(a)
            | TokenConfigurationChangeItem::NewTokensDestinationIdentityControlGroup(a)
            | TokenConfigurationChangeItem::NewTokensDestinationIdentityAdminGroup(a)
            | TokenConfigurationChangeItem::MintingAllowChoosingDestinationControlGroup(a)
            | TokenConfigurationChangeItem::MintingAllowChoosingDestinationAdminGroup(a)
            | TokenConfigurationChangeItem::ManualMinting(a)
            | TokenConfigurationChangeItem::ManualMintingAdminGroup(a)
            | TokenConfigurationChangeItem::ManualBurning(a)
            | TokenConfigurationChangeItem::ManualBurningAdminGroup(a)
            | TokenConfigurationChangeItem::Freeze(a)
            | TokenConfigurationChangeItem::FreezeAdminGroup(a)
            | TokenConfigurationChangeItem::Unfreeze(a)
            | TokenConfigurationChangeItem::UnfreezeAdminGroup(a)
            | TokenConfigurationChangeItem::DestroyFrozenFunds(a)
            | TokenConfigurationChangeItem::DestroyFrozenFundsAdminGroup(a)
            | TokenConfigurationChangeItem::EmergencyAction(a)
            | TokenConfigurationChangeItem::EmergencyActionAdminGroup(a)
            | TokenConfigurationChangeItem::MarketplaceTradeModeControlGroup(a)
            | TokenConfigurationChangeItem::MarketplaceTradeModeAdminGroup(a) => Some(a.to_bytes()),
            TokenConfigurationChangeItem::MaxSupply(max_supply) => {
                max_supply.map(|amount| amount.to_be_bytes().to_vec())
            }
            TokenConfigurationChangeItem::PerpetualDistribution(distribution) => distribution
                .as_ref()
                .map(|dist| {
                    bincode::encode_to_vec(dist, bincode::config::standard())
                        .map_err(|e| ProtocolError::EncodingError(e.to_string()))
                })
                .transpose()?,
            TokenConfigurationChangeItem::NewTokensDestinationIdentity(identity) => {
                identity.map(|id| id.to_vec())
            }
            TokenConfigurationChangeItem::MintingAllowChoosingDestination(allow) => {
                Some(vec![*allow as u8])
            }
            TokenConfigurationChangeItem::MarketplaceTradeMode(mode) => Some(
                bincode::encode_to_vec(mode, bincode::config::standard())
                    .map_err(|e| ProtocolError::EncodingError(e.to_string()))?,
            ),
            TokenConfigurationChangeItem::MainControlGroup(position) => {
                position.map(|pos| pos.to_be_bytes().to_vec())
            }
        })
    }
    pub fn u8_item_index(&self) -> u8 {
        match self {
            TokenConfigurationChangeItem::TokenConfigurationNoChange => 0,
            TokenConfigurationChangeItem::Conventions(_) => 1,
            TokenConfigurationChangeItem::ConventionsControlGroup(_) => 2,
            TokenConfigurationChangeItem::ConventionsAdminGroup(_) => 3,
            TokenConfigurationChangeItem::MaxSupply(_) => 4,
            TokenConfigurationChangeItem::MaxSupplyControlGroup(_) => 5,
            TokenConfigurationChangeItem::MaxSupplyAdminGroup(_) => 6,
            TokenConfigurationChangeItem::PerpetualDistribution(_) => 7,
            TokenConfigurationChangeItem::PerpetualDistributionControlGroup(_) => 8,
            TokenConfigurationChangeItem::PerpetualDistributionAdminGroup(_) => 9,
            TokenConfigurationChangeItem::NewTokensDestinationIdentity(_) => 10,
            TokenConfigurationChangeItem::NewTokensDestinationIdentityControlGroup(_) => 11,
            TokenConfigurationChangeItem::NewTokensDestinationIdentityAdminGroup(_) => 12,
            TokenConfigurationChangeItem::MintingAllowChoosingDestination(_) => 13,
            TokenConfigurationChangeItem::MintingAllowChoosingDestinationControlGroup(_) => 14,
            TokenConfigurationChangeItem::MintingAllowChoosingDestinationAdminGroup(_) => 15,
            TokenConfigurationChangeItem::ManualMinting(_) => 16,
            TokenConfigurationChangeItem::ManualMintingAdminGroup(_) => 17,
            TokenConfigurationChangeItem::ManualBurning(_) => 18,
            TokenConfigurationChangeItem::ManualBurningAdminGroup(_) => 19,
            TokenConfigurationChangeItem::Freeze(_) => 20,
            TokenConfigurationChangeItem::FreezeAdminGroup(_) => 21,
            TokenConfigurationChangeItem::Unfreeze(_) => 22,
            TokenConfigurationChangeItem::UnfreezeAdminGroup(_) => 23,
            TokenConfigurationChangeItem::DestroyFrozenFunds(_) => 24,
            TokenConfigurationChangeItem::DestroyFrozenFundsAdminGroup(_) => 25,
            TokenConfigurationChangeItem::EmergencyAction(_) => 26,
            TokenConfigurationChangeItem::EmergencyActionAdminGroup(_) => 27,
            TokenConfigurationChangeItem::MarketplaceTradeMode(_) => 28,
            TokenConfigurationChangeItem::MarketplaceTradeModeControlGroup(_) => 29,
            TokenConfigurationChangeItem::MarketplaceTradeModeAdminGroup(_) => 30,
            TokenConfigurationChangeItem::MainControlGroup(_) => 31,
        }
    }
}

impl fmt::Display for TokenConfigurationChangeItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenConfigurationChangeItem::TokenConfigurationNoChange => {
                write!(f, "No Change in Token Configuration")
            }
            TokenConfigurationChangeItem::Conventions(convention) => {
                write!(f, "Conventions: {}", convention)
            }
            TokenConfigurationChangeItem::ConventionsControlGroup(control_group) => {
                write!(f, "Conventions Control Group: {}", control_group)
            }
            TokenConfigurationChangeItem::ConventionsAdminGroup(admin_group) => {
                write!(f, "Conventions Admin Group: {}", admin_group)
            }
            TokenConfigurationChangeItem::MaxSupply(max_supply) => match max_supply {
                Some(amount) => write!(f, "Max Supply: {}", amount),
                None => write!(f, "Max Supply: No Limit"),
            },
            TokenConfigurationChangeItem::MaxSupplyControlGroup(control_group) => {
                write!(f, "Max Supply Control Group: {}", control_group)
            }
            TokenConfigurationChangeItem::MaxSupplyAdminGroup(admin_group) => {
                write!(f, "Max Supply Admin Group: {}", admin_group)
            }
            TokenConfigurationChangeItem::PerpetualDistribution(distribution) => match distribution
            {
                Some(dist) => write!(f, "Perpetual Distribution: {}", dist),
                None => write!(f, "Perpetual Distribution: None"),
            },
            TokenConfigurationChangeItem::PerpetualDistributionControlGroup(control_group) => {
                write!(f, "Perpetual Distribution Control Group: {}", control_group)
            }
            TokenConfigurationChangeItem::PerpetualDistributionAdminGroup(admin_group) => {
                write!(f, "Perpetual Distribution Admin Group: {}", admin_group)
            }
            TokenConfigurationChangeItem::NewTokensDestinationIdentity(identity) => {
                match identity {
                    Some(id) => write!(f, "New Tokens Destination Identity: {}", id),
                    None => write!(f, "New Tokens Destination Identity: None"),
                }
            }
            TokenConfigurationChangeItem::NewTokensDestinationIdentityControlGroup(
                control_group,
            ) => {
                write!(
                    f,
                    "New Tokens Destination Identity Control Group: {}",
                    control_group
                )
            }
            TokenConfigurationChangeItem::NewTokensDestinationIdentityAdminGroup(admin_group) => {
                write!(
                    f,
                    "New Tokens Destination Identity Admin Group: {}",
                    admin_group
                )
            }
            TokenConfigurationChangeItem::MintingAllowChoosingDestination(allow) => {
                write!(f, "Minting Allow Choosing Destination: {}", allow)
            }
            TokenConfigurationChangeItem::MintingAllowChoosingDestinationControlGroup(
                control_group,
            ) => {
                write!(
                    f,
                    "Minting Allow Choosing Destination Control Group: {}",
                    control_group
                )
            }
            TokenConfigurationChangeItem::MintingAllowChoosingDestinationAdminGroup(
                admin_group,
            ) => {
                write!(
                    f,
                    "Minting Allow Choosing Destination Admin Group: {}",
                    admin_group
                )
            }
            TokenConfigurationChangeItem::ManualMinting(control_group) => {
                write!(f, "Manual Minting: {}", control_group)
            }
            TokenConfigurationChangeItem::ManualMintingAdminGroup(admin_group) => {
                write!(f, "Manual Minting Admin Group: {}", admin_group)
            }
            TokenConfigurationChangeItem::ManualBurning(control_group) => {
                write!(f, "Manual Burning: {}", control_group)
            }
            TokenConfigurationChangeItem::ManualBurningAdminGroup(admin_group) => {
                write!(f, "Manual Burning Admin Group: {}", admin_group)
            }
            TokenConfigurationChangeItem::Freeze(control_group) => {
                write!(f, "Freeze: {}", control_group)
            }
            TokenConfigurationChangeItem::FreezeAdminGroup(admin_group) => {
                write!(f, "Freeze Admin Group: {}", admin_group)
            }
            TokenConfigurationChangeItem::Unfreeze(control_group) => {
                write!(f, "Unfreeze: {}", control_group)
            }
            TokenConfigurationChangeItem::UnfreezeAdminGroup(admin_group) => {
                write!(f, "Unfreeze Admin Group: {}", admin_group)
            }
            TokenConfigurationChangeItem::DestroyFrozenFunds(control_group) => {
                write!(f, "Destroy Frozen Funds: {}", control_group)
            }
            TokenConfigurationChangeItem::DestroyFrozenFundsAdminGroup(admin_group) => {
                write!(f, "Destroy Frozen Funds Admin Group: {}", admin_group)
            }
            TokenConfigurationChangeItem::EmergencyAction(control_group) => {
                write!(f, "Emergency Action: {}", control_group)
            }
            TokenConfigurationChangeItem::EmergencyActionAdminGroup(admin_group) => {
                write!(f, "Emergency Action Admin Group: {}", admin_group)
            }
            TokenConfigurationChangeItem::MainControlGroup(position) => match position {
                Some(pos) => write!(f, "Main Control Group: {}", pos),
                None => write!(f, "Main Control Group: None"),
            },
            TokenConfigurationChangeItem::MarketplaceTradeMode(mode) => {
                write!(f, "Marketplace Trade Mode: {:?}", mode)
            }
            TokenConfigurationChangeItem::MarketplaceTradeModeControlGroup(control_group) => {
                write!(f, "Marketplace Trade Mode Control Group: {}", control_group)
            }
            TokenConfigurationChangeItem::MarketplaceTradeModeAdminGroup(admin_group) => {
                write!(f, "Marketplace Trade Mode Admin Group: {}", admin_group)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// Helper: build one instance of every variant using default inner values.
    fn all_variants() -> Vec<TokenConfigurationChangeItem> {
        let aat = AuthorizedActionTakers::NoOne;
        vec![
            TokenConfigurationChangeItem::TokenConfigurationNoChange,
            TokenConfigurationChangeItem::Conventions(
                TokenConfigurationConvention::V0(
                    crate::data_contract::associated_token::token_configuration_convention::v0::TokenConfigurationConventionV0::default(),
                ),
            ),
            TokenConfigurationChangeItem::ConventionsControlGroup(aat),
            TokenConfigurationChangeItem::ConventionsAdminGroup(aat),
            TokenConfigurationChangeItem::MaxSupply(None),
            TokenConfigurationChangeItem::MaxSupplyControlGroup(aat),
            TokenConfigurationChangeItem::MaxSupplyAdminGroup(aat),
            TokenConfigurationChangeItem::PerpetualDistribution(None),
            TokenConfigurationChangeItem::PerpetualDistributionControlGroup(aat),
            TokenConfigurationChangeItem::PerpetualDistributionAdminGroup(aat),
            TokenConfigurationChangeItem::NewTokensDestinationIdentity(None),
            TokenConfigurationChangeItem::NewTokensDestinationIdentityControlGroup(aat),
            TokenConfigurationChangeItem::NewTokensDestinationIdentityAdminGroup(aat),
            TokenConfigurationChangeItem::MintingAllowChoosingDestination(false),
            TokenConfigurationChangeItem::MintingAllowChoosingDestinationControlGroup(aat),
            TokenConfigurationChangeItem::MintingAllowChoosingDestinationAdminGroup(aat),
            TokenConfigurationChangeItem::ManualMinting(aat),
            TokenConfigurationChangeItem::ManualMintingAdminGroup(aat),
            TokenConfigurationChangeItem::ManualBurning(aat),
            TokenConfigurationChangeItem::ManualBurningAdminGroup(aat),
            TokenConfigurationChangeItem::Freeze(aat),
            TokenConfigurationChangeItem::FreezeAdminGroup(aat),
            TokenConfigurationChangeItem::Unfreeze(aat),
            TokenConfigurationChangeItem::UnfreezeAdminGroup(aat),
            TokenConfigurationChangeItem::DestroyFrozenFunds(aat),
            TokenConfigurationChangeItem::DestroyFrozenFundsAdminGroup(aat),
            TokenConfigurationChangeItem::EmergencyAction(aat),
            TokenConfigurationChangeItem::EmergencyActionAdminGroup(aat),
            TokenConfigurationChangeItem::MarketplaceTradeMode(TokenTradeMode::default()),
            TokenConfigurationChangeItem::MarketplaceTradeModeControlGroup(aat),
            TokenConfigurationChangeItem::MarketplaceTradeModeAdminGroup(aat),
            TokenConfigurationChangeItem::MainControlGroup(None),
        ]
    }

    // ---- u8_item_index returns unique values 0..=31 ----

    #[test]
    fn u8_item_index_values_are_unique() {
        let variants = all_variants();
        let indices: Vec<u8> = variants.iter().map(|v| v.u8_item_index()).collect();
        let unique: BTreeSet<u8> = indices.iter().cloned().collect();
        assert_eq!(
            indices.len(),
            unique.len(),
            "Duplicate u8_item_index values found: {:?}",
            indices
        );
    }

    #[test]
    fn u8_item_index_covers_0_through_31() {
        let variants = all_variants();
        let indices: BTreeSet<u8> = variants.iter().map(|v| v.u8_item_index()).collect();
        for i in 0u8..=31 {
            assert!(indices.contains(&i), "Missing u8_item_index value: {}", i);
        }
    }

    #[test]
    fn u8_item_index_all_within_range() {
        let variants = all_variants();
        for v in &variants {
            let idx = v.u8_item_index();
            assert!(idx <= 31, "Index {} exceeds expected max of 31", idx);
        }
    }

    #[test]
    fn u8_item_index_specific_known_values() {
        assert_eq!(
            TokenConfigurationChangeItem::TokenConfigurationNoChange.u8_item_index(),
            0
        );
        assert_eq!(
            TokenConfigurationChangeItem::MaxSupply(Some(100)).u8_item_index(),
            4
        );
        assert_eq!(
            TokenConfigurationChangeItem::ManualMinting(AuthorizedActionTakers::NoOne)
                .u8_item_index(),
            16
        );
        assert_eq!(
            TokenConfigurationChangeItem::MainControlGroup(Some(5)).u8_item_index(),
            31
        );
    }

    #[test]
    fn u8_item_index_variant_count() {
        // We expect exactly 32 variants (indices 0..=31)
        let variants = all_variants();
        assert_eq!(variants.len(), 32);
    }

    // --- payload_serialization ---

    #[test]
    fn payload_serialization_no_change_is_none() {
        let item = TokenConfigurationChangeItem::TokenConfigurationNoChange;
        assert!(item.payload_serialization().unwrap().is_none());
    }

    #[test]
    fn payload_serialization_max_supply_none_is_none() {
        let item = TokenConfigurationChangeItem::MaxSupply(None);
        assert!(item.payload_serialization().unwrap().is_none());
    }

    #[test]
    fn payload_serialization_max_supply_some_encodes_be_bytes() {
        let amount: u64 = 0x0102_0304_0506_0708;
        let item = TokenConfigurationChangeItem::MaxSupply(Some(amount));
        let bytes = item.payload_serialization().unwrap().unwrap();
        assert_eq!(bytes, amount.to_be_bytes().to_vec());
        assert_eq!(bytes.len(), 8);
    }

    #[test]
    fn payload_serialization_new_tokens_destination_identity_none_is_none() {
        let item = TokenConfigurationChangeItem::NewTokensDestinationIdentity(None);
        assert!(item.payload_serialization().unwrap().is_none());
    }

    #[test]
    fn payload_serialization_new_tokens_destination_identity_some_is_32_bytes() {
        let id = Identifier::from([0x77u8; 32]);
        let item = TokenConfigurationChangeItem::NewTokensDestinationIdentity(Some(id));
        let bytes = item.payload_serialization().unwrap().unwrap();
        assert_eq!(bytes.len(), 32);
        assert_eq!(bytes, vec![0x77u8; 32]);
    }

    #[test]
    fn payload_serialization_perpetual_distribution_none_is_none() {
        let item = TokenConfigurationChangeItem::PerpetualDistribution(None);
        assert!(item.payload_serialization().unwrap().is_none());
    }

    #[test]
    fn payload_serialization_main_control_group_none_is_none() {
        let item = TokenConfigurationChangeItem::MainControlGroup(None);
        assert!(item.payload_serialization().unwrap().is_none());
    }

    #[test]
    fn payload_serialization_main_control_group_some_is_two_be_bytes() {
        let pos: u16 = 0xABCD;
        let item = TokenConfigurationChangeItem::MainControlGroup(Some(pos));
        let bytes = item.payload_serialization().unwrap().unwrap();
        assert_eq!(bytes, pos.to_be_bytes().to_vec());
        assert_eq!(bytes.len(), 2);
    }

    #[test]
    fn payload_serialization_minting_allow_choosing_destination_true() {
        let item = TokenConfigurationChangeItem::MintingAllowChoosingDestination(true);
        let bytes = item.payload_serialization().unwrap().unwrap();
        assert_eq!(bytes, vec![1]);
    }

    #[test]
    fn payload_serialization_minting_allow_choosing_destination_false() {
        let item = TokenConfigurationChangeItem::MintingAllowChoosingDestination(false);
        let bytes = item.payload_serialization().unwrap().unwrap();
        assert_eq!(bytes, vec![0]);
    }

    #[test]
    fn payload_serialization_authorized_action_takers_variants_use_to_bytes() {
        // The big match arm has 23 variants that all serialize via AuthorizedActionTakers::to_bytes
        // Sanity-check a few representative variants produce the expected tag bytes.
        let aat = AuthorizedActionTakers::ContractOwner;
        let expected = aat.to_bytes();

        let variants = [
            TokenConfigurationChangeItem::ConventionsControlGroup(aat),
            TokenConfigurationChangeItem::ConventionsAdminGroup(aat),
            TokenConfigurationChangeItem::MaxSupplyControlGroup(aat),
            TokenConfigurationChangeItem::MaxSupplyAdminGroup(aat),
            TokenConfigurationChangeItem::PerpetualDistributionControlGroup(aat),
            TokenConfigurationChangeItem::PerpetualDistributionAdminGroup(aat),
            TokenConfigurationChangeItem::NewTokensDestinationIdentityControlGroup(aat),
            TokenConfigurationChangeItem::NewTokensDestinationIdentityAdminGroup(aat),
            TokenConfigurationChangeItem::MintingAllowChoosingDestinationControlGroup(aat),
            TokenConfigurationChangeItem::MintingAllowChoosingDestinationAdminGroup(aat),
            TokenConfigurationChangeItem::ManualMinting(aat),
            TokenConfigurationChangeItem::ManualMintingAdminGroup(aat),
            TokenConfigurationChangeItem::ManualBurning(aat),
            TokenConfigurationChangeItem::ManualBurningAdminGroup(aat),
            TokenConfigurationChangeItem::Freeze(aat),
            TokenConfigurationChangeItem::FreezeAdminGroup(aat),
            TokenConfigurationChangeItem::Unfreeze(aat),
            TokenConfigurationChangeItem::UnfreezeAdminGroup(aat),
            TokenConfigurationChangeItem::DestroyFrozenFunds(aat),
            TokenConfigurationChangeItem::DestroyFrozenFundsAdminGroup(aat),
            TokenConfigurationChangeItem::EmergencyAction(aat),
            TokenConfigurationChangeItem::EmergencyActionAdminGroup(aat),
            TokenConfigurationChangeItem::MarketplaceTradeModeControlGroup(aat),
            TokenConfigurationChangeItem::MarketplaceTradeModeAdminGroup(aat),
        ];
        for v in &variants {
            let bytes = v.payload_serialization().unwrap().unwrap();
            assert_eq!(bytes, expected, "mismatch for variant {:?}", v);
        }
    }

    #[test]
    fn payload_serialization_conventions_roundtrips_via_bincode() {
        // We don't assert the exact bytes (bincode-dependent) but we assert
        // (1) Some(..) is returned and (2) it decodes back to the same convention.
        use crate::data_contract::associated_token::token_configuration_convention::v0::TokenConfigurationConventionV0;
        let convention = TokenConfigurationConvention::V0(TokenConfigurationConventionV0 {
            localizations: Default::default(),
            decimals: 5,
        });
        let item = TokenConfigurationChangeItem::Conventions(convention.clone());
        let bytes = item.payload_serialization().unwrap().unwrap();
        assert!(!bytes.is_empty());
        let (decoded, _): (TokenConfigurationConvention, _) =
            bincode::decode_from_slice(&bytes, bincode::config::standard()).unwrap();
        assert_eq!(decoded, convention);
    }

    #[test]
    fn payload_serialization_marketplace_trade_mode_produces_bytes() {
        let item = TokenConfigurationChangeItem::MarketplaceTradeMode(TokenTradeMode::NotTradeable);
        let bytes = item.payload_serialization().unwrap().unwrap();
        assert!(!bytes.is_empty());
        // Roundtrip decode
        let (decoded, _): (TokenTradeMode, _) =
            bincode::decode_from_slice(&bytes, bincode::config::standard()).unwrap();
        assert_eq!(decoded, TokenTradeMode::NotTradeable);
    }

    // --- Display ---

    #[test]
    fn display_no_change() {
        let s = format!(
            "{}",
            TokenConfigurationChangeItem::TokenConfigurationNoChange
        );
        assert_eq!(s, "No Change in Token Configuration");
    }

    #[test]
    fn display_max_supply_some_vs_none() {
        let some = format!("{}", TokenConfigurationChangeItem::MaxSupply(Some(500)));
        let none = format!("{}", TokenConfigurationChangeItem::MaxSupply(None));
        assert!(some.contains("500"));
        assert!(none.contains("No Limit"));
    }

    #[test]
    fn display_perpetual_distribution_none_uses_none_marker() {
        let s = format!(
            "{}",
            TokenConfigurationChangeItem::PerpetualDistribution(None)
        );
        assert!(s.contains("None"));
    }

    #[test]
    fn display_new_tokens_destination_identity_none_uses_none_marker() {
        let s = format!(
            "{}",
            TokenConfigurationChangeItem::NewTokensDestinationIdentity(None)
        );
        assert!(s.contains("None"));
    }

    #[test]
    fn display_new_tokens_destination_identity_some_includes_id() {
        let id = Identifier::from([3u8; 32]);
        let s = format!(
            "{}",
            TokenConfigurationChangeItem::NewTokensDestinationIdentity(Some(id))
        );
        assert!(s.contains("New Tokens Destination Identity"));
    }

    #[test]
    fn display_main_control_group_none_uses_none_marker() {
        let s = format!("{}", TokenConfigurationChangeItem::MainControlGroup(None));
        assert!(s.contains("None"));
    }

    #[test]
    fn display_main_control_group_some_contains_position() {
        let s = format!(
            "{}",
            TokenConfigurationChangeItem::MainControlGroup(Some(7))
        );
        assert!(s.contains("7"));
    }

    #[test]
    fn display_minting_allow_choosing_destination_contains_value() {
        let s_true = format!(
            "{}",
            TokenConfigurationChangeItem::MintingAllowChoosingDestination(true)
        );
        let s_false = format!(
            "{}",
            TokenConfigurationChangeItem::MintingAllowChoosingDestination(false)
        );
        assert!(s_true.contains("true"));
        assert!(s_false.contains("false"));
    }

    #[test]
    fn display_marketplace_trade_mode_uses_debug() {
        let s = format!(
            "{}",
            TokenConfigurationChangeItem::MarketplaceTradeMode(TokenTradeMode::NotTradeable)
        );
        assert!(s.contains("NotTradeable"));
    }

    #[test]
    fn display_action_takers_group_variants() {
        // Exercise a handful of action-taker-carrying variants to confirm each
        // writes its label.
        let aat = AuthorizedActionTakers::ContractOwner;
        let cases = vec![
            (
                format!("{}", TokenConfigurationChangeItem::ManualMinting(aat)),
                "Manual Minting",
            ),
            (
                format!(
                    "{}",
                    TokenConfigurationChangeItem::ManualMintingAdminGroup(aat)
                ),
                "Manual Minting Admin Group",
            ),
            (
                format!("{}", TokenConfigurationChangeItem::ManualBurning(aat)),
                "Manual Burning",
            ),
            (
                format!("{}", TokenConfigurationChangeItem::Freeze(aat)),
                "Freeze",
            ),
            (
                format!("{}", TokenConfigurationChangeItem::Unfreeze(aat)),
                "Unfreeze",
            ),
            (
                format!("{}", TokenConfigurationChangeItem::DestroyFrozenFunds(aat)),
                "Destroy Frozen Funds",
            ),
            (
                format!("{}", TokenConfigurationChangeItem::EmergencyAction(aat)),
                "Emergency Action",
            ),
            (
                format!(
                    "{}",
                    TokenConfigurationChangeItem::MarketplaceTradeModeControlGroup(aat)
                ),
                "Marketplace Trade Mode Control Group",
            ),
            (
                format!(
                    "{}",
                    TokenConfigurationChangeItem::ConventionsControlGroup(aat)
                ),
                "Conventions Control Group",
            ),
        ];
        for (output, expected_prefix) in cases {
            assert!(
                output.contains(expected_prefix),
                "expected {:?} in {:?}",
                expected_prefix,
                output
            );
        }
    }

    // --- Equality ---

    #[test]
    fn equality_same_variant_same_data() {
        let a = TokenConfigurationChangeItem::MaxSupply(Some(42));
        let b = TokenConfigurationChangeItem::MaxSupply(Some(42));
        assert_eq!(a, b);
    }

    #[test]
    fn equality_same_variant_different_data_unequal() {
        let a = TokenConfigurationChangeItem::MaxSupply(Some(42));
        let b = TokenConfigurationChangeItem::MaxSupply(Some(43));
        assert_ne!(a, b);
    }

    #[test]
    fn equality_different_variants_unequal() {
        let a = TokenConfigurationChangeItem::MaxSupply(None);
        let b = TokenConfigurationChangeItem::MainControlGroup(None);
        assert_ne!(a, b);
    }

    #[test]
    fn equality_authorized_action_takers_sensitivity() {
        let a = TokenConfigurationChangeItem::ManualMinting(AuthorizedActionTakers::NoOne);
        let b = TokenConfigurationChangeItem::ManualMinting(AuthorizedActionTakers::ContractOwner);
        assert_ne!(a, b);
    }

    #[test]
    fn equality_bool_variant_distinguishes_values() {
        let a = TokenConfigurationChangeItem::MintingAllowChoosingDestination(true);
        let b = TokenConfigurationChangeItem::MintingAllowChoosingDestination(false);
        assert_ne!(a, b);
    }

    // --- Default + Clone ---

    #[test]
    fn default_is_no_change() {
        let d: TokenConfigurationChangeItem = Default::default();
        assert_eq!(d, TokenConfigurationChangeItem::TokenConfigurationNoChange);
    }

    #[test]
    fn clone_preserves_variant_and_data() {
        let aat = AuthorizedActionTakers::Identity(Identifier::from([5u8; 32]));
        let a = TokenConfigurationChangeItem::Freeze(aat);
        let b = a.clone();
        assert_eq!(a, b);
    }

    // --- Debug ---

    #[test]
    fn debug_trait_contains_variant_name() {
        let a = TokenConfigurationChangeItem::ManualMinting(AuthorizedActionTakers::NoOne);
        let dbg = format!("{:?}", a);
        assert!(dbg.contains("ManualMinting"));
    }
}

// --- canonical conversion trait impls (unification pass 1) ---
#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for TokenConfigurationChangeItem {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for TokenConfigurationChangeItem {}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests {
    use super::*;
    use platform_value::platform_value;
    use serde_json::json;

    /// Non-default variant (`MaxSupply(Some(...))`) with a non-zero inner amount
    /// so the wire-shape assertion catches a silent variant flip or inner-zero
    /// on round-trip.
    fn fixture() -> TokenConfigurationChangeItem {
        TokenConfigurationChangeItem::MaxSupply(Some(123_456_789u64))
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // `TokenConfigurationChangeItem` is internally tagged via its Repr:
        // `{ "$type":"maxSupply", "value":<inner> }` where `Some(x)` -> `x`.
        // `MaxSupply` now carries json_safe (Option<u64> -> string above
        // MAX_SAFE_INTEGER in HR JSON); here 123_456_789 stays numeric.
        // The value-path assertion uses `123_456_789u64` to lock in `Value::U64`.
        assert_eq!(json, json!({"$type": "maxSupply", "value": 123_456_789u64}));
        let recovered = TokenConfigurationChangeItem::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // `123_456_789u64`: explicit suffix forces `Value::U64`, matching
        // `TokenAmount`'s u64 type. Bare integer would expand to `Value::I32`.
        assert_eq!(
            value,
            platform_value!({"$type": "maxSupply", "value": 123_456_789u64})
        );
        let recovered = TokenConfigurationChangeItem::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }

    #[test]
    fn json_large_max_supply_serializes_as_string_for_js_safety() {
        use crate::serialization::JsonConvertible;
        // `MaxSupply` previously had NO json_safe annotation — a latent JS
        // precision bug. Above `Number.MAX_SAFE_INTEGER` it must now serialize
        // as a string in human-readable JSON. This pins that the Repr's
        // json_safe_option_u64 survives the internal-tag Content buffer.
        let original = TokenConfigurationChangeItem::MaxSupply(Some(9_007_199_254_740_993)); // 2^53 + 1
        let json = original.to_json().expect("to_json");
        assert_eq!(
            json,
            json!({"$type": "maxSupply", "value": "9007199254740993"})
        );
        let recovered = TokenConfigurationChangeItem::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }
}
