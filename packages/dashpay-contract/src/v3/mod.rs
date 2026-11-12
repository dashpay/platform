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
//
// It adds one property:
//
// - `contactRequest.encryptedShieldedAddress`, optional and last in position: the
//   sender's shielded payment address for this contact (43 bytes), encrypted as
//   `encryptedPublicKey` is (`encryptedFor`). That is an IV and three AES blocks,
//   64 bytes; the 80-byte length is kept for a later encoding. A contact request
//   stored before the property existed ends before it and reads back without it.
//
// And it adds one type:
//
// - `privateContactRequest`, a private type of 589-byte entries: a contact request
//   encrypted to the recipient and appended by `PrivateDocumentCreateFromShieldedPool`
//   without an identity. Its properties describe the plaintext the recipient decrypts.
//   At their largest they take 290 bytes (`accountReference` is an i64, as DashPay
//   stores every integer), which with the envelope's 299 bytes fill an entry.

pub fn load_documents_schemas() -> Result<Value, Error> {
    serde_json::from_str(include_str!("../../schema/v3/dashpay.schema.json"))
        .map_err(Error::InvalidSchemaJson)
}
