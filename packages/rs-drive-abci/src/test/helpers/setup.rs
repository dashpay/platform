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

//! Platform setup helpers.
//!
//! This module defines helper functions related to setting up Platform.
//!

use crate::platform::Platform;
#[cfg(feature = "fixtures-and-mocks")]
use crate::rpc::core::MockCoreRPCLike;
use crate::test::fixture::abci::static_system_identity_public_keys;
use crate::{config::PlatformConfig, rpc::core::DefaultCoreRPC};
use tempfile::TempDir;

pub struct TestPlatformBuilder<C> {
    platform: Platform<C>,
}

impl TestPlatformBuilder<DefaultCoreRPC> {
    pub fn new(config: Option<PlatformConfig>) -> Self {
        let tmp_dir = TempDir::new().unwrap();
        let platform = Platform::<DefaultCoreRPC>::open(tmp_dir, config)
            .expect("should open Platform successfully");

        TestPlatformBuilder { platform }
    }
}

impl TestPlatformBuilder<MockCoreRPCLike> {
    pub fn new(config: Option<PlatformConfig>) -> Self {
        let tmp_dir = TempDir::new().unwrap();
        let platform = Platform::<MockCoreRPCLike>::open(tmp_dir, config)
            .expect("should open Platform successfully");

        TestPlatformBuilder { platform }
    }
}

impl<C> TestPlatformBuilder<C> {
    /// A function which sets initial state structure for Platform.
    pub fn set_initial_state_structure(&mut self) -> &mut Self {
        self.platform
            .drive
            .create_initial_state_structure(None)
            .expect("should create root tree successfully");

        &mut self
    }

    /// Sets Platform to genesis state.
    pub fn set_genesis_state(&mut self) -> &mut Self {
        self.platform
            .create_genesis_state(
                Default::default(),
                static_system_identity_public_keys(),
                None,
            )
            .expect("should create root tree successfully");

        &mut self
    }

    pub fn build(self) -> Platform<C> {
        self.platform
    }
}
