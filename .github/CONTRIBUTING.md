# Contributing to @dashevo/dash-wasm-sdk

We welcome contributions to the Dash WASM SDK! This guide will help you get started with contributing code, documentation, and community support.

## 🚀 Getting Started

### **Prerequisites**
- Node.js 18+ and npm/yarn
- Rust 1.70+ and wasm-pack  
- Git and GitHub account
- Familiarity with WebAssembly and TypeScript

### **Development Setup**
```bash
# Clone the repository
git clone https://github.com/dashpay/platform.git
cd platform/packages/wasm-sdk

# Install dependencies
npm install

# Build the project
npm run build

# Run tests
npm test
```

### **Project Structure**
```
packages/wasm-sdk/
├── src/                    # Rust source code
├── pkg/                    # Generated WASM bindings
├── tests/                  # Test files
├── examples/              # Usage examples
├── docs/                  # Documentation
└── scripts/               # Build and utility scripts
```

## 📝 Contribution Types

### **1. Code Contributions**

#### **Bug Fixes**
- Find issues labeled `good first issue` or `bug`
- Create a reproduction case
- Write tests to verify the fix
- Ensure all existing tests pass

#### **New Features**  
- Discuss the feature in GitHub Issues first
- Follow the API design patterns
- Add comprehensive tests
- Update documentation and examples

#### **Performance Improvements**
- Benchmark current performance
- Implement optimization
- Measure improvement with tests
- Document performance characteristics

### **2. Documentation Contributions**

#### **API Documentation**
- Improve function/method descriptions
- Add code examples
- Clarify parameter descriptions
- Fix typos and formatting

#### **Guides and Tutorials**
- Create getting started guides
- Write integration examples
- Develop troubleshooting content
- Produce best practices documentation

### **3. Community Contributions**

#### **Issue Triage**
- Reproduce reported bugs
- Ask for missing information
- Tag issues appropriately
- Close resolved/invalid issues

#### **Support and Mentoring**
- Answer questions in discussions
- Help new contributors
- Review pull requests
- Share knowledge and experience

## 🔧 Development Workflow

### **1. Fork and Clone**
```bash
# Fork the repository on GitHub
# Clone your fork locally
git clone https://github.com/YOUR_USERNAME/platform.git
cd platform/packages/wasm-sdk

# Add upstream remote
git remote add upstream https://github.com/dashpay/platform.git
```

### **2. Create Feature Branch**
```bash
# Update your fork
git fetch upstream
git checkout main
git merge upstream/main

# Create feature branch
git checkout -b feature/your-feature-name
```

### **3. Make Changes**
```bash
# Make your changes
# Add tests for new functionality
# Update documentation

# Build and test
npm run build
npm test

# Lint code
npm run lint
```

### **4. Commit Changes**
```bash
# Stage changes
git add .

# Commit with descriptive message
git commit -m "feat: add new identity verification method

- Add verifyIdentity() function to WasmSdk
- Include comprehensive input validation  
- Add unit tests for success and error cases
- Update documentation with usage examples"
```

### **5. Submit Pull Request**
```bash
# Push to your fork
git push origin feature/your-feature-name

# Create pull request on GitHub
# Fill out the PR template completely
# Request review from maintainers
```

## 📊 Code Quality Standards

### **Rust Code Guidelines**

#### **Style and Formatting**
- Use `cargo fmt` for consistent formatting
- Follow Rust naming conventions
- Add comprehensive documentation comments
- Use `cargo clippy` to catch common issues

#### **Error Handling**
```rust
// Prefer Result types for fallible operations
pub fn create_identity(&self, params: IdentityParams) -> Result<Identity, SdkError> {
    // Validate input
    params.validate()?;
    
    // Perform operation with proper error handling
    let identity = self.platform.create_identity(params)
        .map_err(SdkError::Platform)?;
    
    Ok(identity)
}
```

