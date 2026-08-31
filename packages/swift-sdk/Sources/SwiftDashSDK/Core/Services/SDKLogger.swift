import CryptoKit
import Foundation

// MARK: - Structured Diagnostic Logging

/// Categories intentionally kept small for the first diagnostic-log rollout.
/// They identify the subsystem without putting user-controlled data in the
/// logger name.
public enum SDKLogCategory: String, Sendable {
    case lifecycle
    case persistence
    case shielded
}

public enum SDKLogSeverity: String, Sendable {
    case debug = "DEBUG"
    case info = "INFO"
    case warning = "WARN"
    case error = "ERROR"
}

/// A value that is safe to include in a diagnostic event.
///
/// Use ``reference(_:)`` for identifiers. References are written as the first
/// 12 hexadecimal characters of their SHA-256 digest; the original value is
/// never passed to the sink.
public enum SDKLogValue: Sendable {
    case publicText(String)
    case integer(Int64)
    case unsignedInteger(UInt64)
    case double(Double)
    case boolean(Bool)
    case reference(Data)
    case referenceString(String)
}

/// Serial file sink used by the structured logger. All writes, including
/// callbacks entering Swift concurrently from FFI threads, are protected by a
/// single lock so a diagnostic line is appended atomically.
final class SDKLogFileSink: @unchecked Sendable {
    let fileURL: URL

    private let lock = NSLock()
    private let handle: FileHandle

    init(sessionDirectory: URL, fileManager: FileManager = .default) throws {
        let swiftDirectory = sessionDirectory.appendingPathComponent("swift", isDirectory: true)
        try fileManager.createDirectory(at: swiftDirectory, withIntermediateDirectories: true)

        fileURL = swiftDirectory.appendingPathComponent("run.log", isDirectory: false)
        if !fileManager.fileExists(atPath: fileURL.path) {
            guard fileManager.createFile(atPath: fileURL.path, contents: nil) else {
                throw CocoaError(.fileWriteUnknown)
            }
        }

        handle = try FileHandle(forWritingTo: fileURL)
        try handle.seekToEnd()
    }

    deinit {
        try? handle.close()
    }

    func write(_ line: String) {
        guard let data = (line + "\n").data(using: .utf8) else { return }
        lock.withLock {
            try? handle.write(contentsOf: data)
        }
    }

    func flush() {
        lock.withLock {
            try? handle.synchronize()
        }
    }
}

enum SDKLogFormatter {
    static let maximumErrorDescriptionLength = 1_024

    static func format(
        timestamp: Date = Date(),
        event: String,
        category: SDKLogCategory,
        severity: SDKLogSeverity,
        fields: [String: SDKLogValue] = [:],
        error: Error? = nil,
        redacting sensitiveValues: [String] = []
    ) -> String {
        var renderedFields = fields.mapValues(render)

        if let error {
            let nsError = error as NSError
            renderedFields["error_code"] = String(nsError.code)
            renderedFields["error_domain"] = quoted(redact(nsError.domain, values: sensitiveValues))
            renderedFields["error_message"] = quoted(
                String(
                    redact(nsError.localizedDescription, values: sensitiveValues)
                        .prefix(maximumErrorDescriptionLength)
                )
            )
            renderedFields["error_type"] = quoted(String(reflecting: type(of: error)))
        }

        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        formatter.timeZone = TimeZone(secondsFromGMT: 0)

        var components = [
            formatter.string(from: timestamp),
            severity.rawValue,
            "swift.\(category.rawValue)",
            "event=\(token(event))",
        ]
        components.append(contentsOf: renderedFields.keys.sorted().map { key in
            "\(token(key))=\(renderedFields[key]!)"
        })
        return components.joined(separator: " ")
    }

    static func reference(_ data: Data) -> String {
        SHA256.hash(data: data).prefix(6).map { String(format: "%02x", $0) }.joined()
    }

    static func reference(_ string: String) -> String {
        reference(Data(string.utf8))
    }

    private static func render(_ value: SDKLogValue) -> String {
        switch value {
        case .publicText(let value):
            return quoted(value)
        case .integer(let value):
            return String(value)
        case .unsignedInteger(let value):
            return String(value)
        case .double(let value):
            return String(value)
        case .boolean(let value):
            return value ? "true" : "false"
        case .reference(let value):
            return reference(value)
        case .referenceString(let value):
            return reference(value)
        }
    }

