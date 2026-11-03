mod address_funds;
mod contract;
mod contract_fee_pot;
mod contract_group;
mod contract_moderation;
mod document;
mod drive_methods;
pub(crate) mod finalize_task;
mod group;
mod identity;
mod prefunded_specialized_balance;
mod shielded;
mod system;
mod token;
mod withdrawals;

use crate::util::batch::GroveDbOpBatch;

use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::Credits;

pub use address_funds::AddressFundsOperationType;
pub use contract::DataContractOperationType;
pub use contract_fee_pot::ContractFeePotOperationType;
pub use contract_group::ContractGroupOperationType;
pub use contract_moderation::ContractModerationOperationType;
pub use document::DocumentOperation;
pub use document::DocumentOperationType;
pub use document::DocumentOperationsForContractDocumentType;
pub use document::UpdateOperationInfo;
pub use group::GroupOperationType;
pub use identity::IdentityOperationType;
pub use prefunded_specialized_balance::PrefundedSpecializedBalanceOperationType;
pub use shielded::ShieldedPoolOperationType;
pub use system::SystemOperationType;
pub use token::TokenOperationType;
pub use withdrawals::WithdrawalOperationType;

use grovedb::{EstimatedLayerInformation, TransactionArg};

use crate::fees::op::LowLevelDriveOperation::GroveOperation;

use dpp::version::PlatformVersion;
use grovedb::batch::{KeyInfoPath, QualifiedGroveDbOp};

use crate::error::drive::DriveError;
use crate::error::fee::FeeError;
use crate::util::batch::drive_op_batch::finalize_task::{
    DriveOperationFinalizationTasks, DriveOperationFinalizeTask,
};
use dpp::data_contract::document_type::action_fees::ContractFeePot;
use dpp::identifier::Identifier;

use std::collections::{BTreeMap, BTreeSet, HashMap};

/// A converter that will get Drive Operations from High Level Operations
pub trait DriveLowLevelOperationConverter {
    /// This will get a list of atomic drive operations from a high level operations
    fn into_low_level_drive_operations(
        self,
        drive: &Drive,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error>;
}

/// The drive operation context keeps track of changes that might affect other operations
/// Notably Identity balance changes are kept track of
pub struct DriveOperationContext {
    #[allow(dead_code)]
    #[deprecated(note = "This function is marked as unused.")]
    #[allow(deprecated)]
    identity_balance_changes: BTreeMap<[u8; 32], i64>,
}

/// All types of Drive Operations
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug)]
pub enum DriveOperation<'a> {
    /// A contract operation
    DataContractOperation(DataContractOperationType<'a>),
    /// A document operation
    DocumentOperation(DocumentOperationType<'a>),
    /// A token operation
    TokenOperation(TokenOperationType),
    /// Withdrawal operation
    WithdrawalOperation(WithdrawalOperationType),
    /// An identity operation
    IdentityOperation(IdentityOperationType),
    /// An operation on prefunded balances
    PrefundedSpecializedBalanceOperation(PrefundedSpecializedBalanceOperationType),
    /// A system operation
    SystemOperation(SystemOperationType),
    /// A group operation
    GroupOperation(GroupOperationType),
    /// A contract group operation
    ContractGroupOperation(ContractGroupOperationType),
    /// A contract moderation operation: an entry of a banlist or a suspension list
    ContractModerationOperation(ContractModerationOperationType),
    /// A contract fee pot operation: credits of a contract's document action fees
    ContractFeePotOperation(ContractFeePotOperationType),
    /// An address funds operation
    AddressFundsOperation(AddressFundsOperationType),
    /// A shielded pool operation
    ShieldedPoolOperation(ShieldedPoolOperationType),
    /// A single low level groveDB operation
    GroveDBOperation(QualifiedGroveDbOp),
    /// Multiple low level groveDB operations
    GroveDBOpBatch(GroveDbOpBatch),
    /// An operation that only produces finalization tasks (no low-level ops)
    FinalizeOperation(DriveOperationFinalizeTask),
}

impl DriveLowLevelOperationConverter for DriveOperation<'_> {
    fn into_low_level_drive_operations(
        self,
        drive: &Drive,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match self {
            DriveOperation::DataContractOperation(contract_operation_type) => {
                contract_operation_type.into_low_level_drive_operations(
                    drive,
                    estimated_costs_only_with_layer_info,
                    block_info,
                    transaction,
                    platform_version,
                )
            }
            DriveOperation::DocumentOperation(document_operation_type) => document_operation_type
                .into_low_level_drive_operations(
                    drive,
                    estimated_costs_only_with_layer_info,
                    block_info,
                    transaction,
                    platform_version,
                ),
            DriveOperation::WithdrawalOperation(withdrawal_operation_type) => {
                withdrawal_operation_type.into_low_level_drive_operations(
                    drive,
                    estimated_costs_only_with_layer_info,
                    block_info,
                    transaction,
                    platform_version,
                )
            }
            DriveOperation::IdentityOperation(identity_operation_type) => identity_operation_type
                .into_low_level_drive_operations(
                    drive,
                    estimated_costs_only_with_layer_info,
                    block_info,
                    transaction,
                    platform_version,
                ),
            DriveOperation::PrefundedSpecializedBalanceOperation(
                prefunded_balance_operation_type,
            ) => prefunded_balance_operation_type.into_low_level_drive_operations(
                drive,
                estimated_costs_only_with_layer_info,
                block_info,
                transaction,
                platform_version,
            ),
            DriveOperation::SystemOperation(system_operation_type) => system_operation_type
                .into_low_level_drive_operations(
                    drive,
                    estimated_costs_only_with_layer_info,
                    block_info,
                    transaction,
                    platform_version,
                ),
            DriveOperation::ShieldedPoolOperation(shielded_pool_operation_type) => {
                shielded_pool_operation_type.into_low_level_drive_operations(
                    drive,
                    estimated_costs_only_with_layer_info,
                    block_info,
                    transaction,
                    platform_version,
                )
            }
            DriveOperation::GroveDBOperation(op) => Ok(vec![GroveOperation(op)]),
            DriveOperation::GroveDBOpBatch(operations) => Ok(operations
                .operations
                .into_iter()
                .map(GroveOperation)
                .collect()),
            DriveOperation::TokenOperation(token_operation_type) => token_operation_type
                .into_low_level_drive_operations(
                    drive,
                    estimated_costs_only_with_layer_info,
                    block_info,
                    transaction,
                    platform_version,
                ),
            DriveOperation::GroupOperation(group_operation_type) => group_operation_type
                .into_low_level_drive_operations(
                    drive,
                    estimated_costs_only_with_layer_info,
                    block_info,
                    transaction,
                    platform_version,
                ),
            DriveOperation::ContractGroupOperation(contract_group_operation_type) => {
                contract_group_operation_type.into_low_level_drive_operations(
                    drive,
                    estimated_costs_only_with_layer_info,
                    block_info,
                    transaction,
                    platform_version,
                )
            }
            DriveOperation::ContractModerationOperation(contract_moderation_operation_type) => {
                contract_moderation_operation_type.into_low_level_drive_operations(
                    drive,
                    estimated_costs_only_with_layer_info,
                    block_info,
                    transaction,
                    platform_version,
                )
            }
            DriveOperation::ContractFeePotOperation(contract_fee_pot_operation_type) => {
                contract_fee_pot_operation_type.into_low_level_drive_operations(
                    drive,
                    estimated_costs_only_with_layer_info,
                    block_info,
                    transaction,
                    platform_version,
                )
            }
            DriveOperation::AddressFundsOperation(address_funds_operation_type) => {
                address_funds_operation_type.into_low_level_drive_operations(
                    drive,
                    estimated_costs_only_with_layer_info,
                    block_info,
                    transaction,
                    platform_version,
                )
            }
            DriveOperation::FinalizeOperation(_) => Ok(vec![]),
        }
    }
}

