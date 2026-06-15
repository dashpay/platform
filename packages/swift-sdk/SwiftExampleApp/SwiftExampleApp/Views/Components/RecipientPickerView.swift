import SwiftUI
import SwiftData
import SwiftDashSDK

/// A 32-byte identity selection produced by `RecipientPickerView`.
///
/// `label` is a human-readable form for the confirmation summary and is
/// owned by the picker (e.g. base58 short form, "alice.dash", or the
/// chosen local identity's display name).
struct RecipientSelection: Equatable {
    let identityId: Data
    let label: String
}

/// Self-contained view that produces a `RecipientSelection` from one
/// of three input modes. Designed to be reusable across token actions
/// (Transfer in Wave 1; later waves add Mint, Freeze, etc.).
///
/// - Local: pick one of the `PersistentIdentity` rows on `network`.
/// - Paste: paste a base58 identity id; validated on the fly.
/// - DPNS: resolve `name` (with or without the `.dash` suffix) via
///   `ManagedPlatformWallet.resolveDpnsName`.
struct RecipientPickerView: View {
    @Binding var selection: RecipientSelection?

    /// Wallet used for DPNS resolution. The picker does not mutate the
    /// wallet; it only calls `resolveDpnsName`.
    let wallet: ManagedPlatformWallet
    /// Limits the local-identity list to identities on this network.
    let network: Network
    /// Optional identity id to exclude (e.g. the sender — you can't
    /// transfer to yourself).
    let exclude: Data?

    @Query private var localIdentities: [PersistentIdentity]

    @State private var mode: Mode = .local
    @State private var pastedBase58: String = ""
    @State private var dpnsInput: String = ""
    @State private var dpnsState: DPNSState = .idle
    @State private var resolveTask: Task<Void, Never>?

    private enum Mode: String, CaseIterable, Identifiable {
        case local = "Local"
        case paste = "Paste"
        case dpns = "DPNS"
        var id: String { rawValue }
    }

    private enum DPNSState: Equatable {
        case idle
        case resolving
        case resolved(Data)
        case notFound
        case failed(String)
    }

