import CoreImage.CIFilterBuiltins
import SwiftDashSDK
import SwiftUI

/// Read-only DashPay profile sheet, promoted out of
/// `IdentityDetailView`'s inline card: large avatar, display name,
/// DPNS handle, public message, and an Edit button that hands off to
/// `DashPayProfileEditorView` (the tab root presents the editor
/// after this sheet dismisses, via `onEdit`).
struct DashPayProfileView: View {
    let identity: PersistentIdentity
    let profile: DashPayProfile?
    let onEdit: () -> Void

    @Environment(\.dismiss) private var dismiss
    @EnvironmentObject private var walletManager: PlatformWalletManager

    /// DIP-15 auto-accept QR state (generated lazily on appear).
    @State private var qrImage: UIImage?
    @State private var qrURI: String?
    @State private var qrError: String?

    /// Presents the "Invite a friend" (DIP-13 invitation create) sheet.
    @State private var showCreateInvitation = false

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

                Section("Add me (DIP-15 QR)") {
                    if let qrImage {
                        VStack(spacing: 8) {
                            Image(uiImage: qrImage)
                                .interpolation(.none)
                                .resizable()
                                .scaledToFit()
                                .frame(width: 200, height: 200)
                                .padding(8)
                                .background(Color.white)
                                .cornerRadius(12)
                            Text("Scan to send me a contact request — auto-accepted for 1 hour.")
                                .font(.caption)
                                .foregroundColor(.secondary)
                                .multilineTextAlignment(.center)
                            if let qrURI {
                                Text(qrURI)
                                    .font(.caption2)
                                    .foregroundColor(.secondary)
                                    .lineLimit(2)
                                    .truncationMode(.middle)
                                    .textSelection(.enabled)
                                    .accessibilityIdentifier("dashpay.profile.qrURI")
                            }
                        }
                        .frame(maxWidth: .infinity)
                    } else if let qrError {
                        Text(qrError)
                            .font(.caption)
                            .foregroundColor(.orange)
                    } else {
                        HStack(spacing: 8) {
                            ProgressView()
                            Text("Generating QR…")
                                .font(.caption)
                                .foregroundColor(.secondary)
                        }
                    }
                }
                .task { await generateAutoAcceptQR() }

                Section("Invite a friend (DIP-13)") {
                    Button {
                        showCreateInvitation = true
                    } label: {
                        Label("Create invitation", systemImage: "person.badge.plus")
                    }
                    .accessibilityIdentifier("dashpay.profile.createInvitation")
                    Text("Fund a one-time link so someone with no Dash can register their identity and add you.")
                        .font(.caption)
                        .foregroundColor(.secondary)
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
            .sheet(isPresented: $showCreateInvitation) {
                CreateInvitationSheet(identity: identity)
                    .environmentObject(walletManager)
            }
        }
    }

    /// Build the DIP-15 auto-accept QR for this identity (once), via the Rust
    /// `buildAutoAcceptQR`. The QR's `du` is the owner's DPNS name; Rust resolves
    /// it on-chain when it isn't cached locally, and surfaces a clear error only
    /// if no name is registered for the identity at all.
    private func generateAutoAcceptQR() async {
        guard qrImage == nil, qrError == nil else { return }
        guard let walletId = identity.wallet?.walletId,
              let wallet = walletManager.wallet(for: walletId) else {
            qrError = "No wallet loaded for this identity."
            return
        }
        // Prefer the locally-cached DPNS name; pass "" so Rust resolves it
        // on-chain when the cache is empty (imported/restored identities carry
        // the name on-chain but not in the local field). If no name is
        // registered at all, the Rust call surfaces a clear error.
        let username = (identity.mainDpnsName ?? identity.dpnsName)?
            .trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        do {
            let uri = try await wallet.buildAutoAcceptQR(
                ownerIdentityId: identity.identityId,
                username: username
            )
            qrURI = uri
            qrImage = Self.makeQRCode(from: uri)
        } catch {
            qrError = "Couldn't build the QR: \(error.localizedDescription)"
        }
    }

    /// Render a string as a QR `UIImage` (native CoreImage generator, scaled 10×
    /// for crispness). Mirrors the receive-address QR helper.
    private static func makeQRCode(from string: String) -> UIImage? {
        let context = CIContext()
        let filter = CIFilter.qrCodeGenerator()
        filter.message = Data(string.utf8)
        guard
            let output = filter.outputImage?
                .transformed(by: CGAffineTransform(scaleX: 10, y: 10)),
            let cgImage = context.createCGImage(output, from: output.extent)
        else { return nil }
        return UIImage(cgImage: cgImage)
    }
}
