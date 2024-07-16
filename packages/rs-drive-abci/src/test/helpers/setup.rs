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

use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
#[cfg(any(feature = "mocks", test))]
use crate::rpc::core::MockCoreRPCLike;
use crate::test::fixture::abci::static_system_identity_public_keys_v0;
use crate::{config::PlatformConfig, rpc::core::DefaultCoreRPC};
use dpp::block::block_info::BlockInfo;
use dpp::document::transfer::Transferable;
use dpp::nft::TradeMode;
use dpp::prelude::DataContract;
use dpp::tests::json_document::json_document_to_contract;
use dpp::version::PlatformVersion;
use drive::util::storage_flags::StorageFlags;
use tempfile::TempDir;

/// A test platform builder.
pub struct TestPlatformBuilder {
    config: Option<PlatformConfig>,
    tempdir: TempDir,
}

/// Platform wrapper that takes care of temporary directory.
pub struct TempPlatform<C> {
    /// Platform
    pub platform: Platform<C>,
    /// A temp dir
    pub tempdir: TempDir,
}

impl TestPlatformBuilder {
    /// Create a new test platform builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Add platform config
    pub fn with_config(mut self, config: PlatformConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Create a new temp platform with a mock core rpc
    pub fn build_with_mock_rpc(self) -> TempPlatform<MockCoreRPCLike> {
        let platform = Platform::<MockCoreRPCLike>::open(self.tempdir.path(), self.config)
            .expect("should open Platform successfully");

        TempPlatform {
            platform,
            tempdir: self.tempdir,
        }
    }

    /// Create a new temp platform with a default core rpc
    pub fn build_with_default_rpc(self) -> TempPlatform<DefaultCoreRPC> {
        let platform = Platform::<DefaultCoreRPC>::open(self.tempdir.path(), self.config)
            .expect("should open Platform successfully");

        TempPlatform {
            platform,
            tempdir: self.tempdir,
        }
    }
}

impl Default for TestPlatformBuilder {
    fn default() -> Self {
        let tempdir = TempDir::new().unwrap();
        Self {
            tempdir,
            config: None,
        }
    }
}

impl TempPlatform<MockCoreRPCLike> {
    /// A function which sets initial state structure for Platform.
    pub fn set_initial_state_structure(self) -> Self {
        self.platform
            .drive
            .create_initial_state_structure(
                None,
                self.platform
                    .state
                    .load()
                    .current_platform_version()
                    .expect("expected to get current platform version"),
            )
            .expect("should create root tree successfully");

        self
    }

    /// A function which adds the crypto card game to the state and returns it.
    pub fn with_crypto_card_game_transfer_only(
        self,
        transferable: Transferable,
    ) -> (Self, DataContract) {
        let card_game_path = match transferable {
            Transferable::Never => "tests/supporting_files/contract/crypto-card-game/crypto-card-game-not-transferable.json",
            Transferable::Always => "tests/supporting_files/contract/crypto-card-game/crypto-card-game-all-transferable.json",
        };

        let platform_version = self
            .platform
            .state
            .load()
            .current_platform_version()
            .expect("expected to get current platform version");

        // let's construct the grovedb structure for the card game data contract
        let card_game_contract = json_document_to_contract(card_game_path, true, platform_version)
            .expect("expected to get data contract");
        self.drive
            .apply_contract(
                &card_game_contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply contract successfully");

        (self, card_game_contract)
    }

    /// A function which adds the crypto card game to the state and returns it.
    pub fn with_crypto_card_game_nft(self, marketplace: TradeMode) -> (Self, DataContract) {
        let card_game_path = match marketplace {
            TradeMode::DirectPurchase => "tests/supporting_files/contract/crypto-card-game/crypto-card-game-direct-purchase.json",
            _ => panic!("not yet supported")
        };

        let platform_version = self
            .platform
            .state
            .load()
            .current_platform_version()
            .expect("expected to get current platform version");

        // let's construct the grovedb structure for the card game data contract
        let card_game_contract = json_document_to_contract(card_game_path, true, platform_version)
            .expect("expected to get data contract");
        self.drive
            .apply_contract(
                &card_game_contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply contract successfully");

        (self, card_game_contract)
    }

    /// Sets Platform to genesis state.
    pub fn set_genesis_state(self) -> Self {
        self.platform
            .create_genesis_state(
                Default::default(),
                static_system_identity_public_keys_v0().into(),
                None,
                PlatformVersion::latest(),
            )
            .expect("should create root tree successfully");

        self
    }

    /// Rebuilds Platform from the tempdir as if it was destroyed and restarted
    pub fn open_with_tempdir(tempdir: TempDir, config: PlatformConfig) -> Self {
        let platform = Platform::<MockCoreRPCLike>::open(tempdir.path(), Some(config))
            .expect("should open Platform successfully");

        Self { platform, tempdir }
    }
}

impl<C> Deref for TempPlatform<C> {
    type Target = Platform<C>;

    fn deref(&self) -> &Self::Target {
        &self.platform
    }
}

impl<C> DerefMut for TempPlatform<C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.platform
    }
}