#### **Testing Standards**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_identity_success() {
        // Arrange: Set up test data
        let sdk = WasmSdk::new(test_config());
        let params = valid_identity_params();
        
        // Act: Perform the operation
        let result = sdk.create_identity(params);
        
        // Assert: Verify expected behavior
        assert!(result.is_ok());
        let identity = result.unwrap();
        assert_eq!(identity.get_type(), IdentityType::User);
    }
}
```

### **JavaScript/TypeScript Guidelines**

#### **Type Definitions**
```typescript
// Provide comprehensive TypeScript definitions
export interface IdentityParams {
  readonly publicKey: Uint8Array;
  readonly credits: number;
  readonly metadata?: Record<string, unknown>;
}

export interface Identity {
  getId(): string;
  getPublicKey(): Uint8Array;
  getBalance(): number;
}
```

#### **Documentation Standards**
```typescript
/**
 * Creates a new identity on the Dash Platform
 * 
 * @param params - Identity creation parameters
 * @param params.publicKey - The public key for the identity
 * @param params.credits - Initial credit balance
 * @param params.metadata - Optional identity metadata
 * @returns Promise resolving to the created identity
 * 
 * @example
 * ```typescript
 * const identity = await sdk.createIdentity({
 *   publicKey: publicKeyBytes,
 *   credits: 1000,
 *   metadata: { name: "Alice" }
 * });
 * ```
 */
async createIdentity(params: IdentityParams): Promise<Identity>
```

## 🧪 Testing Guidelines

### **Test Coverage Expectations**
- All public functions must have tests
- Edge cases and error conditions covered
- Integration tests for complex workflows
- Performance tests for critical operations

### **Test Organization**
```rust
// Unit tests - test individual functions
#[cfg(test)]
mod unit_tests { ... }

// Integration tests - test feature workflows
#[cfg(test)]
mod integration_tests { ... }

// Performance tests - benchmark critical paths
#[cfg(test)]
mod performance_tests { ... }
```

### **Test Data and Mocking**
- Use realistic test data
- Mock external dependencies
- Avoid network calls in unit tests
- Clean up resources after tests

## 📋 Pull Request Guidelines

### **PR Title and Description**
- Use conventional commit format
- Describe what the PR accomplishes
- Link related issues
- Explain any breaking changes

### **PR Checklist**
- [ ] Tests pass locally
- [ ] New functionality has tests
- [ ] Documentation updated
- [ ] No breaking changes (or justified)
- [ ] Performance impact considered
- [ ] Security implications reviewed

### **Review Process**
1. **Automated Checks** - CI/CD pipeline runs
2. **Code Review** - Maintainer reviews code
3. **Testing** - Additional testing if needed  
4. **Approval** - Maintainer approves changes
5. **Merge** - PR merged to main branch

## 🏷️ Issue and PR Labels

### **Type Labels**
- `bug` - Something isn't working
- `enhancement` - New feature or improvement
- `documentation` - Documentation improvements
- `performance` - Performance-related changes

### **Priority Labels**
- `priority: critical` - Needs immediate attention
- `priority: high` - Important, but not urgent
- `priority: medium` - Standard priority
- `priority: low` - Nice to have

### **Status Labels**
- `status: needs-info` - More information required
- `status: in-progress` - Being worked on
- `status: ready-for-review` - Ready for maintainer review

## 🎯 Recognition and Rewards

### **Contributor Recognition**
- Contributors listed in release notes
- Special recognition for significant contributions
- Community showcase for innovative uses
- Mentorship opportunities for experienced contributors

### **Contribution Metrics**
- Track contributions across all areas
- Recognize consistent community participation
- Highlight helpful community support
- Feature outstanding examples and tutorials

---

## 📞 Getting Help

Need help with your contribution?
- **Questions**: GitHub Discussions
- **Technical Issues**: Create an issue
- **Process Questions**: Contact maintainers
- **Community Chat**: Join Discord

*Thank you for contributing to the Dash Platform ecosystem!*