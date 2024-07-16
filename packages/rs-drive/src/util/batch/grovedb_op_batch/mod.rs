//! GroveDB Operations Batch.
//!
//! This module defines the GroveDbOpBatch struct and implements its functions.
//!

use crate::drive::credit_pools::epochs;
use crate::drive::identity::IdentityRootStructure;
use crate::drive::{credit_pools, RootTree};
use crate::util::storage_flags::StorageFlags;
use dpp::block::epoch::Epoch;
use dpp::identity::{Purpose, SecurityLevel};
use dpp::prelude::Identifier;
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::{GroveDbOp, GroveDbOpConsistencyResults, KeyInfoPath, Op};
use grovedb::operations::proof::util::hex_to_ascii;
use grovedb::Element;
use std::borrow::Cow;
use std::fmt;

/// A batch of GroveDB operations as a vector.
// TODO move to GroveDB
#[derive(Debug, Default, Clone)]
pub struct GroveDbOpBatch {
    /// Operations
    pub(crate) operations: Vec<GroveDbOp>,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
enum KnownPath {
    Root,                                                             //Level 0
    DataContractAndDocumentsRoot,                                     //Level 1
    DataContractStorage,                                              //Level 2
    DocumentsRoot,                                                    //Level 2
    IdentitiesRoot,                                                   //Level 1
    IdentityTreeRevisionRoot,                                         //Level 2
    IdentityTreeNonceRoot,                                            //Level 2
    IdentityTreeKeysRoot,                                             //Level 2
    IdentityTreeKeyReferencesRoot,                                    //Level 2
    IdentityTreeKeyReferencesInPurpose(Purpose),                      //Level 3
    IdentityTreeKeyReferencesInSecurityLevel(Purpose, SecurityLevel), //Level 4
    IdentityTreeNegativeCreditRoot,                                   //Level 2
    IdentityContractInfoRoot,                                         //Level 2
    UniquePublicKeyHashesToIdentitiesRoot,                            //Level 1
    NonUniquePublicKeyKeyHashesToIdentitiesRoot,                      //Level 1
    PoolsRoot,                                                        //Level 1
    PoolsInsideEpoch(Epoch),                                          //Level 2
    PreFundedSpecializedBalancesRoot,                                 //Level 1
    SpentAssetLockTransactionsRoot,                                   //Level 1
    MiscRoot,                                                         //Level 1
    WithdrawalTransactionsRoot,                                       //Level 1
    BalancesRoot,                                                     //Level 1
    TokenBalancesRoot,                                                //Level 1
    VersionsRoot,                                                     //Level 1
    VotesRoot,                                                        //Level 1
}

impl From<RootTree> for KnownPath {
    fn from(value: RootTree) -> Self {
        match value {
            RootTree::DataContractDocuments => KnownPath::DataContractAndDocumentsRoot,
            RootTree::Identities => KnownPath::IdentitiesRoot,
            RootTree::UniquePublicKeyHashesToIdentities => {
                KnownPath::UniquePublicKeyHashesToIdentitiesRoot
            }
            RootTree::NonUniquePublicKeyKeyHashesToIdentities => {
                KnownPath::NonUniquePublicKeyKeyHashesToIdentitiesRoot
            }
            RootTree::Pools => KnownPath::PoolsRoot,
            RootTree::PreFundedSpecializedBalances => KnownPath::PreFundedSpecializedBalancesRoot,
            RootTree::SpentAssetLockTransactions => KnownPath::SpentAssetLockTransactionsRoot,
            RootTree::Misc => KnownPath::MiscRoot,
            RootTree::WithdrawalTransactions => KnownPath::WithdrawalTransactionsRoot,
            RootTree::Balances => KnownPath::BalancesRoot,
            RootTree::TokenBalances => KnownPath::TokenBalancesRoot,
            RootTree::Versions => KnownPath::VersionsRoot,
            RootTree::Votes => KnownPath::VotesRoot,
        }
    }
}

impl From<IdentityRootStructure> for KnownPath {
    fn from(value: IdentityRootStructure) -> Self {
        match value {
            IdentityRootStructure::IdentityTreeRevision => KnownPath::IdentityTreeRevisionRoot,
            IdentityRootStructure::IdentityTreeNonce => KnownPath::IdentityTreeNonceRoot,
            IdentityRootStructure::IdentityTreeKeys => KnownPath::IdentityTreeKeysRoot,
            IdentityRootStructure::IdentityTreeKeyReferences => {
                KnownPath::IdentityTreeKeyReferencesRoot
            }
            IdentityRootStructure::IdentityTreeNegativeCredit => {
                KnownPath::IdentityTreeNegativeCreditRoot
            }
            IdentityRootStructure::IdentityContractInfo => KnownPath::IdentityContractInfoRoot,
        }
    }
}

fn readable_key_info(known_path: KnownPath, key_info: &KeyInfo) -> (String, Option<KnownPath>) {
    match key_info {
        KeyInfo::KnownKey(key) => {
            match known_path {
                KnownPath::Root => {
                    if let Ok(root_tree) = RootTree::try_from(key[0]) {
                        (
                            format!("{}({})", root_tree.to_string(), key[0]),
                            Some(root_tree.into()),
                        )
                    } else {
                        (hex_to_ascii(key), None)
                    }
                }
                KnownPath::BalancesRoot | KnownPath::IdentitiesRoot if key.len() == 32 => (
                    format!(
                        "IdentityId(bs58::{})",
                        Identifier::from_vec(key.clone()).unwrap()
                    ),
                    None,
                ),
                KnownPath::DataContractAndDocumentsRoot if key.len() == 32 => (
                    format!(
                        "ContractId(bs58::{})",
                        Identifier::from_vec(key.clone()).unwrap()
                    ),
                    None,
                ),
                KnownPath::DataContractAndDocumentsRoot if key.len() == 1 => match key[0] {
                    0 => (
                        "DataContractStorage(0)".to_string(),
                        Some(KnownPath::DataContractStorage),
                    ),
                    1 => (
                        "DataContractDocuments(1)".to_string(),
                        Some(KnownPath::DocumentsRoot),
                    ),
                    _ => (hex_to_ascii(key), None),
                },
                KnownPath::IdentitiesRoot if key.len() == 1 => {
                    if let Ok(root_tree) = IdentityRootStructure::try_from(key[0]) {
                        (
                            format!("{}({})", root_tree.to_string(), key[0]),
                            Some(root_tree.into()),
                        )
                    } else {
                        (hex_to_ascii(key), None)
                    }
                }
                KnownPath::IdentityTreeKeyReferencesRoot if key.len() == 1 => {
                    if let Ok(purpose) = Purpose::try_from(key[0]) {
                        (
                            format!("Purpose::{}({})", purpose, key[0]),
                            Some(KnownPath::IdentityTreeKeyReferencesInPurpose(purpose)),
                        )
                    } else {
                        (hex_to_ascii(key), None)
                    }
                }
                KnownPath::IdentityTreeKeyReferencesInPurpose(purpose) if key.len() == 1 => {
                    if let Ok(security_level) = SecurityLevel::try_from(key[0]) {
                        (
                            format!("SecurityLevel::{}({})", security_level, key[0]),
                            Some(KnownPath::IdentityTreeKeyReferencesInSecurityLevel(
                                purpose,
                                security_level,
                            )),
                        )
                    } else {
                        (hex_to_ascii(key), None)
                    }
                }

                KnownPath::PoolsRoot if key.len() == 1 => match key[0] {
                    epochs::epochs_root_tree_key_constants::KEY_STORAGE_FEE_POOL_U8 => {
                        ("StorageFeePool(ascii:'s')".to_string(), None)
                    }
                    epochs::epochs_root_tree_key_constants::KEY_UNPAID_EPOCH_INDEX_U8 => {
                        ("UnpaidEpochIndex(ascii:'u')".to_string(), None)
                    }
                    epochs::epochs_root_tree_key_constants::KEY_PENDING_EPOCH_REFUNDS_U8 => {
                        ("PendingEpochRefunds(ascii:'p')".to_string(), None)
                    }
                    _ => (hex_to_ascii(key), None),
                },
                KnownPath::PoolsRoot if key.len() == 2 => {
                    // this is an epoch
                    if let Ok(epoch) = Epoch::try_from(key) {
                        (
                            format!("Epoch::{}({})", epoch.index, hex::encode(key)),
                            Some(KnownPath::PoolsInsideEpoch(epoch)),
                        )
                    } else {
                        (hex_to_ascii(key), None)
                    }
                }
                KnownPath::PoolsInsideEpoch(_) if key.len() == 1 => {
                    // this is an epoch
                    match key[0] {
                        credit_pools::epochs::epoch_key_constants::KEY_POOL_PROCESSING_FEES_U8 => {
                            ("PoolProcessingFees(ascii:'p')".to_string(), None)
                        }
                        credit_pools::epochs::epoch_key_constants::KEY_POOL_STORAGE_FEES_U8 => {
                            ("PoolStorageFees(ascii:'s')".to_string(), None)
                        }
                        credit_pools::epochs::epoch_key_constants::KEY_START_TIME_U8 => {
                            ("StartTime(ascii:'t')".to_string(), None)
                        }
                        credit_pools::epochs::epoch_key_constants::KEY_PROTOCOL_VERSION_U8 => {
                            ("ProtocolVersion(ascii:'v')".to_string(), None)
                        }
                        credit_pools::epochs::epoch_key_constants::KEY_START_BLOCK_HEIGHT_U8 => {
                            ("StartBlockHeight(ascii:'h')".to_string(), None)
                        }
                        credit_pools::epochs::epoch_key_constants::KEY_START_BLOCK_CORE_HEIGHT_U8 => {
                            ("StartBlockCoreHeight(ascii:'c')".to_string(), None)
                        }
                        credit_pools::epochs::epoch_key_constants::KEY_PROPOSERS_U8 => {
                            ("Proposers(ascii:'m')".to_string(), None)
                        }
                        credit_pools::epochs::epoch_key_constants::KEY_FEE_MULTIPLIER_U8 => {
                            ("FeeMultiplier(ascii:'x')".to_string(), None)
                        }
                        _ => (hex_to_ascii(key), None),
                    }
                }
                _ => (hex_to_ascii(key), None),
            }
        }
        KeyInfo::MaxKeySize {
            unique_id,
            max_size,
        } => (
            format!(
                "MaxKeySize(unique_id: {:?}, max_size: {})",
                unique_id, max_size
            ),
            None,
        ),
    }
}

fn readable_path(path: &KeyInfoPath) -> (String, KnownPath) {
    let mut known_path = KnownPath::Root;
    let string = path
        .0
        .iter()
        .map(|key_info| {
            let (string, new_known_path) = readable_key_info(known_path, key_info);
            if let Some(new_known_path) = new_known_path {
                known_path = new_known_path;
            }
            string
        })
        .collect::<Vec<_>>()
        .join("/");
    (string, known_path)
}

impl fmt::Display for GroveDbOpBatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for op in &self.operations {
            let (path_string, known_path) = readable_path(&op.path);
            let (key_string, _) = readable_key_info(known_path, &op.key);
            writeln!(f, "{{")?;
            writeln!(f, "   Path: {}", path_string)?;
            writeln!(f, "   Key: {}", key_string)?;
            match &op.op {
                Op::Insert { element } => {
                    let flags = element.get_flags();
                    let flag_info = match flags {
                        None => "No Flags".to_string(),
                        Some(flags) => format!("Flags are 0x{}", hex::encode(flags)),
                    };
                    match element {
                        Element::Item(data, _) => {
                            let num = match data.len() {
                                8 => format!(
                                    " u64({})",
                                    u64::from_be_bytes(data.clone().try_into().unwrap())
                                ),
                                4 => format!(
                                    " u32({})",
                                    u32::from_be_bytes(data.clone().try_into().unwrap())
                                ),
                                _ => String::new(),
                            };
                            writeln!(
                                f,
                                "   Operation: Insert Item with length: {}{} {}",
                                data.len(),
                                num,
                                flag_info
                            )?
                        }
                        Element::Tree(None, _) => {
                            writeln!(f, "   Operation: Insert Empty Tree {}", flag_info)?
                        }
                        Element::SumTree(None, _, _) => {
                            writeln!(f, "   Operation: Insert Empty Sum Tree {}", flag_info)?
                        }
                        _ => writeln!(f, "   Operation: Insert {}", element)?,
                    }
                }
                _ => {
                    writeln!(f, "   Operation: {:?}", op.op)?;
                }
            }
            writeln!(f, "}}")?;
        }
        Ok(())
    }
}

