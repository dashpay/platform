//! Doctype-level keyword names that are already taken by stray keys.
//!
//! The document meta-schema v0 admitted every contract created at protocol
//! versions 1 to 11 and does not refuse unknown doctype-level keys, so such a
//! contract may carry a key no validator ever looked at. The rule for reading
//! keywords off those contracts is stated on `try_from_schema_generation_3`.
//! What stays open is the name of a future keyword: one named after a stray
//! key would change the meaning of the contracts that carry it, from the block
//! that activates the keyword.

use serde_json::Value as JsonValue;
use std::fs;
use std::path::Path;

/// Every doctype-level key that no closed meta-schema declares, carried by a
/// contract admitted under meta-schema v0. From a census of every contract
/// create and update transition on each network (2026-09-20), decoded from the
/// raw bytes. Mainnet: 54 admitted contract versions below height 398435,
/// where protocol version 12 activated. Testnet: 3347 below height 362782.
/// None of them carries a generation 3 keyword.
///
/// The lists are exhaustive and final. Neither network has a stray on a
/// contract admitted later, and the set can no longer grow: every create and
/// update since protocol version 12 is validated against a meta-schema that
/// refuses unknown keys. So a new doctype-level keyword is safe exactly when
/// its name is absent from these lists, and the census never needs repeating.
const MAINNET_STRAY_DOCTYPE_KEYS: &[&str] = &["mutable"];
const TESTNET_STRAY_DOCTYPE_KEYS: &[&str] = &[
    "mutable",
    "comment",
    "position",
    "tokenCosts",
    "indexes",
    "bls_public_key",
    "keywords",
];

/// The meta-schema that leaves the doctype level open. It declares `keywords`,
/// which no later meta-schema does, so it is not checked against the lists.
const OPEN_DOCUMENT_META_SCHEMA_VERSION: &str = "v0";

/// Pick another name, or decide what happens to the contracts that carry the
/// stray key first.
#[test]
fn should_not_name_a_doctype_keyword_after_a_stray_key_of_a_meta_schema_v0_contract() {
    let meta_schemas = Path::new(env!("CARGO_MANIFEST_DIR")).join("schema/meta_schemas/document");

    let mut checked = 0;
    for entry in fs::read_dir(&meta_schemas).expect("the document meta-schema directory exists") {
        let directory = entry.expect("a readable directory entry").path();
        let version = directory
            .file_name()
            .and_then(|name| name.to_str())
            .expect("a meta-schema version directory name")
            .to_owned();
        if version == OPEN_DOCUMENT_META_SCHEMA_VERSION {
            continue;
        }

        let source = fs::read_to_string(directory.join("document-meta.json"))
            .unwrap_or_else(|e| panic!("document meta-schema {version} must be readable: {e}"));
        let meta_schema: JsonValue =
            serde_json::from_str(&source).expect("document meta-schema JSON must be valid");
        let keywords = meta_schema["properties"]
            .as_object()
            .expect("the document meta-schema declares its doctype-level keywords");

        for stray in MAINNET_STRAY_DOCTYPE_KEYS
            .iter()
            .chain(TESTNET_STRAY_DOCTYPE_KEYS)
        {
            assert!(
                !keywords.contains_key(*stray),
                "document meta-schema {version} declares `{stray}`, which contracts admitted \
                 under meta-schema v0 already carry as a stray key"
            );
        }
        checked += 1;
    }

    assert!(
        checked >= 3,
        "expected the closed document meta-schemas v1 to v3 at least, found {checked}"
    );
}
