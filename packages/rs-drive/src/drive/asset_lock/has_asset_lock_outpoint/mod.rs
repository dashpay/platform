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

//! Implements in Drive functions which check if a given `outpoint` is present as an asset lock in the transaction and potentially applies operations to it.

mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
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
    /// * `outpoint` - An `OutPoint` reference to be checked in the transaction.
    /// * `transaction` - The `TransactionArg` in which to check for the `outpoint`.
    ///
    /// # Returns
    ///
    /// Returns a `Result` which is `Ok` if the outpoint exists in the transaction or an `Error` otherwise.
    pub fn has_asset_lock_outpoint(
        &self,
        outpoint: &Bytes36,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<bool, Error> {
        match drive_version.methods.asset_lock.has_asset_lock_outpoint {
            0 => self.has_asset_lock_outpoint_v0(outpoint, transaction, drive_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "has_asset_lock_outpoint".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Checks if a given `outpoint` is present as an asset lock in the transaction and potentially applies operations to it.
    ///
    /// # Arguments
    ///
    /// * `apply` - A boolean which when true applies the operations to the asset lock.
    /// * `drive_operations` - A mutable reference to a vector of `LowLevelDriveOperation` to be possibly executed.
    /// * `outpoint` - An `OutPoint` reference to be checked in the transaction.
    /// * `transaction` - The `TransactionArg` in which to check for the `outpoint`.
    ///
    /// # Returns
    ///
    /// Returns a `Result` which is `Ok` if the outpoint exists in the transaction or an `Error` otherwise.
    pub(crate) fn has_asset_lock_outpoint_add_operations(
        &self,
        apply: bool,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        outpoint: &Bytes36,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<bool, Error> {
        match drive_version.methods.asset_lock.has_asset_lock_outpoint {
            0 => self.has_asset_lock_outpoint_add_operations_v0(
                apply,
                drive_operations,
                outpoint,
                transaction,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "has_asset_lock_outpoint_add_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