    /// Identifiers and paths often arrive embedded in a dependency's
    /// localized error text. Callers can provide exact sensitive values; the
    /// generic patterns are a second line of defence for paths and the long
    /// hex/base58 forms used by wallet, identity and outpoint identifiers.
    private static func redact(_ value: String, values: [String]) -> String {
        var result = value
        for sensitive in values.filter({ !$0.isEmpty }).sorted(by: { $0.count > $1.count }) {
            let isShortToken = sensitive.count < 8
                && sensitive.unicodeScalars.allSatisfy(CharacterSet.alphanumerics.contains)
            if isShortToken {
                let escaped = NSRegularExpression.escapedPattern(for: sensitive)
                let pattern = "(?<![A-Za-z0-9])\(escaped)(?![A-Za-z0-9])"
                guard let regex = try? NSRegularExpression(pattern: pattern) else { continue }
                let range = NSRange(result.startIndex..<result.endIndex, in: result)
                result = regex.stringByReplacingMatches(
                    in: result,
                    range: range,
                    withTemplate: "<redacted>"
                )
            } else {
                result = result.replacingOccurrences(of: sensitive, with: "<redacted>")
            }
        }

        let patterns = [
            #"(?<![A-Za-z0-9])(?:file://)?/(?:[^\s\"']+/)*[^\s\"']*"#,
            #"\b[0-9a-fA-F]{32,}\b"#,
            #"\b[1-9A-HJ-NP-Za-km-z]{32,}\b"#,
        ]
        for pattern in patterns {
            guard let regex = try? NSRegularExpression(pattern: pattern) else { continue }
            let range = NSRange(result.startIndex..<result.endIndex, in: result)
            result = regex.stringByReplacingMatches(
                in: result,
                range: range,
                withTemplate: "<redacted>"
            )
        }
        return result
    }

    private static func token(_ value: String) -> String {
        let allowed = CharacterSet.alphanumerics.union(CharacterSet(charactersIn: "._-"))
        let scalars = value.unicodeScalars.map { allowed.contains($0) ? String($0) : "_" }
        let result = scalars.joined()
        return result.isEmpty ? "unknown" : result
    }

    private static func quoted(_ value: String) -> String {
        var escaped = ""
        for scalar in value.unicodeScalars {
            switch scalar.value {
            case 0x22: escaped += "\\\""
            case 0x5c: escaped += "\\\\"
            case 0x0a: escaped += "\\n"
            case 0x0d: escaped += "\\r"
            case 0x09: escaped += "\\t"
            case 0x00...0x1f, 0x7f:
                escaped += String(format: "\\u%04x", scalar.value)
            default:
                escaped.unicodeScalars.append(scalar)
            }
        }
        return "\"\(escaped)\""
    }
}

private final class SDKLoggerState: @unchecked Sendable {
    private let lock = NSLock()
    private var sink: SDKLogFileSink?
    private var includeDebug = false

    func installSink(at sessionDirectory: URL, includeDebug: Bool) -> Bool {
        do {
            let newSink = try SDKLogFileSink(sessionDirectory: sessionDirectory)
            lock.withLock {
                sink = newSink
                self.includeDebug = includeDebug
            }
            return true
        } catch {
            return false
        }
    }

    func updateDebugSetting(_ includeDebug: Bool) {
        lock.withLock {
            self.includeDebug = includeDebug
        }
    }

    func destination(for severity: SDKLogSeverity) -> SDKLogFileSink? {
        lock.withLock {
            guard severity != .debug || includeDebug else { return nil }
            return sink
        }
    }

    func flush() {
        lock.withLock { sink }?.flush()
    }
}

// MARK: - Logging Preferences

public enum LoggingPreset: String {
    case low
    case medium
    case high

    var priority: Int {
        switch self {
        case .low: return 0
        case .medium: return 1
        case .high: return 2
        }
    }

    func allows(_ threshold: LoggingPreset) -> Bool {
        priority >= threshold.priority
    }
}

public enum LoggingPreferences {
    private static let defaultsKey = "SwiftSDKLogLevel"

    /// Sessions beyond this count are deleted at startup, oldest
    /// first. File logging runs on devices too (beta diagnostics),
    /// so growth must stay bounded without anyone thinking about it.
    private static let maxRetainedSessions = 20

    /// Total-bytes quota across retained sessions. The Rust side
    /// appends without rotation, so individual sessions can be
    /// arbitrarily large — a count cap alone doesn't bound disk
    /// use. Sessions are kept newest-first until the quota is hit;
    /// the current session is always kept (its in-flight growth
    /// can only be bounded by rotation on the Rust side).
    private static let maxRetainedBytes: UInt64 = 100 * 1024 * 1024

