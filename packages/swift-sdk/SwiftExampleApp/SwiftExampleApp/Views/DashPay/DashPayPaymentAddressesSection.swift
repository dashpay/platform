import SwiftData
import SwiftDashSDK
import SwiftUI
import UIKit

// The public payment addresses a DashPay profile publishes (Core, Platform,
// shielded). Parsing, checking and rendering an address, and whether the
// network lets a profile set one, all come from platform-wallet through
// `DashPayProfile.paymentAddressBytes/paymentAddressString` and
// `ManagedPlatformWallet.dashPayPaymentAddressSupport()`. These views only
// show the results and collect the user's choice.

extension DashPayPaymentAddressKind {
    var title: String {
        switch self {
        case .core: return "Core"
        case .platform: return "Platform"
        case .shielded: return "Shielded"
        }
    }

    /// Accessibility identifier component.
    var identifierName: String {
        switch self {
        case .core: return "core"
        case .platform: return "platform"
        case .shielded: return "shielded"
        }
    }

    var placeholder: String {
        switch self {
        case .core: return "Core address (X… / y…)"
        case .platform: return "Platform address (dash1… / tdash1…)"
        case .shielded: return "Shielded address"
        }
    }
}

// MARK: - Read-only list

/// Read-only "Payment addresses" section for a profile: one row per
/// published address with a copy button. With `showsUnpublished` false the
/// section is hidden when the profile publishes none (contact detail); with
/// it true each unpublished kind reads "Not published" (own profile).
struct DashPayPublishedPaymentAddressesSection: View {
    let profile: DashPayProfile?
    let network: Network
    let identifierPrefix: String
    let showsUnpublished: Bool

    @State private var copiedKind: DashPayPaymentAddressKind?

    private var publishedKinds: [DashPayPaymentAddressKind] {
        DashPayPaymentAddressKind.allCases.filter { profile?.paymentAddress($0) != nil }
    }

    private var shownKinds: [DashPayPaymentAddressKind] {
        showsUnpublished ? DashPayPaymentAddressKind.allCases : publishedKinds
    }

    var body: some View {
        if !shownKinds.isEmpty {
            Section {
                ForEach(shownKinds, id: \.self) { kind in
                    row(kind)
                }
            } header: {
                Text("Payment addresses")
            } footer: {
                Text("Published addresses are public. Anyone can see them, and payments to them can be linked to this profile.")
            }
        }
    }

