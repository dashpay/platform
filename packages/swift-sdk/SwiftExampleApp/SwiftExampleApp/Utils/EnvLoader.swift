import Foundation

/// Environment variable loader for test configuration
struct EnvLoader {
    private static var envVars: [String: String] = [:]
    
    /// Load environment variables from .env file
    static func loadEnvFile() {
        // Try common project locations for .env file
        let possiblePaths = findCommonEnvPaths()
        
        var envPath: String?
        for path in possiblePaths {
            if FileManager.default.fileExists(atPath: path) {
                envPath = path
                break
            }
        }
        
        guard let finalPath = envPath else {
            print("Warning: .env file not found in any of the following locations:")
            possiblePaths.forEach { print("  - \($0)") }
            return
        }
        
        guard let envContent = try? String(contentsOfFile: finalPath, encoding: .utf8) else {
            print("Warning: Could not read .env file at \(finalPath)")
            return
        }
        
        print("✅ Loading .env file from: \(finalPath)")
        
        // Parse .env file
        let lines = envContent.components(separatedBy: .newlines)
        for line in lines {
            let trimmed = line.trimmingCharacters(in: .whitespaces)
            
            // Skip empty lines and comments
            if trimmed.isEmpty || trimmed.hasPrefix("#") {
                continue
            }
            
            // Parse KEY=VALUE
            let parts = trimmed.split(separator: "=", maxSplits: 1)
            if parts.count == 2 {
                let key = String(parts[0]).trimmingCharacters(in: .whitespaces)
                let value = String(parts[1]).trimmingCharacters(in: .whitespaces)
                envVars[key] = value
            }
        }
    }
    
    /// Get environment variable value
    static func get(_ key: String) -> String? {
        // Check process environment first
        if let value = ProcessInfo.processInfo.environment[key] {
            return value
        }
        
        // Check loaded .env file
        return envVars[key]
    }
    
    /// Get required environment variable or throw error
    static func getRequired(_ key: String) throws -> String {
        guard let value = get(key) else {
            throw EnvError.missingRequired(key)
        }
        return value
    }
    
    /// Find common .env file locations
    private static func findCommonEnvPaths() -> [String] {
        var paths: [String] = []
        
        // First try bundle resource (if .env was copied to bundle)
        if let bundlePath = Bundle.main.path(forResource: ".env", ofType: nil) {
            paths.append(bundlePath)
        }
        
        // Try actual file system paths (these work when running from Xcode)
        // Note: homeDirectoryForCurrentUser is not available on iOS, 
        // so we construct the home path using NSHomeDirectory or use fallbacks
        
        #if os(iOS)
        // On iOS simulator, NSHomeDirectory returns the app's sandbox, not the user's home
        // We need to use hardcoded paths for common usernames
        let username = NSUserName()
        let possibleHomeDirs = [
            "/Users/\(username)",
            "/Users/quantum",
            "/Users/samuelw"
        ]
        
        for homeDir in possibleHomeDirs {
            paths.append(contentsOf: [
                "\(homeDir)/src/platform-ios/packages/swift-sdk/SwiftExampleApp/.env",
                "\(homeDir)/src/platform/packages/swift-sdk/SwiftExampleApp/.env",
                "\(homeDir)/Documents/src/platform/packages/swift-sdk/SwiftExampleApp/.env",
            ])
        }
        #else
        // On macOS, we can use homeDirectoryForCurrentUser
        let homeDir = FileManager.default.homeDirectoryForCurrentUser.path
        paths.append(contentsOf: [
            "\(homeDir)/src/platform-ios/packages/swift-sdk/SwiftExampleApp/.env",
            "\(homeDir)/src/platform/packages/swift-sdk/SwiftExampleApp/.env",
            "\(homeDir)/Documents/src/platform/packages/swift-sdk/SwiftExampleApp/.env",
        ])
        #endif
        
        // Add current directory relative paths
        paths.append(contentsOf: [
            FileManager.default.currentDirectoryPath + "/.env",
            FileManager.default.currentDirectoryPath + "/packages/swift-sdk/SwiftExampleApp/.env",
        ])
        
        return paths
    }
}

enum EnvError: LocalizedError {
    case missingRequired(String)
    
    var errorDescription: String? {
        switch self {
        case .missingRequired(let key):
            return "Missing required environment variable: \(key)"
        }
    }
}