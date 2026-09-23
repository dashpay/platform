use crate::error::Error;
use serde_json::Value;

// Document-type name and property constants live in `crate::v1::document_types`;
// v2 does not change any names v1 defined. It adds the optional
// `corePaymentAddress`, `platformPaymentAddress`, and `shieldedAddress`
// properties to `profile`, and declares on `contactRequest` what consensus
// checks (`toUserId`: `distinctFrom` and an `identityPublicKey` `refersTo`;
// `encryptedPublicKey` / `encryptedAccountLabel`: `encryptedFor`).
//
// v2 replaces v1 in place at protocol version 14 without the contract update
// checks, and the contract keeps `sizedIntegerTypes` off, so every property v1
// declares must keep its stored encoding: a key reference on the key id
// property itself (`identityProperty`) would store it as a u32 instead of an
// i64 and misread every stored document.
//
// The schema carries no descriptions for the three profile addresses, to keep
// the stored contract small. Their formats:
// - `corePaymentAddress` / `platformPaymentAddress`: 21 bytes, a type byte
//   (0x00 P2PKH, 0x01 P2SH, enforced by the profile data trigger) followed by
//   the 20-byte HASH160. Clients render the Core one as Base58Check for their
//   network. Payments to either are publicly linkable to the profile.
// - `shieldedAddress`: 43 bytes, a raw Orchard address (11-byte diversifier
//   then the 32-byte diversified transmission key). Only the length is checked
//   on chain; clients validate it before paying.

pub fn load_documents_schemas() -> Result<Value, Error> {
    serde_json::from_str(include_str!("../../schema/v2/dashpay.schema.json"))
        .map_err(Error::InvalidSchemaJson)
}