impl DriveOperation<'_> {
    /// Whether the batch this operation is in refunds nobody for the storage it removes: see
    /// [`ContractModerationOperationType::ForfeitStorageRefunds`].
    pub fn forfeits_storage_refunds(&self) -> bool {
        matches!(
            self,
            Self::ContractModerationOperation(
                ContractModerationOperationType::ForfeitStorageRefunds
            )
        )
    }

    /// Convert a member of a batch whose document TTL cleanup is complete.
    pub(crate) fn into_low_level_drive_operations_after_ttl_drain(
        self,
        drive: &Drive,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match self {
            Self::DocumentOperation(operation) => operation
                .into_low_level_drive_operations_after_ttl_drain(
                    drive,
                    estimated_costs_only_with_layer_info,
                    block_info,
                    transaction,
                    platform_version,
                ),
            operation => operation.into_low_level_drive_operations(
                drive,
                estimated_costs_only_with_layer_info,
                block_info,
                transaction,
                platform_version,
            ),
        }
    }
}

impl Drive {
    /// Prepare every document before conversion starts. Repeated grids may
    /// spend several per-write budgets, all against the same pre-batch state.
    pub(crate) fn prepare_drive_operations_time_range_ttl(
        &self,
        operations: &[DriveOperation],
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        for operation in operations {
            if let DriveOperation::DocumentOperation(document) = operation {
                document.prepare_time_range_ttl(self, block_info, transaction, platform_version)?;
            }
        }
        Ok(())
    }
}

