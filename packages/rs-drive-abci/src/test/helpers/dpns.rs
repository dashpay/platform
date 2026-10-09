use dpp::prelude::Identifier;
use dpp::util::hash::hash_double;
use dpp::version::PlatformVersion;

/// The `saltedDomainHash` a DPNS preorder by `owner_id` commits to for the name
/// `normalized_label` under `parent`, as the DPNS system contract of
/// `platform_version` reveals it: from DPNS v3 (protocol version 14) the double
/// SHA-256 of the owner's id, the salt, the normalized label, `"."` and the
/// parent; before it, of the salt and `<normalized_label>.<parent>`.
pub fn dpns_salted_domain_hash(
    owner_id: Identifier,
    salt: &[u8; 32],
    normalized_label: &str,
    parent: &str,
    platform_version: &PlatformVersion,
) -> [u8; 32] {
    let mut preimage = vec![];
    if platform_version.system_data_contracts.dpns >= 3 {
        preimage.extend(owner_id.to_buffer());
    }
    preimage.extend(salt);
    preimage.extend(format!("{normalized_label}.{parent}").as_bytes());
    hash_double(preimage)
}
