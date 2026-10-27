// WalletSyncCoordinatorLifecycle.swift
// SwiftExampleApp

/// Execute each wallet-sync coordinator lifecycle operation independently.
/// A failure in one coordinator is reported but never prevents the remaining
/// coordinators from receiving their stop or check/start operation.
@MainActor
private func runWalletSyncCoordinatorOperationsBestEffort(
    _ operations: [(name: String, perform: () throws -> Void)],
    onError: (String, Error) -> Void
) {
    for operation in operations {
        do {
            try operation.perform()
        } catch {
            onError(operation.name, error)
        }
    }
}

/// Stop every wallet-scoped sync coordinator, even when an earlier stop fails.
@MainActor
func stopWalletSyncCoordinatorsBestEffort(
    stopPlatformAddress: @escaping () throws -> Void,
    stopShielded: @escaping () throws -> Void,
    stopDashPay: @escaping () throws -> Void,
    stopDpns: @escaping () throws -> Void,
    onError: (String, Error) -> Void
) {
    runWalletSyncCoordinatorOperationsBestEffort(
        [
            ("platform address", stopPlatformAddress),
            ("shielded", stopShielded),
            ("DashPay", stopDashPay),
            ("DPNS", stopDpns),
        ],
        onError: onError
    )
}

/// Check and, when necessary, start every wallet-scoped sync coordinator.
/// Each closure owns one coordinator's check/start pair so that pair cannot
/// block any later coordinator's check/start pair.
@MainActor
func ensureWalletSyncCoordinatorsRunningBestEffort(
    ensurePlatformAddress: @escaping () throws -> Void,
    ensureShielded: @escaping () throws -> Void,
    ensureDashPay: @escaping () throws -> Void,
    ensureDpns: @escaping () throws -> Void,
    onError: (String, Error) -> Void
) {
    runWalletSyncCoordinatorOperationsBestEffort(
        [
            ("platform address", ensurePlatformAddress),
            ("shielded", ensureShielded),
            ("DashPay", ensureDashPay),
            ("DPNS", ensureDpns),
        ],
        onError: onError
    )
}
