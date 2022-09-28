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
pub const BASE_CONTRACT_ROOT_PATH_SIZE: usize = 33; // 1 + 32
/// Base contract keeping_history_storage path size
pub const BASE_CONTRACT_KEEPING_HISTORY_STORAGE_PATH_SIZE: usize = 34; // 1 + 32 + 1
/// Base contract documents_keeping_history_storage_time_reference path size
pub const BASE_CONTRACT_DOCUMENTS_KEEPING_HISTORY_STORAGE_TIME_REFERENCE_PATH: usize = 75;
/// Base contract documents_keeping_history_primary_key path for document ID size
pub const BASE_CONTRACT_DOCUMENTS_KEEPING_HISTORY_PRIMARY_KEY_PATH_FOR_DOCUMENT_ID_SIZE: usize = 67; // 1 + 32 + 1 + 1 + 32, then we need to add document_type_name.len()
/// Base Contract Documents path size
pub const BASE_CONTRACT_DOCUMENTS_PATH: usize = 34;
/// Base Contract Documents primary key path
pub const BASE_CONTRACT_DOCUMENTS_PRIMARY_KEY_PATH: usize = 35;
/// Default hash size
pub const DEFAULT_HASH_SIZE: usize = 32;
/// Default float size
pub const DEFAULT_FLOAT_SIZE: usize = 8;
/// Empty tree storage size
pub const EMPTY_TREE_STORAGE_SIZE: usize = 33;
/// Max index size
pub const MAX_INDEX_SIZE: usize = 255;
/// Storage flags size
pub const STORAGE_FLAGS_SIZE: usize = 2;
