import SwiftUI
import SwiftDashSDK

/// Entry point for the contracts-tab "Register Contract" flow. Lets
/// the user pick where the new contract's JSON comes from before
/// dropping into the actual register form. Four sources today:
///
/// 1. **From Pasteboard** — auto-detect contract-shaped JSON in the
///    system clipboard. Row turns green when the buffer parses into
///    something we can register; tapping it pre-fills the manual
///    form with `documentSchemas` / `tokenSchemas` / `groups` etc.
/// 2. **From QR Codes** — placeholder. Camera capture + Info.plist
///    permission setup is a separate change; for now this surfaces
///    a "Coming soon" message so the option stays visible in the UI.
/// 3. **Manual** — drop straight into the existing
///    `TransitionDetailView` for `dataContractCreate` with no
///    pre-fills.
/// 4. **Quick Basic Token** — small focused form (singular / plural
///    name, decimals, base supply) that builds a minimal token
///    contract and routes through the same submit path.
struct RegisterContractSourceView: View {
    @EnvironmentObject var platformState: AppState
    @EnvironmentObject var transitionState: TransitionState
    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) private var dismiss

    /// Three-state classification of the current pasteboard contents,
    /// refreshed on appear / when the app returns to the foreground.
    /// Drives both the row's tint + subtitle and the
    /// "tap for details" alert that fires on the invalid path.
    @State private var pasteboardState: PasteboardClassification = .empty
    /// Reason text shown in the "Why isn't this a valid contract?"
    /// alert. Pulled out of the enum case so the alert can survive
    /// the case-association destruction once the alert dismisses.
    @State private var pasteboardInvalidReason: String?

    /// Push state for the four destination views. Two flavors:
    ///   - manual / pasteboard land directly in `TransitionDetailView`.
    ///   - quick-basic-token routes through its own form first.
    @State private var presentManual = false
    @State private var presentPasteboard = false
    @State private var presentQuickBasicToken = false
    @State private var showQRComingSoon = false

    var body: some View {
        NavigationStack {
            List {
                Section {
                    pasteboardRow
                    qrRow
                    manualRow
                    quickTokenRow
                }
            }
            .navigationTitle("Register Contract")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Cancel") { dismiss() }
                }
            }
            .onAppear { refreshPasteboardCandidate() }
            // SwiftUI doesn't surface a clean foreground hook on
            // sheets; the scenePhase observer below covers the case
            // where the user copies the contract JSON in another app
            // and comes back to ours.
            .onReceive(NotificationCenter.default.publisher(
                for: UIApplication.didBecomeActiveNotification
            )) { _ in refreshPasteboardCandidate() }
            .alert("Coming Soon", isPresented: $showQRComingSoon) {
                Button("OK", role: .cancel) { }
            } message: {
                Text("QR code scanning will land in a follow-up change. For now use “From Pasteboard” after copying the contract JSON.")
            }
            .alert(
                "Why isn’t this a valid contract?",
                isPresented: Binding(
                    get: { pasteboardInvalidReason != nil },
                    set: { if !$0 { pasteboardInvalidReason = nil } }
                )
            ) {
                Button("OK", role: .cancel) { pasteboardInvalidReason = nil }
            } message: {
                Text(pasteboardInvalidReason ?? "")
            }
            .navigationDestination(isPresented: $presentManual) {
                TransitionDetailView(
                    transitionKey: "dataContractCreate",
                    transitionLabel: "Register Contract"
                )
            }
            .navigationDestination(isPresented: $presentPasteboard) {
                if case let .valid(candidate) = pasteboardState {
                    TransitionDetailView(
                        transitionKey: "dataContractCreate",
                        transitionLabel: "Register from Pasteboard",
                        initialInputs: candidate.formInputs,
                        initialCheckboxInputs: candidate.checkboxInputs
                    )
                }
            }
            .navigationDestination(isPresented: $presentQuickBasicToken) {
                QuickBasicTokenView()
            }
        }
    }

    // MARK: Rows

    /// Pasteboard option. Tri-state: green when a contract candidate
    /// is detected, orange when the clipboard parses as JSON but
    /// doesn't pass our register-able-contract check (tap to see
    /// why), gray when the clipboard is empty or non-JSON.
    @ViewBuilder
    private var pasteboardRow: some View {
        Button {
            switch pasteboardState {
            case .valid:
                presentPasteboard = true
            case .invalid(let reason):
                pasteboardInvalidReason = reason
            case .empty:
                break
            }
        } label: {
            sourceRow(
                title: "From Pasteboard",
                subtitle: pasteboardSubtitle,
                systemImage: "doc.on.clipboard",
                tint: pasteboardTint
            )
        }
        .disabled(pasteboardState.isEmpty)
    }

    @ViewBuilder
    private var qrRow: some View {
        Button {
            showQRComingSoon = true
        } label: {
            sourceRow(
                title: "From QR Codes",
                subtitle: "Scan one or more QR codes (coming soon)",
                systemImage: "qrcode.viewfinder",
                tint: .secondary
            )
        }
    }

    @ViewBuilder
    private var manualRow: some View {
        Button {
            presentManual = true
        } label: {
            sourceRow(
                title: "Manual",
                subtitle: "Build the contract step by step",
                systemImage: "square.and.pencil",
                tint: .blue
            )
        }
    }

    @ViewBuilder
    private var quickTokenRow: some View {
        Button {
            presentQuickBasicToken = true
        } label: {
            sourceRow(
                title: "Quick Basic Token",
                subtitle: "Name, supply, decimals — that's it",
                systemImage: "circle.hexagongrid",
                tint: .blue
            )
        }
    }

    /// Shared row layout for the four source options. Pulled into a
    /// helper so the green / gray accent state lives in one place.
    @ViewBuilder
    private func sourceRow(
        title: String,
        subtitle: String,
        systemImage: String,
        tint: Color
    ) -> some View {
        HStack(spacing: 14) {
            Image(systemName: systemImage)
                .font(.title2)
                .foregroundColor(tint)
                .frame(width: 30)
            VStack(alignment: .leading, spacing: 2) {
                Text(title)
                    .font(.headline)
                    .foregroundColor(.primary)
                Text(subtitle)
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .lineLimit(2)
            }
            Spacer()
            Image(systemName: "chevron.right")
                .font(.caption)
                .foregroundColor(.secondary)
        }
        .contentShape(Rectangle())
        .padding(.vertical, 4)
    }

    // MARK: Pasteboard handling

    private var pasteboardSubtitle: String {
        switch pasteboardState {
        case .valid(let candidate):
            return "Detected: \(candidate.summary)"
        case .invalid:
            return "JSON detected, not a valid contract — tap for details"
        case .empty:
            return "No valid contract on clipboard"
        }
    }

    private var pasteboardTint: Color {
        switch pasteboardState {
        case .valid: return .green
        case .invalid: return .orange
        case .empty: return .secondary
        }
    }

    /// Re-classify the current pasteboard contents. Gated behind
    /// `UIPasteboard.general.hasStrings` because reading
    /// `UIPasteboard.general.string` directly fires the iOS 14+
    /// "Pasted from <App>" privacy banner — which is noisy when
    /// triggered on every foreground transition for users who
    /// just switched apps without intending to paste anything.
    /// `hasStrings` is the documented iOS API for "is there a
    /// string-like item on the clipboard" that does NOT trigger
    /// the banner. When it returns false we treat the pasteboard
    /// as empty.
    private func refreshPasteboardCandidate() {
        guard UIPasteboard.general.hasStrings else {
            pasteboardState = PasteboardContractCandidate.classify("")
            return
        }
        let raw = UIPasteboard.general.string ?? ""
        pasteboardState = PasteboardContractCandidate.classify(raw)
    }
}