    @ViewBuilder
    private func row(_ kind: DashPayPaymentAddressKind) -> some View {
        let bytes = profile?.paymentAddress(kind)
        let rendered = bytes.flatMap {
            DashPayProfile.paymentAddressString(kind: kind, bytes: $0, network: network)
        }
        HStack(alignment: .top, spacing: 8) {
            VStack(alignment: .leading, spacing: 2) {
                Text(kind.title)
                    .font(.subheadline)
                    .fontWeight(.medium)
                if let rendered {
                    Text(rendered)
                        .font(.caption.monospaced())
                        .foregroundColor(.secondary)
                        .textSelection(.enabled)
                } else if bytes != nil {
                    Text("Published, but not a valid \(kind.title) address on \(network.displayName)")
                        .font(.caption)
                        .foregroundColor(.orange)
                } else {
                    Text("Not published")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
            .accessibilityElement(children: .combine)
            .accessibilityIdentifier("\(identifierPrefix).\(kind.identifierName)")
            Spacer()
            if let rendered {
                Button {
                    UIPasteboard.general.string = rendered
                    copiedKind = kind
                } label: {
                    Image(systemName: copiedKind == kind ? "checkmark" : "doc.on.doc")
                }
                .buttonStyle(.borderless)
                .accessibilityLabel("Copy \(kind.title) address")
                .accessibilityIdentifier("\(identifierPrefix).\(kind.identifierName).copy")
            }
        }
    }
}

// MARK: - Editor model

/// Keep / set / remove choice for each payment address of the profile being
/// edited. A typed or filled-in address is checked by platform-wallet as it
/// changes; its error is shown under the field.
@MainActor
final class DashPayPaymentAddressEditorModel: ObservableObject {
    enum Mode: Hashable {
        case keep
        case set
        case remove
    }

    struct Row {
        var mode: Mode = .keep
        var text: String = ""
        /// Stored bytes of `text`, when platform-wallet accepts it.
        var bytes: Data?
        /// platform-wallet's reason `text` is not an address.
        var error: String?
        var isFetchingWalletAddress = false
        var walletAddressError: String?
    }

    let existing: DashPayProfile?

    @Published private(set) var rows: [DashPayPaymentAddressKind: Row] = [:]
    @Published private(set) var network: Network?
    @Published private(set) var networkError: String?
    @Published private(set) var support: DashPayPaymentAddressSupport?
    @Published private(set) var supportError: String?
    @Published private(set) var isLoadingSupport = false

    /// The Core address `nextReceiveAddress` handed out this editor session.
    /// That call advances the receive pool, so it runs at most once.
    private var coreWalletAddress: String?
    private var started = false

    init(existing: DashPayProfile?) {
        self.existing = existing
        for kind in DashPayPaymentAddressKind.allCases {
            rows[kind] = Row()
        }
    }

    func row(_ kind: DashPayPaymentAddressKind) -> Row {
        rows[kind] ?? Row()
    }

    func published(_ kind: DashPayPaymentAddressKind) -> Data? {
        existing?.paymentAddress(kind)
    }

    /// Read the wallet's network and ask which addresses the network lets a
    /// profile set. Runs once per editor.
    func start(wallet: ManagedPlatformWallet?) async {
        guard !started else { return }
        started = true
        guard let wallet else {
            networkError = "No wallet available for this identity."
            supportError = networkError
            return
        }
        do {
            network = try wallet.coreWallet().network()
        } catch {
            networkError = "Couldn't read the wallet's network: \(error.localizedDescription)"
        }
        isLoadingSupport = true
        defer { isLoadingSupport = false }
        do {
            support = try await wallet.dashPayPaymentAddressSupport()
        } catch {
            supportError = "Couldn't check which addresses the network supports: \(error.localizedDescription)"
        }
    }

    /// Why Set is unavailable for `kind`, or nil when it is available.
    func setUnavailableReason(_ kind: DashPayPaymentAddressKind) -> String? {
        if network == nil {
            return networkError == nil
                ? "Reading the wallet's network…"
                : "Set is unavailable because the wallet's network could not be read."
        }
        if supportError != nil {
            return "Set is unavailable because the network support check failed."
        }
        guard let support else {
            return "Checking network support…"
        }
        if !support.supports(kind) {
            if kind == .shielded {
                return "The connected network or this build does not support publishing a shielded address yet."
            }
            return "The connected network does not support publishing a \(kind.title) address yet."
        }
        return nil
    }

    func availableModes(_ kind: DashPayPaymentAddressKind) -> [Mode] {
        published(kind) == nil ? [.keep, .set] : [.keep, .set, .remove]
    }

    func setMode(_ mode: Mode, for kind: DashPayPaymentAddressKind) {
        if mode == .set, setUnavailableReason(kind) != nil { return }
        rows[kind]?.mode = mode
    }

    func setText(_ text: String, for kind: DashPayPaymentAddressKind) {
        rows[kind]?.text = text
        revalidate(kind)
    }

    private func revalidate(_ kind: DashPayPaymentAddressKind) {
        guard var row = rows[kind] else { return }
        row.bytes = nil
        row.error = nil
        if !row.text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            if let network {
                do {
                    row.bytes = try DashPayProfile.paymentAddressBytes(
                        kind: kind,
                        from: row.text,
                        network: network
                    )
                } catch {
                    row.error = error.localizedDescription
                }
            } else {
                row.error = networkError ?? "The wallet's network is unknown."
            }
        }
        rows[kind] = row
    }

    // MARK: Use this wallet's address

    /// Fill the Core field with this wallet's next receive address
    /// (`ManagedCoreWallet.nextReceiveAddress(accountIndex: 0)`). That call
    /// advances the BIP-44 receive pool, so the address is fetched once and
    /// reused for later taps in this editor.
    func useCoreWalletAddress(wallet: ManagedPlatformWallet?) {
        rows[.core]?.walletAddressError = nil
        if coreWalletAddress == nil {
            guard let wallet else {
                rows[.core]?.walletAddressError = "No wallet available for this identity."
                return
            }
            do {
                coreWalletAddress = try wallet.coreWallet().nextReceiveAddress(accountIndex: 0)
            } catch {
                rows[.core]?.walletAddressError = "Couldn't get a Core receive address: \(error.localizedDescription)"
                return
            }
        }
        if let coreWalletAddress {
            setText(coreWalletAddress, for: .core)
        }
    }

    /// Fill the Platform field with the wallet's Platform receive address,
    /// the one Receive shows (persisted `PersistentPlatformAddress`).
    func usePlatformWalletAddress(_ address: String?) {
        rows[.platform]?.walletAddressError = nil
        guard let address else {
            rows[.platform]?.walletAddressError = "No Platform receive address is available for this wallet yet."
            return
        }
        setText(address, for: .platform)
    }

    /// Fill the shielded field with this identity's dedicated shielded tip
    /// address, prepared now. It is the only shielded address of this wallet
    /// a profile may publish.
    func useShieldedWalletAddress(
        walletManager: PlatformWalletManager,
        walletId: Data?,
        identityId: Data
    ) async {
        rows[.shielded]?.walletAddressError = nil
        guard let walletId else {
            rows[.shielded]?.walletAddressError = "No wallet available for this identity."
            return
        }
        guard let network else {
            rows[.shielded]?.walletAddressError = networkError ?? "The wallet's network is unknown."
            return
        }
        rows[.shielded]?.isFetchingWalletAddress = true
        defer { rows[.shielded]?.isFetchingWalletAddress = false }
        do {
            let raw = try await walletManager.prepareShieldedIdentityAddress(
                walletId: walletId,
                identityId: identityId,
                resolver: MnemonicResolver()
            )
            guard let text = DashPayProfile.paymentAddressString(
                kind: .shielded,
                bytes: raw,
                network: network
            ) else {
                rows[.shielded]?.walletAddressError = "The prepared shielded address could not be shown on \(network.displayName)."
                return
            }
            setText(text, for: .shielded)
        } catch {
            rows[.shielded]?.walletAddressError = "Couldn't prepare the shielded address: \(error.localizedDescription)"
        }
    }

    // MARK: Submit

    /// True when a row is set to Set without an address platform-wallet
    /// accepted.
    var hasIncompleteSet: Bool {
        DashPayPaymentAddressKind.allCases.contains { kind in
            let row = row(kind)
            return row.mode == .set && row.bytes == nil
        }
    }

    func update(for kind: DashPayPaymentAddressKind) throws -> DashPayPaymentAddressUpdate {
        let row = row(kind)
        switch row.mode {
        case .keep:
            return .keep
        case .remove:
            return .remove
        case .set:
            guard let bytes = row.bytes else {
                throw DashPayPaymentAddressEditorError.incomplete(kind)
            }
            return .set(bytes)
        }
    }
}

enum DashPayPaymentAddressEditorError: LocalizedError {
    case incomplete(DashPayPaymentAddressKind)

