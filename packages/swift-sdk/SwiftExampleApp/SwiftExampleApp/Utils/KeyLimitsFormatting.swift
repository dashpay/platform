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

    /// The inverse of `dashNumber`: DASH typed by a person, as credits.
    /// Exact digit arithmetic, never `Double`. The decimal separator is "."
    /// or the locale's own (the decimal pad types that one). Surrounding
    /// whitespace is ignored. `nil` for text that is not a plain unsigned
    /// decimal number, for more than 11 decimals (finer than a credit),
    /// and for an amount past `UInt64.max` credits.
    static func credits(fromDash text: String, locale: Locale = .current) -> UInt64? {
        var normalized = text.trimmingCharacters(in: .whitespacesAndNewlines)
        if let separator = locale.decimalSeparator, separator != "." {
            normalized = normalized.replacingOccurrences(of: separator, with: ".")
        }
        let parts = normalized.split(separator: ".", omittingEmptySubsequences: false)
        guard parts.count == 1 || parts.count == 2 else { return nil }
        let wholeDigits = parts[0]
        let fractionDigits = parts.count == 2 ? parts[1] : ""
        guard !(wholeDigits.isEmpty && fractionDigits.isEmpty),
              wholeDigits.allSatisfy({ $0.isASCII && $0.isNumber }),
              fractionDigits.allSatisfy({ $0.isASCII && $0.isNumber }),
              fractionDigits.count <= 11
        else { return nil }

        let whole = wholeDigits.isEmpty ? 0 : UInt64(wholeDigits)
        let paddedFraction = fractionDigits + String(repeating: "0", count: 11 - fractionDigits.count)
        guard let whole, let fraction = UInt64(paddedFraction) else { return nil }
        let (wholeCredits, multiplyOverflow) = whole.multipliedReportingOverflow(by: creditsPerDash)
        guard !multiplyOverflow else { return nil }
        let (total, addOverflow) = wholeCredits.addingReportingOverflow(fraction)
        return addOverflow ? nil : total
    }

    /// A picked date as block time milliseconds, the unit of `expiresAt`,
    /// rounded down to the millisecond. `nil` before 1970.
    static func millis(from date: Date) -> TimestampMillis? {
        let millis = (date.timeIntervalSince1970 * 1000).rounded(.down)
        guard millis.isFinite, millis >= 0 else { return nil }
        return TimestampMillis(exactly: millis)
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
