use crate::drive::contract::DataContractFetchInfo;
use crate::drive::identity::contract_info::keys::IdentityDataContractKeyApplyInfo::{
    ContractBased, ContractGroupBased,
};
use crate::drive::identity::contract_info::ContractInfoStructure;
use crate::drive::identity::IdentityRootStructure;
use crate::drive::{Drive, RootTree};
use crate::error::identity::IdentityError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::epoch::Epoch;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::fee::fee_result::FeeResult;
use dpp::identifier::Identifier;
use dpp::identity::contract_bounds::ContractBounds;
use dpp::identity::{KeyID, Purpose};
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::{GroveOp, KeyInfoPath, QualifiedGroveDbOp};
use grovedb::reference_path::ReferencePathType;
use grovedb::{Element, TransactionArg};
use integer_encoding::VarInt;
use platform_version::version::PlatformVersion;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

mod add_potential_contract_info_for_contract_bounded_key;
mod refresh_potential_contract_info_key_references;

/// Coalesces the current-key alias writes of contract-info purpose subtrees in one batch.
///
/// Every contract-bound authentication key covering a contract (or a contract document type)
/// writes the alias at the empty key of the AUTHENTICATION purpose subtree, and disabling such a
/// key refreshes it. An identity update can therefore queue several operations for one slot: two
/// registered keys, or a registration and a revocation built by separate operation builders.
/// GroveDB rejects two operations on one slot under batching consistency verification, and would
/// otherwise apply whichever came last. Keep exactly one per slot: an insertion beats a refresh
/// (the insertion rewrites the element and its hash), the insertion naming the highest key id wins
/// so the newest key is current regardless of input order, and duplicate refreshes collapse.
pub(crate) fn coalesce_current_key_alias_operations(operations: &mut Vec<LowLevelDriveOperation>) {
    let mut winners: HashMap<&KeyInfoPath, (usize, Option<KeyID>)> = HashMap::new();
    let mut alias_indices = Vec::new();
    for (index, operation) in operations.iter().enumerate() {
        let Some((path, key_id)) = current_key_alias_write(operation) else {
            continue;
        };
        alias_indices.push(index);
        let replaces = match winners.get(path) {
            None => true,
            Some((_, current)) => match (current, key_id) {
                (None, Some(_)) => true,
                (Some(current_id), Some(new_id)) => new_id > *current_id,
                (Some(_), None) | (None, None) => false,
            },
        };
        if replaces {
            winners.insert(path, (index, key_id));
        }
    }
    if alias_indices.len() == winners.len() {
        return;
    }
    let kept: HashSet<usize> = winners.into_values().map(|(index, _)| index).collect();
    let dropped: HashSet<usize> = alias_indices
        .into_iter()
        .filter(|index| !kept.contains(index))
        .collect();
    let mut index = 0;
    operations.retain(|_| {
        let keep = !dropped.contains(&index);
        index += 1;
        keep
    });
}

/// Recognizes a bound authentication key's current-key alias write: a sibling reference inserted
/// at, or refreshed at, the empty key of an AUTHENTICATION contract-info purpose subtree. Returns
/// the subtree path and, for an insertion, the key id the alias names.
///
/// Only that subtree is recognized. Encryption and decryption bounds keep their frozen v0 layout,
/// with the alias one level up at the keys level, and are never coalesced.
fn current_key_alias_write(
    operation: &LowLevelDriveOperation,
) -> Option<(&KeyInfoPath, Option<KeyID>)> {
    let LowLevelDriveOperation::GroveOperation(QualifiedGroveDbOp {
        path,
        key: Some(KeyInfo::KnownKey(key)),
        op,
    }) = operation
    else {
        return None;
    };
    if !key.is_empty() || !is_bound_authentication_keys_path(path) {
        return None;
    }
    match op {
        GroveOp::InsertOrReplace {
            element: Element::Reference(ReferencePathType::SiblingReference(sibling), _, _),
        }
        | GroveOp::InsertOrReplaceDontCheckForBackwardsReferences {
            element: Element::Reference(ReferencePathType::SiblingReference(sibling), _, _),
        } => KeyID::decode_var(sibling).map(|(key_id, _)| (path, Some(key_id))),
        GroveOp::RefreshReference {
            reference_path_type: ReferencePathType::SiblingReference(_),
            ..
        } => Some((path, None)),
        _ => None,
    }
}