    var errorDescription: String? {
        switch self {
        case .incomplete(let kind):
            return "Enter a valid \(kind.title) address, or choose Keep."
        }
    }
}

// MARK: - Editor section

/// Editor rows for the three payment addresses: the published address, a
/// Keep / Set / Remove choice, and for Set a field checked by platform-wallet
/// plus a "Use this wallet's address" button.
struct DashPayPaymentAddressEditorSection: View {
    @ObservedObject var model: DashPayPaymentAddressEditorModel
    let wallet: ManagedPlatformWallet?
    let walletId: Data?
    let identityId: Data

    @EnvironmentObject private var walletManager: PlatformWalletManager

    /// The wallet's Platform receive addresses, as Receive reads them.
    @Query private var platformAddresses: [PersistentPlatformAddress]

    init(
        model: DashPayPaymentAddressEditorModel,
        wallet: ManagedPlatformWallet?,
        walletId: Data?,
        identityId: Data
    ) {
        self.model = model
        self.wallet = wallet
        self.walletId = walletId
        self.identityId = identityId
        _platformAddresses = Query(
            filter: PersistentPlatformAddress.predicate(walletId: walletId ?? Data())
        )
    }

    /// Lowest-indexed unused Platform address, the one Receive shows.
    private var platformReceiveAddress: String? {
        platformAddresses
            .filter { !$0.isUsed }
            .min(by: { $0.addressIndex < $1.addressIndex })?
            .address
    }