    /// Root under which each launch creates one timestamped session
    /// directory of per-crate `run.log` files. Exposed so diagnostics
    /// features (log export) can enumerate sessions without
    /// re-deriving the layout.
    public static var logsRootDirectory: URL? {
        FileManager.default
            .urls(for: .libraryDirectory, in: .userDomainMask).first?
            .appendingPathComponent("Logs", isDirectory: true)
            .appendingPathComponent("SwiftDashSDK", isDirectory: true)
    }

    /// Session directory of the current launch, set once `configure()`
    /// installs at least one of the independent Swift or Rust sinks.
    /// `nil` only when neither sink could be installed.
    @MainActor
    public private(set) static var currentSessionDirectory: URL?

    /// The tracing subscriber is process-global and first-init-wins,
    /// so the install must run at most once per process. Without
    /// this guard, every `configure()` call after the first (e.g.
    /// a bootstrap retry) would make the Rust initializer lay out a
    /// fresh session directory of empty log files before `try_init`
    /// discovers the existing subscriber and bails — leaving decoy
    /// "newest" sessions that logging never writes to.
    @MainActor
    private static var didInstallLogging = false

    @discardableResult
    @MainActor
    public static func configure() -> LoggingPreset {
        let preset = loadPreset()
        let enableSwiftVerbose = preset == .high

        if !didInstallLogging {
            didInstallLogging = true
            let sessionRoot = launchLogPaths()
            let swiftSinkInstalled = sessionRoot.map {
                SDKLogger.installFileSink(at: $0, includeDebug: enableSwiftVerbose)
            } ?? false
            let rustSinkInstalled = sessionRoot.map {
                SDK.enableFileLogging(level: .info, sessionRoot: $0.path)
            } ?? false

            if !rustSinkInstalled {
                // Rust tracing is process-global and first-init-wins, but a
                // failed file destination must not silence console logging.
                SDK.enableLogging(level: .info)
            }

            if let sessionRoot, swiftSinkInstalled || rustSinkInstalled {
                currentSessionDirectory = sessionRoot
                pruneOldSessions(keeping: sessionRoot)
            }

            SDKLogger.event(
                "logging_configured",
                category: .lifecycle,
                severity: .info,
                fields: [
                    "preset": .publicText(preset.rawValue),
                    "rust_sink_active": .boolean(rustSinkInstalled),
                    "swift_sink_active": .boolean(swiftSinkInstalled),
                ]
            )
        } else {
            SDKLogger.updateDebugSetting(enableSwiftVerbose)
        }

        setenv("SPV_SWIFT_LOG", enableSwiftVerbose ? "1" : "0", 1)

        return preset
    }

    private static func launchLogPaths() -> URL? {
        guard let root = logsRootDirectory else { return nil }

        let formatter = DateFormatter()
        formatter.locale = Locale(identifier: "en_US_POSIX")
        formatter.timeZone = TimeZone(secondsFromGMT: 0)
        formatter.dateFormat = "yyyy-MM-dd'T'HH-mm-ss'Z'"

        return root
            .appendingPathComponent(formatter.string(from: Date()), isDirectory: true)
    }

    /// Delete session directories past either retention bound —
    /// `maxRetainedSessions` count or `maxRetainedBytes` total —
    /// walking newest-first so what survives is always the most
    /// recent history. The current session is always kept
    /// regardless of where its stamp sorts or how big it is.
    /// Runs detached — deleting multi-megabyte directories has no
    /// business on the main actor during launch.
    private static func pruneOldSessions(keeping current: URL) {
        guard let root = logsRootDirectory else { return }
        Task.detached(priority: .utility) {
            let fm = FileManager.default
            guard let entries = try? fm.contentsOfDirectory(
                at: root,
                includingPropertiesForKeys: [.isDirectoryKey],
                options: [.skipsHiddenFiles]
            ) else { return }

            // Session stamps are fixed-width UTC ISO timestamps, so
            // lexicographic order is chronological order.
            let sessions = entries
                .filter { (try? $0.resourceValues(forKeys: [.isDirectoryKey]))?.isDirectory == true }
                .sorted { $0.lastPathComponent > $1.lastPathComponent }

            let currentName = current.lastPathComponent
            var keptCount = 0
            var keptBytes: UInt64 = 0
            for session in sessions {
                if session.lastPathComponent == currentName {
                    // Counted against the byte quota so a huge
                    // just-finished-syncing session pushes old
                    // history out, but never deleted itself.
                    keptCount += 1
                    keptBytes += directoryBytes(of: session)
                    continue
                }
                let bytes = directoryBytes(of: session)
                if keptCount >= maxRetainedSessions
                    || keptBytes + bytes > maxRetainedBytes {
                    try? fm.removeItem(at: session)
                } else {
                    keptCount += 1
                    keptBytes += bytes
                }
            }
        }
    }

