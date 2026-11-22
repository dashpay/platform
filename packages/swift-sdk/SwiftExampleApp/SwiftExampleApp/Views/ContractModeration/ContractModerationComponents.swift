import SwiftUI
import SwiftDashSDK

/// Where the contract moderation screen drills down to. Value-based, so a
/// re-render of the contract detail screen does not pop a pushed screen.
enum ContractModerationRoute: Hashable {
    case list(ContractModerationList)
    case status
    case removals(documentTypeName: String)
    case teamActions
    /// An action, the status it was listed under and the generation of the
    /// connection the listing was read through (``AppState/sdkGeneration``).
    case teamAction(ContractTeamActionEntry, ContractTeamActionStatus, connection: Int)
    case actionCounts
}

/// The state of one read: what the node answered, or why it did not.
enum ModerationLoad<Value> {
    case idle
    case loading
    case loaded(Value)
    case failed(String)
}

/// The message a failed read shows: the SDK's description of the refusal
/// (a list the contract does not keep, a type that keeps no records, a
/// network below protocol version 14), word for word.
func moderationErrorText(_ error: Error) -> String {
    error.localizedDescription
}

/// A block time in milliseconds, for display.
func moderationDate(ms: UInt64) -> String {
    AppDate.formatted(Date(timeIntervalSince1970: TimeInterval(ms) / 1000))
}

/// Shows `content` only while the connection `scope` was opened on is still
/// current; once it was replaced, only a notice, so no row read through the
/// old connection stays on screen and nothing reads through the new one.
struct ModerationScopeGate<Content: View>: View {
    @EnvironmentObject var appState: AppState
    let scope: ModerationScope
    @ViewBuilder let content: () -> Content

    var body: some View {
        if scope.isOpen(currentGeneration: appState.sdkGeneration, hasSDK: appState.sdk != nil) {
            content()
        } else {
            List {
                ModerationMessageRow(
                    kind: .error,
                    text: "The connection changed since this contract was opened. Reopen the contract.",
                    identifier: "contractModeration.connectionChanged"
                )
            }
        }
    }
}

/// A status line: a spinner while loading, otherwise a message.
struct ModerationMessageRow: View {
    enum Kind {
        case loading
        case error
        case empty
    }

    let kind: Kind
    let text: String
    let identifier: String

    var body: some View {
        HStack(spacing: 8) {
            switch kind {
            case .loading:
                ProgressView()
                Text(text)
                    .font(.caption)
                    .foregroundColor(.secondary)
            case .error:
                Label(text, systemImage: "exclamationmark.triangle")
                    .font(.caption)
                    .foregroundColor(.orange)
                    .textSelection(.enabled)
            case .empty:
                Text(text)
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
        }
        .accessibilityElement(children: .combine)
        .accessibilityIdentifier(identifier)
    }
}

/// A labelled value: an id, a time, a number.
struct ModerationFieldRow: View {
    let label: String
    let value: String
    var monospaced = false

    var body: some View {
        HStack(alignment: .firstTextBaseline) {
            Text(label)
                .font(.caption)
                .foregroundColor(.secondary)
            Spacer()
            Text(value)
                .font(monospaced ? .system(.caption, design: .monospaced) : .caption)
                .lineLimit(1)
                .truncationMode(.middle)
                .textSelection(.enabled)
        }
    }
}

/// A moderation reason as the moderator wrote it.
struct ModerationReasonView: View {
    let reason: ContractModerationReason

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            // The moderator's free text, set apart in a labeled box so it cannot
            // pass for the structured fields below it; its bidi controls shown
            VStack(alignment: .leading, spacing: 2) {
                Text("Moderator's text")
                    .font(.caption2)
                    .foregroundColor(.secondary)
                Text(reason.text.isEmpty ? "(empty)" : BidiControls.visible(reason.text))
                    .font(.caption)
                    .textSelection(.enabled)
            }
            .padding(6)
            .frame(maxWidth: .infinity, alignment: .leading)
            .overlay(
                RoundedRectangle(cornerRadius: 6)
                    .stroke(Color.secondary.opacity(0.4), lineWidth: 1)
            )
            if let code = reason.code {
                Text("Code \(code)")
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }
            ForEach(reason.documents ?? [], id: \.self) { document in
                // The type name is the moderator's text: its bidi controls are shown
                // as escapes, and the id is set apart so the name cannot reorder it
                HStack(spacing: 4) {
                    Text(BidiControls.visible(document.documentTypeName))
                    Text(document.documentId)
                        .lineLimit(1)
                        .truncationMode(.middle)
                }
                .font(.system(.caption2, design: .monospaced))
                .foregroundColor(.secondary)
            }
            if let reasonDocumentId = reason.reasonDocumentId {
                Text("Reason document \(reasonDocumentId)")
                    .font(.system(.caption2, design: .monospaced))
                    .foregroundColor(.secondary)
                    .lineLimit(1)
                    .truncationMode(.middle)
            }
        }
    }
}

