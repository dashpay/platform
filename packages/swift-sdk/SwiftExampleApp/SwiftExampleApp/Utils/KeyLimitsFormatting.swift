import Foundation
import SwiftDashSDK

/// Renders a key's protocol 14 limits and contract bounds for the key
/// screens: credits as DASH, expiry as a date plus how far away it is,
/// and bounds as the contract they point at.
enum KeyLimitsFormatting {
    /// 1 DASH = 100 000 000 duffs = 100 000 000 000 credits.
    static let creditsPerDash: UInt64 = 100_000_000_000

    /// Credits rendered as DASH, trimming trailing zeros: 1_000_000_000 → "0.01 DASH".
    static func dash(_ credits: UInt64) -> String {
        "\(dashNumber(credits)) DASH"
    }

    /// The DASH amount alone, at full credit precision: 1_000_000_000 → "0.01".
    static func dashNumber(_ credits: UInt64) -> String {
        let whole = credits / creditsPerDash
        let fraction = credits % creditsPerDash
        var digits = String(fraction)
        digits = String(repeating: "0", count: 11 - digits.count) + digits
        while digits.hasSuffix("0") { digits.removeLast() }
        return digits.isEmpty ? "\(whole)" : "\(whole).\(digits)"
    }

    /// "0.004 of 0.01 DASH left" or, without a remaining figure, "0.01 DASH".
    static func budget(total: UInt64, remaining: UInt64?) -> String {
        guard let remaining else { return dash(total) }
        return "\(dashNumber(remaining)) of \(dash(total)) left"
    }

    /// Whether the key has expired at `now`. The expiry instant itself is
    /// already expired, as consensus counts it.
    static func isExpired(expiresAt: TimestampMillis, now: Date) -> Bool {
        UInt64(now.timeIntervalSince1970 * 1000) >= expiresAt
    }

    /// The expiry as an absolute local date and time.
    static func expiryDate(_ expiresAt: TimestampMillis, locale: Locale = .current) -> String {
        let date = Date(timeIntervalSince1970: TimeInterval(expiresAt) / 1000)
        let formatter = DateFormatter()
        formatter.locale = locale
        formatter.dateStyle = .medium
        formatter.timeStyle = .short
        return formatter.string(from: date)
    }

    /// "Expires in 3 hours" or "Expired 2 days ago".
    static func expiryRelative(_ expiresAt: TimestampMillis, now: Date, locale: Locale = .current) -> String {
        let date = Date(timeIntervalSince1970: TimeInterval(expiresAt) / 1000)
        let formatter = RelativeDateTimeFormatter()
        formatter.locale = locale
        formatter.unitsStyle = .full
        let relative = formatter.localizedString(for: date, relativeTo: now)
        return isExpired(expiresAt: expiresAt, now: now) ? "Expired \(relative)" : "Expires \(relative)"
    }

    /// Short, readable rendering of contract bounds.
    static func bounds(_ bounds: ContractBounds) -> String {
        switch bounds {
        case .singleContract(let id):
            return "Contract \(shortId(id))"
        case .singleContractDocumentType(let id, let documentTypeName):
            return "Contract \(shortId(id)), type \(documentTypeName)"
        }
    }

    static func shortId(_ id: Data) -> String {
        let base58 = id.toBase58String()
        guard base58.count > 13 else { return base58 }
        return "\(base58.prefix(6))…\(base58.suffix(6))"
    }
}
