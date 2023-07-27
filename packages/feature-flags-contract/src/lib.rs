// TODO: Use std when Lazy is stabilized
use once_cell::sync::Lazy;
use platform_value::Identifier;
use serde_json::Error;
use serde_json::Value;

pub const ID_BYTES: [u8; 32] = [
    245, 172, 216, 200, 193, 110, 185, 172, 40, 110, 7, 132, 190, 86, 127, 80, 9, 244, 86, 26, 243,
    212, 255, 2, 91, 7, 90, 243, 68, 55, 152, 34,
];

pub const OWNER_ID_BYTES: [u8; 32] = [
    240, 1, 0, 176, 193, 227, 118, 43, 139, 193, 66, 30, 17, 60, 118, 178, 166, 53, 197, 147, 11,
    154, 191, 43, 51, 101, 131, 190, 89, 135, 167, 21,
];

pub mod document_types {
    pub mod update_consensus_params {
        pub const NAME: &str = "updateConsensusParams";
    }
}

pub static ID: Lazy<Identifier> = Lazy::new(|| Identifier::from(ID_BYTES));
pub static OWNER_ID: Lazy<Identifier> = Lazy::new(|| Identifier::from(OWNER_ID_BYTES));

pub fn load_documents_schemas() -> Result<Value, Error> {
    serde_json::from_str(include_str!("../schema/feature-flags-documents.json"))
}
