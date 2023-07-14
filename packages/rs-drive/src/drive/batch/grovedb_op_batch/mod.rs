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
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::{GroveDbOp, GroveDbOpConsistencyResults, KeyInfoPath, Op};
use grovedb::Element;
use std::borrow::Cow;

/// A batch of GroveDB operations as a vector.
// TODO move to GroveDB
#[derive(Debug, Default, Clone)]
pub struct GroveDbOpBatch {
    /// Operations
    pub(crate) operations: Vec<GroveDbOp>,
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
            if &op.path == &path && op.key == KeyInfo::KnownKey(key.to_vec()) {
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
            .position(|op| &op.path == &path && op.key == KeyInfo::KnownKey(key.to_vec()))
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
            .position(|op| &op.path == &path && op.key == KeyInfo::KnownKey(key.to_vec()))
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