/// Trait defining a batch of GroveDB operations.
pub trait GroveDbOpBatchV0Methods {
    /// Creates a new empty batch of GroveDB operations.
    fn new() -> Self;

    /// Gets the number of operations from a list of GroveDB ops.
    fn len(&self) -> usize;

    /// Checks to see if the operation batch is empty.
    fn is_empty(&self) -> bool;

    /// Pushes an operation into a list of GroveDB ops.
    fn push(&mut self, op: GroveDbOp);

    /// Appends operations into a list of GroveDB ops.
    fn append(&mut self, other: &mut Self);

    /// Extend operations into a list of GroveDB ops.
    fn extend<I: IntoIterator<Item = GroveDbOp>>(&mut self, other_ops: I);

    /// Puts a list of GroveDB operations into a batch.
    fn from_operations(operations: Vec<GroveDbOp>) -> Self;

    /// Adds an `Insert` operation with an empty tree at the specified path and key to a list of GroveDB ops.
    fn add_insert_empty_tree(&mut self, path: Vec<Vec<u8>>, key: Vec<u8>);

    /// Adds an `Insert` operation with an empty tree with storage flags to a list of GroveDB ops.
    fn add_insert_empty_tree_with_flags(
        &mut self,
        path: Vec<Vec<u8>>,
        key: Vec<u8>,
        storage_flags: &Option<Cow<StorageFlags>>,
    );

