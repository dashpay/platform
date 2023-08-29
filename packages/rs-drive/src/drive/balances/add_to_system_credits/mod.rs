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

mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Adds to the total platform system credits when:
    /// - we create an identity
    /// - we top up an identity
    /// - through the block reward
    ///
    /// # Arguments
    ///
    /// * `amount` - The amount of system credits to be added.
    /// * `transaction` - A `TransactionArg` object representing the transaction to be used for adding to the system credits.
    /// * `platform_version` - A `PlatformVersion` object specifying the version of Platform.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - If successful, returns `Ok(())`. If an error occurs during the operation, returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function will return an error if the version of Platform is unknown.
    pub fn add_to_system_credits(
        &self,
        amount: u64,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .balances
            .add_to_system_credits
        {
            0 => self.add_to_system_credits_v0(amount, transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_to_system_credits".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
