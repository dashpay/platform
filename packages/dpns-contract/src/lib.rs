use serde_json::{Error, Value};

pub const ID_BYTES: [u8; 32] = [
    230, 104, 198, 89, 175, 102, 174, 225, 231, 44, 24, 109, 222, 123, 91, 126, 10, 29, 113, 42, 9,
    196, 13, 87, 33, 246, 34, 191, 83, 197, 49, 85,
];

pub const OWNER_ID_BYTES: [u8; 32] = [
    48, 18, 193, 155, 152, 236, 0, 51, 173, 219, 54, 205, 100, 183, 245, 16, 103, 15, 42, 53, 26,
    67, 4, 181, 246, 153, 65, 68, 40, 110, 253, 172,
];

pub fn load_documents_schemas() -> Result<Value, Error> {
    serde_json::from_str(include_str!("../schema/dpns-contract-documents.json"))
}
