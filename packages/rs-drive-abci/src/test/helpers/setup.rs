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

use std::ops::{Deref, DerefMut};

use crate::platform::Platform;
use crate::rpc::core::MockCoreRPCLike;
use crate::test::fixture::abci::static_system_identity_public_keys;
use crate::{config::PlatformConfig, rpc::core::DefaultCoreRPC};
use tempfile::TempDir;

/// A test platform builder.
pub struct TestPlatformBuilder {
    config: Option<PlatformConfig>,
    tempdir: TempDir,
}

/// Platform wrapper that takes care of temporary directory.
pub struct TempPlatform<'a, C> {
    platform: Platform<'a, C>,
    _tempdir: TempDir,
}

impl TestPlatformBuilder {
    /// Create a new test platform builder
    pub fn new() -> Self {
        let tempdir = TempDir::new().unwrap();
        TestPlatformBuilder {
            tempdir,
            config: None,
        }
    }

    /// Add platform config
    pub fn with_config(mut self, config: PlatformConfig) -> Self {
        self.config = Some(config);
        self
    }
}

impl TestPlatformBuilder {
    /// Create a new temp platform with a mock core rpc
    pub fn build_with_mock_rpc<'a>(self) -> TempPlatform<'a, MockCoreRPCLike> {
        let platform = Platform::<MockCoreRPCLike>::open(self.tempdir.path(), self.config)
            .expect("should open Platform successfully");

        TempPlatform {
            platform,
            _tempdir: self.tempdir,
        }
    }

    /// Create a new temp platform with a default core rpc
    pub fn build_with_default_rpc<'a>(self) -> TempPlatform<'a, DefaultCoreRPC> {
        let platform = Platform::<DefaultCoreRPC>::open(self.tempdir.path(), self.config)
            .expect("should open Platform successfully");

        TempPlatform {
            platform,
            _tempdir: self.tempdir,
        }
    }
}

impl<'a, C> TempPlatform<'a, C> {
    /// A function which sets initial state structure for Platform.
    pub fn set_initial_state_structure(self) -> Self {
        self.platform
            .drive
            .create_initial_state_structure(None)
            .expect("should create root tree successfully");

        self
    }

    /// Sets Platform to genesis state.
    pub fn set_genesis_state(self) -> Self {
        self.platform
            .create_genesis_state(
                Default::default(),
                static_system_identity_public_keys(),
                None,
            )
            .expect("should create root tree successfully");

        self
    }
}

impl<'a, C> Deref for TempPlatform<'a, C> {
    type Target = Platform<'a, C>;

    fn deref(&self) -> &Self::Target {
        &self.platform
    }
}

impl<'a, C> DerefMut for TempPlatform<'a, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.platform
    }
}
