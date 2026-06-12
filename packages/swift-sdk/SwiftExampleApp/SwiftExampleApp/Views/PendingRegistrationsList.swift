import SwiftUI
import SwiftData
import SwiftDashSDK

/// Section wrapper that observes a `RegistrationCoordinator` directly
/// so its `@Published controllers` map mutations trigger SwiftUI
/// re-renders. Hosted inside `IdentitiesContentView` — separated out
/// because nesting an `@ObservedObject` inside a `@ViewBuilder`
/// computed property doesn't subscribe (the property recomputes per
/// parent body call, never owning a stable reference).
struct PendingRegistrationsList: View {
    @ObservedObject var coordinator: RegistrationCoordinator

    var body: some View {
        let active = coordinator.activeControllers()
        if !active.isEmpty {
            Section("Pending Registrations") {
                // `identityIndex` is per-wallet, NOT globally unique.
                // Two wallets registering identities at the same slot
                // (e.g. both starting at #1) would collide on the
                // ForEach diff and SwiftUI would collapse / replace
                // one row with the other. Composite `(walletId,
                // identityIndex)` matches the coordinator's actual
                // key shape.
                ForEach(
                    active,
                    id: \.registrationRowID
                ) { controller in
                    PendingRegistrationRow(controller: controller)
                }
            }
        }
    }
}

private extension IdentityRegistrationController {
    /// Composite `(walletId, identityIndex)` key used by SwiftUI's
    /// `ForEach` to diff Pending Registrations rows. The walletId is
    /// rendered as hex so the resulting `String` is `Hashable` without
    /// any custom conformance work on `Data` / the controller itself.
    var registrationRowID: String {
        "\(walletId.map { String(format: "%02x", $0) }.joined())-\(identityIndex)"
    }
}

/// Single row in the "Pending Registrations" list. Each row hangs off
/// an `IdentityRegistrationController` via `@ObservedObject` so the
/// status label updates as the controller's phase transitions; tapping
/// the row navigates into `RegistrationProgressView` so the user can
/// follow the stage progression in detail.
struct PendingRegistrationRow: View {
    @ObservedObject var controller: IdentityRegistrationController
    @EnvironmentObject var walletManager: PlatformWalletManager

    /// Persisted identity rows for this slot, queried live so the
    /// `.unconfirmed` dismiss gate becomes enabled the moment the
    /// identity-sync writes the `PersistentIdentity` row. Filtered by
    /// `(wallet.walletId, identityIndex)` — the same `(walletId,
    /// identityIndex)` slot key `RegistrationProgressSection` uses to
    /// query its `PersistentAssetLock` row. `controller.walletId` /
    /// `controller.identityIndex` are immutable `let`s, so the predicate
    /// captured in `init` stays correct for the row's lifetime.
    @Query private var slotIdentities: [PersistentIdentity]

    init(controller: IdentityRegistrationController) {
        self.controller = controller
        let walletId = controller.walletId
        let identityIndex = controller.identityIndex
        _slotIdentities = Query(
            filter: #Predicate<PersistentIdentity> { identity in
                identity.wallet?.walletId == walletId
                    && identity.identityIndex == identityIndex
            }
        )
    }

    var body: some View {
        NavigationLink(destination: RegistrationProgressView(controller: controller)) {
            VStack(alignment: .leading, spacing: 4) {
                HStack {
                    Image(systemName: phaseIcon)
                        .foregroundColor(phaseTint)
                    Text("Identity #\(controller.identityIndex)")
                        .font(.body)
                    Spacer()
                    Text(phaseLabel)
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                Text(controller.walletId.prefix(8).map { String(format: "%02x", $0) }.joined() + "…")
                    .font(.caption2)
                    .foregroundColor(.secondary)
                    .lineLimit(1)
            }
            .padding(.vertical, 2)
        }
        .swipeActions(edge: .trailing, allowsFullSwipe: false) {
            // `.failed` is always dismissable. `.unconfirmed` only becomes
            // dismissable once the matching `PersistentIdentity` row appears
            // via sync (see `isDismissable`): the persisted `isUsed`
            // reservation is best-effort, so until the identity row lands the
            // live controller is a load-bearing guard keeping the slot
            // un-selectable — dismissing it early could let the same index be
            // re-selected and burn funds against the registered-key-hash check.
            // Dismissing only drops the Pending row; it never undoes the
            // on-chain registration.
            if isDismissable {
                Button {
                    walletManager.registrationCoordinator.dismiss(
                        walletId: controller.walletId,
                        identityIndex: controller.identityIndex
                    )
                } label: {
                    Label("Dismiss", systemImage: "xmark")
                }
                .tint(.gray)
            }
        }
    }

    private var isDismissable: Bool {
        switch controller.phase {
        case .failed:
            // The user is expected to read the error and retry; always
            // dismissable.
            return true
        case .unconfirmed:
            // The slot is held to block a re-submission that would burn funds
            // (the identity is probably live on chain). The picker's
            // `usedIdentityIndices` unions the persisted `isUsed` reservation
            // with the `PersistentIdentity` rows, but the reservation write is
            // best-effort (silent no-op when the slot row is beyond the
            // derived lookahead), so the live controller remains a
            // load-bearing guard. Allow dismiss only once the identity-sync
            // has written the `PersistentIdentity` row — after that the slot
            // is protected by the persisted row and dropping the controller
            // is safe.
            return !slotIdentities.isEmpty
        default:
            return false
        }
    }

    private var phaseIcon: String {
        switch controller.phase {
        case .idle, .preparingKeys, .inFlight: return "clock.fill"
        case .completed: return "checkmark.circle.fill"
        case .failed: return "xmark.octagon.fill"
        case .unconfirmed: return "exclamationmark.triangle.fill"
        }
    }

    private var phaseTint: Color {
        switch controller.phase {
        case .idle, .preparingKeys, .inFlight: return .blue
        case .completed: return .green
        case .failed: return .red
        case .unconfirmed: return .orange
        }
    }

    private var phaseLabel: String {
        switch controller.phase {
        case .idle: return "Queued"
        case .preparingKeys: return "Preparing keys"
        case .inFlight: return "In flight"
        case .completed: return "Registered"
        case .failed: return "Failed"
        case .unconfirmed: return "Confirmation pending"
        }
    }
}
