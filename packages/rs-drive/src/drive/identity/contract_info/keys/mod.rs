use crate::drive::identity::contract_info::keys::IdentityDataContractKeyApplyInfo::ContractBased;
use crate::drive::Drive;
use crate::error::identity::IdentityError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::epoch::Epoch;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
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

/// Recognizes a current-key alias write: a sibling reference inserted at, or refreshed at, the
/// empty key. Returns the subtree path and, for an insertion, the key id the alias names.
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
    if !key.is_empty() {
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

pub enum IdentityDataContractKeyApplyInfo {
    /// The root_id is either a contract id or an owner id
    /// It is a contract id for in the case of contract bound keys or contract
    /// document bound keys
    ContractBased {
        contract_id: Identifier,
        document_type_keys: BTreeMap<String, Vec<(KeyID, Purpose)>>,
        contract_keys: Vec<(KeyID, Purpose)>,
    },
    // ContractFamilyBased {
    //     contracts_owner_id: Identifier,
    //     family_keys: Vec<KeyID>,
    // },
}

impl IdentityDataContractKeyApplyInfo {
    fn root_id(&self) -> [u8; 32] {
        match self {
            ContractBased { contract_id, .. } => contract_id.to_buffer(),
            // ContractFamilyBased {
            //     contracts_owner_id, ..
            // } => contracts_owner_id.to_buffer(),
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
            // ContractFamilyBased { family_keys, .. } => (BTreeMap::new(), family_keys),
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
        let contract_id = contract_bounds.identifier().to_buffer();
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
        match contract_bounds {
            ContractBounds::SingleContract { .. } => Ok(ContractBased {
                contract_id: contract.id(),
                document_type_keys: Default::default(),
                contract_keys: vec![(key_id, purpose)],
            }),
            ContractBounds::SingleContractDocumentType {
                document_type_name: document_type,
                ..
            } => {
                let document_type = contract.document_type_for_name(document_type)?;
                Ok(ContractBased {
                    contract_id: contract.id(),
                    document_type_keys: BTreeMap::from([(
                        document_type.name().clone(),
                        vec![(key_id, purpose)],
                    )]),
                    contract_keys: vec![],
                })
            } // ContractBounds::MultipleContractsOfSameOwner { .. } => Ok(ContractFamilyBased {
              //     contracts_owner_id: contract.owner_id(),
              //     family_keys: vec![key_id],
              // }),
        }
    }
}
