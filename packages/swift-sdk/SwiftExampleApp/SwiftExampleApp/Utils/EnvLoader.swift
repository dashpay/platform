import Foundation

/// Environment variable loader for test configuration
struct EnvLoader {
    private static var envVars: [String: String] = [:]
    
    /// Load environment variables from .env file
    static func loadEnvFile() {
        // Try multiple locations for .env file
        let possiblePaths = [
            // From PROJECT_DIR env var
            ProcessInfo.processInfo.environment["PROJECT_DIR"].map { "\($0)/.env" },
            // From current directory
            "\(FileManager.default.currentDirectoryPath)/.env",
            // From bundle resource (for tests)
            Bundle.main.path(forResource: ".env", ofType: nil),
            // Hardcoded path for SwiftExampleApp (fallback for tests)
            "/Users/quantum/src/platform-ios/packages/swift-sdk/SwiftExampleApp/.env"
        ].compactMap { $0 }
        
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