impl DriveOperationFinalizationTasks for DriveOperation<'_> {
    fn finalization_tasks(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Vec<DriveOperationFinalizeTask>>, Error> {
        match platform_version
            .drive
            .methods
            .state_transitions
            .operations
            .finalization_tasks
        {
            0 => self.finalization_tasks_v0(platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "DriveOperation.finalization_tasks".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

impl DriveOperation<'_> {
    fn finalization_tasks_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Vec<DriveOperationFinalizeTask>>, Error> {
        match self {
            DriveOperation::DataContractOperation(o) => o.finalization_tasks(platform_version),
            DriveOperation::FinalizeOperation(task) => Ok(Some(vec![task.clone()])),
            _ => Ok(None),
        }
    }

    /// Sums the credits the batch mints into Platform (its `AddToSystemCredits` operations,
    /// saturating). This is the gross inflow of the batch — the net rule of the daily
    /// withdrawal limit records it per block, and netting against removals here instead would
    /// let a same-block deposit and withdrawal hide the inflow.
    pub fn credit_mints(operations: &[DriveOperation]) -> Credits {
        operations
            .iter()
            .filter_map(|operation| match operation {
                DriveOperation::SystemOperation(SystemOperationType::AddToSystemCredits {
                    amount,
                }) => Some(*amount),
                _ => None,
            })
            .fold(0u64, |total, amount| total.saturating_add(amount))
    }

    /// Merges every write of one identity balance, of one contract fee pot, and of one
    /// prefunded specialized balance, into a single net operation.
    ///
    /// Each of these operations computes the new value from the one committed before its
    /// batch, and GroveDB keeps only the last write of a key, so two of them in one batch lose
    /// the first: a purchase price and a purchase fee leaving the buyer, a contested document's
    /// voting fund and its creation fee, a sale and the action fee of a contract owner who
    /// sponsors the gas. Merged, they apply as if in turn. The merged operation takes the place
    /// of the first one on its key, and a balance or pot the writes leave as it was gets none.
    /// A key written once keeps its operation untouched.
    ///
    /// Two other kinds of write compute from the committed value too and are not merged here,
    /// because no batch repeats their keys: the total system credits
    /// ([`SystemOperationType`]) and address balances ([`AddressFundsOperationType`]). Their
    /// docs say what keeps it so; a change that could repeat one must extend this merge first.
    pub fn merge_balance_writes(operations: Vec<Self>) -> Result<Vec<Self>, Error> {
        // Most batches make at most one such write: nothing to merge, and no map to build.
        if operations
            .iter()
            .filter(|operation| balance_write(operation).is_some())
            .nth(1)
            .is_none()
        {
            return Ok(operations);
        }
        let mut writes: BTreeMap<BalanceKey, (usize, i128)> = BTreeMap::new();
        for (key, change) in operations.iter().filter_map(balance_write) {
            let (count, net) = writes.entry(key).or_default();
            *count += 1;
            *net = net
                .checked_add(change)
                .ok_or(Error::Fee(FeeError::Overflow(
                    "the writes one batch makes to one balance overflow",
                )))?;
        }
        if writes.values().all(|(count, _)| *count == 1) {
            return Ok(operations);
        }
        let mut merged = Vec::with_capacity(operations.len());
        for operation in operations {
            let Some((key, _)) = balance_write(&operation) else {
                merged.push(operation);
                continue;
            };
            match writes.get_mut(&key) {
                Some((1, _)) => merged.push(operation),
                // The first write of a repeated key: the merged one goes here, and the entry
                // is marked done so the later writes are dropped.
                Some((count, net)) if *count > 1 => {
                    merged.extend(net_balance_write(key, *net)?);
                    *count = 0;
                }
                _ => {}
            }
        }
        Ok(merged)
    }

    /// Refuses a batch whose token operations write one identity's balance of a token, or
    /// one token's total supply, more than once.
    ///
    /// These writes compute the new value from the one committed before the batch too, but a
    /// transfer writes two balances and a mint or a burn a balance and the supply, so they
    /// cannot be merged into operations of their own kinds. No state transition makes two of
    /// them on one key: a batch carries one transition, and each writes a key at most once. A
    /// batch that did would lose all but the last write, so it is refused instead.
    pub fn refuse_repeated_token_balance_writes(operations: &[Self]) -> Result<(), Error> {
        let mut written = BTreeSet::new();
        for key in operations.iter().flat_map(token_balance_writes) {
            if !written.insert(key) {
                return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                    "a batch writes one token balance or token supply more than once",
                )));
            }
        }
        Ok(())
    }
}

/// A key an identity balance, a contract fee pot or a prefunded specialized balance operation
/// writes
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum BalanceKey {
    Identity([u8; 32]),
    FeePot(Identifier, ContractFeePot),
    PrefundedSpecializedBalance(Identifier),
}

/// A key a token operation writes: an identity's balance of a token, or a token's total supply
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum TokenBalanceKey {
    Holder(Identifier, Identifier),
    Supply(Identifier),
}

/// The keys `operation` writes if it is a token operation that moves tokens
fn token_balance_writes(operation: &DriveOperation) -> Vec<TokenBalanceKey> {
    let DriveOperation::TokenOperation(operation) = operation else {
        return vec![];
    };
    match operation {
        TokenOperationType::TokenBurn {
            token_id,
            identity_balance_holder_id,
            ..
        }
        | TokenOperationType::TokenMint {
            token_id,
            identity_balance_holder_id,
            ..
        } => vec![
            TokenBalanceKey::Holder(*token_id, *identity_balance_holder_id),
            TokenBalanceKey::Supply(*token_id),
        ],
        TokenOperationType::TokenMintMany {
            token_id,
            recipients,
            ..
        } => recipients
            .iter()
            .map(|(recipient_id, _)| TokenBalanceKey::Holder(*token_id, *recipient_id))
            .chain([TokenBalanceKey::Supply(*token_id)])
            .collect(),
        TokenOperationType::TokenTransfer {
            token_id,
            sender_id,
            recipient_id,
            ..
        } => vec![
            TokenBalanceKey::Holder(*token_id, *sender_id),
            TokenBalanceKey::Holder(*token_id, *recipient_id),
        ],
        _ => vec![],
    }
}

/// The key `operation` writes and the signed change it makes there, if it is an identity
/// balance, contract fee pot or prefunded specialized balance operation
fn balance_write(operation: &DriveOperation) -> Option<(BalanceKey, i128)> {
    match operation {
        DriveOperation::IdentityOperation(IdentityOperationType::AddToIdentityBalance {
            identity_id,
            added_balance,
        }) => Some((BalanceKey::Identity(*identity_id), *added_balance as i128)),
        DriveOperation::IdentityOperation(IdentityOperationType::RemoveFromIdentityBalance {
            identity_id,
            balance_to_remove,
        }) => Some((
            BalanceKey::Identity(*identity_id),
            -(*balance_to_remove as i128),
        )),
        DriveOperation::ContractFeePotOperation(ContractFeePotOperationType::AddToPot {
            contract_id,
            pot,
            amount,
        }) => Some((BalanceKey::FeePot(*contract_id, *pot), *amount as i128)),
        DriveOperation::ContractFeePotOperation(ContractFeePotOperationType::DeductFromPot {
            contract_id,
            pot,
            amount,
        }) => Some((BalanceKey::FeePot(*contract_id, *pot), -(*amount as i128))),
        DriveOperation::PrefundedSpecializedBalanceOperation(
            PrefundedSpecializedBalanceOperationType::CreateNewPrefundedBalance {
                prefunded_specialized_balance_id,
                add_balance,
            },
        ) => Some((
            BalanceKey::PrefundedSpecializedBalance(*prefunded_specialized_balance_id),
            *add_balance as i128,
        )),
        DriveOperation::PrefundedSpecializedBalanceOperation(
            PrefundedSpecializedBalanceOperationType::DeductFromPrefundedBalance {
                prefunded_specialized_balance_id,
                remove_balance,
            },
        ) => Some((
            BalanceKey::PrefundedSpecializedBalance(*prefunded_specialized_balance_id),
            -(*remove_balance as i128),
        )),
        _ => None,
    }
}

