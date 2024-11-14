//! Drive Constants
//!
//! Constants values that will NEVER change.
//!

///DataContract Documents subtree path height
pub const CONTRACT_DOCUMENTS_PATH_HEIGHT: u16 = 4;
/// Base contract root path size
pub const BASE_CONTRACT_ROOT_PATH_SIZE: u32 = 33; // 1 + 32
/// Base contract keeping_history_storage path size
pub const BASE_CONTRACT_KEEPING_HISTORY_STORAGE_PATH_SIZE: u32 = 34; // 1 + 32 + 1
/// Base contract documents_keeping_history_storage_time_reference path size
pub const BASE_CONTRACT_DOCUMENTS_KEEPING_HISTORY_STORAGE_TIME_REFERENCE_PATH: u32 = 75;
/// Base contract documents_keeping_history_primary_key path for document ID size
pub const BASE_CONTRACT_DOCUMENTS_KEEPING_HISTORY_PRIMARY_KEY_PATH_FOR_DOCUMENT_ID_SIZE: u32 = 67; // 1 + 32 + 1 + 1 + 32, then we need to add document_type_name.len()
/// BaseDataContract Documents path size
pub const BASE_CONTRACT_DOCUMENTS_PATH: u32 = 34;
/// BaseDataContract Documents primary key path
pub const BASE_CONTRACT_DOCUMENTS_PRIMARY_KEY_PATH: u32 = 35;

/// Some optimized document reference size
pub const OPTIMIZED_DOCUMENT_REFERENCE: u16 = 34;

/// Empty tree storage size
pub const EMPTY_TREE_STORAGE_SIZE: u32 = 33;
/// Max index size
pub const MAX_INDEX_SIZE: usize = 255;
/// Storage flags size
pub const STORAGE_FLAGS_SIZE: u32 = 2;

/// Default required bytes to hold a user balance
/// TODO We probably don't need it anymore since we always pay for 9 bytes
pub const AVERAGE_BALANCE_SIZE: u32 = 6;

/// Default required bytes to hold a public key
pub const AVERAGE_KEY_SIZE: u32 = 50;

/// How many updates would occur on average for an item
pub const AVERAGE_NUMBER_OF_UPDATES: u8 = 10;

/// How many bytes are added on average per update
/// 1 here signifies less than 128
pub const AVERAGE_UPDATE_BYTE_COUNT_REQUIRED_SIZE: u8 = 1;

/// The estimated average document type name size
pub const ESTIMATED_AVERAGE_DOCUMENT_TYPE_NAME_SIZE: u8 = 12;

/// The estimated average index name size
pub const ESTIMATED_AVERAGE_INDEX_NAME_SIZE: u8 = 16;

/// The estimated count of identities having the same key if they are not unique
pub const ESTIMATED_NON_UNIQUE_KEY_DUPLICATES: u32 = 2;

/// The average size of an item that is acting as a tree reference towards the contested item vote
pub const AVERAGE_CONTESTED_RESOURCE_ITEM_REFERENCE_SIZE: u32 = 150;

/// Contested document reference size
// we need to construct the reference from the split height of the contract document
// type which is at 4
// 0 represents document storage
// Then we add document id
// Then we add 0 if the document type keys history
// vec![vec![0], Vec::from(document.id)];
// 1 (vec size) + 1 (subvec size) + 32 (document id size) = 34
// + 6 = 40
// 6 because of:
// 1 for type reference
// 1 for reference type
// 1 for root height offset
// reference path size
// 1 reference_hops options
// 1 reference_hops count
// 1 element flags option
pub const CONTESTED_DOCUMENT_REFERENCE_SIZE: u32 = 40;
