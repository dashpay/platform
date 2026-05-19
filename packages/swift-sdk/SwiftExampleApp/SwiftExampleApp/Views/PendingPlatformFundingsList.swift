// PendingPlatformFundingsList.swift
// SwiftExampleApp
//
// Wallet-scoped "Pending Platform Funding" surface that mirrors the
// identity-side `PendingRegistrationsList` + `ResumableRegistrationsList`
// pair. Two distinct row sources are merged here:
//
//   1. In-flight controllers from `AddressFundingCoordinator` — the
//      live submit-still-running case.
//   2. Orphaned `PersistentAssetLock` rows with
//      `fundingTypeRaw == AssetLockAddressTopUp` (4) and
//      `statusRaw ∈ [1, 3]` — the crash-recovery case where the user
//      killed the app between asset-lock broadcast and ST submission.
//
// Anti-join: an orphaned lock is hidden if its outpoint is already
// claimed by an in-flight controller. (We index by outpoint here
// rather than by the `(walletId, platformAccountIndex, recipientHash)`
// triple because the orphaned lock doesn't know its recipient — the
// recipient is picked at ST-submit time.)

import SwiftUI
import SwiftData
import SwiftDashSDK

/// Section view backing the Wallet Detail screen's "Pending Platform
/// Funding" surface for a single wallet. Observes
/// `AddressFundingCoordinator` directly (`@ObservedObject`) so its
/// `@Published controllers` map mutations trigger SwiftUI re-renders
/// of the in-flight rows.
struct PendingPlatformFundingsList: View {
    @ObservedObject var coordinator: AddressFundingCoordinator
    /// Wallet to scope the section to. The Identities-tab equivalent
    /// is cross-wallet because identities are a global concept; here
    /// the wallet detail screen is already wallet-scoped so we
    /// follow suit.
    let walletId: Data
    /// All asset-lock rows for the wallet. Pre-filtered by the
    /// parent (`WalletDetailView`) so this section doesn't run
    /// another `@Query`.
    let assetLocks: [PersistentAssetLock]
    /// Bound to the parent's "resume sheet" state. Setting non-nil
    /// presents `FundPlatformAddressView` in resume mode.
    @Binding var resumingAssetLock: PersistentAssetLock?

    var body: some View {
        let inFlight = activeControllersForWallet
        let orphans = resumableLocks(excludingControllerOutpoints: Set(inFlight.compactMap { _ in
            // Controllers don't currently store the outpoint of the
            // asset lock they're driving. The de-dupe set therefore
            // never has entries today — but the SwiftData status
            // filter (`>=1, <=3`) already excludes locks that have
            // been Consumed, so an in-flight controller whose lock
            // is mid-transition lands at status 2/3 and would only
            // briefly co-render. The plumbing is here so a future
            // tweak (controller exposes its outpoint after broadcast)
            // can de-dupe by returning a non-nil here.
            nil
        }))

        if !inFlight.isEmpty || !orphans.isEmpty {
            VStack(alignment: .leading, spacing: 8) {
                HStack {
                    Text("Pending Platform Funding (\(inFlight.count + orphans.count))")
                        .font(.headline)
                    Spacer()
                }
                .padding(.horizontal)

                VStack(spacing: 0) {
                    ForEach(Array(inFlight.enumerated()), id: \.element.platformFundingRowID) { idx, controller in
                        PendingPlatformFundingRow(controller: controller)
                            .padding(.horizontal)
                            .padding(.vertical, 10)
                        if idx < inFlight.count - 1 || !orphans.isEmpty {
                            Divider()
                        }
                    }
                    ForEach(Array(orphans.enumerated()), id: \.element.id) { idx, lock in
                        ResumablePlatformFundingRow(
                            lock: lock,
                            onResume: { resumingAssetLock = lock }
                        )
                        .padding(.horizontal)
                        .padding(.vertical, 10)
                        if idx < orphans.count - 1 {
                            Divider()
                        }
                    }
                }
                .background(Color(UIColor.secondarySystemBackground))
                .cornerRadius(10)
                .padding(.horizontal)
            }
        }
    }

    /// In-flight controllers scoped to this wallet, newest-first.
    private var activeControllersForWallet: [AddressFundingController] {
        coordinator.activeControllers().filter { $0.walletId == walletId }
    }

    /// Resumable asset-lock rows for this wallet — fundingType 4
    /// (AssetLockAddressTopUp) and status in 1..3 (Broadcast through
    /// ChainLocked, excluding Consumed). Excludes outpoints already
    /// owned by an in-flight controller.
    private func resumableLocks(
        excludingControllerOutpoints excluded: Set<String>
    ) -> [PersistentAssetLock] {
        assetLocks
            .filter { $0.fundingTypeRaw == 4 }
            .filter { $0.isVisibleAsResumable }
            .filter { !excluded.contains($0.outPointHex) }
    }
}