/// Whether `path` is the AUTHENTICATION purpose subtree of an identity's contract-info group:
/// `Identities / <identity id> / IdentityContractInfo / <contract id or contract id and document
/// type name> / ContractInfoKeysKey / AUTHENTICATION`.
fn is_bound_authentication_keys_path(path: &KeyInfoPath) -> bool {
    let segments = path.to_path_refs();
    segments.len() == 6
        && segments[0] == [RootTree::Identities as u8]
        && segments[1].len() == 32
        && segments[2] == [IdentityRootStructure::IdentityContractInfo as u8]
        && segments[4] == [ContractInfoStructure::ContractInfoKeysKey as u8]
        && segments[5] == [Purpose::AUTHENTICATION as u8]
}

pub enum IdentityDataContractKeyApplyInfo {
    /// The root_id is either a contract id or an owner id
    /// It is a contract id for in the case of contract bound keys or contract
    /// document bound keys
    ContractBased {
        contract_id: Identifier,
        document_type_keys: BTreeMap<String, Vec<(KeyID, Purpose)>>,
        contract_keys: Vec<(KeyID, Purpose)>,
    },
    /// Keys bound to a contract group. The group id is the contract-info group key, next to
    /// the contract ids: same level, same alias machinery, distinct hash domain.
    ContractGroupBased {
        contract_group_id: Identifier,
        keys: Vec<(KeyID, Purpose)>,
    },
}

impl IdentityDataContractKeyApplyInfo {
    fn root_id(&self) -> [u8; 32] {
        match self {
            ContractBased { contract_id, .. } => contract_id.to_buffer(),
            ContractGroupBased {
                contract_group_id, ..
            } => contract_group_id.to_buffer(),
        }
    }

