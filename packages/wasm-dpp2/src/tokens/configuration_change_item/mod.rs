pub mod items;

use crate::identifier::IdentifierWasm;
use crate::tokens::configuration::authorized_action_takers::AuthorizedActionTakersWasm;
use crate::tokens::configuration::configuration_convention::TokenConfigurationConventionWasm;
use crate::tokens::configuration::perpetual_distribution::TokenPerpetualDistributionWasm;
use crate::tokens::configuration::trade_mode::TokenTradeModeWasm;
use dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Debug, Clone, PartialEq)]
#[wasm_bindgen(js_name = "TokenConfigurationChangeItem")]
pub struct TokenConfigurationChangeItemWasm(TokenConfigurationChangeItem);

impl From<TokenConfigurationChangeItemWasm> for TokenConfigurationChangeItem {
    fn from(item: TokenConfigurationChangeItemWasm) -> Self {
        item.0
    }
}

impl From<TokenConfigurationChangeItem> for TokenConfigurationChangeItemWasm {
    fn from(item: TokenConfigurationChangeItem) -> Self {
        TokenConfigurationChangeItemWasm(item)
    }
}

#[wasm_bindgen(js_class = TokenConfigurationChangeItem)]
impl TokenConfigurationChangeItemWasm {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "TokenConfigurationChangeItem".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "TokenConfigurationChangeItem".to_string()
    }

    #[wasm_bindgen(js_name = "getItemName")]
    pub fn get_item_name(&self) -> String {
        match self.0.clone() {
            TokenConfigurationChangeItem::TokenConfigurationNoChange => {
                String::from("TokenConfigurationNoChange")
            }
            TokenConfigurationChangeItem::Conventions(_) => String::from("Conventions"),
            TokenConfigurationChangeItem::ConventionsControlGroup(_) => {
                String::from("ConventionsControlGroup")
            }
            TokenConfigurationChangeItem::ConventionsAdminGroup(_) => {
                String::from("ConventionsAdminGroup")
            }
            TokenConfigurationChangeItem::MaxSupply(_) => String::from("MaxSupply"),
            TokenConfigurationChangeItem::MaxSupplyControlGroup(_) => {
                String::from("MaxSupplyControlGroup")
            }
            TokenConfigurationChangeItem::MaxSupplyAdminGroup(_) => {
                String::from("MaxSupplyAdminGroup")
            }
            TokenConfigurationChangeItem::PerpetualDistribution(_) => {
                String::from("PerpetualDistribution")
            }
            TokenConfigurationChangeItem::PerpetualDistributionControlGroup(_) => {
                String::from("PerpetualDistributionControlGroup")
            }
            TokenConfigurationChangeItem::PerpetualDistributionAdminGroup(_) => {
                String::from("PerpetualDistributionAdminGroup")
            }
            TokenConfigurationChangeItem::NewTokensDestinationIdentity(_) => {
                String::from("NewTokensDestinationIdentity")
            }
            TokenConfigurationChangeItem::NewTokensDestinationIdentityControlGroup(_) => {
                String::from("NewTokensDestinationIdentityControlGroup")
            }
            TokenConfigurationChangeItem::NewTokensDestinationIdentityAdminGroup(_) => {
                String::from("NewTokensDestinationIdentityAdminGroup")
            }
            TokenConfigurationChangeItem::MintingAllowChoosingDestination(_) => {
                String::from("MintingAllowChoosingDestination")
            }
            TokenConfigurationChangeItem::MintingAllowChoosingDestinationControlGroup(_) => {
                String::from("MintingAllowChoosingDestinationControlGroup")
            }
            TokenConfigurationChangeItem::MintingAllowChoosingDestinationAdminGroup(_) => {
                String::from("MintingAllowChoosingDestinationAdminGroup")
            }
            TokenConfigurationChangeItem::ManualMinting(_) => String::from("ManualMinting"),
            TokenConfigurationChangeItem::ManualMintingAdminGroup(_) => {
                String::from("ManualMintingAdminGroup")
            }
            TokenConfigurationChangeItem::ManualBurning(_) => String::from("ManualBurning"),
            TokenConfigurationChangeItem::ManualBurningAdminGroup(_) => {
                String::from("ManualBurningAdminGroup")
            }
            TokenConfigurationChangeItem::Freeze(_) => String::from("Freeze"),
            TokenConfigurationChangeItem::FreezeAdminGroup(_) => String::from("FreezeAdminGroup"),
            TokenConfigurationChangeItem::Unfreeze(_) => String::from("Unfreeze"),
            TokenConfigurationChangeItem::UnfreezeAdminGroup(_) => {
                String::from("UnfreezeAdminGroup")
            }
            TokenConfigurationChangeItem::DestroyFrozenFunds(_) => {
                String::from("DestroyFrozenFunds")
            }
            TokenConfigurationChangeItem::DestroyFrozenFundsAdminGroup(_) => {
                String::from("DestroyFrozenFundsAdminGroup")
            }
            TokenConfigurationChangeItem::EmergencyAction(_) => String::from("EmergencyAction"),
            TokenConfigurationChangeItem::EmergencyActionAdminGroup(_) => {
                String::from("EmergencyActionAdminGroup")
            }
            TokenConfigurationChangeItem::MarketplaceTradeMode(_) => {
                String::from("MarketplaceTradeMode")
            }
            TokenConfigurationChangeItem::MarketplaceTradeModeControlGroup(_) => {
                String::from("MarketplaceTradeModeControlGroup")
            }
            TokenConfigurationChangeItem::MarketplaceTradeModeAdminGroup(_) => {
                String::from("MarketplaceTradeModeAdminGroup")
            }
            TokenConfigurationChangeItem::MainControlGroup(_) => String::from("MainControlGroup"),
        }
    }

    #[wasm_bindgen(js_name = "getItem")]
    pub fn get_item(&self) -> JsValue {
        match self.0.clone() {
            TokenConfigurationChangeItem::TokenConfigurationNoChange => {
                JsValue::from_str("TokenConfigurationNoChange")
            }
            TokenConfigurationChangeItem::Conventions(convention) => {
                JsValue::from(TokenConfigurationConventionWasm::from(convention))
            }
            TokenConfigurationChangeItem::ConventionsControlGroup(action_takers) => {
                JsValue::from(AuthorizedActionTakersWasm::from(action_takers))
            }
            TokenConfigurationChangeItem::ConventionsAdminGroup(action_takers) => {
                JsValue::from(AuthorizedActionTakersWasm::from(action_takers))
            }
            TokenConfigurationChangeItem::MaxSupply(amount) => JsValue::from(amount),
            TokenConfigurationChangeItem::MaxSupplyControlGroup(action_takers) => {
                JsValue::from(AuthorizedActionTakersWasm::from(action_takers))
            }
            TokenConfigurationChangeItem::MaxSupplyAdminGroup(action_takers) => {
                JsValue::from(AuthorizedActionTakersWasm::from(action_takers))
            }
            TokenConfigurationChangeItem::PerpetualDistribution(perpetual_distribution) => {
                match perpetual_distribution {
                    Some(token_perpetual_distribution) => JsValue::from(
                        TokenPerpetualDistributionWasm::from(token_perpetual_distribution),
                    ),
                    None => JsValue::null(),
                }
            }
            TokenConfigurationChangeItem::PerpetualDistributionControlGroup(action_takers) => {
                JsValue::from(AuthorizedActionTakersWasm::from(action_takers))
            }
            TokenConfigurationChangeItem::PerpetualDistributionAdminGroup(action_takers) => {
                JsValue::from(AuthorizedActionTakersWasm::from(action_takers))
            }
            TokenConfigurationChangeItem::NewTokensDestinationIdentity(identifier) => {
                match identifier {
                    Some(id) => JsValue::from(IdentifierWasm::from(id)),
                    None => JsValue::null(),
                }
            }
            TokenConfigurationChangeItem::NewTokensDestinationIdentityControlGroup(
                action_takers,
            ) => JsValue::from(AuthorizedActionTakersWasm::from(action_takers)),
            TokenConfigurationChangeItem::NewTokensDestinationIdentityAdminGroup(action_takers) => {
                JsValue::from(AuthorizedActionTakersWasm::from(action_takers))
            }
            TokenConfigurationChangeItem::MintingAllowChoosingDestination(flag) => {
                JsValue::from_bool(flag)
            }
            TokenConfigurationChangeItem::MintingAllowChoosingDestinationControlGroup(
                action_takers,
            ) => JsValue::from(AuthorizedActionTakersWasm::from(action_takers)),
            TokenConfigurationChangeItem::MintingAllowChoosingDestinationAdminGroup(
                action_takers,
            ) => JsValue::from(AuthorizedActionTakersWasm::from(action_takers)),
            TokenConfigurationChangeItem::ManualMinting(action_takers) => {
                JsValue::from(AuthorizedActionTakersWasm::from(action_takers))
            }
            TokenConfigurationChangeItem::ManualMintingAdminGroup(action_takers) => {
                JsValue::from(AuthorizedActionTakersWasm::from(action_takers))
            }
            TokenConfigurationChangeItem::ManualBurning(action_takers) => {
                JsValue::from(AuthorizedActionTakersWasm::from(action_takers))
            }
            TokenConfigurationChangeItem::ManualBurningAdminGroup(action_takers) => {
                JsValue::from(AuthorizedActionTakersWasm::from(action_takers))
            }
            TokenConfigurationChangeItem::Freeze(action_takers) => {
                JsValue::from(AuthorizedActionTakersWasm::from(action_takers))
            }
            TokenConfigurationChangeItem::FreezeAdminGroup(action_takers) => {
                JsValue::from(AuthorizedActionTakersWasm::from(action_takers))
            }
            TokenConfigurationChangeItem::Unfreeze(action_takers) => {
                JsValue::from(AuthorizedActionTakersWasm::from(action_takers))
            }
            TokenConfigurationChangeItem::UnfreezeAdminGroup(action_takers) => {
                JsValue::from(AuthorizedActionTakersWasm::from(action_takers))
            }
            TokenConfigurationChangeItem::DestroyFrozenFunds(action_takers) => {
                JsValue::from(AuthorizedActionTakersWasm::from(action_takers))
            }
            TokenConfigurationChangeItem::DestroyFrozenFundsAdminGroup(action_takers) => {
                JsValue::from(AuthorizedActionTakersWasm::from(action_takers))
            }
            TokenConfigurationChangeItem::EmergencyAction(action_takers) => {
                JsValue::from(AuthorizedActionTakersWasm::from(action_takers))
            }
            TokenConfigurationChangeItem::EmergencyActionAdminGroup(action_takers) => {
                JsValue::from(AuthorizedActionTakersWasm::from(action_takers))
            }
            TokenConfigurationChangeItem::MarketplaceTradeMode(trade_mode) => {
                JsValue::from(TokenTradeModeWasm::from(trade_mode))
            }
            TokenConfigurationChangeItem::MarketplaceTradeModeControlGroup(action_takers) => {
                JsValue::from(AuthorizedActionTakersWasm::from(action_takers))
            }
            TokenConfigurationChangeItem::MarketplaceTradeModeAdminGroup(action_takers) => {
                JsValue::from(AuthorizedActionTakersWasm::from(action_takers))
            }
            TokenConfigurationChangeItem::MainControlGroup(group_contract_position) => {
                JsValue::from(group_contract_position)
            }
        }
    }
}
