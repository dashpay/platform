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
pub const PROTOCOL_VERSION: u32 = 1;
/// Contract Documents subtree path height
pub const CONTRACT_DOCUMENTS_PATH_HEIGHT: u16 = 4;
/// Base contract root path size
pub const BASE_CONTRACT_ROOT_PATH_SIZE: u32 = 33; // 1 + 32
/// Base contract keeping_history_storage path size
pub const BASE_CONTRACT_KEEPING_HISTORY_STORAGE_PATH_SIZE: u32 = 34; // 1 + 32 + 1
/// Base contract documents_keeping_history_storage_time_reference path size
pub const BASE_CONTRACT_DOCUMENTS_KEEPING_HISTORY_STORAGE_TIME_REFERENCE_PATH: u32 = 75;
/// Base contract documents_keeping_history_primary_key path for document ID size
pub const BASE_CONTRACT_DOCUMENTS_KEEPING_HISTORY_PRIMARY_KEY_PATH_FOR_DOCUMENT_ID_SIZE: u32 = 67; // 1 + 32 + 1 + 1 + 32, then we need to add document_type_name.len()
/// Base Contract Documents path size
pub const BASE_CONTRACT_DOCUMENTS_PATH: u32 = 34;
/// Base Contract Documents primary key path
pub const BASE_CONTRACT_DOCUMENTS_PRIMARY_KEY_PATH: u32 = 35;
/// Default hash size
pub const DEFAULT_HASH_SIZE: u32 = 32;
/// Default float size
pub const SOME_TREE_SIZE: Option<u16> = Some(32);
/// Some optimized document reference size
pub const SOME_OPTIMIZED_DOCUMENT_REFERENCE: Option<u16> = Some(34); // 1 + hops + DEFAULT_HASH_SIZE
/// Default float size
pub const DEFAULT_FLOAT_SIZE: u32 = 8;
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
