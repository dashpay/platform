import SwiftUI
import SwiftDashSDK

/// Read-only DashPay profile sheet (SPEC §6.2), promoted out of
/// `IdentityDetailView`'s inline card: large avatar, display name,
/// DPNS handle, public message, and an Edit button that hands off to
/// `DashPayProfileEditorView` (the tab root presents the editor
/// after this sheet dismisses, via `onEdit`).
struct DashPayProfileView: View {
    let identity: PersistentIdentity
    let profile: DashPayProfile?
    let onEdit: () -> Void

    @Environment(\.dismiss) private var dismiss

    private var displayName: String {
        if let name = profile?.displayName?
            .trimmingCharacters(in: .whitespacesAndNewlines),
           !name.isEmpty {
            return name
        }
        if let dpns = identity.mainDpnsName ?? identity.dpnsName {
            return dpns
        }
        return String(identity.identityIdBase58.prefix(12)) + "…"
    }

    var body: some View {
        NavigationStack {
            List {
                Section {
                    VStack(spacing: 12) {
                        DashPayAvatarView(
                            avatarUrl: profile?.avatarUrl,
                            displayName: displayName,
                            size: 96
                        )
                        Text(displayName)
                            .font(.title2)
                            .fontWeight(.semibold)
                        if let dpns = identity.mainDpnsName ?? identity.dpnsName {
                            Text(dpns)
                                .font(.subheadline)
                                .foregroundColor(.blue)
                        }
                        if let msg = profile?.publicMessage?
                            .trimmingCharacters(in: .whitespacesAndNewlines),
                           !msg.isEmpty {
                            Text(msg)
                                .font(.callout)
                                .foregroundColor(.secondary)
                                .multilineTextAlignment(.center)
                        }
                    }
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 12)
                    .listRowBackground(Color.clear)
                }

                Section("Identity") {
                    Text(identity.identityIdBase58)
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .textSelection(.enabled)
                }

                if let url = profile?.avatarUrl?
                    .trimmingCharacters(in: .whitespacesAndNewlines),
                   !url.isEmpty {
                    Section("Avatar URL") {
                        Text(url)
                            .font(.caption)
                            .foregroundColor(.secondary)
                            .lineLimit(1)
                            .truncationMode(.middle)
                    }
                }
            }
            .navigationTitle("Your Profile")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Done") { dismiss() }
                        .accessibilityIdentifier("dashpay.profile.done")
                }
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button {
                        onEdit()
                    } label: {
                        Label("Edit", systemImage: "pencil")
                    }
                    .accessibilityIdentifier("dashpay.profile.edit")
                }
            }
        }
    }
}
