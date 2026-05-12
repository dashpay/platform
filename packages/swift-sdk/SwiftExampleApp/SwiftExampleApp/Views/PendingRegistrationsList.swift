import SwiftUI
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
                ForEach(active, id: \.identityIndex) { controller in
                    PendingRegistrationRow(controller: controller)
                }
            }
        }
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
            if case .failed = controller.phase {
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

    private var phaseIcon: String {
        switch controller.phase {
        case .idle, .preparingKeys, .inFlight: return "clock.fill"
        case .completed: return "checkmark.circle.fill"
        case .failed: return "xmark.octagon.fill"
        }
    }

    private var phaseTint: Color {
        switch controller.phase {
        case .idle, .preparingKeys, .inFlight: return .blue
        case .completed: return .green
        case .failed: return .red
        }
    }

    private var phaseLabel: String {
        switch controller.phase {
        case .idle: return "Queued"
        case .preparingKeys: return "Preparing keys"
        case .inFlight: return "In flight"
        case .completed: return "Registered"
        case .failed: return "Failed"
        }
    }
}
