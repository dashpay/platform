use dpp::document::{Document, DocumentV0Setters};
use dpp::platform_value::Value;
use dpp::prelude::Identifier;
use dpp::util::hash::hash_double;
use dpp::version::PlatformVersion;

/// The hash properties a DPNS preorder by `owner_id` holds for the name
/// `normalized_label` under the normalized `parent`, as the DPNS system contract
/// of `platform_version` declares them. From DPNS v3 (protocol version 14):
/// `ownerNameHash`, the double SHA-256 of the owner's id, the normalized label,
/// `"."` and the parent, and `nameHash`, the same without the owner; `salt` is
/// not read. Before it: `saltedDomainHash`, of `salt` and
/// `<normalized_label>.<parent>`.
pub fn dpns_preorder_properties(
    owner_id: Identifier,
    salt: &[u8; 32],
    normalized_label: &str,
    parent: &str,
    platform_version: &PlatformVersion,
) -> Vec<(String, Value)> {
    let name = format!("{normalized_label}.{parent}");
    if platform_version.system_data_contracts.dpns >= 3 {
        let mut owner_and_name = owner_id.to_buffer().to_vec();
        owner_and_name.extend(name.as_bytes());
        vec![
            (
                "ownerNameHash".to_string(),
                Value::Bytes32(hash_double(owner_and_name)),
            ),
            (
                "nameHash".to_string(),
                Value::Bytes32(hash_double(name.into_bytes())),
            ),
        ]
    } else {
        let mut preimage = salt.to_vec();
        preimage.extend(name.as_bytes());
        vec![(
            "saltedDomainHash".to_string(),
            Value::Bytes32(hash_double(preimage)),
        )]
    }
}

/// Sets [`dpns_preorder_properties`] on `preorder`.
pub fn set_dpns_preorder_hashes(
    preorder: &mut Document,
    owner_id: Identifier,
    salt: &[u8; 32],
    normalized_label: &str,
    parent: &str,
    platform_version: &PlatformVersion,
) {
    for (property, value) in
        dpns_preorder_properties(owner_id, salt, normalized_label, parent, platform_version)
    {
        preorder.set(&property, value);
    }
}
