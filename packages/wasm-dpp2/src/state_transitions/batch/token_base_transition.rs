use crate::error::WasmDppResult;
use crate::identifier::IdentifierWasm;
use crate::impl_wasm_type_info;
use crate::state_transitions::GroupStateTransitionInfoWasm;
use crate::utils::IntoWasm;
use dpp::group::GroupStateTransitionInfo;
use dpp::prelude::IdentityNonce;
use dpp::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use dpp::state_transition::batch_transition::token_base_transition::v0::TokenBaseTransitionV0;
use dpp::state_transition::batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Debug, Clone, PartialEq)]
#[wasm_bindgen(js_name=TokenBaseTransition)]
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
    pub fn new(
        identity_contract_nonce: IdentityNonce,
        token_contract_position: u16,
        #[wasm_bindgen(unchecked_param_type = "Identifier | Uint8Array | string")]
        data_contract_id: &JsValue,
        #[wasm_bindgen(unchecked_param_type = "Identifier | Uint8Array | string")]
        token_id: &JsValue,
        using_group_info: &JsValue,
    ) -> WasmDppResult<TokenBaseTransitionWasm> {
        let group_info: Option<GroupStateTransitionInfo> =
            match using_group_info.is_undefined() {
                false => Some(
                    using_group_info
                        .to_wasm::<GroupStateTransitionInfoWasm>("GroupStateTransitionInfo")?
                        .clone()
                        .into(),
                ),
                true => None,
            };

        Ok(TokenBaseTransitionWasm(TokenBaseTransition::V0(
            TokenBaseTransitionV0 {
                identity_contract_nonce,
                token_contract_position,
                data_contract_id: IdentifierWasm::try_from(data_contract_id)?.into(),
                token_id: IdentifierWasm::try_from(token_id)?.into(),
                using_group_info: group_info,
            },
        )))
    }

    #[wasm_bindgen(getter = identityContractNonce)]
    pub fn get_identity_contract_nonce(&self) -> IdentityNonce {
        self.0.identity_contract_nonce()
    }

    #[wasm_bindgen(getter = tokenContractPosition)]
    pub fn get_token_contract_position(&self) -> u16 {
        self.0.token_contract_position()
    }

    #[wasm_bindgen(getter = dataContractId)]
    pub fn get_data_contract_id(&self) -> IdentifierWasm {
        self.0.data_contract_id().into()
    }

    #[wasm_bindgen(getter = tokenId)]
    pub fn get_token_id(&self) -> IdentifierWasm {
        self.0.token_id().into()
    }

    #[wasm_bindgen(getter = usingGroupInfo)]
    pub fn get_using_group_info(&self) -> Option<GroupStateTransitionInfoWasm> {
        self.0
            .using_group_info()
            .map(|using_group_info| using_group_info.into())
    }

    #[wasm_bindgen(setter = identityContractNonce)]
    pub fn set_identity_contract_nonce(&mut self, identity_contract_nonce: IdentityNonce) {
        self.0.set_identity_contract_nonce(identity_contract_nonce)
    }

    #[wasm_bindgen(setter = tokenContractPosition)]
    pub fn set_token_contract_position(&mut self, pos: u16) {
        self.0.set_token_contract_position(pos)
    }

    #[wasm_bindgen(setter = dataContractId)]
    pub fn set_data_contract_id(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "Identifier | Uint8Array | string")]
        data_contract_id: &JsValue,
    ) -> WasmDppResult<()> {
        self.0
            .set_data_contract_id(IdentifierWasm::try_from(data_contract_id)?.into());
        Ok(())
    }

    #[wasm_bindgen(setter = tokenId)]
    pub fn set_token_id(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "Identifier | Uint8Array | string")]
        token_id: &JsValue,
    ) -> WasmDppResult<()> {
        self.0
            .set_token_id(IdentifierWasm::try_from(token_id)?.into());

        Ok(())
    }

    #[wasm_bindgen(setter = usingGroupInfo)]
    pub fn set_using_group_info(&mut self, using_group_info: &JsValue) -> WasmDppResult<()> {
        let group_info: Option<GroupStateTransitionInfo> =
            match using_group_info.is_undefined() {
                false => Some(
                    using_group_info
                        .to_wasm::<GroupStateTransitionInfoWasm>("GroupStateTransitionInfo")?
                        .clone()
                        .into(),
                ),
                true => None,
            };

        self.0.set_using_group_info(group_info);

        Ok(())
    }
}

impl_wasm_type_info!(TokenBaseTransitionWasm, TokenBaseTransition);