/// Three-state pasteboard classification used by
/// `RegisterContractSourceView` to decide row tint + tap behavior.
enum PasteboardClassification {
    /// Clipboard is empty / whitespace. Row is disabled.
    case empty
    /// Clipboard has content but it isn't a register-able contract.
    /// `reason` is shown in the alert when the user taps the row to
    /// learn why we rejected it.
    case invalid(reason: String)
    /// Clipboard parses cleanly into a register-able contract.
    case valid(PasteboardContractCandidate)

    var isEmpty: Bool {
        if case .empty = self { return true }
        return false
    }
}

/// Result of attempting to interpret a clipboard string as a register-able
/// contract. Carries form-input dictionaries shaped for
/// `TransitionDetailView` so the receiving view can apply them
/// directly without re-parsing.
struct PasteboardContractCandidate {
    /// Pre-filled `formInputs` for `TransitionDetailView`. Always
    /// supplies `documentSchemas` and/or `tokenSchemas` JSON,
    /// optionally `groups`, `keywords`, `description`.
    let formInputs: [String: String]
    /// Pre-filled checkbox values: `canBeDeleted`, `readonly`,
    /// `keepsHistory`, etc.
    let checkboxInputs: [String: Bool]
    /// Compact human-readable summary shown on the source row to tell
    /// the user what we detected — "2 doc types · 1 token", etc.
    let summary: String

