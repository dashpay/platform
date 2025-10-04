use crate::group_state_transition_info::GroupStateTransitionInfoWASM;
use crate::identifier::IdentifierWASM;
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
pub struct TokenBaseTransitionWASM(TokenBaseTransition);

impl From<TokenBaseTransition> for TokenBaseTransitionWASM {
    fn from(t: TokenBaseTransition) -> Self {
        TokenBaseTransitionWASM(t)
    }
}

impl From<TokenBaseTransitionWASM> for TokenBaseTransition {
    fn from(t: TokenBaseTransitionWASM) -> Self {
        t.0
    }
}

#[wasm_bindgen(js_class = TokenBaseTransition)]
impl TokenBaseTransitionWASM {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "TokenBaseTransition".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "TokenBaseTransition".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(
        identity_contract_nonce: IdentityNonce,
        token_contract_position: u16,
        js_data_contract_id: &JsValue,
        js_token_id: &JsValue,
        js_using_group_info: &JsValue,
    ) -> Result<TokenBaseTransitionWASM, JsValue> {
        let using_group_info: Option<GroupStateTransitionInfo> =
            match js_using_group_info.is_undefined() {
                false => Some(
                    js_using_group_info
                        .to_wasm::<GroupStateTransitionInfoWASM>("GroupStateTransitionInfo")?
                        .clone()
                        .into(),
                ),
                true => None,
            };

        Ok(TokenBaseTransitionWASM(TokenBaseTransition::V0(
            TokenBaseTransitionV0 {
                identity_contract_nonce,
                token_contract_position,
                data_contract_id: IdentifierWASM::try_from(js_data_contract_id)?.into(),
                token_id: IdentifierWASM::try_from(js_token_id)?.into(),
                using_group_info,
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
    pub fn get_data_contract_id(&self) -> IdentifierWASM {
        self.0.data_contract_id().into()
    }

    #[wasm_bindgen(getter = tokenId)]
    pub fn get_token_id(&self) -> IdentifierWASM {
        self.0.token_id().into()
    }

    #[wasm_bindgen(getter = usingGroupInfo)]
    pub fn get_using_group_info(&self) -> Option<GroupStateTransitionInfoWASM> {
        match self.0.using_group_info() {
            Some(using_group_info) => Some(using_group_info.clone().into()),
            None => None,
        }
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
    pub fn set_data_contract_id(&mut self, js_identifier: &JsValue) -> Result<(), JsValue> {
        self.0
            .set_data_contract_id(IdentifierWASM::try_from(js_identifier)?.into());
        Ok(())
    }

    #[wasm_bindgen(setter = tokenId)]
    pub fn set_token_id(&mut self, js_identifier: &JsValue) -> Result<(), JsValue> {
        self.0
            .set_token_id(IdentifierWASM::try_from(js_identifier)?.into());

        Ok(())
    }

    #[wasm_bindgen(setter = usingGroupInfo)]
    pub fn set_using_group_info(&mut self, js_using_group_info: &JsValue) -> Result<(), JsValue> {
        let using_group_info: Option<GroupStateTransitionInfo> =
            match js_using_group_info.is_undefined() {
                false => Some(
                    js_using_group_info
                        .to_wasm::<GroupStateTransitionInfoWASM>("GroupStateTransitionInfo")?
                        .clone()
                        .into(),
                ),
                true => None,
            };

        self.0.set_using_group_info(using_group_info);

        Ok(())
    }
}
