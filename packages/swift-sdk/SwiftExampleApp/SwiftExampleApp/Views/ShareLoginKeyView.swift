import SwiftData
import SwiftDashSDK
import SwiftUI

/// Hand a browser a bounded login key over Bluetooth LE.
///
/// The user picks how long the key lives and how many credits it may
/// spend, the phone advertises the `BrowserLoginKeyProtocol` service,
/// and once a browser writes its request the user compares the pairing
/// code on both screens and confirms. The phone then:
///
///   1. draws a random login key and derives the AUTHENTICATION key
///      the browser will use from it (`HKDF(loginKey, identityId, "auth")`),
///   2. registers `ECDSA_HASH160(authPub)` as a HIGH key carrying the
///      chosen `totalBudget` and `expiresAt` through
///      `wallet.updateIdentity(addPublicKeys:)`, signed by the
///      identity's MASTER key from the Keychain,
///   3. encrypts the login key to the browser's ephemeral public key
///      and exposes it on the response characteristic.
///
/// The login key is never stored on the phone: it exists only in this
/// view's memory until the response is served, and the identity key it
/// controls can only sign Batch transitions within its budget until it
/// expires. A HASH160 key needs no ownership proof, which is why the
/// phone can register a key whose private half it never keeps.
struct ShareLoginKeyView: View {
    let identity: PersistentIdentity