    /// Best-effort classifier. Returns:
    ///   - `.empty` for an empty / whitespace clipboard.
    ///   - `.valid(candidate)` for content we recognize as a
    ///     register-able contract.
    ///   - `.invalid(reason)` for everything else, with a single-line
    ///     reason the UI shows in the "Why isn't this a valid
    ///     contract?" alert.
    ///
    /// Recognized shapes:
    ///   1. A full contract dict with top-level `documents` /
    ///      `documentSchemas` / `tokens` / `tokenSchemas`.
    ///   2. A bare document-schemas dict (every value is itself an
    ///      object containing a `properties` key).
    ///   3. A bare token-schemas dict (keys parse as integers — the
    ///      slot positions for tokens).
    static func classify(_ raw: String) -> PasteboardClassification {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.isEmpty {
            return .empty
        }

        guard let data = trimmed.data(using: .utf8) else {
            return .invalid(reason: "Pasteboard contains text that isn’t UTF-8 encodable.")
        }

        let json: Any
        do {
            json = try JSONSerialization.jsonObject(with: data, options: [.fragmentsAllowed])
        } catch {
            // The first ~80 chars of the pasteboard text help the
            // user spot whether they grabbed a URL, hex, etc. by
            // mistake. Truncate so the alert stays readable.
            let preview = trimmed.prefix(80)
            let suffix = trimmed.count > 80 ? "…" : ""
            return .invalid(
                reason: "Pasteboard contains text but it isn’t valid JSON.\n\nFirst characters: \"\(preview)\(suffix)\""
            )
        }

        guard let dict = json as? [String: Any] else {
            return .invalid(
                reason: "JSON parsed, but the top-level value is a \(typeName(of: json)) — a contract must be a JSON object."
            )
        }

        // Case 1: full contract dict.
        if dict["documents"] != nil
            || dict["documentSchemas"] != nil
            || dict["tokens"] != nil
            || dict["tokenSchemas"] != nil {
            return classifyFullContractDict(dict)
        }

        // Case 2: looks like document-schemas — every value is an
        // object that itself has a `properties` key (the marker used
        // by every document type schema).
        let looksLikeDocumentSchemas = !dict.isEmpty && dict.allSatisfy { _, value in
            guard let inner = value as? [String: Any] else { return false }
            return inner["properties"] != nil
                || inner["type"] as? String == "object"
        }
        if looksLikeDocumentSchemas {
            if let candidate = wrap(documentSchemasDict: dict) {
                return .valid(candidate)
            }
            return .invalid(reason: "Document-schema-shaped JSON, but it failed to re-serialize.")
        }

        // Case 3: looks like token-schemas — every key parses as an
        // integer (the slot position).
        let looksLikeTokenSchemas = !dict.isEmpty && dict.allSatisfy { key, value in
            Int(key) != nil && (value is [String: Any])
        }
        if looksLikeTokenSchemas {
            if let candidate = wrap(tokenSchemasDict: dict) {
                return .valid(candidate)
            }
            return .invalid(reason: "Token-schema-shaped JSON, but it failed to re-serialize.")
        }

        // Fall-through: JSON object but doesn't match any contract
        // shape we know about. Tell the user the top-level keys we
        // saw — most often the user pasted some other JSON document
        // (an identity, a document instance, etc.).
        let keys = Array(dict.keys.sorted().prefix(8))
        let keyList = keys.isEmpty
            ? "no top-level keys"
            : "top-level keys: \(keys.joined(separator: ", "))"
        return .invalid(
            reason: "The JSON parsed, but it doesn’t look like a contract.\n\nA contract needs at least a `tokens` (or `tokenSchemas`) or a `documents` (or `documentSchemas`) field. Found \(keyList)."
        )
    }

