mod patch_0_15_example;

use crate::error::Error;
use crate::execution::platform_events::block_start::patch_platform_version::patch_0_15_example::patch_0_15_example;
use dpp::block::block_info::BlockInfo;
use dpp::prelude::BlockHeight;
use dpp::util::deserializer::ProtocolVersion;
use dpp::version::PlatformVersion;
use indexmap::IndexMap;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::ops::Range;
use std::sync::RwLock;

use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;

static PREVIOUS_PATCH_PATTERN: RwLock<Option<(ProtocolVersion, Range<BlockHeight>)>> =
    RwLock::new(None);

type PatchFn = fn(PlatformVersion) -> PlatformVersion;

type HeightToPatchRanges = IndexMap<Range<BlockHeight>, PatchFn>;

static PATCHES: Lazy<HashMap<ProtocolVersion, HeightToPatchRanges>> = Lazy::new(|| {
    HashMap::from_iter(vec![
        (
            1,
            IndexMap::from_iter(vec![
                (15..30, patch_0_15_example as PatchFn),
                (30..45, patch_0_15_example as PatchFn),
                // (45..BlockHeight::MAX, patch_0_15_example as PatchFn),
            ]),
        ),
        // (
        //     2,
        //     IndexMap::from_iter(vec![(15..24, patch_0_15_example as PatchFn)]),
        // ),
    ])
});

/// Patch platform version to change function versions to fix chain halt bugs
pub fn patch_platform_version(
    block_info: &BlockInfo,
    current_platform_version: &PlatformVersion,
    block_platform_state: &mut PlatformState,
) -> Result<(), Error> {
    // Check if a patch that matches protocol version and block height is already applied
    if block_platform_state.patched_platform_version().is_some() {
        let previous_patch_pattern = PREVIOUS_PATCH_PATTERN.read().unwrap();
        if let Some((protocol_version, height_range)) = previous_patch_pattern.as_ref() {
            if protocol_version == &current_platform_version.protocol_version
                && height_range.contains(&block_info.height)
            {
                tracing::trace!(
                    protocol_version = current_platform_version.protocol_version,
                    height = block_info.height,
                    "Continue using patched platform version {} and height range {:?}",
                    protocol_version,
                    height_range
                );

                return Ok(());
            } else {
                let height_range = height_range.clone();
                drop(previous_patch_pattern);

                let mut previous_path_pattern = PREVIOUS_PATCH_PATTERN.write().unwrap();
                *previous_path_pattern = None;

                tracing::debug!(
                    protocol_version = current_platform_version.protocol_version,
                    height = block_info.height,
                    "Disable patch for platform version {} and height range {:?}",
                    current_platform_version.protocol_version,
                    height_range
                );
            }
        }
    }

    // Find a patch that matches protocol version first
    let Some(height_to_patch_ranges) = PATCHES.get(&current_platform_version.protocol_version)
    else {
        // Drop patch
        block_platform_state.set_patched_platform_version(None);

        return Ok(());
    };

    // Find a patch that matches block height
    let Some((height_range, patch_fn)) = height_to_patch_ranges
        .iter()
        .find(|(height_range, _)| height_range.contains(&block_info.height))
    else {
        // Drop patch
        block_platform_state.set_patched_platform_version(None);

        return Ok(());
    };

    // Apply the patch
    let patched_version = patch_fn(current_platform_version.clone());

    // Make patch version as static ref to transparently replace original version
    let boxed_version = Box::new(patched_version);
    let static_version: &'static PlatformVersion = Box::leak(boxed_version);

    // Set patched version to the Platform (execution) state that will be used
    // instead of the current version
    block_platform_state.set_patched_platform_version(Some(static_version));

    // Store the patch pattern to avoid applying the same patch multiple times
    PREVIOUS_PATCH_PATTERN.write().unwrap().replace((
        current_platform_version.protocol_version,
        height_range.clone(),
    ));

    tracing::debug!(
        protocol_version = current_platform_version.protocol_version,
        height = block_info.height,
        "Apply patch for platform version {} and height range {:?}",
        current_platform_version.protocol_version,
        height_range
    );

    Ok(())
}
