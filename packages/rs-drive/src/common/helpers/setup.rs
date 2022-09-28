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

//! Drive Setup Helpers.
//!
//! Defines helper functions pertinent to setting up Drive.
//!

use crate::drive::config::DriveConfig;
use crate::drive::Drive;
use tempfile::TempDir;

/// Struct with options regarding setting up fee pools.
pub struct SetupFeePoolsOptions {
    /// Bool indicating whether the fee pool structure should be applied upon setup.
    pub apply_fee_pool_structure: bool,
}

impl Default for SetupFeePoolsOptions {
    /// The default is true for applying the fee pool structure upon setting up fee pools.
    fn default() -> SetupFeePoolsOptions {
        SetupFeePoolsOptions {
            apply_fee_pool_structure: true,
        }
    }
}

/// Sets up Drive using the optionally given Drive configuration settings.
pub fn setup_drive(drive_config: Option<DriveConfig>) -> Drive {
    let tmp_dir = TempDir::new().unwrap();
    let drive: Drive = Drive::open(tmp_dir, drive_config).expect("should open Drive successfully");

    drive
}

/// Sets up Drive with the initial state structure.
pub fn setup_drive_with_initial_state_structure() -> Drive {
    let drive = setup_drive(None);
    drive
        .create_initial_state_structure(None)
        .expect("should create root tree successfully");

    drive
}
