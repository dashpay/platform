mod for_insert_contract_group;
mod for_insert_contract_group_memberships;

/// The estimated serialized size of a backwards reference from a member contract to a forward
/// entry: an `UpstreamRootHeightReference` keeping the root tree key and appending up to five
/// path segments, two of them 32 byte identifiers, plus the max hop and flags.
pub(crate) const CONTRACT_GROUP_BACKWARDS_REFERENCE_SIZE: u32 = 128;

/// The estimated serialized size of a stored `ContractGroupInfo`: an owner with admins at the
/// admin cap and a name and a description at their caps stays under this.
pub(crate) const CONTRACT_GROUP_INFO_ESTIMATED_SIZE: u32 = 1024;

/// The estimated key size of a document type name inside the contract group trees.
pub(crate) const DOCUMENT_TYPE_NAME_ESTIMATED_KEY_SIZE: u8 = 16;
