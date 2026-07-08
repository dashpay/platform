import CoreImage.CIFilterBuiltins
import SwiftDashSDK
import SwiftUI
import UniformTypeIdentifiers

/// Create a DashPay invitation (DIP-13): pick an amount, optionally opt into the
/// contact-bootstrap, fund a one-time asset-lock voucher, and share the resulting
/// `dashpay://invite` link (as text + QR) so a friend with no Dash can register
/// their own identity from it.
///
/// The returned link **contains the voucher private key** — it is a bearer
/// credential. It is never logged, and the "Copy" action uses a local-only
/// pasteboard so it isn't mirrored across devices via Universal Clipboard.
struct CreateInvitationSheet: View {
    /// The inviter's identity (the current DashPay identity). Its id + DPNS name
    /// seed the optional "send a contact request back to me" info in the link.
    let identity: PersistentIdentity

    @EnvironmentObject private var walletManager: PlatformWalletManager
    @Environment(\.dismiss) private var dismiss

    /// 1 DASH = 100,000,000 duffs.
    private static let duffsPerDash: UInt64 = 100_000_000
    /// Rust-enforced cap (`MAX_INVITATION_DUFFS`, 0.01 DASH). Mirrored here so the
    /// UI rejects an over-cap amount before the FFI does.
    private static let maxInvitationDuffs: UInt64 = 1_000_000
    /// BIP44 standard account that supplies the asset-lock's funding UTXOs. The
    /// example app funds identity operations from account 0; the `IdentityInvitation`
    /// funding type derives the voucher credit key internally (not this account).
    private static let fundingAccount: UInt32 = 0

    /// Amount to lock in the voucher, as a DASH string (decimal). Default 0.0005
    /// DASH — enough for identity registration plus a small starting balance.
    @State private var amountDashText: String = "0.0005"
    /// Opt into the contact-bootstrap: the link carries the inviter so the invitee
    /// can send a contact request back. Requires a registered username.
    @State private var sendRequestBack = true

    @State private var isCreating = false
    @State private var inviteURI: String?
    @State private var qrImage: UIImage?
    @State private var errorMessage: String?
    @State private var showShareSheet = false
    @State private var didCopy = false

    /// The inviter's DPNS username, if registered. The contact-bootstrap can only
    /// be offered when the inviter has a username to advertise in the link.
    private var username: String? {
        let name = (identity.mainDpnsName ?? identity.dpnsName)?
            .trimmingCharacters(in: .whitespacesAndNewlines)
        return (name?.isEmpty == false) ? name : nil
    }

    /// Parse the DASH text field into duffs, or `nil` if it isn't a valid,
    /// in-range positive amount.
    private var amountDuffs: UInt64? {
        guard let dash = Double(amountDashText.replacingOccurrences(of: ",", with: ".")),
              dash > 0
        else { return nil }
        let duffs = (dash * Double(Self.duffsPerDash)).rounded()
        guard duffs >= 1, duffs <= Double(Self.maxInvitationDuffs) else { return nil }
        return UInt64(duffs)
    }

