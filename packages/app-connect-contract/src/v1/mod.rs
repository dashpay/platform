use crate::Error;
use serde_json::Value;

pub mod document_types {
    pub mod login_key_response {
        pub const NAME: &str = "loginKeyResponse";

        pub mod properties {
            pub const APP_EPHEMERAL_PUB_KEY_HASH: &str = "appEphemeralPubKeyHash";
            pub const WALLET_EPHEMERAL_PUB_KEY: &str = "walletEphemeralPubKey";
            pub const ENCRYPTED_PAYLOAD: &str = "encryptedPayload";
        }

        pub mod indexes {
            pub const BY_REQUEST: &str = "byRequest";
        }
    }
}

pub fn load_documents_schemas() -> Result<Value, Error> {
    serde_json::from_str(include_str!(
        "../../schema/v1/app-connect-contract-documents.json"
    ))
    .map_err(Error::InvalidSchemaJson)
}
