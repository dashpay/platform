use crate::error::execution::ExecutionError;
use crate::error::Error;
use dpp::prelude::BlockHeight;
use dpp::version::patches::PATCHES;
use dpp::version::PlatformVersion;
use dpp::version::INITIAL_PROTOCOL_VERSION;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::platform_types::platform_state::v0::{
    PlatformStateV0Methods, PlatformStateV0PrivateMethods,
};
use crate::platform_types::platform_state::PlatformState;

static PATCHED_PROTOCOL_VERSION: AtomicU32 = AtomicU32::new(INITIAL_PROTOCOL_VERSION);

impl PlatformState {
    /// Apply all patches to platform version up to specified height
    /// It changes protocol version to function version mapping to apply hotfixes
    /// PlatformVersion can be already patched, so a patch will be applied on the top
    ///
    /// This function appends the patch to PlatformState and returns patched version
    pub fn apply_all_patches_to_platform_version_up_to_height(
        &mut self,
        height: BlockHeight,
    ) -> Result<Option<&'static PlatformVersion>, Error> {
        if self.patched_platform_version().is_some() {
            return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                "platform version already patched",
            )));
        }

        let protocol_version = self.current_protocol_version_in_consensus();

        let patches = PATCHES.read().unwrap();

        // Find a patch that matches protocol version first
        let Some(patches_per_heights) = patches.get(&protocol_version) else {
            return Ok(None);
        };

        if patches_per_heights.is_empty() {
            return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                "patches per height can't be empty",
            )));
        }

        let platform_version_to_patch = self.current_platform_version()?;

        let mut patched_version = platform_version_to_patch.clone();

        // Apply all patches up to specified height
        for (height, patch_fn) in patches_per_heights.range(..=height) {
            patched_version = patch_fn(patched_version);

            tracing::debug!(
                protocol_version,
                height,
                "Applied patch for platform version {} and height {:?}",
                protocol_version,
                height
            );
        }

        // Make patch version as static ref to transparently replace original version
        let boxed_version = Box::new(patched_version);
        let static_patched_version: &'static PlatformVersion = Box::leak(boxed_version);

        // Set patched version to the Platform (execution) state that will be used
        // instead of the current version
        self.set_patched_platform_version(Some(static_patched_version));

        Ok(Some(static_patched_version))
    }

    /// Apply a patch to platform version based on specified height
    /// It changes protocol version to function version mapping to apply hotfixes
    /// PlatformVersion can be already patched, so a patch will be applied on the top
    ///
    /// This function appends the patch to PlatformState and returns patched version
    pub fn apply_platform_version_patch_for_height(
        &mut self,
        height: BlockHeight,
    ) -> Result<Option<&'static PlatformVersion>, Error> {
        let protocol_version = self.current_protocol_version_in_consensus();

        // If we switched protocol version we  need to
        // drop patched version from PlatformState
        if self.patched_platform_version().is_some() {
            let previous_protocol_version = PATCHED_PROTOCOL_VERSION.load(Ordering::Relaxed);
            if previous_protocol_version != protocol_version {
                tracing::debug!(
                    protocol_version,
                    height,
                    "Disable patches for platform version {} because we switched to version {}",
                    previous_protocol_version,
                    protocol_version,
                );

                self.set_patched_platform_version(None);
            }
        }

        let patches = PATCHES.read().unwrap();

        // Find a patch that matches protocol version first
        let Some(patches_per_heights) = patches.get(&protocol_version) else {
            return Ok(None);
        };

        // Find a patch that matches block height
        let Some(patch_fn) = patches_per_heights.get(&height) else {
            return Ok(None);
        };

        // Potentially already patched version
        let platform_version_to_patch = self.current_platform_version()?;

        // Apply the patch
        let patched_version = patch_fn(platform_version_to_patch.clone());

        // Make patch version as static ref to transparently replace original version
        let boxed_version = Box::new(patched_version);
        let static_patched_version: &'static PlatformVersion = Box::leak(boxed_version);

        // Set current protocol version if not set yet
        if self.patched_platform_version().is_none() {
            PATCHED_PROTOCOL_VERSION.store(protocol_version, Ordering::Relaxed);
        }

        // Set patched version to the Platform (execution) state that will be used
        // instead of the current version
        self.set_patched_platform_version(Some(static_patched_version));

        tracing::debug!(
            protocol_version,
            height,
            "Applied patch for platform version {} and height {:?}",
            protocol_version,
            height
        );

        Ok(Some(static_patched_version))
    }
}
