use platform_version::version::PlatformVersion;

/// Bytes the Halo 2 proof and the action itself add to a serialized shielded transition for each
/// Orchard action. Measured on the serialized transitions: every extra action of a spend or
/// `ShieldFromAssetLock` transition adds exactly this much (2 actions 8,189 B, 3 actions
/// 10,870 B, 6 actions 18,913 B for a transfer).
const SHIELDED_ACTION_WIRE_SIZE: u64 = 2_681;

/// Bytes of a serialized shielded transition that do not depend on its action count: the proof
/// base, the anchor, the binding signature and the transition's own fields. Measured for 2 actions:
/// transfer 2,827 B, unshield 2,848 B, withdrawal with a P2PKH script 2,855 B, identity top-up
/// 2,859 B, `ShieldFromAssetLock` 2,932 B (6 actions: 19,018 B). The constant is the largest of
/// these rounded up, so it holds for every kind and leaves room for a longer Core script.
const SHIELDED_TRANSITION_FIXED_WIRE_SIZE: u64 = 3_000;

/// The most Orchard actions one shielded transition can carry on `platform_version`: the
/// smaller of `max_shielded_transition_actions` and the number of actions whose serialized
/// transition fits `max_state_transition_size`. A spend carries one action per note, so this is
/// also the most notes one spend can take.
///
/// The size limit, not the action cap, binds today: with the 20 KiB limit 6 actions fit
/// (about 19 KB) and 7 do not (about 21.7 KB). The size of a transition is its fixed part plus
/// [`SHIELDED_ACTION_WIRE_SIZE`] per action, and the fixed part differs by a few dozen bytes
/// between the kinds, so the largest measured one is used for all of them.
pub fn max_shielded_actions_per_transition(platform_version: &PlatformVersion) -> u16 {
    let limits = &platform_version.system_limits;
    let fitting = limits
        .max_state_transition_size
        .saturating_sub(SHIELDED_TRANSITION_FIXED_WIRE_SIZE)
        / SHIELDED_ACTION_WIRE_SIZE;
    u16::try_from(fitting)
        .unwrap_or(u16::MAX)
        .min(limits.max_shielded_transition_actions)
}
