// MIT LICENSE
//
// Copyright (c) 2022 Dash Core Group
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

//! GroveDB Operations Batch.
//!
//! This module defines the GroveDbOpBatch struct and implements its functions.
//!

use crate::drive::flags::StorageFlags;
use grovedb::batch::{key_info::KeyInfo, GroveDbOp, GroveDbOpConsistencyResults, KeyInfoPath};
use grovedb::Element;

/// A batch of GroveDB operations as a vector.
// TODO move to GroveDB
#[derive(Debug, Default)]
pub struct GroveDbOpBatch {
    /// Operations
    pub(crate) operations: Vec<GroveDbOp>,
}

impl GroveDbOpBatch {
    /// Creates a new empty batch of GroveDB operations.
    pub fn new() -> Self {
        GroveDbOpBatch {
            operations: Vec::new(),
        }
    }

    /// Gets the number of operations from a list of GroveDB ops.
    pub fn len(&self) -> usize {
        self.operations.len()
    }

    /// Checks to see if the operation batch is empty
    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }

    /// Pushes an operation into a list of GroveDB ops.
    pub fn push(&mut self, op: GroveDbOp) {
        self.operations.push(op);
    }

    /// Puts a list of GroveDB operations into a batch.
    pub fn from_operations(operations: Vec<GroveDbOp>) -> Self {
        GroveDbOpBatch { operations }
    }

    /// Adds an `Insert` operation with an empty tree at the specified path and key to a list of GroveDB ops.
    pub fn add_insert_empty_tree(&mut self, path: Vec<Vec<u8>>, key: Vec<u8>) {
        self.operations
            .push(GroveDbOp::insert_run_op(path, key, Element::empty_tree()))
    }

    /// Adds an `Insert` operation with an empty tree with storage flags to a list of GroveDB ops.
    pub fn add_insert_empty_tree_with_flags(
        &mut self,
        path: Vec<Vec<u8>>,
        key: Vec<u8>,
        storage_flags: Option<&StorageFlags>,
    ) {
        self.operations.push(GroveDbOp::insert_run_op(
            path,
            key,
            Element::empty_tree_with_flags(StorageFlags::map_to_some_element_flags(storage_flags)),
        ))
    }

    /// Adds a `Delete` operation to a list of GroveDB ops.
    pub fn add_delete(&mut self, path: Vec<Vec<u8>>, key: Vec<u8>) {
        self.operations.push(GroveDbOp::delete_run_op(path, key))
    }

    /// Adds a `Delete` tree operation to a list of GroveDB ops.
    pub fn add_delete_tree(&mut self, path: Vec<Vec<u8>>, key: Vec<u8>) {
        self.operations
            .push(GroveDbOp::delete_tree_run_op(path, key))
    }

    /// Adds an `Insert` operation with an element to a list of GroveDB ops.
    pub fn add_insert(&mut self, path: Vec<Vec<u8>>, key: Vec<u8>, element: Element) {
        self.operations
            .push(GroveDbOp::insert_run_op(path, key, element))
    }

    /// Adds a worst case `Insert` operation to a list of GroveDB ops
    pub fn add_worst_case_insert_empty_tree(&mut self, path: Vec<KeyInfo>, key: KeyInfo) {
        self.operations.push(GroveDbOp::insert_worst_case_op(
            KeyInfoPath::from_vec(path),
            key,
            Element::empty_tree(),
        ));
    }

    /// Adds a worst case `Insert` operation with empty tree to a list of GroveDB ops
    pub fn add_worst_case_insert_empty_tree_with_flags(
        &mut self,
        path: Vec<KeyInfo>,
        key: KeyInfo,
        storage_flags: Option<&StorageFlags>,
    ) {
        self.operations.push(GroveDbOp::insert_worst_case_op(
            KeyInfoPath::from_vec(path),
            key,
            Element::empty_tree_with_flags(StorageFlags::map_to_some_element_flags(storage_flags)),
        ));
    }

    /// Adds a worst case `Delete` operation with empty tree to a list of GroveDB ops
    pub fn add_worst_case_delete(&mut self, path: Vec<KeyInfo>, key: KeyInfo) {
        self.operations.push(GroveDbOp::delete_worst_case_op(
            KeyInfoPath::from_vec(path),
            key,
        ));
    }

    /// Adds a worst case `Insert` operation with element to a list of GroveDB ops
    pub fn add_worst_case_insert(&mut self, path: Vec<KeyInfo>, key: KeyInfo, element: Element) {
        self.operations.push(GroveDbOp::insert_worst_case_op(
            KeyInfoPath::from_vec(path),
            key,
            element,
        ));
    }

    /// Verify consistency of operations
    pub fn verify_consistency_of_operations(&self) -> GroveDbOpConsistencyResults {
        GroveDbOp::verify_consistency_of_operations(&self.operations)
    }
}