/// The one operation that makes the signed change `net` at `key`, or none when it is zero
fn net_balance_write<'a>(key: BalanceKey, net: i128) -> Result<Option<DriveOperation<'a>>, Error> {
    if net == 0 {
        return Ok(None);
    }
    let amount = Credits::try_from(net.unsigned_abs()).map_err(|_| {
        Error::Fee(FeeError::Overflow(
            "the merged writes of one balance overflow credits",
        ))
    })?;
    Ok(Some(match (key, net > 0) {
        (BalanceKey::Identity(identity_id), true) => {
            DriveOperation::IdentityOperation(IdentityOperationType::AddToIdentityBalance {
                identity_id,
                added_balance: amount,
            })
        }
        (BalanceKey::Identity(identity_id), false) => {
            DriveOperation::IdentityOperation(IdentityOperationType::RemoveFromIdentityBalance {
                identity_id,
                balance_to_remove: amount,
            })
        }
        (BalanceKey::FeePot(contract_id, pot), true) => {
            DriveOperation::ContractFeePotOperation(ContractFeePotOperationType::AddToPot {
                contract_id,
                pot,
                amount,
            })
        }
        (BalanceKey::FeePot(contract_id, pot), false) => {
            DriveOperation::ContractFeePotOperation(ContractFeePotOperationType::DeductFromPot {
                contract_id,
                pot,
                amount,
            })
        }
        // Adds to the balance, creating it when it does not exist yet.
        (BalanceKey::PrefundedSpecializedBalance(prefunded_specialized_balance_id), true) => {
            DriveOperation::PrefundedSpecializedBalanceOperation(
                PrefundedSpecializedBalanceOperationType::CreateNewPrefundedBalance {
                    prefunded_specialized_balance_id,
                    add_balance: amount,
                },
            )
        }
        (BalanceKey::PrefundedSpecializedBalance(prefunded_specialized_balance_id), false) => {
            DriveOperation::PrefundedSpecializedBalanceOperation(
                PrefundedSpecializedBalanceOperationType::DeductFromPrefundedBalance {
                    prefunded_specialized_balance_id,
                    remove_balance: amount,
                },
            )
        }
    }))
}

#[cfg(feature = "server")]
#[cfg(test)]
mod tests {
    use grovedb::Element;
    use std::borrow::Cow;
    use std::option::Option::None;

    use super::*;

    use crate::util::test_helpers::setup_contract;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::DataContract;
    use dpp::serialization::PlatformSerializableWithPlatformVersion;
    use dpp::tests::json_document::{json_document_to_contract, json_document_to_document};
    use dpp::util::cbor_serializer;
    use rand::Rng;
    use serde_json::json;

    use crate::util::batch::drive_op_batch::document::DocumentOperation::{
        AddOperation, UpdateOperation,
    };
    use crate::util::batch::drive_op_batch::document::DocumentOperationType::MultipleDocumentOperationsForSameContractDocumentType;
    use crate::util::batch::drive_op_batch::document::{
        DocumentOperationsForContractDocumentType, UpdateOperationInfo,
    };
    use crate::util::batch::DataContractOperationType::ApplyContract;
    use crate::util::batch::DocumentOperationType::AddDocument;
    use crate::util::batch::DriveOperation::{DataContractOperation, DocumentOperation};

    use crate::drive::contract::paths::contract_root_path;
    use crate::drive::Drive;
    use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
    use crate::util::object_size_info::{DataContractInfo, DocumentTypeInfo, OwnedDocumentInfo};
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;