    init(
        selection: Binding<RecipientSelection?>,
        wallet: ManagedPlatformWallet,
        network: Network,
        exclude: Data? = nil
    ) {
        self._selection = selection
        self.wallet = wallet
        self.network = network
        self.exclude = exclude
        self._localIdentities = Query(
            filter: PersistentIdentity.walletOwnedIdentitiesPredicate(network: network),
            sort: [SortDescriptor(\.identityIndex), SortDescriptor(\.alias)]
        )
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Picker("Recipient source", selection: $mode) {
                ForEach(Mode.allCases) { m in
                    Text(m.rawValue).tag(m)
                }
            }
            .pickerStyle(.segmented)
            .onChange(of: mode) { _, _ in
                // Cancel any in-flight DPNS resolution and reset its
                // visible state *before* clearing the selection — a
                // late-arriving "resolved" callback could otherwise
                // overwrite a fresh selection in the new mode.
                resolveTask?.cancel()
                resolveTask = nil
                dpnsState = .idle
                selection = nil
            }

            switch mode {
            case .local:
                localPicker
            case .paste:
                pastePicker
            case .dpns:
                dpnsPicker
            }

            if let selection = selection {
                Divider()
                selectionSummary(selection)
            }
        }
        .onDisappear {
            resolveTask?.cancel()
            resolveTask = nil
        }
    }

    // MARK: - Sub-pickers

    @ViewBuilder
    private var localPicker: some View {
        let candidates = localIdentities.filter { id in
            guard let exclude = exclude else { return true }
            return id.identityId != exclude
        }
        if candidates.isEmpty {
            Text("No other local identities on this network")
                .font(.subheadline)
                .foregroundColor(.secondary)
        } else {
            Picker("Identity", selection: localBinding) {
                Text("Choose…")
                    .tag(Data?.none)
                    .accessibilityIdentifier("recipient.identity.none")
                ForEach(candidates, id: \.identityId) { id in
                    Text(id.displayName)
                        .tag(Optional(id.identityId))
                        .accessibilityIdentifier("recipient.identity.\(id.identityIdBase58)")
                }
            }
            // navigationLink (not inline): every call site embeds this
            // view in a Form `Section`, so the picker pushes a real list
            // whose rows expose the per-row `recipient.identity.<base58>`
            // identifiers above (inline would drop them — see
            // AccessiblePicker.swift).
            .accessibleFormPicker("recipient.identityPicker")
        }
    }

    private var localBinding: Binding<Data?> {
        Binding<Data?>(
            get: { selection?.identityId },
            set: { newId in
                guard let newId = newId,
                      let target = localIdentities.first(where: { $0.identityId == newId })
                else {
                    selection = nil
                    return
                }
                selection = RecipientSelection(
                    identityId: target.identityId,
                    label: target.displayName
                )
            }
        )
    }

    @ViewBuilder
    private var pastePicker: some View {
        VStack(alignment: .leading, spacing: 6) {
            TextField("Identity ID (base58)", text: $pastedBase58)
                .textInputAutocapitalization(.never)
                .autocorrectionDisabled()
                .font(.system(.body, design: .monospaced))
                .onChange(of: pastedBase58) { _, newValue in
                    let trimmed = newValue.trimmingCharacters(in: .whitespacesAndNewlines)
                    if trimmed.isEmpty {
                        selection = nil
                    } else if let id = Data.identifier(fromBase58: trimmed),
                              id.count == 32,
                              exclude != id {
                        selection = RecipientSelection(
                            identityId: id,
                            label: shortBase58(trimmed)
                        )
                    } else {
                        selection = nil
                    }
                }
            if !pastedBase58.isEmpty && selection == nil {
                Text(pasteValidationMessage)
                    .font(.caption)
                    .foregroundColor(.red)
            }
        }
    }

    private var pasteValidationMessage: String {
        let trimmed = pastedBase58.trimmingCharacters(in: .whitespacesAndNewlines)
        if let id = Data.identifier(fromBase58: trimmed), id.count == 32 {
            if exclude == id {
                return "That's the sender — pick a different recipient."
            }
            return "Identity id is invalid."
        }
        return "Identity id is invalid base58."
    }

    @ViewBuilder
    private var dpnsPicker: some View {
        VStack(alignment: .leading, spacing: 6) {
            TextField("DPNS name (e.g. alice.dash)", text: $dpnsInput)
                .textInputAutocapitalization(.never)
                .autocorrectionDisabled()
                .onChange(of: dpnsInput) { _, newValue in
                    scheduleDpnsResolve(newValue)
                }
            switch dpnsState {
            case .idle:
                EmptyView()
            case .resolving:
                Label("Resolving…", systemImage: "arrow.triangle.2.circlepath")
                    .font(.caption)
                    .foregroundColor(.secondary)
            case .resolved:
                EmptyView()
            case .notFound:
                Text("Name not registered")
                    .font(.caption)
                    .foregroundColor(.red)
            case .failed(let msg):
                Text("Lookup failed: \(msg)")
                    .font(.caption)
                    .foregroundColor(.red)
            }
        }
    }

    // MARK: - DPNS resolution

    private func scheduleDpnsResolve(_ rawInput: String) {
        let input = rawInput.trimmingCharacters(in: .whitespacesAndNewlines)
        resolveTask?.cancel()
        if input.isEmpty {
            dpnsState = .idle
            selection = nil
            return
        }
        dpnsState = .resolving
        let wallet = self.wallet
        let exclude = self.exclude
        resolveTask = Task {
            // 250 ms debounce so each keystroke doesn't fire a new
            // network round-trip.
            try? await Task.sleep(nanoseconds: 250_000_000)
            if Task.isCancelled { return }
            do {
                let resolved = try await wallet.resolveDpnsName(input)
                if Task.isCancelled { return }
                await MainActor.run {
                    // Only commit the result if the user is still on
                    // DPNS mode AND the input they're looking at
                    // matches the input we just resolved. Late
                    // callbacks otherwise stomp on a fresh selection
                    // (e.g. user switched to Paste, or kept typing).
                    guard self.mode == .dpns,
                          self.dpnsInput.trimmingCharacters(in: .whitespacesAndNewlines) == input
                    else { return }
                    if let id = resolved {
                        if let exclude = exclude, id == exclude {
                            self.dpnsState = .failed("That's the sender.")
                            self.selection = nil
                        } else {
                            self.dpnsState = .resolved(id)
                            self.selection = RecipientSelection(
                                identityId: id,
                                label: input
                            )
                        }
                    } else {
                        self.dpnsState = .notFound
                        self.selection = nil
                    }
                }
            } catch {
                if Task.isCancelled { return }
                await MainActor.run {
                    guard self.mode == .dpns,
                          self.dpnsInput.trimmingCharacters(in: .whitespacesAndNewlines) == input
                    else { return }
                    self.dpnsState = .failed(error.localizedDescription)
                    self.selection = nil
                }
            }
        }
    }

    // MARK: - Summary

    @ViewBuilder
    private func selectionSummary(_ selection: RecipientSelection) -> some View {
        HStack(spacing: 8) {
            Image(systemName: "person.crop.circle.badge.checkmark")
                .foregroundColor(.green)
            VStack(alignment: .leading, spacing: 2) {
                Text(selection.label)
                    .font(.subheadline)
                Text(shortBase58(selection.identityId.toBase58String()))
                    .font(.caption.monospaced())
                    .foregroundColor(.secondary)
            }
        }
    }

    private func shortBase58(_ s: String) -> String {
        guard s.count > 12 else { return s }
        return "\(s.prefix(6))..\(s.suffix(4))"
    }
}