    /// Classify a JSON dict that already contains at least one of
    /// the contract-shaped keys. Returns `.valid` when we can
    /// extract registerable schemas, `.invalid(reason)` when those
    /// fields are present but empty / malformed.
    private static func classifyFullContractDict(
        _ dict: [String: Any]
    ) -> PasteboardClassification {
        guard let candidate = fromFullContractDict(dict) else {
            // The user's payload had `documents`/`tokens` keys but
            // every one of them was empty or non-object. Surface a
            // specific reason instead of a generic "not a contract".
            let docsCount = (dict["documents"] as? [String: Any])?.count
                ?? (dict["documentSchemas"] as? [String: Any])?.count
                ?? -1
            let tokensCount = (dict["tokens"] as? [String: Any])?.count
                ?? (dict["tokenSchemas"] as? [String: Any])?.count
                ?? -1
            if docsCount == 0 && tokensCount == 0 {
                return .invalid(
                    reason: "The JSON has the shape of a contract but both `documents` / `documentSchemas` and `tokens` / `tokenSchemas` are empty. Add at least one document type or token to register a contract."
                )
            }
            return .invalid(
                reason: "The JSON has contract-shaped keys but they didn’t re-serialize cleanly. Check the payload for invalid values."
            )
        }
        return .valid(candidate)
    }

    /// Pretty type name used by `classify` when the top-level JSON
    /// value isn't an object — gives the user a concrete hint about
    /// what they pasted instead.
    private static func typeName(of value: Any) -> String {
        switch value {
        case is [Any]: return "array"
        case is String: return "string"
        case is NSNumber: return "number"
        case is NSNull: return "null"
        default: return "value"
        }
    }