private extension AddressFundingController {
    /// Composite ForEach id: `(walletId hex)-(platformAccountIndex)-(recipientHash hex)`.
    /// The recipient hash is the within-account discriminator: two
    /// concurrent fund calls to different addresses on the same
    /// account otherwise collide on `(walletId, accountIndex)`.
    var platformFundingRowID: String {
        let walletHex = walletId.map { String(format: "%02x", $0) }.joined()
        let recipientHex = recipientHash.map { String(format: "%02x", $0) }.joined()
        return "\(walletHex)-\(platformAccountIndex)-\(recipientHex)"
    }
}

/// Single row representing an in-flight `AddressFundingController`.
/// Tappable navigation pushes to `AddressFundingProgressView`.
struct PendingPlatformFundingRow: View {
    @ObservedObject var controller: AddressFundingController
    @EnvironmentObject var walletManager: PlatformWalletManager

    var body: some View {
        NavigationLink(destination: AddressFundingProgressView(controller: controller)) {
            VStack(alignment: .leading, spacing: 4) {
                HStack {
                    Image(systemName: phaseIcon)
                        .foregroundColor(phaseTint)
                    Text("Platform Account #\(controller.platformAccountIndex)")
                        .font(.body)
                    Spacer()
                    Text(phaseLabel)
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                Text(recipientLabel)
                    .font(.caption2)
                    .foregroundColor(.secondary)
                    .lineLimit(1)
            }
            .padding(.vertical, 2)
        }
        .swipeActions(edge: .trailing, allowsFullSwipe: false) {
            if case .failed = controller.phase {
                Button {
                    walletManager.addressFundingCoordinator.dismiss(
                        walletId: controller.walletId,
                        platformAccountIndex: controller.platformAccountIndex,
                        recipientHash: controller.recipientHash
                    )
                } label: {
                    Label("Dismiss", systemImage: "trash")
                }
                .tint(.red)
            }
        }
    }

    private var recipientLabel: String {
        let prefix = controller.recipientHash
            .prefix(6)
            .map { String(format: "%02x", $0) }
            .joined()
        return "→ addr \(prefix)…"
    }

    private var phaseIcon: String {
        switch controller.phase {
        case .idle: return "circle.dashed"
        case .inFlight: return "arrow.triangle.2.circlepath"
        case .completed: return "checkmark.seal.fill"
        case .failed: return "xmark.octagon.fill"
        }
    }

    private var phaseTint: Color {
        switch controller.phase {
        case .idle, .inFlight: return .blue
        case .completed: return .green
        case .failed: return .red
        }
    }

    private var phaseLabel: String {
        switch controller.phase {
        case .idle: return "Idle"
        case .inFlight: return "Funding…"
        case .completed: return "Done"
        case .failed: return "Failed"
        }
    }
}

/// Single row in the orphaned-asset-lock section. Renders the lock
/// summary (txid prefix, amount, status) plus a compact Resume button
/// that opens `FundPlatformAddressView` in resume mode pre-seeded with
/// the outpoint.
struct ResumablePlatformFundingRow: View {
    let lock: PersistentAssetLock
    let onResume: () -> Void

    var body: some View {
        HStack(alignment: .top) {
            VStack(alignment: .leading, spacing: 4) {
                Text("Asset Lock \(lock.shortOutPointDisplay)")
                    .font(.body)
                    .lineLimit(1)
                HStack(spacing: 6) {
                    Text(formatDuffs(lock.amountDuffs))
                        .font(.caption)
                        .foregroundColor(.secondary)
                    Text("·")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    Text(lock.statusLabel)
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
            Spacer(minLength: 8)
            trailingAffordance
        }
        .padding(.vertical, 2)
    }

    @ViewBuilder
    private var trailingAffordance: some View {
        if lock.canFundIdentity {
            // `canFundIdentity` is identity-named but the predicate
            // it encodes — `statusRaw ∈ {2, 3}` — is exactly the
            // "lock has a usable IS or CL proof" gate the address-
            // funding submit path needs. Naming carryover only.
            Button(action: onResume) {
                Label("Resume", systemImage: "arrow.clockwise")
                    .labelStyle(.titleAndIcon)
                    .font(.callout)
            }
            .buttonStyle(.borderedProminent)
            .controlSize(.small)
        } else {
            HStack(spacing: 6) {
                ProgressView()
                    .controlSize(.small)
                Text("Waiting for InstantSend / ChainLock…")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
        }
    }

    private func formatDuffs(_ amountDuffs: Int64) -> String {
        let dash = Double(amountDuffs) / 1e8
        return String(format: "%g DASH", dash)
    }
}
