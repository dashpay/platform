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

//! Platform Init
//!

use crate::block::BlockExecutionContext;
use crate::error::Error;
use rs_drive::drive::config::DriveConfig;
use rs_drive::drive::Drive;
use std::cell::RefCell;
use std::path::Path;

/// Platform
pub struct Platform {
    /// Drive
    pub drive: Drive,
    /// Block execution context
    pub block_execution_context: RefCell<Option<BlockExecutionContext>>,
}

impl Platform {
    /// Open Platform with Drive and block execution context.
    pub fn open<P: AsRef<Path>>(path: P, config: Option<DriveConfig>) -> Result<Self, Error> {
        let drive = Drive::open(path, config).map_err(Error::Drive)?;
        Ok(Platform {
            drive,
            block_execution_context: RefCell::new(None),
        })
    }
}