    /// Adds an `Insert` operation with an empty sum tree at the specified path and key to a list of GroveDB ops.
    fn add_insert_empty_sum_tree(&mut self, path: Vec<Vec<u8>>, key: Vec<u8>);

    /// Adds an `Insert` operation with an empty sum tree with storage flags to a list of GroveDB ops.
    fn add_insert_empty_sum_tree_with_flags(
        &mut self,
        path: Vec<Vec<u8>>,
        key: Vec<u8>,
        storage_flags: &Option<Cow<StorageFlags>>,
    );

    /// Adds a `Delete` operation to a list of GroveDB ops.
    fn add_delete(&mut self, path: Vec<Vec<u8>>, key: Vec<u8>);

    /// Adds a `Delete` tree operation to a list of GroveDB ops.
    fn add_delete_tree(&mut self, path: Vec<Vec<u8>>, key: Vec<u8>, is_sum_tree: bool);

    /// Adds an `Insert` operation with an element to a list of GroveDB ops.
    fn add_insert(&mut self, path: Vec<Vec<u8>>, key: Vec<u8>, element: Element);

    /// Verify consistency of operations
    fn verify_consistency_of_operations(&self) -> GroveDbOpConsistencyResults;

    /// Check if the batch contains a specific path and key.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to search for.
    /// * `key` - The key to search for.
    ///
    /// # Returns
    ///
    /// * `Option<&Op>` - Returns a reference to the `Op` if found, or `None` otherwise.
    fn contains<'c, P>(&self, path: P, key: &[u8]) -> Option<&Op>
    where
        P: IntoIterator<Item = &'c [u8]>,
        <P as IntoIterator>::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone;

