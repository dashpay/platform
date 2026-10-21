mod error;
pub mod v1;
/// Version 2 (protocol version 14): the `claim` document's `distributionType` admits the value 2
/// (OncePerIdentity), written for once-per-identity distribution claims.
pub mod v2;
/// Version 3 (protocol version 15): the `orderPlaced`, `orderFilled` and `orderCancelled` document
/// types record the token marketplace's events for a token that keeps transfer history.
pub mod v3;

pub use crate::error::Error;
use platform_value::{Identifier, IdentifierBytes32};
use platform_version::version::PlatformVersion;
use serde_json::Value;

pub const ID_BYTES: [u8; 32] = [
    45, 67, 89, 21, 34, 216, 145, 78, 156, 243, 17, 58, 202, 190, 13, 92, 61, 40, 122, 201, 84, 99,
    187, 110, 233, 128, 63, 48, 172, 29, 210, 108,
];

pub const OWNER_ID_BYTES: [u8; 32] = [0; 32];

pub const ID: Identifier = Identifier(IdentifierBytes32(ID_BYTES));
pub const OWNER_ID: Identifier = Identifier(IdentifierBytes32(OWNER_ID_BYTES));
pub fn load_definitions(platform_version: &PlatformVersion) -> Result<Option<Value>, Error> {
    match platform_version.system_data_contracts.token_history {
        1..=3 => Ok(None),
        version => Err(Error::UnknownVersionMismatch {
            method: "token_history_contract::load_definitions".to_string(),
            known_versions: vec![1, 2, 3],
            received: version,
        }),
    }
}
pub fn load_documents_schemas(platform_version: &PlatformVersion) -> Result<Value, Error> {
    match platform_version.system_data_contracts.token_history {
        1 => v1::load_documents_schemas(),
        2 => v2::load_documents_schemas(),
        3 => v3::load_documents_schemas(),
        version => Err(Error::UnknownVersionMismatch {
            method: "token_history_contract::load_documents_schemas".to_string(),
            known_versions: vec![1, 2, 3],
            received: version,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// v3 is v2 with the marketplace's three document types added: every v2 type is unchanged.
    #[test]
    fn v3_adds_the_order_types_to_v2_and_changes_nothing_else() {
        let v2 = v2::load_documents_schemas().expect("expected the v2 schemas");
        let mut v3 = v3::load_documents_schemas().expect("expected the v3 schemas");
        let v3_types = v3.as_object_mut().expect("expected the v3 schemas to be an object");
        for name in ["orderPlaced", "orderFilled", "orderCancelled"] {
            assert!(
                v3_types.remove(name).is_some(),
                "expected v3 to have the {name} type"
            );
        }
        assert_eq!(serde_json::Value::Object(v3_types.clone()), v2);
    }

    #[test]
    fn protocol_version_15_loads_v3() {
        let platform_version = PlatformVersion::get(15).expect("expected protocol version 15");
        let schemas =
            load_documents_schemas(platform_version).expect("expected the v3 schemas to load");
        assert!(schemas.get("orderFilled").is_some());
        let platform_version = PlatformVersion::get(14).expect("expected protocol version 14");
        let schemas =
            load_documents_schemas(platform_version).expect("expected the v2 schemas to load");
        assert!(schemas.get("orderFilled").is_none());
    }
}
