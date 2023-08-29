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

//! Implements in Drive functions which check if a given `outpoint` is present as an asset lock in the transaction and potentially applies operations to it (version 0).

use crate::drive::asset_lock::asset_lock_storage_path;
use crate::drive::grove_operations::DirectQueryType::{StatefulDirectQuery, StatelessDirectQuery};
use crate::drive::grove_operations::QueryTarget::QueryTargetValue;

use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::platform_value::Bytes36;
use dpp::version::drive_versions::DriveVersion;

use grovedb::TransactionArg;

impl Drive {
    /// Checks if a given `outpoint` is present as an asset lock in the transaction.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the current object.
    /// * `outpoint` - An `OutPoint` reference to be checked in the transaction.
    /// * `transaction` - The `TransactionArg` in which to check for the `outpoint`.
    ///
    /// # Returns
    ///
    /// Returns a `Result` which is `Ok` if the outpoint exists in the transaction or an `Error` otherwise.
    pub(super) fn has_asset_lock_outpoint_v0(
        &self,
        outpoint: &Bytes36,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<bool, Error> {
        self.has_asset_lock_outpoint_add_operations(
            true,
            &mut vec![],
            outpoint,
            transaction,
            drive_version,
        )
    }

    /// Checks if a given `outpoint` is present as an asset lock in the transaction and potentially applies operations to it.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the current object.
    /// * `apply` - A boolean which when true applies the operations to the asset lock.
    /// * `drive_operations` - A mutable reference to a vector of `LowLevelDriveOperation` to be possibly executed.
    /// * `outpoint` - An `OutPoint` reference to be checked in the transaction.
    /// * `transaction` - The `TransactionArg` in which to check for the `outpoint`.
    ///
    /// # Returns
    ///
    /// Returns a `Result` which is `Ok` if the outpoint exists in the transaction or an `Error` otherwise.
    pub(super) fn has_asset_lock_outpoint_add_operations_v0(
        &self,
        apply: bool,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        outpoint: &Bytes36,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<bool, Error> {
        let asset_lock_storage_path = asset_lock_storage_path();
        let query_type = if apply {
            StatefulDirectQuery
        } else {
            StatelessDirectQuery {
                in_tree_using_sums: false,
                query_target: QueryTargetValue(36),
            }
        };
        self.grove_has_raw(
            (&asset_lock_storage_path).into(),
            outpoint.as_slice(),
            query_type,
            transaction,
            drive_operations,
            drive_version,
        )
    }
}