    /// Remove a specific path and key from the batch and return the removed `Op`.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to search for.
    /// * `key` - The key to search for.
    ///
    /// # Returns
    ///
    /// * `Option<Op>` - Returns the removed `Op` if found, or `None` otherwise.
    fn remove<'c, P>(&mut self, path: P, key: &[u8]) -> Option<Op>
    where
        P: IntoIterator<Item = &'c [u8]>,
        <P as IntoIterator>::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone;

    /// Find and remove a specific path and key from the batch if it is an
    /// `Op::Insert`, `Op::Replace`, or `Op::Patch`. Return the found `Op` regardless of whether it was removed.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to search for.
    /// * `key` - The key to search for.
    ///
    /// # Returns
    ///
    /// * `Option<Op>` - Returns the found `Op` if it exists. If the `Op` is an `Op::Insert`, `Op::Replace`,
    ///                  or `Op::Patch`, it will be removed from the batch.
    fn remove_if_insert(&mut self, path: Vec<Vec<u8>>, key: &[u8]) -> Option<Op>;
}

impl GroveDbOpBatchV0Methods for GroveDbOpBatch {
    /// Creates a new empty batch of GroveDB operations.
    fn new() -> Self {
        GroveDbOpBatch {
            operations: Vec::new(),
        }
    }

