use crate::identity::state_transition::asset_lock_proof::AssetLockProof;
use platform_version::version::PlatformVersion;

/// The most Orchard actions one shielded transition can carry on `platform_version`: the
/// smaller of `max_shielded_transition_actions` and the number of actions whose serialized
/// transition fits `max_state_transition_size`. A spend carries one action per note, so this is
/// also the most notes one spend can take.
///
/// The size limit, not the action cap, binds today: with the 20 KiB limit 6 actions fit
/// (about 19 KB) and 7 do not (about 21.7 KB). The size of a transition is its fixed part
/// (`shielded_transition_fixed_wire_size`) plus `shielded_action_wire_size` per action, both
/// measured and held in `platform_version.system_limits`; the fixed part differs by a few dozen
/// bytes between the kinds, so the largest measured one is used for all of them.
pub fn max_shielded_actions_per_transition(platform_version: &PlatformVersion) -> u16 {
    let limits = &platform_version.system_limits;
    let fitting = limits
        .max_state_transition_size
        .saturating_sub(limits.shielded_transition_fixed_wire_size)
        .checked_div(limits.shielded_action_wire_size)
        // A table with no per-action size bounds nothing by size: the action cap decides.
        .unwrap_or(u64::MAX);
    u16::try_from(fitting)
        .unwrap_or(u16::MAX)
        .min(limits.max_shielded_transition_actions)
}

/// The most Orchard actions a `ShieldFromAssetLock` carrying `asset_lock_proof` can have on
/// `platform_version`, when the transition has to fit `max_state_transition_size`.
///
/// [`max_shielded_actions_per_transition`] assumes the fixed part of a transition that carries
/// a ChainLock proof, which names only an outpoint and a height. An InstantSend proof embeds the
/// whole funding transaction and its InstantLock instead, about 184 bytes per funding input
/// (measured on built transitions: 6 actions are 19,420 B with 1 input, 20,528 B with 7, over
/// the 20,480 B limit), so a lock with many inputs fits fewer actions. The bound adds the proof's
/// own serialized length to the fixed part, so it holds for any proof; it counts the ChainLock
/// proof already inside `shielded_transition_fixed_wire_size` twice, a few dozen bytes.
///
/// It can be below the 2 actions a shielded transition has at least: the lock then does not
/// fit an InstantSend proof at all and only a ChainLock proof can consume it.
pub fn max_shield_from_asset_lock_actions(
    platform_version: &PlatformVersion,
    asset_lock_proof: &AssetLockProof,
) -> u16 {
    let limits = &platform_version.system_limits;
    let proof_size = asset_lock_proof
        .serialized_len_in_state_transition()
        .map_or(u64::MAX, |len| len as u64);
    let fixed = limits
        .shielded_transition_fixed_wire_size
        .saturating_add(proof_size);
    let fitting = limits
        .max_state_transition_size
        .saturating_sub(fixed)
        .checked_div(limits.shielded_action_wire_size)
        .unwrap_or(u64::MAX);
    u16::try_from(fitting)
        .unwrap_or(u16::MAX)
        .min(limits.max_shielded_transition_actions)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The wire-size model of the latest protocol version: the measured per-action and fixed
    /// sizes, under which 6 actions fit the 20 KiB limit and 7 do not.
    #[test]
    fn should_pin_the_shielded_wire_size_model_of_the_latest_version() {
        let platform_version = PlatformVersion::latest();
        let limits = &platform_version.system_limits;
        assert_eq!(limits.shielded_action_wire_size, 2_681);
        assert_eq!(limits.shielded_transition_fixed_wire_size, 3_000);

        let size_of = |actions: u64| {
            limits.shielded_transition_fixed_wire_size + actions * limits.shielded_action_wire_size
        };
        assert!(
            size_of(6) <= limits.max_state_transition_size,
            "6 actions fit"
        );
        assert!(size_of(7) > limits.max_state_transition_size, "7 do not");
        assert_eq!(max_shielded_actions_per_transition(platform_version), 6);
    }
}
