use platform_version::version::PlatformVersion;

/// Whether `denomination` is a member of `platform_version`'s exit-denomination set for
/// `IdentityCreateFromShieldedPool`.
///
/// The single definition of an allowed identity-create exit: transition structure validation,
/// the transition builder and wallet preflights all call it. Consensus-critical — the set is
/// versioned in the tables, so a new rule is a table change, never an edit here. Every member is
/// positive, so zero is never allowed; versions without the shielded pool allow nothing.
pub fn is_allowed_identity_create_denomination(
    denomination: u64,
    platform_version: &PlatformVersion,
) -> bool {
    platform_version
        .drive_abci
        .validation_and_processing
        .event_constants
        .shielded_identity_create_denominations
        .contains(&denomination)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 0.1 DASH in credits — a member of every shipped exit-denomination set.
    const ONE_TENTH_DASH: u64 = 10_000_000_000;

    #[test]
    fn should_allow_a_member_denomination() {
        assert!(is_allowed_identity_create_denomination(
            ONE_TENTH_DASH,
            PlatformVersion::latest()
        ));
    }

    #[test]
    fn should_reject_zero_and_non_member_denominations() {
        for denomination in [0, 12_345, ONE_TENTH_DASH + 1, u64::MAX] {
            assert!(
                !is_allowed_identity_create_denomination(denomination, PlatformVersion::latest()),
                "denomination {denomination} must not be allowed"
            );
        }
    }

    #[test]
    fn should_reject_every_denomination_before_the_shielded_pool_exists() {
        assert!(!is_allowed_identity_create_denomination(
            ONE_TENTH_DASH,
            PlatformVersion::first()
        ));
    }
}