    #[test]
    fn test_add_dashpay_documents() {
        let drive: Drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let mut drive_operations = vec![];
        let db_transaction = drive.grove.start_transaction();

        let contract = json_document_to_contract(
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            false,
            platform_version,
        )
        .expect("expected to get contract");

        let _document_type = contract
            .document_type_for_name("contactRequest")
            .expect("expected to get document type");

        drive_operations.push(DataContractOperation(ApplyContract {
            contract: Cow::Borrowed(&contract),
            storage_flags: None,
        }));

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();

        let document_type = contract
            .document_type_for_name("contactRequest")
            .expect("expected to get document type");

        let dashpay_cr_document = json_document_to_document(
            "tests/supporting_files/contract/dashpay/contact-request0.json",
            Some(random_owner_id.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get document");

        drive_operations.push(DocumentOperation(AddDocument {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefInfo((
                    &dashpay_cr_document,
                    StorageFlags::optional_default_as_cow(),
                )),
                owner_id: None,
            },
            contract_info: DataContractInfo::BorrowedDataContract(&contract),
            document_type_info: DocumentTypeInfo::DocumentTypeRef(document_type),
            override_document: false,
        }));

        drive
            .apply_drive_operations(
                drive_operations,
                true,
                &BlockInfo::default(),
                Some(&db_transaction),
                platform_version,
                None,
            )
            .expect("expected to insert contract and document");

        let element = drive
            .grove
            .get(
                &contract_root_path(&contract.id().to_buffer()),
                &[0],
                Some(&db_transaction),
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .expect("expected to get contract back");

        assert_eq!(
            element,
            Element::Item(
                contract
                    .serialize_to_bytes_with_platform_version(platform_version)
                    .expect("expected to serialize contract"),
                None
            )
        );

        let query_value = json!({
            "where": [
            ],
            "limit": 100,
            "orderBy": [
                ["$ownerId", "asc"],
            ]
        });
        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");

        let (docs, _, _) = drive
            .query_documents_cbor_from_contract(
                &contract,
                document_type,
                where_cbor.as_slice(),
                None,
                Some(&db_transaction),
                Some(platform_version.protocol_version),
            )
            .expect("expected to query");
        assert_eq!(docs.len(), 1);
    }

    #[test]
    fn test_add_multiple_dashpay_documents_individually_should_succeed() {
        let drive = setup_drive_with_initial_state_structure(None);

        let platform_version = PlatformVersion::latest();

        let mut drive_operations = vec![];
        let db_transaction = drive.grove.start_transaction();

        let contract = json_document_to_contract(
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            false,
            platform_version,
        )
        .expect("expected to get contract");

        let document_type = contract
            .document_type_for_name("contactRequest")
            .expect("expected to get document type");

        drive_operations.push(DataContractOperation(ApplyContract {
            contract: Cow::Borrowed(&contract),
            storage_flags: None,
        }));
        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();

        let dashpay_cr_document = json_document_to_document(
            "tests/supporting_files/contract/dashpay/contact-request0.json",
            Some(random_owner_id.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get contract");

        drive_operations.push(DocumentOperation(AddDocument {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefInfo((&dashpay_cr_document, None)),
                owner_id: None,
            },
            contract_info: DataContractInfo::BorrowedDataContract(&contract),
            document_type_info: DocumentTypeInfo::DocumentTypeNameAsStr("contactRequest"),
            override_document: false,
        }));

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();

        let dashpay_cr_1_document = json_document_to_document(
            "tests/supporting_files/contract/dashpay/contact-request1.json",
            Some(random_owner_id.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get contract");

        drive_operations.push(DocumentOperation(AddDocument {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefInfo((&dashpay_cr_1_document, None)),
                owner_id: None,
            },
            contract_info: DataContractInfo::BorrowedDataContract(&contract),
            document_type_info: DocumentTypeInfo::DocumentTypeNameAsStr("contactRequest"),
            override_document: false,
        }));

        drive
            .apply_drive_operations(
                drive_operations,
                true,
                &BlockInfo::default(),
                Some(&db_transaction),
                platform_version,
                None,
            )
            .expect("expected to be able to insert documents");

        let query_value = json!({
            "where": [
            ],
            "limit": 100,
            "orderBy": [
                ["$ownerId", "asc"],
            ]
        });
        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");

        let (docs, _, _) = drive
            .query_documents_cbor_from_contract(
                &contract,
                document_type,
                where_cbor.as_slice(),
                None,
                Some(&db_transaction),
                Some(platform_version.protocol_version),
            )
            .expect("expected to query");
        assert_eq!(docs.len(), 2);
    }

    #[test]
    fn test_add_multiple_dashpay_documents() {
        let drive: Drive = setup_drive_with_initial_state_structure(None);

        let platform_version = PlatformVersion::latest();

        let mut drive_operations = vec![];
        let db_transaction = drive.grove.start_transaction();

        let contract = json_document_to_contract(
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            false,
            platform_version,
        )
        .expect("expected to get contract");

        let document_type = contract
            .document_type_for_name("contactRequest")
            .expect("expected to get document type");

        drive_operations.push(DataContractOperation(ApplyContract {
            contract: Cow::Borrowed(&contract),
            storage_flags: None,
        }));

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();

        let document0 = json_document_to_document(
            "tests/supporting_files/contract/dashpay/contact-request0.json",
            Some(random_owner_id.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get document 0");

        let document1 = json_document_to_document(
            "tests/supporting_files/contract/dashpay/contact-request1.json",
            Some(random_owner_id.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get document 1");

        let operations = vec![
            AddOperation {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((
                        &document0,
                        StorageFlags::optional_default_as_cow(),
                    )),
                    owner_id: Some(random_owner_id),
                },
                override_document: false,
            },
            AddOperation {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((
                        &document1,
                        StorageFlags::optional_default_as_cow(),
                    )),
                    owner_id: Some(random_owner_id),
                },
                override_document: false,
            },
        ];

        drive_operations.push(DocumentOperation(
            MultipleDocumentOperationsForSameContractDocumentType {
                document_operations: DocumentOperationsForContractDocumentType {
                    operations,
                    contract: &contract,
                    document_type,
                },
            },
        ));

        drive
            .apply_drive_operations(
                drive_operations,
                true,
                &BlockInfo::default(),
                Some(&db_transaction),
                platform_version,
                None,
            )
            .expect("expected to be able to insert documents");

        let element = drive
            .grove
            .get(
                &contract_root_path(&contract.id().to_buffer()),
                &[0],
                Some(&db_transaction),
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .expect("expected to get contract back");

        assert_eq!(
            element,
            Element::Item(
                contract
                    .serialize_to_bytes_with_platform_version(platform_version)
                    .expect("expected to serialize contract"),
                None
            )
        );

        let query_value = json!({
            "where": [
            ],
            "limit": 100,
            "orderBy": [
                ["$ownerId", "asc"],
            ]
        });
        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");

        let (docs, _, _) = drive
            .query_documents_cbor_from_contract(
                &contract,
                document_type,
                where_cbor.as_slice(),
                None,
                Some(&db_transaction),
                Some(platform_version.protocol_version),
            )
            .expect("expected to query");
        assert_eq!(docs.len(), 2);
    }

    #[test]
    fn test_add_multiple_family_documents() {
        let drive: Drive = setup_drive_with_initial_state_structure(None);

        let platform_version = PlatformVersion::latest();

        let mut drive_operations = vec![];
        let db_transaction = drive.grove.start_transaction();

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/family/family-contract.json",
            None,
            None,
            None::<fn(&mut DataContract)>,
            Some(&db_transaction),
            None,
        );

        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get document type");

        let random_owner_id0 = rand::thread_rng().gen::<[u8; 32]>();

        let person_document0 = json_document_to_document(
            "tests/supporting_files/contract/family/person0.json",
            Some(random_owner_id0.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get document");

        let random_owner_id1 = rand::thread_rng().gen::<[u8; 32]>();

        let person_document1 = json_document_to_document(
            "tests/supporting_files/contract/family/person3.json",
            Some(random_owner_id1.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get document");

        let mut operations = vec![];

        operations.push(AddOperation {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefInfo((
                    &person_document0,
                    StorageFlags::optional_default_as_cow(),
                )),
                owner_id: Some(random_owner_id0),
            },
            override_document: false,
        });

        let random_owner_id1 = rand::thread_rng().gen::<[u8; 32]>();

        operations.push(AddOperation {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefInfo((
                    &person_document1,
                    StorageFlags::optional_default_as_cow(),
                )),
                owner_id: Some(random_owner_id1),
            },
            override_document: false,
        });

        drive_operations.push(DocumentOperation(
            MultipleDocumentOperationsForSameContractDocumentType {
                document_operations: DocumentOperationsForContractDocumentType {
                    operations,
                    contract: &contract,
                    document_type,
                },
            },
        ));

        drive
            .apply_drive_operations(
                drive_operations,
                true,
                &BlockInfo::default(),
                Some(&db_transaction),
                platform_version,
                None,
            )
            .expect("expected to be able to insert documents");

        let query_value = json!({
            "where": [
            ],
            "limit": 100,
            "orderBy": [
                ["$ownerId", "asc"],
            ]
        });
        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");

        let (docs, _, _) = drive
            .query_documents_cbor_from_contract(
                &contract,
                document_type,
                where_cbor.as_slice(),
                None,
                Some(&db_transaction),
                Some(platform_version.protocol_version),
            )
            .expect("expected to query");
        assert_eq!(docs.len(), 2);
    }

    #[test]
    fn test_update_multiple_family_documents() {
        let drive: Drive = setup_drive_with_initial_state_structure(None);

        let platform_version = PlatformVersion::latest();

        let mut drive_operations = vec![];
        let db_transaction = drive.grove.start_transaction();

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/family/family-contract-only-age-index.json",
            None,
            None,
            None::<fn(&mut DataContract)>,
            Some(&db_transaction),
            None,
        );

        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get document type");

        let random_owner_id0 = rand::thread_rng().gen::<[u8; 32]>();

        let person_document0 = json_document_to_document(
            "tests/supporting_files/contract/family/person0.json",
            Some(random_owner_id0.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get document");

        let random_owner_id1 = rand::thread_rng().gen::<[u8; 32]>();

        let person_document1 = json_document_to_document(
            "tests/supporting_files/contract/family/person3.json",
            Some(random_owner_id1.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get document");

        let operations = vec![
            AddOperation {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((
                        &person_document0,
                        StorageFlags::optional_default_as_cow(),
                    )),
                    owner_id: Some(random_owner_id0),
                },
                override_document: false,
            },
            AddOperation {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((
                        &person_document1,
                        StorageFlags::optional_default_as_cow(),
                    )),
                    owner_id: Some(random_owner_id1),
                },
                override_document: false,
            },
        ];

        drive_operations.push(DocumentOperation(
            MultipleDocumentOperationsForSameContractDocumentType {
                document_operations: DocumentOperationsForContractDocumentType {
                    operations,
                    contract: &contract,
                    document_type,
                },
            },
        ));

        drive
            .apply_drive_operations(
                drive_operations,
                true,
                &BlockInfo::default(),
                Some(&db_transaction),
                platform_version,
                None,
            )
            .expect("expected to be able to insert documents");

        // This was the setup now let's do the update

        drive_operations = vec![];

        let random_owner_id0 = rand::thread_rng().gen::<[u8; 32]>();

        let person_document0 = json_document_to_document(
            "tests/supporting_files/contract/family/person0-older.json",
            Some(random_owner_id0.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get document");

        let random_owner_id1 = rand::thread_rng().gen::<[u8; 32]>();

        let person_document1 = json_document_to_document(
            "tests/supporting_files/contract/family/person3-older.json",
            Some(random_owner_id1.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get document");

        let operations = vec![
            UpdateOperation(UpdateOperationInfo {
                document: &person_document0,
                serialized_document: None,
                owner_id: Some(random_owner_id0),
                storage_flags: None,
            }),
            UpdateOperation(UpdateOperationInfo {
                document: &person_document1,
                serialized_document: None,
                owner_id: Some(random_owner_id1),
                storage_flags: None,
            }),
        ];

        drive_operations.push(DocumentOperation(
            MultipleDocumentOperationsForSameContractDocumentType {
                document_operations: DocumentOperationsForContractDocumentType {
                    operations,
                    contract: &contract,
                    document_type,
                },
            },
        ));

        drive
            .apply_drive_operations(
                drive_operations,
                true,
                &BlockInfo::default(),
                Some(&db_transaction),
                platform_version,
                None,
            )
            .expect("expected to be able to update documents");

        let query_value = json!({
            "where": [
            ],
            "limit": 100,
            "orderBy": [
                ["age", "asc"],
            ]
        });
        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");

        let (docs, _, _) = drive
            .query_documents_cbor_from_contract(
                &contract,
                document_type,
                where_cbor.as_slice(),
                None,
                Some(&db_transaction),
                Some(platform_version.protocol_version),
            )
            .expect("expected to query");
        assert_eq!(docs.len(), 2);

        let query_value = json!({
            "where": [
                ["age", "==", 35]
            ],
            "limit": 100,
            "orderBy": [
                ["age", "asc"],
            ]
        });
        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");

        let (docs, _, _) = drive
            .query_documents_cbor_from_contract(
                &contract,
                document_type,
                where_cbor.as_slice(),
                None,
                Some(&db_transaction),
                Some(platform_version.protocol_version),
            )
            .expect("expected to query");
        assert_eq!(docs.len(), 0);

        let query_value = json!({
            "where": [
                ["age", "==", 36]
            ],
            "limit": 100,
            "orderBy": [
                ["age", "asc"],
            ]
        });
        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");

        let (docs, _, _) = drive
            .query_documents_cbor_from_contract(
                &contract,
                document_type,
                where_cbor.as_slice(),
                None,
                Some(&db_transaction),
                Some(platform_version.protocol_version),
            )
            .expect("expected to query");
        assert_eq!(docs.len(), 2);
    }

    #[test]
    fn test_update_multiple_family_documents_with_index_being_removed_and_added() {
        let drive: Drive = setup_drive_with_initial_state_structure(None);

        let platform_version = PlatformVersion::latest();

        let db_transaction = drive.grove.start_transaction();

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/family/family-contract-only-age-index.json",
            None,
            None,
            None::<fn(&mut DataContract)>,
            Some(&db_transaction),
            None,
        );

        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get document type");

        let random_owner_id0 = rand::thread_rng().gen::<[u8; 32]>();

        let person_document0 = json_document_to_document(
            "tests/supporting_files/contract/family/person0.json",
            Some(random_owner_id0.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get document");

        let random_owner_id1 = rand::thread_rng().gen::<[u8; 32]>();

        let person_document1 = json_document_to_document(
            "tests/supporting_files/contract/family/person3-older.json",
            Some(random_owner_id1.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get document");

        let operations = vec![
            AddOperation {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((
                        &person_document0,
                        StorageFlags::optional_default_as_cow(),
                    )),
                    owner_id: Some(random_owner_id0),
                },
                override_document: false,
            },
            AddOperation {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((
                        &person_document1,
                        StorageFlags::optional_default_as_cow(),
                    )),
                    owner_id: Some(random_owner_id1),
                },
                override_document: false,
            },
        ];
        let drive_operations = vec![DocumentOperation(
            MultipleDocumentOperationsForSameContractDocumentType {
                document_operations: DocumentOperationsForContractDocumentType {
                    operations,
                    contract: &contract,
                    document_type,
                },
            },
        )];

        drive
            .apply_drive_operations(
                drive_operations,
                true,
                &BlockInfo::default(),
                Some(&db_transaction),
                platform_version,
                None,
            )
            .expect("expected to be able to insert documents");

        // This was the setup now let's do the update

        let person_document0 = json_document_to_document(
            "tests/supporting_files/contract/family/person0-older.json",
            Some(random_owner_id0.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get document");

        let person_document1 = json_document_to_document(
            "tests/supporting_files/contract/family/person3.json",
            Some(random_owner_id1.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get document");

        let operations = vec![
            UpdateOperation(UpdateOperationInfo {
                document: &person_document0,
                serialized_document: None,
                owner_id: Some(random_owner_id0),
                storage_flags: None,
            }),
            UpdateOperation(UpdateOperationInfo {
                document: &person_document1,
                serialized_document: None,
                owner_id: Some(random_owner_id1),
                storage_flags: None,
            }),
        ];

        let drive_operations = vec![DocumentOperation(
            MultipleDocumentOperationsForSameContractDocumentType {
                document_operations: DocumentOperationsForContractDocumentType {
                    operations,
                    contract: &contract,
                    document_type,
                },
            },
        )];

        drive
            .apply_drive_operations(
                drive_operations,
                true,
                &BlockInfo::default(),
                Some(&db_transaction),
                platform_version,
                None,
            )
            .expect("expected to be able to update documents");

        let query_value = json!({
            "where": [
                ["age", ">=", 5]
            ],
            "limit": 100,
            "orderBy": [
                ["age", "asc"],
            ]
        });
        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");

        let (docs, _, _) = drive
            .query_documents_cbor_from_contract(
                &contract,
                document_type,
                where_cbor.as_slice(),
                None,
                Some(&db_transaction),
                Some(platform_version.protocol_version),
            )
            .expect("expected to query");
        assert_eq!(docs.len(), 2);

        let query_value = json!({
            "where": [
                ["age", "==", 35]
            ],
            "limit": 100,
            "orderBy": [
                ["age", "asc"],
            ]
        });
        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");

        let (docs, _, _) = drive
            .query_documents_cbor_from_contract(
                &contract,
                document_type,
                where_cbor.as_slice(),
                None,
                Some(&db_transaction),
                Some(platform_version.protocol_version),
            )
            .expect("expected to query");
        assert_eq!(docs.len(), 1);

        let query_value = json!({
            "where": [
                ["age", "==", 36]
            ],
            "limit": 100,
            "orderBy": [
                ["age", "asc"],
            ]
        });
        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");

        let (docs, _, _) = drive
            .query_documents_cbor_from_contract(
                &contract,
                document_type,
                where_cbor.as_slice(),
                None,
                Some(&db_transaction),
                Some(platform_version.protocol_version),
            )
            .expect("expected to query");
        assert_eq!(docs.len(), 1);
    }

    fn add(identity: u8, added_balance: Credits) -> DriveOperation<'static> {
        DriveOperation::IdentityOperation(IdentityOperationType::AddToIdentityBalance {
            identity_id: [identity; 32],
            added_balance,
        })
    }

    fn remove(identity: u8, balance_to_remove: Credits) -> DriveOperation<'static> {
        DriveOperation::IdentityOperation(IdentityOperationType::RemoveFromIdentityBalance {
            identity_id: [identity; 32],
            balance_to_remove,
        })
    }

    fn add_to_pot(pot: ContractFeePot, amount: Credits) -> DriveOperation<'static> {
        DriveOperation::ContractFeePotOperation(ContractFeePotOperationType::AddToPot {
            contract_id: Identifier::new([9; 32]),
            pot,
            amount,
        })
    }

    fn deduct_from_pot(pot: ContractFeePot, amount: Credits) -> DriveOperation<'static> {
        DriveOperation::ContractFeePotOperation(ContractFeePotOperationType::DeductFromPot {
            contract_id: Identifier::new([9; 32]),
            pot,
            amount,
        })
    }

    fn merged(operations: Vec<DriveOperation<'static>>) -> String {
        format!(
            "{:?}",
            DriveOperation::merge_balance_writes(operations).expect("expected to merge")
        )
    }

    #[test]
    fn should_merge_every_write_of_one_identity_balance_into_one_in_place_of_the_first() {
        // A purchase price and a purchase fee leaving the buyer, and a sale paying the seller
        assert_eq!(
            merged(vec![remove(1, 100), add(2, 100), remove(1, 7)]),
            format!("{:?}", vec![remove(1, 107), add(2, 100)])
        );
        // A sale paying a contract owner who also pays the moderators part of the fee
        assert_eq!(
            merged(vec![add(3, 100), remove(3, 7)]),
            format!("{:?}", vec![add(3, 93)])
        );
    }

    #[test]
    fn should_merge_the_writes_of_one_fee_pot_and_drop_writes_that_change_nothing() {
        assert_eq!(
            merged(vec![
                add_to_pot(ContractFeePot::Moderators, 5),
                add_to_pot(ContractFeePot::Owner, 2),
                deduct_from_pot(ContractFeePot::Moderators, 8),
                add(4, 10),
                remove(4, 10),
            ]),
            format!(
                "{:?}",
                vec![
                    deduct_from_pot(ContractFeePot::Moderators, 3),
                    add_to_pot(ContractFeePot::Owner, 2),
                ]
            )
        );
    }

    fn fund_vote_poll(add_balance: Credits) -> DriveOperation<'static> {
        DriveOperation::PrefundedSpecializedBalanceOperation(
            PrefundedSpecializedBalanceOperationType::CreateNewPrefundedBalance {
                prefunded_specialized_balance_id: Identifier::new([8; 32]),
                add_balance,
            },
        )
    }

    fn pay_from_vote_poll(remove_balance: Credits) -> DriveOperation<'static> {
        DriveOperation::PrefundedSpecializedBalanceOperation(
            PrefundedSpecializedBalanceOperationType::DeductFromPrefundedBalance {
                prefunded_specialized_balance_id: Identifier::new([8; 32]),
                remove_balance,
            },
        )
    }

    #[test]
    fn should_merge_the_writes_of_one_prefunded_specialized_balance() {
        assert_eq!(
            merged(vec![pay_from_vote_poll(3), pay_from_vote_poll(4)]),
            format!("{:?}", vec![pay_from_vote_poll(7)])
        );
        assert_eq!(
            merged(vec![fund_vote_poll(10), pay_from_vote_poll(4)]),
            format!("{:?}", vec![fund_vote_poll(6)])
        );
    }

    fn token(position: u8) -> Identifier {
        Identifier::new([20 + position; 32])
    }

    fn transfer(from: u8, to: u8) -> DriveOperation<'static> {
        DriveOperation::TokenOperation(TokenOperationType::TokenTransfer {
            token_id: token(0),
            sender_id: Identifier::new([from; 32]),
            recipient_id: Identifier::new([to; 32]),
            amount: 5,
        })
    }

    fn burn(token_position: u8, holder: u8) -> DriveOperation<'static> {
        DriveOperation::TokenOperation(TokenOperationType::TokenBurn {
            token_id: token(token_position),
            identity_balance_holder_id: Identifier::new([holder; 32]),
            burn_amount: 5,
        })
    }

    fn mint(token_position: u8, holder: u8) -> DriveOperation<'static> {
        DriveOperation::TokenOperation(TokenOperationType::TokenMint {
            token_id: token(token_position),
            identity_balance_holder_id: Identifier::new([holder; 32]),
            mint_amount: 5,
            allow_first_mint: false,
            allow_saturation: false,
        })
    }

    #[test]
    fn should_refuse_a_batch_that_writes_one_token_balance_or_supply_twice() {
        // The sender of a transfer burns too: their balance is written twice.
        assert!(DriveOperation::refuse_repeated_token_balance_writes(&[
            transfer(1, 2),
            burn(0, 1)
        ])
        .is_err());
        // A mint and a burn of one token, by different holders: its supply is written twice.
        assert!(
            DriveOperation::refuse_repeated_token_balance_writes(&[mint(0, 1), burn(0, 2)])
                .is_err()
        );
    }

    #[test]
    fn should_admit_token_writes_that_each_touch_their_own_keys() {
        assert!(DriveOperation::refuse_repeated_token_balance_writes(&[
            transfer(1, 2),
            mint(1, 1),
            burn(2, 2),
            add(1, 10),
        ])
        .is_ok());
    }

    #[test]
    fn should_keep_a_batch_that_writes_every_key_once_as_it_is() {
        let operations = vec![
            remove(1, 100),
            add(2, 100),
            add_to_pot(ContractFeePot::Owner, 2),
        ];
        assert_eq!(merged(operations.clone()), format!("{operations:?}"));
    }
}
