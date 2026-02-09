use crate::error::WasmDppResult;
use crate::identifier::{IdentifierLikeJs, IdentifierWasm};
use crate::impl_try_from_js_value;
use crate::impl_wasm_type_info;
use crate::state_transitions::GroupStateTransitionInfoWasm;
use crate::utils::{
    try_from_options, try_from_options_optional, try_from_options_with, try_to_u16, try_to_u64,
};
use dpp::group::GroupStateTransitionInfo;
use dpp::prelude::IdentityNonce;
use dpp::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use dpp::state_transition::batch_transition::token_base_transition::v0::TokenBaseTransitionV0;
use dpp::state_transition::batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const TOKEN_BASE_TRANSITION_OPTIONS_TS: &str = r#"
export interface TokenBaseTransitionOptions {
    identityContractNonce: bigint;
    tokenContractPosition: number;
    dataContractId: IdentifierLike;
    tokenId: IdentifierLike;
    usingGroupInfo?: GroupStateTransitionInfo;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "TokenBaseTransitionOptions")]
    pub type TokenBaseTransitionOptionsJs;
}

#[derive(Debug, Clone, PartialEq)]
#[wasm_bindgen(js_name = "TokenBaseTransition")]
pub struct TokenBaseTransitionWasm(TokenBaseTransition);

impl From<TokenBaseTransition> for TokenBaseTransitionWasm {
    fn from(t: TokenBaseTransition) -> Self {
        TokenBaseTransitionWasm(t)
    }
}

impl From<TokenBaseTransitionWasm> for TokenBaseTransition {
    fn from(t: TokenBaseTransitionWasm) -> Self {
        t.0
    }
}

#[wasm_bindgen(js_class = TokenBaseTransition)]
impl TokenBaseTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        options: TokenBaseTransitionOptionsJs,
    ) -> WasmDppResult<TokenBaseTransitionWasm> {
        let identity_contract_nonce =
            try_from_options_with(&options, "identityContractNonce", |v| {
                try_to_u64(v, "identityContractNonce")
            })?;

        let token_contract_position =
            try_from_options_with(&options, "tokenContractPosition", |v| {
                try_to_u16(v, "tokenContractPosition")
            })?;

        let data_contract_id: IdentifierWasm = try_from_options(&options, "dataContractId")?;

        let token_id: IdentifierWasm = try_from_options(&options, "tokenId")?;

        let group_info: Option<GroupStateTransitionInfoWasm> =
            try_from_options_optional(&options, "usingGroupInfo")?;

        Ok(TokenBaseTransitionWasm(TokenBaseTransition::V0(
            TokenBaseTransitionV0 {
                identity_contract_nonce,
                token_contract_position,
                data_contract_id: data_contract_id.into(),
                token_id: token_id.into(),
                using_group_info: group_info.map(Into::into),
            },
        )))
    }

    #[wasm_bindgen(getter = identityContractNonce)]
    pub fn identity_contract_nonce(&self) -> IdentityNonce {
        self.0.identity_contract_nonce()
    }

    #[wasm_bindgen(getter = tokenContractPosition)]
    pub fn token_contract_position(&self) -> u16 {
        self.0.token_contract_position()
    }

    #[wasm_bindgen(getter = dataContractId)]
    pub fn data_contract_id(&self) -> IdentifierWasm {
        self.0.data_contract_id().into()
    }

    #[wasm_bindgen(getter = tokenId)]
    pub fn token_id(&self) -> IdentifierWasm {
        self.0.token_id().into()
    }

    #[wasm_bindgen(getter = usingGroupInfo)]
    pub fn using_group_info(&self) -> Option<GroupStateTransitionInfoWasm> {
        self.0
            .using_group_info()
            .map(|using_group_info| using_group_info.into())
    }

    #[wasm_bindgen(setter = identityContractNonce)]
    pub fn set_identity_contract_nonce(
        &mut self,
        #[wasm_bindgen(js_name = "identityContractNonce")] identity_contract_nonce: IdentityNonce,
    ) {
        self.0.set_identity_contract_nonce(identity_contract_nonce)
    }

    #[wasm_bindgen(setter = tokenContractPosition)]
    pub fn set_token_contract_position(&mut self, pos: JsValue) -> WasmDppResult<()> {
        self.0
            .set_token_contract_position(try_to_u16(&pos, "tokenContractPosition")?);
        Ok(())
    }

    #[wasm_bindgen(setter = dataContractId)]
    pub fn set_data_contract_id(
        &mut self,
        #[wasm_bindgen(js_name = "dataContractId")] data_contract_id: IdentifierLikeJs,
    ) -> WasmDppResult<()> {
        self.0.set_data_contract_id(data_contract_id.try_into()?);
        Ok(())
    }

    #[wasm_bindgen(setter = tokenId)]
    pub fn set_token_id(
        &mut self,
        #[wasm_bindgen(js_name = "tokenId")] token_id: IdentifierLikeJs,
    ) -> WasmDppResult<()> {
        self.0.set_token_id(token_id.try_into()?);

        Ok(())
    }

    #[wasm_bindgen(setter = usingGroupInfo)]
    pub fn set_using_group_info(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "GroupStateTransitionInfo | undefined")]
        using_group_info: &JsValue,
    ) -> WasmDppResult<()> {
        let group_info: Option<GroupStateTransitionInfo> = if using_group_info.is_undefined() {
            None
        } else {
            Some(GroupStateTransitionInfoWasm::try_from(using_group_info)?.into())
        };

        self.0.set_using_group_info(group_info);

        Ok(())
    }
}

impl_try_from_js_value!(TokenBaseTransitionWasm, "TokenBaseTransition");
impl_wasm_type_info!(TokenBaseTransitionWasm, TokenBaseTransition);
