import SwiftUI
import SwiftDashSDK

/// Where the contract moderation screen drills down to. Value-based, so a
/// re-render of the contract detail screen does not pop a pushed screen.
enum ContractModerationRoute: Hashable {
    case list(ContractModerationList)
    case status
    case removals(documentTypeName: String)
    case teamActions
    case teamAction(ContractTeamActionEntry, ContractTeamActionStatus)
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
            Text(reason.text.isEmpty ? "(no reason given)" : reason.text)
                .font(.caption)
            if let code = reason.code {
                Text("Code \(code)")
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }
            ForEach(reason.documents ?? [], id: \.self) { document in
                Text("\(document.documentTypeName) \(document.documentId)")
                    .font(.system(.caption2, design: .monospaced))
                    .foregroundColor(.secondary)
                    .lineLimit(1)
                    .truncationMode(.middle)
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
        case .double(let value): return String(value)
        case .string(let value): return Self.quoted(value)
        case .array(let values): return "[" + values.map(\.displayText).joined(separator: ", ") + "]"
        case .object(let members):
            let fields = members.keys.sorted().map { key in
                "\(Self.quoted(key)): \(members[key]?.displayText ?? "null")"
            }
            return "{" + fields.joined(separator: ", ") + "}"
        }
    }

    /// `text` as a JSON string literal.
    static func quoted(_ text: String) -> String {
        let encoder = JSONEncoder()
        encoder.outputFormatting = .withoutEscapingSlashes
        guard let data = try? encoder.encode(text),
              let literal = String(data: data, encoding: .utf8) else {
            return "\"" + text + "\""
        }
        return literal
    }
}
