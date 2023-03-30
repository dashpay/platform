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

use crate::config::PlatformConfig;
use crate::platform::Platform;
use crate::test::fixture::abci::static_system_identity_public_keys;
use tempfile::TempDir;
#[cfg(feature = "fixtures-and-mocks")]
use crate::rpc::core::MockCoreRPCLike;
#[cfg(not(feature = "fixtures-and-mocks"))]
use crate::rpc::core::DefaultCoreRPC;

/// A function which sets up Platform.
pub fn setup_platform_raw<CoreRPCLike>(config: Option<PlatformConfig>) -> Platform<CoreRPCLike> {
    let tmp_dir = TempDir::new().unwrap();

    #[cfg(not(feature = "fixtures-and-mocks"))]
        let mut platform: Platform<DefaultCoreRPC> =
            Platform::open_with_default_core_rpc(tmp_dir, config).expect("should open Platform successfully");
    #[cfg(feature = "fixtures-and-mocks")]
        let mut platform: Platform<MockCoreRPCLike> =
        Platform::open_with_mock_core_rpc(tmp_dir, config).expect("should open Platform successfully");

    platform
}

/// A function which sets up Platform with its initial state structure.
pub fn setup_platform_with_initial_state_structure<CoreRPCLike>(config: Option<PlatformConfig>) -> Platform<CoreRPCLike> {
    let mut platform = setup_platform_raw(config);

    platform
        .drive
        .create_initial_state_structure(None)
        .expect("should create root tree successfully");

    #[cfg(feature = "fixtures-and-mocks")]
    platform.mock_core_rpc_client();

    platform
}

/// A function which sets up Platform with its genesis state
pub fn setup_platform_with_genesis_state<CoreRPCLike>(config: Option<PlatformConfig>) -> Platform<CoreRPCLike> {
    let platform = setup_platform_raw(config);

    platform
        .create_genesis_state(
            Default::default(),
            static_system_identity_public_keys(),
            None,
        )
        .expect("should create root tree successfully");

    platform
}
