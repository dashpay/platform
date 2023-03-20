use serde_json::Error;
use serde_json::Value;

pub const ID_BYTES: [u8; 32] = [
    54, 98, 187, 97, 225, 127, 174, 62, 162, 148, 207, 96, 49, 151, 251, 10, 171, 109, 81, 24, 11,
    216, 182, 16, 76, 73, 68, 166, 47, 226, 217, 127,
];

pub const OWNER_ID_BYTES: [u8; 32] = [
    170, 138, 235, 213, 173, 122, 202, 36, 243, 48, 61, 185, 146, 50, 146, 255, 194, 133, 221, 176,
    188, 82, 144, 69, 234, 198, 106, 35, 245, 167, 46, 192,
];

pub fn load_documents_schemas() -> Result<Value, Error> {
    serde_json::from_str(include_str!("../schema/withdrawals-documents.json"))
}