    @EnvironmentObject var appState: AppState
    @EnvironmentObject var walletManager: PlatformWalletManager
    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) private var dismiss

    @StateObject private var peripheral = BrowserLoginPeripheral(localName: "Dash Wallet")

    enum Phase: Equatable {
        case configuring
        case advertising
        case awaitingConfirmation
        case registering
        case delivered
        case failed(String)
    }

    /// How long the browser's key stays valid.
    enum Lifetime: String, CaseIterable, Identifiable {
        case oneHour = "1 hour"
        case oneDay = "24 hours"
        case oneWeek = "7 days"
        case oneMonth = "30 days"

        var id: String { rawValue }

        var seconds: TimeInterval {
            switch self {
            case .oneHour: return 3_600
            case .oneDay: return 86_400
            case .oneWeek: return 7 * 86_400
            case .oneMonth: return 30 * 86_400
            }
        }
    }

    /// How many credits the browser's key may spend in total.
    enum Budget: String, CaseIterable, Identifiable {
        case milliDash = "0.001 DASH"
        case centiDash = "0.01 DASH"
        case deciDash = "0.1 DASH"
        case oneDash = "1 DASH"

        var id: String { rawValue }

        /// 1 DASH = 100 000 000 duffs = 100 000 000 000 credits.
        var credits: UInt64 {
            switch self {
            case .milliDash: return 100_000_000
            case .centiDash: return 1_000_000_000
            case .deciDash: return 10_000_000_000
            case .oneDash: return 100_000_000_000
            }
        }
    }

    @State private var phase: Phase = .configuring
    @State private var lifetime: Lifetime = .oneDay
    @State private var budget: Budget = .centiDash
    @State private var request: BrowserLoginKeyProtocol.BrowserLoginRequest?
    @State private var registeredKeyId: UInt32?

    var body: some View {
        Form {
            Section("Identity") {
                LabeledContent("Identity", value: identity.alias ?? identity.dpnsName ?? shortId(identity.identityId))
                LabeledContent("Balance", value: identity.formattedBalance)
            }

            Section {
                Picker("Valid for", selection: $lifetime) {
                    ForEach(Lifetime.allCases) { Text($0.rawValue).tag($0) }
                }
                Picker("Spend limit", selection: $budget) {
                    ForEach(Budget.allCases) { Text($0.rawValue).tag($0) }
                }
            } header: {
                Text("Limits")
            } footer: {
                Text("The browser gets its own authentication key. Platform refuses anything it signs after the key expires or once it has spent its limit, and the key can never change your identity's keys.")
            }
            .disabled(phase != .configuring)

            Section("Bluetooth") {
                switch phase {
                case .configuring:
                    Button {
                        startAdvertising()
                    } label: {
                        Label("Start sharing", systemImage: "antenna.radiowaves.left.and.right")
                    }
                    .disabled(!hasLoadedWallet || !hasMasterKey)
                    if !hasLoadedWallet {
                        Text("This identity's wallet is not loaded, so the phone cannot sign the key registration.")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    } else if !hasMasterKey {
                        Text("This identity has no MASTER key on the phone, so it cannot add keys.")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                case .advertising:
                    HStack {
                        ProgressView()
                        Text(radioText)
                    }
                    Text("In the browser, choose “Sign in with phone” and pick “Dash Wallet” from the Bluetooth list.")
                        .font(.caption)
                        .foregroundColor(.secondary)
                case .awaitingConfirmation:
                    if let request {
                        LabeledContent("App", value: request.label)
                        LabeledContent("Contract", value: shortId(request.contractId))
                        LabeledContent("Network", value: String(describing: request.network.network))
                        VStack(alignment: .leading, spacing: 8) {
                            Text("Pairing code")
                                .font(.caption)
                                .foregroundColor(.secondary)
                            Text(request.pairingCode)
                                .font(.system(.largeTitle, design: .monospaced))
                                .fontWeight(.bold)
                            Text("Only confirm if the browser shows the same six digits.")
                                .font(.caption)
                                .foregroundColor(.secondary)
                        }
                        Button {
                            Task { await confirmAndRegister() }
                        } label: {
                            Label("Share a \(lifetime.rawValue) key limited to \(budget.rawValue)", systemImage: "checkmark.shield")
                        }
                        Button(role: .destructive) {
                            reject()
                        } label: {
                            Label("Decline", systemImage: "xmark")
                        }
                    }
                case .registering:
                    HStack {
                        ProgressView()
                        Text("Registering the key on Platform…")
                    }
                case .delivered:
                    Label("Login key delivered", systemImage: "checkmark.circle.fill")
                        .foregroundColor(.green)
                    if let registeredKeyId {
                        LabeledContent("Key id", value: "\(registeredKeyId)")
                    }
                    Text("The browser can now sign in. You can disable the key early from the key list.")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    Button("Done") { dismiss() }
                case .failed(let message):
                    Label(message, systemImage: "exclamationmark.triangle")
                        .foregroundColor(.red)
                    Button("Try again") {
                        request = nil
                        phase = .configuring
                        peripheral.stop()
                    }
                }
            }

            if let error = peripheral.lastError {
                Section {
                    Text(error).foregroundColor(.red)
                }
            }
        }
        .navigationTitle("Share Login Key")
        .navigationBarTitleDisplayMode(.inline)
        .onAppear {
            peripheral.onRequest = { bytes in handle(requestBytes: bytes) }
        }
        .onDisappear {
            peripheral.stop()
        }
    }

    // MARK: - Derived state

    private var hasLoadedWallet: Bool {
        guard let walletId = identity.wallet?.walletId else { return false }
        return walletManager.wallet(for: walletId) != nil
    }

    private var hasMasterKey: Bool {
        identity.identityPublicKeys.contains {
            $0.purpose == .authentication && $0.securityLevel == .master && $0.disabledAt == nil
        }
    }

    private var radioText: String {
        switch peripheral.radioState {
        case .advertising: return "Waiting for a browser…"
        case .poweredOn: return "Publishing the service…"
        case .poweredOff: return "Bluetooth is off."
        case .unauthorized: return "Bluetooth permission was denied."
        case .unsupported: return "This device has no Bluetooth LE."
        case .unknown: return "Starting Bluetooth…"
        }
    }

    private func shortId(_ id: Data) -> String {
        let base58 = id.toBase58String()
        guard base58.count > 12 else { return base58 }
        return "\(base58.prefix(6))…\(base58.suffix(6))"
    }

    // MARK: - Flow

    private func startAdvertising() {
        phase = .advertising
        peripheral.start()
    }

    private func handle(requestBytes: Data) {
        guard phase == .advertising else { return }
        do {
            let parsed = try BrowserLoginKeyProtocol.BrowserLoginRequest.parse(requestBytes)
            guard parsed.network.network == identity.network else {
                peripheral.setStatus(.failed)
                phase = .failed("The browser asked for \(parsed.network.network) but this identity lives on \(identity.network).")
                return
            }
            request = parsed
            phase = .awaitingConfirmation
            peripheral.setStatus(.awaitingConfirmation)
        } catch {
            peripheral.setStatus(.failed)
            phase = .failed(error.localizedDescription)
        }
    }

    private func reject() {
        peripheral.setStatus(.rejected)
        request = nil
        phase = .configuring
        peripheral.stop()
    }

    @MainActor
    private func confirmAndRegister() async {
        guard let request else { return }
        guard let walletId = identity.wallet?.walletId,
              let wallet = walletManager.wallet(for: walletId) else {
            fail("Wallet not loaded in the wallet manager.")
            return
        }

        phase = .registering
        peripheral.setStatus(.registering)

        var loginKey = Data()
        var walletEphemeralPrivateKey = Data()
        defer {
            loginKey.resetBytes(in: 0..<loginKey.count)
            walletEphemeralPrivateKey.resetBytes(in: 0..<walletEphemeralPrivateKey.count)
        }

        do {
            loginKey = try BrowserLoginKeyProtocol.generateLoginKey()
            var authPrivateKey = try BrowserLoginKeyProtocol.deriveAuthPrivateKey(
                loginKey: loginKey,
                identityId: identity.identityId
            )
            defer { authPrivateKey.resetBytes(in: 0..<authPrivateKey.count) }
            let authPublicKey = try Secp256k1Primitives.compressedPublicKey(privateKey: authPrivateKey)
            let authKeyHash = BrowserLoginKeyProtocol.hash160(authPublicKey)
            guard authKeyHash.count == 20 else {
                fail("Could not hash the browser's public key.")
                return
            }

            let expiresAt = UInt64((Date().timeIntervalSince1970 + lifetime.seconds) * 1000)
            let keyId = (identity.identityPublicKeys.map { $0.id }.max() ?? 0) + 1
            let newKey = ManagedPlatformWallet.IdentityPubkey(
                keyId: keyId,
                keyType: .ecdsaHash160,
                purpose: .authentication,
                securityLevel: .high,
                pubkeyBytes: authKeyHash,
                totalBudget: budget.credits,
                expiresAt: expiresAt
            )

            let signer = KeychainSigner(modelContainer: modelContext.container)
            try await wallet.updateIdentity(
                identityId: identity.identityId,
                addPublicKeys: [newKey],
                signer: signer
            )
            _ = signer  // keepalive: see KeychainSigner lifetime contract.

            let ephemeral = try BrowserLoginKeyProtocol.generateEphemeralKeyPair()
            walletEphemeralPrivateKey = ephemeral.privateKey
            let encryptedPayload = try BrowserLoginKeyProtocol.seal(
                loginKey: loginKey,
                walletEphemeralPrivateKey: walletEphemeralPrivateKey,
                appEphemeralPublicKey: request.appEphemeralPublicKey
            )
            let response = BrowserLoginKeyProtocol.BrowserLoginResponse(
                identityId: identity.identityId,
                walletEphemeralPublicKey: ephemeral.publicKey,
                encryptedPayload: encryptedPayload,
                keyId: keyId,
                expiresAt: expiresAt,
                totalBudget: budget.credits
            )
            peripheral.deliver(response: response.serialized())
            registeredKeyId = keyId
            phase = .delivered

            if let sdk = appState.sdk {
                try? await IdentityKeyRefresher.refreshBalanceAndKeys(
                    identity: identity,
                    sdk: sdk,
                    modelContext: modelContext
                )
            }
        } catch {
            fail(error.localizedDescription)
        }
    }

    private func fail(_ message: String) {
        peripheral.setStatus(.failed)
        phase = .failed(message)
    }
}