    private static func directoryBytes(of directory: URL) -> UInt64 {
        guard let enumerator = FileManager.default.enumerator(
            at: directory,
            includingPropertiesForKeys: [.totalFileAllocatedSizeKey, .fileSizeKey]
        ) else { return 0 }

        var total: UInt64 = 0
        for case let file as URL in enumerator {
            let values = try? file.resourceValues(
                forKeys: [.totalFileAllocatedSizeKey, .fileSizeKey]
            )
            total += UInt64(values?.totalFileAllocatedSize ?? values?.fileSize ?? 0)
        }
        return total
    }

    public static var preset: LoggingPreset { loadPreset() }

    public static var shouldEmitDefaultLogs: Bool { preset == .high }

    public static func allows(_ threshold: LoggingPreset) -> Bool {
        preset.allows(threshold)
    }

    private static func loadPreset() -> LoggingPreset {
        if let stored = UserDefaults.standard.string(forKey: defaultsKey)?.lowercased(),
           let preset = LoggingPreset(rawValue: stored) {
            return preset
        }
        return .low
    }
}

public enum SDKLogger {
    private static let state = SDKLoggerState()

    /// Record a structured diagnostic event. Only this API writes to
    /// `swift/run.log`; the legacy free-form methods below remain console-only
    /// until their call sites have been explicitly reviewed for privacy.
    public static func event(
        _ event: String,
        category: SDKLogCategory,
        severity: SDKLogSeverity = .info,
        fields: [String: SDKLogValue] = [:],
        error: Error? = nil,
        redacting sensitiveValues: [String] = []
    ) {
        let line = SDKLogFormatter.format(
            event: event,
            category: category,
            severity: severity,
            fields: fields,
            error: error,
            redacting: sensitiveValues
        )

        state.destination(for: severity)?.write(line)

        let shouldMirrorToConsole: Bool
        switch severity {
        case .debug:
            shouldMirrorToConsole = LoggingPreferences.allows(.high)
        case .info:
            shouldMirrorToConsole = LoggingPreferences.allows(.medium)
        case .warning, .error:
            shouldMirrorToConsole = true
        }
        if shouldMirrorToConsole {
            NSLog("%@", line)
            Swift.print(line)
        }
    }

    /// Synchronize the Swift file handle before diagnostics copy the session.
    public static func flush() {
        state.flush()
    }

    static func installFileSink(at sessionDirectory: URL, includeDebug: Bool) -> Bool {
        state.installSink(at: sessionDirectory, includeDebug: includeDebug)
    }

    static func updateDebugSetting(_ includeDebug: Bool) {
        state.updateDebugSetting(includeDebug)
    }

    public static func log(_ message: String, minimumLevel level: LoggingPreset = .medium) {
        guard LoggingPreferences.allows(level) else { return }
        // Mirror to NSLog (unified logging) in addition to stdout so
        // `xcrun simctl spawn booted log stream` and Console.app see
        // the message even when no Xcode debugger is attached. The
        // `print` path is preserved because the dev loop still wants
        // stdout for in-Xcode use; NSLog goes to os_log.
        NSLog("%@", message)
        Swift.print(message)
    }

    public static func error(_ message: String) {
        // Route through both `NSLog` (unified log — Console.app, device
        // console, Xcode debug area without depending on stdout
        // capture) and `Swift.print` (stdout — preserves the existing
        // dev-loop behaviour where `print` output is what's visible).
        // Errors are rare; double-emit is fine and makes them harder
        // to miss when something does go wrong.
        NSLog("%@", message)
        Swift.print(message)
    }
}

// Package-internal override of `print` that honors the logging preference.
// Kept non-public so it only applies within SwiftDashSDK.
func print(_ items: Any..., separator: String = " ", terminator: String = "\n") {
    let output = items.map { String(describing: $0) }.joined(separator: separator)
    let lowercased = output.lowercased()
    let shouldAlwaysPrint = output.contains("❌") || output.contains("⚠️") || lowercased.contains("error")

    guard LoggingPreferences.shouldEmitDefaultLogs || shouldAlwaysPrint else { return }
    Swift.print(output, terminator: terminator)
}
