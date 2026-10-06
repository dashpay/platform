use crate::fee::Credits;
use crate::ProtocolError;
use dashcore::Network;
use platform_version::version::PlatformVersion;

mod v0;

/// Core's credit pool window on `network`: how many Core blocks before an asset unlock's block
/// lies the balance Core v24 measures the unlock limit from (`CreditPoolPeriodBlocks` in Dash
/// Core's chain parameters), as the protocol version's system limits pin it
/// (`core_credit_pool_window_blocks`, `regtest_core_credit_pool_window_blocks` on regtest).
///
/// # Errors
///
/// `ProtocolError::CorruptedCodeExecution` when the protocol version predates the
/// Core-anchored withdrawal limit and sets no window.
pub fn core_credit_pool_window_blocks(
    network: Network,
    platform_version: &PlatformVersion,
) -> Result<u32, ProtocolError> {
    let system_limits = &platform_version.system_limits;
    let window_blocks = match network {
        Network::Mainnet | Network::Testnet | Network::Devnet => {
            system_limits.core_credit_pool_window_blocks
        }
        Network::Regtest => system_limits.regtest_core_credit_pool_window_blocks,
    };
    window_blocks.ok_or_else(|| {
        ProtocolError::CorruptedCodeExecution(
            "the protocol version sets no Core credit pool window".to_string(),
        )
    })
}

/// Returns how much Core's credit pool may still give up to asset unlocks, given its balance
/// now and its balance at the start of the window the limit is measured over, both in credits.
///
/// This is the Core-anchored half of the withdrawal limit: a stricter copy of Core v24's own
/// asset unlock rule, so Platform never pools a withdrawal Core will refuse to mine. The pool
/// may end no lower than its window start balance minus an allowed drop
/// (`core_credit_pool_unlock_limit_percent` of that balance, at least
/// `core_credit_pool_unlock_limit_floor`); what it gained since the window start (asset locks,
/// the per-block Platform reward) is withdrawable on top, and the pool can never go negative.
///
/// # Parameters
///
/// * `balance`: Core's credit pool balance now, in credits.
/// * `window_start_balance`: Core's credit pool balance at the window start, in credits; `0`
///   when that block has no credit pool.
/// * `platform_version`: The platform version.
///
/// # Returns
///
/// * `Ok(Credits)`: The credits that may still be unlocked, between `0` and `balance`.
/// * `Err(ProtocolError)`: When the method version is unknown or not active, or the system
///   limits it reads are not configured.
pub fn core_credit_pool_unlock_limit(
    balance: Credits,
    window_start_balance: Credits,
    platform_version: &PlatformVersion,
) -> Result<Credits, ProtocolError> {
    match platform_version.dpp.methods.core_credit_pool_unlock_limit {
        Some(0) => {
            v0::core_credit_pool_unlock_limit_v0(balance, window_start_balance, platform_version)
        }
        Some(version) => Err(ProtocolError::UnknownVersionMismatch {
            method: "core_credit_pool_unlock_limit".to_string(),
            known_versions: vec![0],
            received: version,
        }),
        None => Err(ProtocolError::UnknownVersionError(
            "core_credit_pool_unlock_limit is not active in this protocol version".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dash_to_credits;

    #[test]
    fn should_use_cores_window_of_each_network() {
        let v14 = PlatformVersion::get(14).expect("expected protocol version 14");
        for (network, window_blocks) in [
            (Network::Mainnet, 576),
            (Network::Testnet, 576),
            (Network::Devnet, 576),
            (Network::Regtest, 100),
        ] {
            assert_eq!(
                core_credit_pool_window_blocks(network, v14).expect("expected the window"),
                window_blocks
            );
        }

        let v13 = PlatformVersion::get(13).expect("expected protocol version 13");
        assert!(core_credit_pool_window_blocks(Network::Mainnet, v13).is_err());
    }

    #[test]
    fn should_only_exist_from_protocol_version_14() {
        let v13 = PlatformVersion::get(13).expect("expected protocol version 13");
        assert!(core_credit_pool_unlock_limit(
            dash_to_credits!(10000),
            dash_to_credits!(10000),
            v13
        )
        .is_err());

        let v14 = PlatformVersion::get(14).expect("expected protocol version 14");
        // 15% of a 20,000 Dash pool that has not moved.
        assert_eq!(
            core_credit_pool_unlock_limit(dash_to_credits!(20000), dash_to_credits!(20000), v14)
                .expect("expected the limit"),
            dash_to_credits!(3000)
        );
    }
}
