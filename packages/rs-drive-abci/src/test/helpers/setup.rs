//! Platform setup helpers.
//!
//! This module defines helper functions related to setting up Platform.
//!

use std::ops::{Deref, DerefMut};

use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
#[cfg(any(feature = "mocks", test))]
use crate::rpc::core::MockCoreRPCLike;
use crate::{config::PlatformConfig, rpc::core::DefaultCoreRPC};
use dpp::block::block_info::BlockInfo;
use dpp::document::transfer::Transferable;
use dpp::nft::TradeMode;
use dpp::prelude::{CoreBlockHeight, DataContract, TimestampMillis};
use dpp::tests::json_document::json_document_to_contract;
use dpp::version::PlatformVersion;
use dpp::version::ProtocolVersion;
use drive::util::storage_flags::StorageFlags;
use tempfile::TempDir;

/// A test platform builder.
pub struct TestPlatformBuilder {
    config: Option<PlatformConfig>,
    initial_protocol_version: Option<ProtocolVersion>,
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

    /// Add initial protocol version
    pub fn with_initial_protocol_version(
        mut self,
        initial_protocol_version: ProtocolVersion,
    ) -> Self {
        self.initial_protocol_version = Some(initial_protocol_version);
        self
    }

    /// Add initial protocol version as latest
    pub fn with_latest_protocol_version(mut self) -> Self {
        self.initial_protocol_version = Some(PlatformVersion::latest().protocol_version);
        self
    }

    /// Create a new temp platform with a mock core rpc
    pub fn build_with_mock_rpc(self) -> TempPlatform<MockCoreRPCLike> {
        let use_initial_protocol_version =
            if let Some(initial_protocol_version) = self.initial_protocol_version {
                // We should use the latest if nothing is set
                Some(initial_protocol_version)
            } else {
                Some(PlatformVersion::latest().protocol_version)
            };
        let platform = Platform::<MockCoreRPCLike>::open(
            self.tempdir.path(),
            self.config,
            use_initial_protocol_version,
        )
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
            initial_protocol_version: None,
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
        let platform_state = self.platform.state.load();
        self.platform
            .create_genesis_state(
                1,
                Default::default(),
                None,
                PlatformVersion::get(platform_state.current_protocol_version_in_consensus())
                    .expect("expected to get platform version"),
            )
            .expect("should create root tree successfully");

        self
    }

    /// Sets Platform to genesis state with information that came at activation.
    pub fn set_genesis_state_with_activation_info(
        self,
        genesis_time: TimestampMillis,
        start_core_block_height: CoreBlockHeight,
    ) -> Self {
        let platform_state = self.platform.state.load();
        let platform_version =
            PlatformVersion::get(platform_state.current_protocol_version_in_consensus())
                .expect("expected to get platform version");
        self.platform
            .create_genesis_state(
                start_core_block_height,
                genesis_time,
                None,
                platform_version,
            )
            .expect("should create root tree successfully");

        self
    }

    /// Rebuilds Platform from the tempdir as if it was destroyed and restarted
    pub fn open_with_tempdir(tempdir: TempDir, config: PlatformConfig) -> Self {
        let platform = Platform::<MockCoreRPCLike>::open(tempdir.path(), Some(config), None)
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