    var body: some View {
        NavigationStack {
            Form {
                if let inviteURI {
                    resultSection(uri: inviteURI)
                } else {
                    inputSection
                }
            }
            .navigationTitle("Invite a Friend")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button(inviteURI == nil ? "Cancel" : "Done") { dismiss() }
                        .accessibilityIdentifier("dashpay.invite.create.done")
                }
            }
            .sheet(isPresented: $showShareSheet) {
                if let inviteURI {
                    ShareSheet(items: [inviteURI])
                }
            }
        }
    }

    // MARK: - Input

    @ViewBuilder
    private var inputSection: some View {
        Section("Amount") {
            HStack {
                TextField("0.0005", text: $amountDashText)
                    .keyboardType(.decimalPad)
                    .accessibilityIdentifier("dashpay.invite.create.amount")
                Text("DASH")
                    .foregroundColor(.secondary)
            }
            Text("Funds a one-time voucher your friend uses to register their identity. Max 0.01 DASH.")
                .font(.caption)
                .foregroundColor(.secondary)
        }

        Section("Contact") {
            Toggle("Send a contact request back to me", isOn: $sendRequestBack)
                .disabled(username == nil)
                .accessibilityIdentifier("dashpay.invite.create.sendBack")
            if let username {
                Text("Your friend will be asked to add \(username) after they register.")
                    .font(.caption)
                    .foregroundColor(.secondary)
            } else {
                Text("Register a username to let invitees add you back automatically.")
                    .font(.caption)
                    .foregroundColor(.orange)
            }
        }

        Section {
            Button {
                Task { await create() }
            } label: {
                HStack {
                    if isCreating {
                        ProgressView()
                        Text("Creating…")
                    } else {
                        Text("Create Invitation")
                    }
                }
                .frame(maxWidth: .infinity)
            }
            .disabled(isCreating || amountDuffs == nil)
            .accessibilityIdentifier("dashpay.invite.create.submit")
        } footer: {
            if amountDuffs == nil {
                Text("Enter an amount between 0.00000001 and 0.01 DASH.")
                    .foregroundColor(.orange)
            }
        }

        if let errorMessage {
            Section {
                Text(errorMessage)
                    .font(.caption)
                    .foregroundColor(.red)
            }
        }
    }

    // MARK: - Result

    @ViewBuilder
    private func resultSection(uri: String) -> some View {
        Section {
            VStack(spacing: 12) {
                if let qrImage {
                    Image(uiImage: qrImage)
                        .interpolation(.none)
                        .resizable()
                        .scaledToFit()
                        .frame(width: 220, height: 220)
                        .padding(8)
                        .background(Color.white)
                        .cornerRadius(12)
                }
                Text("Share this link with your friend. It funds their new identity — treat it like cash.")
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .multilineTextAlignment(.center)
            }
            .frame(maxWidth: .infinity)
            .listRowBackground(Color.clear)
        }

        Section {
            Button {
                showShareSheet = true
            } label: {
                Label("Share link", systemImage: "square.and.arrow.up")
            }
            .accessibilityIdentifier("dashpay.invite.create.share")

            Button {
                copyLink(uri)
            } label: {
                Label(didCopy ? "Copied" : "Copy link", systemImage: didCopy ? "checkmark" : "doc.on.doc")
            }
            .accessibilityIdentifier("dashpay.invite.create.copy")
        } footer: {
            Text("The link contains a one-time key. Anyone who has it can claim the funds, so share it privately.")
        }
    }

    // MARK: - Actions

    private func create() async {
        guard !isCreating else { return }
        errorMessage = nil
        guard let amountDuffs else {
            errorMessage = "Enter a valid amount."
            return
        }
        guard let walletId = identity.wallet?.walletId,
              let wallet = walletManager.wallet(for: walletId)
        else {
            errorMessage = "No wallet loaded for this identity."
            return
        }
        let optIn = sendRequestBack && username != nil
        isCreating = true
        defer { isCreating = false }
        do {
            let uri = try await wallet.createInvitation(
                amountDuffs: amountDuffs,
                fundingAccount: Self.fundingAccount,
                inviterIdentityId: optIn ? identity.identityId : nil,
                inviterUsername: optIn ? username : nil,
                nowUnix: UInt32(Date().timeIntervalSince1970)
            )
            qrImage = Self.makeQRCode(from: uri)
            inviteURI = uri
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    /// Copy the link to a **local-only** pasteboard so the bearer key isn't
    /// mirrored to the user's other devices via Universal Clipboard.
    private func copyLink(_ uri: String) {
        UIPasteboard.general.setItems(
            [[UTType.plainText.identifier: uri]],
            options: [.localOnly: true]
        )
        didCopy = true
        DispatchQueue.main.asyncAfter(deadline: .now() + 2) { didCopy = false }
    }

    /// Render a string as a QR `UIImage` (native CoreImage generator, scaled 10×
    /// for crispness). Mirrors the profile / receive-address QR helper.
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