/// A list of warnings, oldest first.
struct ModerationWarningsView: View {
    let warnings: [ContractWarning]
    let identifierPrefix: String

    var body: some View {
        ForEach(Array(warnings.enumerated()), id: \.offset) { index, warning in
            VStack(alignment: .leading, spacing: 2) {
                Text("Warned \(moderationDate(ms: warning.warnedAtMs))")
                    .font(.caption2)
                    .foregroundColor(.secondary)
                ModerationReasonView(reason: warning.reason)
            }
            .accessibilityElement(children: .combine)
            .accessibilityIdentifier("\(identifierPrefix).warning.\(index)")
        }
    }
}

extension ContractKeptFieldValue {
    /// The value as one line of JSON-like text: strings and member names quoted
    /// and escaped as JSON escapes them, so text a document held cannot read as
    /// another field.
    var displayText: String {
        switch self {
        case .null: return "null"
        case .bool(let value): return String(value)
        case .int(let value): return String(value)
        case .uint(let value): return String(value)
        case .int128(let value), .uint128(let value): return value
        case .double(let value): return String(value)
        case .string(let value): return Self.quoted(value)
        case .identifier(let value): return Self.quoted(value)
        case .bytes(let value): return "bytes(\(value))"
        case .array(let values): return "[" + values.map(\.displayText).joined(separator: ", ") + "]"
        case .object(let members):
            let fields = members.keys.sorted().map { key in
                "\(Self.quoted(key)): \(members[key]?.displayText ?? "null")"
            }
            return "{" + fields.joined(separator: ", ") + "}"
        }
    }

    /// `text` as a JSON string literal, with the Unicode bidirectional
    /// formatting controls written as visible escapes (`\u{202E}`), so text a
    /// document held cannot reorder what is shown around it.
    static func quoted(_ text: String) -> String {
        let encoder = JSONEncoder()
        encoder.outputFormatting = .withoutEscapingSlashes
        let literal: String
        if let data = try? encoder.encode(text), let encoded = String(data: data, encoding: .utf8) {
            literal = encoded
        } else {
            literal = "\"" + text + "\""
        }
        return BidiControls.visible(literal)
    }
}

/// The Unicode bidirectional formatting controls (the Arabic letter mark, the
/// left-to-right and right-to-left marks, the embeddings and overrides, and
/// the isolates), made visible where text a document or a moderator wrote is
/// shown, so it cannot reorder what is shown around it. Only the display
/// changes: the SDK's values stay as they are.
enum BidiControls {
    static let scalars: Set<UInt32> = [
        0x061C, 0x200E, 0x200F,
        0x202A, 0x202B, 0x202C, 0x202D, 0x202E,
        0x2066, 0x2067, 0x2068, 0x2069,
    ]

    /// `text` with each control written as a visible `\u{202E}` escape.
    static func visible(_ text: String) -> String {
        escaping(text) { String(format: "\\u{%04X}", $0) }
    }

    /// JSON text with each control written as the JSON escape `\u202E`. The
    /// controls can only stand inside a string of encoder output (the JSON
    /// structure is ASCII), where that escape is valid, so the text stays
    /// parseable JSON with the same values.
    static func jsonEscaped(_ json: String) -> String {
        escaping(json) { String(format: "\\u%04X", $0) }
    }

    private static func escaping(_ text: String, _ escape: (UInt32) -> String) -> String {
        var shown = ""
        for scalar in text.unicodeScalars {
            if scalars.contains(scalar.value) {
                shown += escape(scalar.value)
            } else {
                shown.unicodeScalars.append(scalar)
            }
        }
        return shown
    }
}
