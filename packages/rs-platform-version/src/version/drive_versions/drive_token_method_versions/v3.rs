use super::v2::DRIVE_TOKEN_METHOD_VERSIONS_V2;
use super::DriveTokenMethodVersions;

/// Protocol 15 includes shielded pool balances in token conservation.
pub const DRIVE_TOKEN_METHOD_VERSIONS_V3: DriveTokenMethodVersions = DriveTokenMethodVersions {
    calculate_total_tokens_balance: 1,
    ..DRIVE_TOKEN_METHOD_VERSIONS_V2
};
