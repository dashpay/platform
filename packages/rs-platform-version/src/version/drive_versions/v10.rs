use crate::version::drive_versions::v9::DRIVE_VERSION_V9;
use crate::version::drive_versions::{
    DriveBalancesMethodVersions, DriveInitializationMethodVersions, DriveMethodVersions,
    DriveVersion,
};

/// Drive version 10.
/// Introduced in protocol v17, the 5.0 protocol version, for the contract
/// credits root tree: a sum tree at root key 100 that holds, from later
/// changes, one sum subtree per contract with one sum item per credit bucket.
///
/// * **Genesis**: `initialization.create_initial_state_structure` 4 -> 5
///   inserts the empty `ContractCredits` sum tree as a standalone root
///   insert right after `ShieldedBalances`; the upgrade path creates the same
///   element with an insert-if-not-exists on the first block at v17, so a
///   fresh genesis and an upgraded node hold a byte-identical root element.
/// * **Credit conservation**: `balances.calculate_total_credits_balance`
///   2 -> 3 reads the tree's aggregate as the sixth term of the equation.
///   Contracts wiped later have their subtree wrapped in a not-summed
///   element, so the aggregate only ever covers live contract credits.
///
/// Everything else matches `DRIVE_VERSION_V9`.
pub const DRIVE_VERSION_V10: DriveVersion = DriveVersion {
    methods: DriveMethodVersions {
        initialization: DriveInitializationMethodVersions {
            create_initial_state_structure: 5, // changed in v10: adds the contract credits root sum tree (v4 added the ContractGroups root tree)
        },
        balances: DriveBalancesMethodVersions {
            calculate_total_credits_balance: 3, // changed in v10: ContractCredits root tree adds a sixth term to the equation
            ..DRIVE_VERSION_V9.methods.balances
        },
        ..DRIVE_VERSION_V9.methods
    },
    ..DRIVE_VERSION_V9
};