    private static func fromFullContractDict(
        _ dict: [String: Any]
    ) -> PasteboardContractCandidate? {
        var formInputs: [String: String] = [:]
        var checkboxInputs: [String: Bool] = [:]

        // Pull document schemas. Some payloads use `documents`
        // (legacy on-chain shape), others use `documentSchemas`
        // (the create-state-transition input name); we accept both.
        // Skip empty maps — sending `"{}"` to the form would let it
        // pass the "at least one schema" guard with no real content.
        let docs = (dict["documents"] as? [String: Any])
            ?? (dict["documentSchemas"] as? [String: Any])
        if let docs = docs, !docs.isEmpty, let str = jsonString(from: docs) {
            formInputs["documentSchemas"] = str
        }

        // Pull token schemas (same legacy/new key dance, same
        // empty-map filter).
        let tokens = (dict["tokens"] as? [String: Any])
            ?? (dict["tokenSchemas"] as? [String: Any])
        if let tokens = tokens, !tokens.isEmpty, let str = jsonString(from: tokens) {
            formInputs["tokenSchemas"] = str
        }

        // Groups travel as either a dict (chain shape) or an array
        // (state-transition input shape). Convert the dict form into
        // the array `TransitionDetailView` expects.
        if let groupsArr = dict["groups"] as? [[String: Any]],
           let str = jsonString(from: groupsArr) {
            formInputs["groups"] = str
        } else if let groupsDict = dict["groups"] as? [String: Any] {
            // Sort by integer key when possible so the array order
            // matches the on-chain group-position semantics —
            // unordered Swift dict iteration would otherwise yield
            // a non-deterministic id sequence in
            // `TransitionDetailView`. Always carry an `id` field
            // through: integer-parseable keys become Int (matches
            // the chain shape), other keys round-trip as the
            // original String so the form has *some* identifier
            // to render. Earlier revisions silently dropped the
            // id when the key wasn't integer-parseable, leaving
            // the form with anonymous group entries.
            let sortedEntries = groupsDict.sorted { lhs, rhs in
                switch (Int(lhs.key), Int(rhs.key)) {
                case let (.some(l), .some(r)): return l < r
                case (.some, .none): return true
                case (.none, .some): return false
                case (.none, .none): return lhs.key < rhs.key
                }
            }
            let asArray: [[String: Any]] = sortedEntries
                .compactMap { (key, value) in
                    guard var entry = value as? [String: Any] else { return nil }
                    if let id = Int(key) {
                        entry["id"] = id
                    } else {
                        entry["id"] = key
                    }
                    return entry
                }
            if let str = jsonString(from: asArray) {
                formInputs["groups"] = str
            }
        }

        // Optional metadata: keywords + description.
        if let kw = dict["keywords"] as? [String] {
            formInputs["keywords"] = kw.joined(separator: ", ")
        }
        if let desc = dict["description"] as? String {
            formInputs["description"] = desc
        }

        // Top-level boolean flags. The state-transition input shape
        // has these at the root, while the on-chain format-version-1
        // shape nests them under `config`. Check both, top-level
        // first — explicit roots win when the payload is a hybrid.
        let configDict = dict["config"] as? [String: Any]
        for key in [
            "canBeDeleted", "readonly", "keepsHistory",
            "documentsKeepHistoryContractDefault",
            "documentsMutableContractDefault",
            "documentsCanBeDeletedContractDefault",
            "requiresIdentityEncryptionBoundedKey",
            "requiresIdentityDecryptionBoundedKey",
        ] {
            let value = (dict[key] as? Bool) ?? (configDict?[key] as? Bool)
            if let v = value {
                // The existing form mixes string-encoded booleans
                // (formInputs) with native Bool checkboxInputs.
                // Set both so whichever path reads the value picks
                // it up.
                formInputs[key] = v ? "true" : "false"
                checkboxInputs[key] = v
            }
        }

        let summary = describe(
            documentCount: docs?.count ?? 0,
            tokenCount: tokens?.count ?? 0,
            hasGroups: dict["groups"] != nil
        )

        // Reject empty payloads — a parseable JSON object that
        // doesn't actually carry any schemas isn't useful here and
        // would just trip the form's "at least one schema is
        // required" guard later.
        guard formInputs["documentSchemas"] != nil
            || formInputs["tokenSchemas"] != nil else {
            return nil
        }

        return PasteboardContractCandidate(
            formInputs: formInputs,
            checkboxInputs: checkboxInputs,
            summary: summary
        )
    }

    private static func wrap(
        documentSchemasDict: [String: Any]
    ) -> PasteboardContractCandidate? {
        guard let str = jsonString(from: documentSchemasDict) else { return nil }
        return PasteboardContractCandidate(
            formInputs: ["documentSchemas": str],
            checkboxInputs: [:],
            summary: describe(
                documentCount: documentSchemasDict.count,
                tokenCount: 0,
                hasGroups: false
            )
        )
    }

    private static func wrap(
        tokenSchemasDict: [String: Any]
    ) -> PasteboardContractCandidate? {
        guard let str = jsonString(from: tokenSchemasDict) else { return nil }
        return PasteboardContractCandidate(
            formInputs: ["tokenSchemas": str],
            checkboxInputs: [:],
            summary: describe(
                documentCount: 0,
                tokenCount: tokenSchemasDict.count,
                hasGroups: false
            )
        )
    }

    /// Compact "2 doc types · 1 token · groups" summary used by the
    /// row subtitle.
    private static func describe(
        documentCount: Int,
        tokenCount: Int,
        hasGroups: Bool
    ) -> String {
        var parts: [String] = []
        if documentCount > 0 {
            parts.append("\(documentCount) doc type\(documentCount == 1 ? "" : "s")")
        }
        if tokenCount > 0 {
            parts.append("\(tokenCount) token\(tokenCount == 1 ? "" : "s")")
        }
        if hasGroups {
            parts.append("groups")
        }
        return parts.isEmpty ? "valid contract JSON" : parts.joined(separator: " · ")
    }

    /// Round-trip a value back to its compact JSON-string form.
    private static func jsonString(from value: Any) -> String? {
        guard let data = try? JSONSerialization.data(
            withJSONObject: value,
            options: [.sortedKeys]
        ),
              let str = String(data: data, encoding: .utf8) else {
            return nil
        }
        return str
    }
}
