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

//! Implements in Drive a function which adds estimated costs to a hashmap for adding an asset lock (version 0).

mod add_estimation_costs_for_adding_asset_lock;

use crate::drive::Drive;

use grovedb::batch::KeyInfoPath;

use grovedb::EstimatedLayerInformation;

use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::drive_versions::DriveVersion;

use std::collections::HashMap;

impl Drive {
    /// Add estimated costs to a hashmap for adding an asset lock (version 0).
    ///
    /// This function modifies the provided hashmap, `estimated_costs_only_with_layer_info`,
    /// by inserting two sets of key-value pairs related to the estimation costs for adding an asset lock.
    pub(crate) fn add_estimation_costs_for_adding_asset_lock(
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version
            .methods
            .asset_lock
            .add_estimation_costs_for_adding_asset_lock
        {
            0 => {
                Self::add_estimation_costs_for_adding_asset_lock_v0(
                    estimated_costs_only_with_layer_info,
                );
                Ok(())
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_estimation_costs_for_adding_asset_lock".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
