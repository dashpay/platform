use dpp::consensus::basic::identity::InvalidIdentityAssetLockTransactionError;

use dpp::consensus::ConsensusError;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
#[derive(Debug, Default, Clone)]
pub struct TransactionDecodeError {
    message: Option<String>,
}

impl TransactionDecodeError {
    pub fn get_message(&self) -> &Option<String> {
        &self.message
    }
}

#[wasm_bindgen(js_name=InvalidIdentityAssetLockTransactionError)]
pub struct InvalidIdentityAssetLockTransactionErrorWasm {
    validation_error: TransactionDecodeError,
    message: String,
}

// TODO: since the validation error can not be cloned,
//  this error is only partially implemented
impl From<&InvalidIdentityAssetLockTransactionError>
    for InvalidIdentityAssetLockTransactionErrorWasm
{
    fn from(e: &InvalidIdentityAssetLockTransactionError) -> Self {
        Self {
            validation_error: TransactionDecodeError {
                message: e.validation_error().map(|e| e.to_string()),
            },
            message: e.to_string(),
        }
    }
}

#[wasm_bindgen(js_class=InvalidIdentityAssetLockTransactionError)]
impl InvalidIdentityAssetLockTransactionErrorWasm {
    #[wasm_bindgen(js_name=getValidationError)]
    pub fn validation_error(&self) -> JsValue {
        let kek = self.validation_error.clone();
        kek.into()
    }

    #[wasm_bindgen(js_name=getMessage)]
    pub fn get_message(&self) -> String {
        self.message.clone()
    }

    // TODO: finish implementing getCode
    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        ConsensusError::InvalidIdentityAssetLockTransactionError(
            InvalidIdentityAssetLockTransactionError::new(""),
        )
        .code()
    }
}
