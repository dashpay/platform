// MIT LICENSE
//
// Copyright (c) 2021 Dash Core Group
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
//

//! Drive Defaults
//!
//! Default values for Drive constants.
//!

/// Protocol version
pub const INITIAL_PROTOCOL_VERSION: u32 = 1;
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
/// Default hash size
pub const DEFAULT_HASH_SIZE: u32 = 32;
/// Default hash 160 size as u8
pub const DEFAULT_HASH_160_SIZE_U8: u8 = 20;
/// Default hash size as u8
pub const DEFAULT_HASH_SIZE_U8: u8 = 32;
/// Default hash size as u16
pub const DEFAULT_HASH_SIZE_U16: u16 = 32;
/// Default hash size as u32
pub const DEFAULT_HASH_SIZE_U32: u32 = 32;
/// Some optimized document reference size
pub const OPTIMIZED_DOCUMENT_REFERENCE: u16 = 34; // 1 + hops + DEFAULT_HASH_SIZE
/// Default float size
pub const DEFAULT_FLOAT_SIZE: u32 = 8;
/// Default float size as u16
pub const DEFAULT_FLOAT_SIZE_U16: u16 = 8;
/// Default float size as u8
pub const DEFAULT_FLOAT_SIZE_U8: u8 = 8;
/// Empty tree storage size
pub const EMPTY_TREE_STORAGE_SIZE: u32 = 33;
/// Max index size
pub const MAX_INDEX_SIZE: usize = 255;
/// Storage flags size
pub const STORAGE_FLAGS_SIZE: u32 = 2;

/// Serialized contract max size
pub const CONTRACT_MAX_SERIALIZED_SIZE: u16 = 16384;
// TODO: Insert correct value here
/// Max element size
pub const MAX_ELEMENT_SIZE: u32 = 5000;

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
