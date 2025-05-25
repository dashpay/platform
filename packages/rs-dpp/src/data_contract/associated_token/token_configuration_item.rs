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
#[cfg(feature = "state-transition-serde-conversion")]
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
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
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
impl TokenConfigurationChangeItem {
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