    /// Resolves what the bounds point at, billing the read: the contract for a contract bound
    /// (returned, since the storage rule comes from its config), or nothing for a contract group
    /// bound (the group must exist; its keys use a fixed storage rule). Runs in estimation mode
    /// as well, so the estimate bills the same read the apply path does.
    fn fetch_bound_root_with_fee(
        &self,
        drive: &Drive,
        epoch: &Epoch,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(FeeResult, Option<Arc<DataContractFetchInfo>>), Error> {
        match self {
            ContractBased { contract_id, .. } => {
                let (fee, contract) = drive.get_contract_with_fetch_info_and_fee(
                    contract_id.to_buffer(),
                    Some(epoch),
                    true,
                    transaction,
                    platform_version,
                )?;
                let fee = fee.ok_or(Error::Identity(
                    IdentityError::IdentityKeyDataContractNotFound,
                ))?;
                let contract = contract.ok_or(Error::Identity(
                    IdentityError::IdentityKeyDataContractNotFound,
                ))?;
                Ok((fee, Some(contract)))
            }
            ContractGroupBased {
                contract_group_id, ..
            } => {
                let (fee, info) = drive.fetch_contract_group_info_with_fee(
                    *contract_group_id,
                    epoch,
                    transaction,
                    platform_version,
                )?;
                if info.is_none() {
                    return Err(Error::Identity(IdentityError::IdentityKeyBoundsError(
                        "Contract group for key bounds not found",
                    )));
                }
                Ok((fee, None))
            }
        }
    }

    // TODO: Use type alias or struct
    #[allow(clippy::type_complexity)]
    fn keys(
        self,
    ) -> (
        BTreeMap<String, Vec<(KeyID, Purpose)>>,
        Vec<(KeyID, Purpose)>,
    ) {
        match self {
            ContractBased {
                document_type_keys,
                contract_keys,
                ..
            } => (document_type_keys, contract_keys),
            ContractGroupBased { keys, .. } => (BTreeMap::new(), keys),
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn new_from_single_key(
        key_id: KeyID,
        purpose: Purpose,
        contract_bounds: &ContractBounds,
        drive: &Drive,
        epoch: &Epoch,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        let (contract_id, document_type_name) = match contract_bounds {
            ContractBounds::ContractGroup {
                id: contract_group_id,
            } => {
                // Only authentication keys may be bound to a group (consensus refuses the
                // rest). Nothing of the group is needed to build the apply info, so it is not
                // read here: `fetch_bound_root_with_fee` checks that the group exists and
                // bills that one read when the references are written.
                if purpose != Purpose::AUTHENTICATION {
                    return Err(Error::Identity(IdentityError::IdentityKeyBoundsError(
                        "only authentication keys can be bound to a contract group",
                    )));
                }
                return Ok(ContractGroupBased {
                    contract_group_id: *contract_group_id,
                    keys: vec![(key_id, purpose)],
                });
            }
            ContractBounds::SingleContract { id } => (id.to_buffer(), None),
            ContractBounds::SingleContractDocumentType {
                id,
                document_type_name,
            } => (id.to_buffer(), Some(document_type_name)),
        };
        // we are getting with fetch info to add the cost to the drive operations
        let maybe_contract_fetch_info = drive.get_contract_with_fetch_info_and_add_to_operations(
            contract_id,
            Some(epoch),
            false,
            transaction,
            drive_operations,
            platform_version,
        )?;
        let Some(contract_fetch_info) = maybe_contract_fetch_info else {
            return Err(Error::Identity(IdentityError::IdentityKeyBoundsError(
                "Contract for key bounds not found",
            )));
        };
        let contract = &contract_fetch_info.contract;
        match document_type_name {
            None => Ok(ContractBased {
                contract_id: contract.id(),
                document_type_keys: Default::default(),
                contract_keys: vec![(key_id, purpose)],
            }),
            Some(document_type_name) => {
                let document_type = contract.document_type_for_name(document_type_name)?;
                Ok(ContractBased {
                    contract_id: contract.id(),
                    document_type_keys: BTreeMap::from([(
                        document_type.name().clone(),
                        vec![(key_id, purpose)],
                    )]),
                    contract_keys: vec![],
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::identity::{
        identity_contract_info_group_keys_path_vec,
        identity_contract_info_group_path_key_purpose_vec,
    };
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use grovedb::reference_path::ReferencePathType::SiblingReference;

    /// The group of a group bound is read, checked and billed once, by
    /// `fetch_bound_root_with_fee` when the references are written. Building the apply info
    /// must not read it a second time.
    #[test]
    fn should_not_read_the_contract_group_when_building_the_apply_info() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(None);
        let contract_group_id = Identifier::from([0x61; 32]);
        let mut drive_operations = vec![];
        let apply_info = IdentityDataContractKeyApplyInfo::new_from_single_key(
            7,
            Purpose::AUTHENTICATION,
            &ContractBounds::ContractGroup {
                id: contract_group_id,
            },
            &drive,
            &Epoch::new(0).expect("expected epoch 0"),
            None,
            &mut drive_operations,
            platform_version,
        )
        .expect("expected the apply info of a group bound");
        assert!(drive_operations.is_empty(), "{drive_operations:?}");
        assert_eq!(apply_info.root_id(), contract_group_id.to_buffer());
    }

    fn alias_insert(path: Vec<Vec<u8>>, key_id: KeyID) -> LowLevelDriveOperation {
        LowLevelDriveOperation::insert_for_known_path_key_element(
            path,
            vec![],
            Element::Reference(SiblingReference(key_id.encode_var_vec()), Some(2), None),
        )
    }

    fn alias_refresh(path: Vec<Vec<u8>>, key_id: KeyID) -> LowLevelDriveOperation {
        LowLevelDriveOperation::refresh_reference_for_known_path_key_reference_info(
            path,
            vec![],
            SiblingReference(key_id.encode_var_vec()),
            Some(2),
            None,
            false,
        )
    }

    fn key_id_entry(path: Vec<Vec<u8>>, key_id: KeyID) -> LowLevelDriveOperation {
        LowLevelDriveOperation::insert_for_known_path_key_element(
            path,
            key_id.encode_var_vec(),
            Element::Reference(SiblingReference(key_id.encode_var_vec()), Some(2), None),
        )
    }

    #[test]
    fn should_coalesce_only_bound_authentication_alias_writes() {
        let identity_id = [1u8; 32];
        let contract_id = [2u8; 32];
        let document_type_group = [contract_id.as_slice(), b"preorder"].concat();

        let contract_auth_path = identity_contract_info_group_path_key_purpose_vec(
            &identity_id,
            &contract_id,
            Purpose::AUTHENTICATION,
        );
        let document_type_auth_path = identity_contract_info_group_path_key_purpose_vec(
            &identity_id,
            &document_type_group,
            Purpose::AUTHENTICATION,
        );
        // Legacy encryption and decryption bounds alias the current key at the keys level.
        let legacy_alias_path =
            identity_contract_info_group_keys_path_vec(&identity_id, &contract_id);
        // A purpose-level alias for another purpose must not be recognized either.
        let encryption_purpose_path = identity_contract_info_group_path_key_purpose_vec(
            &identity_id,
            &contract_id,
            Purpose::ENCRYPTION,
        );

        let mut operations = vec![
            alias_refresh(contract_auth_path.clone(), 1),
            alias_insert(contract_auth_path.clone(), 2),
            alias_insert(legacy_alias_path.clone(), 5),
            alias_insert(document_type_auth_path.clone(), 9),
            alias_insert(contract_auth_path.clone(), 3),
            alias_insert(legacy_alias_path.clone(), 6),
            alias_refresh(encryption_purpose_path.clone(), 7),
            alias_insert(encryption_purpose_path.clone(), 8),
            alias_insert(document_type_auth_path.clone(), 10),
            alias_refresh(contract_auth_path.clone(), 4),
        ];

        coalesce_current_key_alias_operations(&mut operations);

        assert_eq!(
            operations,
            vec![
                alias_insert(legacy_alias_path.clone(), 5),
                alias_insert(contract_auth_path, 3),
                alias_insert(legacy_alias_path, 6),
                alias_refresh(encryption_purpose_path.clone(), 7),
                alias_insert(encryption_purpose_path, 8),
                alias_insert(document_type_auth_path, 10),
            ],
            "each AUTHENTICATION slot keeps the insertion naming its highest key id, in place; \
             legacy and other-purpose aliases are untouched"
        );
    }

    #[test]
    fn should_collapse_duplicate_authentication_alias_refreshes_to_one() {
        let identity_id = [1u8; 32];
        let contract_id = [2u8; 32];
        let contract_auth_path = identity_contract_info_group_path_key_purpose_vec(
            &identity_id,
            &contract_id,
            Purpose::AUTHENTICATION,
        );

        let mut operations = vec![
            alias_refresh(contract_auth_path.clone(), 3),
            alias_refresh(contract_auth_path.clone(), 3),
        ];

        coalesce_current_key_alias_operations(&mut operations);

        assert_eq!(operations, vec![alias_refresh(contract_auth_path, 3)]);
    }

    #[test]
    fn should_leave_key_id_entries_in_the_authentication_subtree_alone() {
        let identity_id = [1u8; 32];
        let contract_id = [2u8; 32];
        let contract_auth_path = identity_contract_info_group_path_key_purpose_vec(
            &identity_id,
            &contract_id,
            Purpose::AUTHENTICATION,
        );

        let mut operations = vec![
            key_id_entry(contract_auth_path.clone(), 2),
            key_id_entry(contract_auth_path.clone(), 3),
        ];

        coalesce_current_key_alias_operations(&mut operations);

        assert_eq!(
            operations,
            vec![
                key_id_entry(contract_auth_path.clone(), 2),
                key_id_entry(contract_auth_path, 3),
            ],
            "only the empty-key alias slot coalesces"
        );
    }
}
