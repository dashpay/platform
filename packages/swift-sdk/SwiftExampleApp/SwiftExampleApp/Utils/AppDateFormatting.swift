import Foundation

/// Date formatting helpers that pin the year to the Gregorian
/// calendar regardless of the device's region preference.
///
/// `Date.formatted(...)` and bare `DateFormatter()` pick up
/// `Calendar.current`, which is `Buddhist` on Thai regions,
/// `Japanese` on some Japanese regions, `ROC` on Taiwan, etc. The
/// app's UI strings are English and its on-chain timestamps are
/// Unix epoch seconds — showing a "2569 BE" or "R7" year in the
/// timestamp section is wrong for both the content and the
/// surrounding text. These helpers keep the year CE without
/// having to set a process-wide calendar override.
///
/// Two entry points:
///
///  - [`AppDate.formatted`] for display text. Keeps the user's
///    locale for month name / 12h vs 24h preference; only the
///    calendar is swapped.
///  - [`DateFormatter.gregorian`] factory for the places that
///    still reach for the legacy `DateFormatter` API (e.g. custom
///    `dateFormat` strings for UTC or log output). A
///    [`DateFormatter.posixGregorian`] sibling pins the locale as
///    well for stable machine-readable output.
enum AppDate {
    /// Format `date` with Gregorian years, respecting the user's
    /// locale for everything else. Defaults match the app's most
    /// common usage ("Apr 24, 2026, 6:58 AM").
    static func formatted(
        _ date: Date,
        dateStyle: Date.FormatStyle.DateStyle = .abbreviated,
        timeStyle: Date.FormatStyle.TimeStyle = .shortened
    ) -> String {
        var style = Date.FormatStyle(date: dateStyle, time: timeStyle)
        style.calendar = Calendar(identifier: .gregorian)
        return date.formatted(style)
    }

    /// Format an optional `Date`, returning the placeholder for
    /// nil — matches the ergonomics of the previous ad-hoc
    /// `dateString(_ date: Date?)` helpers scattered around the
    /// app.
    static func formatted(
        optional date: Date?,
        dateStyle: Date.FormatStyle.DateStyle = .abbreviated,
        timeStyle: Date.FormatStyle.TimeStyle = .shortened,
        placeholder: String = "None"
    ) -> String {
        guard let date else { return placeholder }
        return formatted(date, dateStyle: dateStyle, timeStyle: timeStyle)
    }

    /// Relative presentation ("5 minutes ago", "yesterday").
    /// Tracks the user's locale; the calendar override doesn't
    /// move the needle here (relative wording is day-based), but
    /// kept for parity in case the style ever gains an epoch
    /// marker.
    static func relative(_ date: Date) -> String {
        date.formatted(
            .relative(presentation: .named)
        )
    }
}

extension DateFormatter {
    /// `DateFormatter` preconfigured with Gregorian calendar.
    /// Callers still set `dateStyle` / `timeStyle` / `dateFormat`
    /// as usual; this just neutralises the calendar drift that
    /// leaks from `Calendar.current` on Thai / Japanese / ROC
    /// regions.
    static func gregorian() -> DateFormatter {
        let f = DateFormatter()
        f.calendar = Calendar(identifier: .gregorian)
        return f
    }

    /// `DateFormatter` pinned to Gregorian calendar + POSIX
    /// locale. Use for machine-readable output (UTC strings, log
    /// timestamps, file names) where both the year epoch and the
    /// numbering system must stay stable across devices.
    static func posixGregorian() -> DateFormatter {
        let f = DateFormatter()
        f.calendar = Calendar(identifier: .gregorian)
        f.locale = Locale(identifier: "en_US_POSIX")
        return f
    }
}
