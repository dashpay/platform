use crate::error::Error;
use serde_json::Value;

// Document-type name and property constants live in `crate::v1::document_types`;
// v3 keeps every name and property v2 declares, stored as v2 stores it, and adds
// declarations only:
//
// - `contactRequest.toUserId` refers to an identity and must differ from the
//   writer (`refersTo` identity, `distinctFrom` `$ownerId`), the two checks the
//   contact request data trigger made up to protocol version 14.
// - `contactRequest.encryptedPublicKey` and `encryptedAccountLabel` name their
//   encryption (`encryptedFor`, ECDH between the sender's `senderKeyIndex` key and
//   the recipient's `recipientKeyIndex` key, then AES-256-CBC with the IV first),
//   so each must be an IV plus whole AES blocks. Both key indexes gain the u32
//   `maximum` that `encryptedFor` requires of a key id; DashPay keeps sized integer
//   types off, so they are still stored as i64.
// - `profile`'s `corePaymentAddressType` and `platformPaymentAddressType` rules
//   hold the first byte of each DIP-33 payment address to the address type, the
//   check the profile data trigger made up to protocol version 14. Their descriptions say
//   so in place of v2's mention of the data trigger.

pub fn load_documents_schemas() -> Result<Value, Error> {
    serde_json::from_str(include_str!("../../schema/v3/dashpay.schema.json"))
        .map_err(Error::InvalidSchemaJson)
}