    /// Gets the number of operations from a list of GroveDB ops.
    fn len(&self) -> usize {
        self.operations.len()
    }

    /// Checks to see if the operation batch is empty
    fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }

    /// Pushes an operation into a list of GroveDB ops.
    fn push(&mut self, op: GroveDbOp) {
        self.operations.push(op);
    }

    /// Appends operations into a list of GroveDB ops.
    fn append(&mut self, other: &mut Self) {
        self.operations.append(&mut other.operations);
    }

    /// Extend operations into a list of GroveDB ops.
    fn extend<I: IntoIterator<Item = GroveDbOp>>(&mut self, other_ops: I) {
        self.operations.extend(other_ops);
    }

    /// Puts a list of GroveDB operations into a batch.
    fn from_operations(operations: Vec<GroveDbOp>) -> Self {
        GroveDbOpBatch { operations }
    }

    /// Adds an `Insert` operation with an empty tree at the specified path and key to a list of GroveDB ops.
    fn add_insert_empty_tree(&mut self, path: Vec<Vec<u8>>, key: Vec<u8>) {
        self.operations
            .push(GroveDbOp::insert_op(path, key, Element::empty_tree()))
    }

    /// Adds an `Insert` operation with an empty tree with storage flags to a list of GroveDB ops.
    fn add_insert_empty_tree_with_flags(
        &mut self,
        path: Vec<Vec<u8>>,
        key: Vec<u8>,
        storage_flags: &Option<Cow<StorageFlags>>,
    ) {
        self.operations.push(GroveDbOp::insert_op(
            path,
            key,
            Element::empty_tree_with_flags(StorageFlags::map_borrowed_cow_to_some_element_flags(
                storage_flags,
            )),
        ))
    }

    /// Adds an `Insert` operation with an empty sum tree at the specified path and key to a list of GroveDB ops.
    fn add_insert_empty_sum_tree(&mut self, path: Vec<Vec<u8>>, key: Vec<u8>) {
        self.operations
            .push(GroveDbOp::insert_op(path, key, Element::empty_sum_tree()))
    }

    /// Adds an `Insert` operation with an empty sum tree with storage flags to a list of GroveDB ops.
    fn add_insert_empty_sum_tree_with_flags(
        &mut self,
        path: Vec<Vec<u8>>,
        key: Vec<u8>,
        storage_flags: &Option<Cow<StorageFlags>>,
    ) {
        self.operations.push(GroveDbOp::insert_op(
            path,
            key,
            Element::empty_sum_tree_with_flags(
                StorageFlags::map_borrowed_cow_to_some_element_flags(storage_flags),
            ),
        ))
    }

    /// Adds a `Delete` operation to a list of GroveDB ops.
    fn add_delete(&mut self, path: Vec<Vec<u8>>, key: Vec<u8>) {
        self.operations.push(GroveDbOp::delete_op(path, key))
    }

    /// Adds a `Delete` tree operation to a list of GroveDB ops.
    fn add_delete_tree(&mut self, path: Vec<Vec<u8>>, key: Vec<u8>, is_sum_tree: bool) {
        self.operations
            .push(GroveDbOp::delete_tree_op(path, key, is_sum_tree))
    }

    /// Adds an `Insert` operation with an element to a list of GroveDB ops.
    fn add_insert(&mut self, path: Vec<Vec<u8>>, key: Vec<u8>, element: Element) {
        self.operations
            .push(GroveDbOp::insert_op(path, key, element))
    }

    /// Verify consistency of operations
    fn verify_consistency_of_operations(&self) -> GroveDbOpConsistencyResults {
        GroveDbOp::verify_consistency_of_operations(&self.operations)
    }

    /// Check if the batch contains a specific path and key.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to search for.
    /// * `key` - The key to search for.
    ///
    /// # Returns
    ///
    /// * `Option<&Op>` - Returns a reference to the `Op` if found, or `None` otherwise.
    fn contains<'c, P>(&self, path: P, key: &[u8]) -> Option<&Op>
    where
        P: IntoIterator<Item = &'c [u8]>,
        <P as IntoIterator>::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        let path = KeyInfoPath(
            path.into_iter()
                .map(|item| KeyInfo::KnownKey(item.to_vec()))
                .collect(),
        );

        self.operations.iter().find_map(|op| {
            if op.path == path && op.key == KeyInfo::KnownKey(key.to_vec()) {
                Some(&op.op)
            } else {
                None
            }
        })
    }

    /// Remove a specific path and key from the batch and return the removed `Op`.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to search for.
    /// * `key` - The key to search for.
    ///
    /// # Returns
    ///
    /// * `Option<Op>` - Returns the removed `Op` if found, or `None` otherwise.
    fn remove<'c, P>(&mut self, path: P, key: &[u8]) -> Option<Op>
    where
        P: IntoIterator<Item = &'c [u8]>,
        <P as IntoIterator>::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        let path = KeyInfoPath(
            path.into_iter()
                .map(|item| KeyInfo::KnownKey(item.to_vec()))
                .collect(),
        );

        if let Some(index) = self
            .operations
            .iter()
            .position(|op| op.path == path && op.key == KeyInfo::KnownKey(key.to_vec()))
        {
            Some(self.operations.remove(index).op)
        } else {
            None
        }
    }

    /// Find and remove a specific path and key from the batch if it is an
    /// `Op::Insert`, `Op::Replace`, or `Op::Patch`. Return the found `Op` regardless of whether it was removed.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to search for.
    /// * `key` - The key to search for.
    ///
    /// # Returns
    ///
    /// * `Option<Op>` - Returns the found `Op` if it exists. If the `Op` is an `Op::Insert`, `Op::Replace`,
    ///                  or `Op::Patch`, it will be removed from the batch.
    fn remove_if_insert(&mut self, path: Vec<Vec<u8>>, key: &[u8]) -> Option<Op> {
        let path = KeyInfoPath(
            path.into_iter()
                .map(|item| KeyInfo::KnownKey(item.to_vec()))
                .collect(),
        );

        if let Some(index) = self
            .operations
            .iter()
            .position(|op| op.path == path && op.key == KeyInfo::KnownKey(key.to_vec()))
        {
            let op = &self.operations[index].op;
            let op = if matches!(
                op,
                &Op::Insert { .. } | &Op::Replace { .. } | &Op::Patch { .. }
            ) {
                self.operations.remove(index).op
            } else {
                op.clone()
            };
            Some(op)
        } else {
            None
        }
    }
}

impl IntoIterator for GroveDbOpBatch {
    type Item = GroveDbOp;
    type IntoIter = std::vec::IntoIter<GroveDbOp>;

    fn into_iter(self) -> Self::IntoIter {
        self.operations.into_iter()
    }
}
