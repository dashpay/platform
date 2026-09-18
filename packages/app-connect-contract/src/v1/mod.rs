use crate::Error;
use serde_json::Value;

pub mod document_types {
    pub mod login_key_response {
        pub const NAME: &str = "loginKeyResponse";

        pub mod properties {
            pub const CONTRACT_ID: &str = "contractId";
            pub const APP_EPHEMERAL_PUB_KEY_HASH: &str = "appEphemeralPubKeyHash";
            pub const WALLET_EPHEMERAL_PUB_KEY: &str = "walletEphemeralPubKey";
            pub const ENCRYPTED_PAYLOAD: &str = "encryptedPayload";
        }

        pub mod indexes {
            pub const BY_CONTRACT_AND_EPHEMERAL_KEY: &str = "byContractAndEphemeralKey";
        }
    }

    pub mod app_manifest {
        pub const NAME: &str = "appManifest";

        pub mod properties {
            pub const APP_CONTRACT_ID: &str = "appContractId";
            pub const APP_NAME: &str = "name";
            pub const URL: &str = "url";
            pub const AUTH_BOUNDS_KIND: &str = "authBoundsKind";
            pub const AUTH_BOUNDS_ID: &str = "authBoundsId";
            pub const AUTH_BOUNDS_DOC_TYPE: &str = "authBoundsDocType";
            pub const SESSION_SECONDS: &str = "sessionSeconds";
            pub const SESSION_BUDGET: &str = "sessionBudget";
            pub const ENC_BINDINGS: &str = "encBindings";
        }

        pub mod indexes {
            pub const BY_OWNER_AND_APP: &str = "byOwnerAndApp";
        }

        /// Values of the `authBoundsKind` property: the contract bounds the app asks the
        /// wallet to put on the login key.
        pub mod auth_bounds_kind {
            /// No bounds: the key can act anywhere; expiry and budget still apply.
            pub const NONE: u8 = 0;
            /// Bound to the contract named by `authBoundsId`.
            pub const CONTRACT: u8 = 1;
            /// Bound to the document type `authBoundsDocType` of the contract named by
            /// `authBoundsId`.
            pub const CONTRACT_DOCUMENT_TYPE: u8 = 2;
            /// Bound to the contract group named by `authBoundsId`.
            pub const CONTRACT_GROUP: u8 = 3;
        }

        /// Layout of the packed `encBindings` byte array: zero to
        /// [`MAX_RECORDS`](enc_bindings::MAX_RECORDS) fixed-size records, each naming a
        /// contract (or one of its document types) and which encryption key purposes the
        /// app wants bound there.
        pub mod enc_bindings {
            /// Size of one record: contract id, purpose mask, document type name.
            pub const RECORD_SIZE: usize = 96;
            /// Maximum number of records, so the array is at most 768 bytes.
            pub const MAX_RECORDS: usize = 8;
            /// Byte offset and length of the contract id within a record.
            pub const CONTRACT_ID_OFFSET: usize = 0;
            pub const CONTRACT_ID_SIZE: usize = 32;
            /// Byte offset of the one-byte purpose mask within a record.
            pub const PURPOSE_MASK_OFFSET: usize = 32;
            /// Purpose mask bit asking for an ENCRYPTION key bound there.
            pub const PURPOSE_ENCRYPTION: u8 = 0b01;
            /// Purpose mask bit asking for a DECRYPTION key bound there.
            pub const PURPOSE_DECRYPTION: u8 = 0b10;
            /// Byte offset and length of the zero-padded document type name within a
            /// record; all zero for a contract-level binding.
            pub const DOCUMENT_TYPE_NAME_OFFSET: usize = 33;
            pub const DOCUMENT_TYPE_NAME_SIZE: usize = 63;
        }
    }
}

pub fn load_documents_schemas() -> Result<Value, Error> {
    serde_json::from_str(include_str!(
        "../../schema/v1/app-connect-contract-documents.json"
    ))
    .map_err(Error::InvalidSchemaJson)
}