    var body: some View {
        Section {
            Text("Published addresses are public. Anyone can see them, and payments to them can be linked to this profile.")
                .font(.caption)
                .foregroundColor(.secondary)
            if model.isLoadingSupport {
                HStack(spacing: 8) {
                    ProgressView()
                    Text("Checking which addresses the network supports…")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
            if let error = model.supportError ?? model.networkError {
                Text(error)
                    .font(.caption)
                    .foregroundColor(.red)
                    .accessibilityIdentifier("dashpay.profile.paymentAddress.supportError")
            }
        } header: {
            Text("Payment addresses")
        }

        ForEach(DashPayPaymentAddressKind.allCases, id: \.self) { kind in
            Section {
                rowContent(kind)
            } header: {
                Text("\(kind.title) payment address")
            }
        }
    }

    @ViewBuilder
    private func rowContent(_ kind: DashPayPaymentAddressKind) -> some View {
        let prefix = "dashpay.profile.paymentAddress.\(kind.identifierName)"
        let row = model.row(kind)
        let setReason = model.setUnavailableReason(kind)

        currentAddress(kind)
            .accessibilityElement(children: .combine)
            .accessibilityIdentifier("\(prefix).current")

        modePicker(kind, row: row, setReason: setReason, prefix: prefix)

        if let setReason, !model.isLoadingSupport {
            Text(setReason)
                .font(.caption)
                .foregroundColor(.secondary)
                .accessibilityIdentifier("\(prefix).setUnavailable")
        }

        if row.mode == .set {
            TextField(
                kind.placeholder,
                text: Binding(
                    get: { model.row(kind).text },
                    set: { model.setText($0, for: kind) }
                ),
                axis: .vertical
            )
            .font(.caption.monospaced())
            .textInputAutocapitalization(.never)
            .autocorrectionDisabled()
            .accessibilityIdentifier("\(prefix).field")

            if let error = row.error {
                Text(error)
                    .font(.caption)
                    .foregroundColor(.red)
                    .accessibilityIdentifier("\(prefix).error")
            } else if row.bytes != nil {
                Label("Valid \(kind.title) address", systemImage: "checkmark.circle")
                    .font(.caption)
                    .foregroundColor(.green)
            }

            useWalletButton(kind, row: row, prefix: prefix)

            if let error = row.walletAddressError {
                Text(error)
                    .font(.caption)
                    .foregroundColor(.red)
                    .accessibilityIdentifier("\(prefix).useWalletError")
            }
        } else if row.mode == .remove {
            Text("The \(kind.title) address will be removed from your profile when you save.")
                .font(.caption)
                .foregroundColor(.orange)
        }
    }

    @ViewBuilder
    private func currentAddress(_ kind: DashPayPaymentAddressKind) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            Text("Published")
                .font(.caption2)
                .foregroundColor(.secondary)
            if let bytes = model.published(kind) {
                if let network = model.network,
                   let rendered = DashPayProfile.paymentAddressString(
                       kind: kind,
                       bytes: bytes,
                       network: network
                   ) {
                    Text(rendered)
                        .font(.caption.monospaced())
                        .textSelection(.enabled)
                } else if model.network == nil {
                    Text("Published (the wallet's network is unknown, so it cannot be shown)")
                        .font(.caption)
                        .foregroundColor(.secondary)
                } else {
                    Text("Published, but not a valid \(kind.title) address on this network")
                        .font(.caption)
                        .foregroundColor(.orange)
                }
            } else {
                Text("Not published")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
        }
    }

    private func modePicker(
        _ kind: DashPayPaymentAddressKind,
        row: DashPayPaymentAddressEditorModel.Row,
        setReason: String?,
        prefix: String
    ) -> some View {
        HStack(spacing: 8) {
            ForEach(model.availableModes(kind), id: \.self) { mode in
                let selected = row.mode == mode
                Button {
                    model.setMode(mode, for: kind)
                } label: {
                    Text(title(mode))
                        .font(.subheadline)
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 6)
                        .background(selected ? Color.accentColor.opacity(0.2) : Color.secondary.opacity(0.1))
                        .cornerRadius(8)
                }
                .buttonStyle(.borderless)
                .disabled(mode == .set && setReason != nil)
                .accessibilityAddTraits(selected ? .isSelected : [])
                .accessibilityIdentifier("\(prefix).mode.\(identifierName(mode))")
            }
        }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("\(prefix).mode")
    }

    @ViewBuilder
    private func useWalletButton(
        _ kind: DashPayPaymentAddressKind,
        row: DashPayPaymentAddressEditorModel.Row,
        prefix: String
    ) -> some View {
        Button {
            switch kind {
            case .core:
                model.useCoreWalletAddress(wallet: wallet)
            case .platform:
                model.usePlatformWalletAddress(platformReceiveAddress)
            case .shielded:
                Task {
                    await model.useShieldedWalletAddress(
                        walletManager: walletManager,
                        walletId: walletId,
                        identityId: identityId
                    )
                }
            }
        } label: {
            HStack(spacing: 8) {
                if row.isFetchingWalletAddress {
                    ProgressView()
                }
                Label("Use this wallet's address", systemImage: "wallet.pass")
            }
        }
        .buttonStyle(.borderless)
        .disabled(row.isFetchingWalletAddress)
        .accessibilityIdentifier("\(prefix).useWallet")

        if kind == .shielded {
            Text("Uses this identity's dedicated shielded tip address. Other shielded addresses of this wallet cannot be published.")
                .font(.caption2)
                .foregroundColor(.secondary)
        }
    }

    private func title(_ mode: DashPayPaymentAddressEditorModel.Mode) -> String {
        switch mode {
        case .keep: return "Keep"
        case .set: return "Set"
        case .remove: return "Remove"
        }
    }

    private func identifierName(_ mode: DashPayPaymentAddressEditorModel.Mode) -> String {
        switch mode {
        case .keep: return "keep"
        case .set: return "set"
        case .remove: return "remove"
        }
    }
}
