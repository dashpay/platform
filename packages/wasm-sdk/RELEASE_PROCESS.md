# WASM SDK Release Process

## Overview

This document outlines the release process for the `dash-wasm-sdk` package, including alpha, beta, and stable releases.

## Release Types

### Alpha Releases (0.x.x-alpha.x)
- **Purpose**: Early testing and feedback collection
- **Stability**: Limited stability, breaking changes expected
- **Frequency**: As needed for major features or testing
- **Distribution**: npm with `@alpha` tag

### Beta Releases (0.x.x-beta.x)
- **Purpose**: Feature-complete testing before stable release
- **Stability**: API stable, minor bug fixes expected
- **Frequency**: Before each stable release
- **Distribution**: npm with `@beta` tag

### Stable Releases (0.x.x)
- **Purpose**: Production-ready releases
- **Stability**: Full stability guarantees
- **Frequency**: Based on feature completeness and testing
- **Distribution**: npm with `@latest` tag

## Automated Release Process

### GitHub Actions Workflow

Releases are automated via `.github/workflows/publish-wasm-sdk-alpha.yml`:

1. **Manual Trigger**: Use workflow_dispatch for on-demand releases
2. **Tag-based**: Automatically triggered by `wasm-sdk-v*` tags
3. **Quality Gates**: 
   - Build validation
   - Security audit
   - Package integrity checks
   - Installation testing

### Release Steps

1. **Pre-Release Validation**
   ```bash
   cd packages/wasm-sdk
   ./build-optimized.sh  # Build optimized package
   ./validate-build.sh   # Validate build integrity
   ```

2. **Version Management**
   ```bash
   # Alpha release
   npm version prerelease --preid=alpha
   
   # Beta release  
   npm version prerelease --preid=beta
   
   # Stable release
   npm version minor  # or major/patch
   ```

3. **Automated Publishing**
   - GitHub Actions handles npm publish
   - Creates GitHub release with changelog
   - Updates package distribution tags

## Manual Release Process (Fallback)

### Pre-requisites
- npm publish access to `@dashevo` scope
- GitHub release permissions
- Access to package build environment

### Steps

1. **Build Package**
   ```bash
   cd packages/wasm-sdk
   ./build-optimized.sh
   ```

2. **Validate Package**
   ```bash
   cd pkg/
   npm pack
   npm audit
   
   # Test installation
   mkdir -p /tmp/test-install
   cd /tmp/test-install
   npm init -y
   npm install ../dashevo-dash-wasm-sdk-*.tgz
   ```

3. **Publish to NPM**
   ```bash
   cd pkg/
   
   # Alpha release
   npm publish --tag alpha
   
   # Beta release
   npm publish --tag beta
   
   # Stable release
   npm publish
   ```

4. **Create GitHub Release**
   ```bash
   git tag wasm-sdk-v0.1.0-alpha.1
   git push origin wasm-sdk-v0.1.0-alpha.1
   
   # Or use GitHub CLI
   gh release create wasm-sdk-v0.1.0-alpha.1 \
     --title "WASM SDK v0.1.0-alpha.1" \
     --notes-file RELEASE_NOTES.md \
     --prerelease
   ```

## Quality Gates

### Enhanced Build Validation
- ✅ Rust compilation succeeds with unified build system
- ✅ WASM optimization completes (dash_wasm_sdk.js generation)
- ✅ Services directory integration (6 service classes copied)
- ✅ Package.json generation with service files included
- ✅ JavaScript wrapper integration successful
- ✅ TypeScript definitions generated and integrated
- ✅ Package size within limits (~14MB, optimized from 28MB)
- ✅ Bundle validation and structure verification

### Security Validation  
- ✅ `npm audit` passes with no critical issues
- ✅ `cargo audit` passes for Rust dependencies
- ✅ No known security vulnerabilities

### Enhanced Functionality Validation
- ✅ Package imports correctly in Node.js and browsers with clean syntax
- ✅ Service architecture functions correctly (6 service classes)
- ✅ Modern JavaScript wrapper API works seamlessly
- ✅ Resource management and cleanup operates automatically  
- ✅ Configuration-driven initialization succeeds
- ✅ WASM module loading via correct import path (dash_wasm_sdk.js)
- ✅ TypeScript definitions are comprehensive and accurate
- ✅ Error handling with structured error types functions correctly

### Integration Validation
- ✅ Can connect to testnet
- ✅ Can create SDK instances
- ✅ Basic identity operations work
- ✅ No regression in existing functionality

## Release Channels

### NPM Distribution Tags
- `latest`: Stable releases (default npm install)
- `beta`: Beta testing releases (`npm install dash-wasm-sdk@beta`)
- `alpha`: Alpha testing releases (`npm install dash-wasm-sdk@alpha`)

### CDN Distribution
Packages are automatically available via:
- unpkg: `https://unpkg.com/dash-wasm-sdk@alpha/`
- jsdelivr: `https://cdn.jsdelivr.net/npm/dash-wasm-sdk@alpha/`

## Rollback Procedures

### NPM Package Rollback

#### For Alpha/Beta Releases
```bash
# Remove problematic version
npm unpublish dash-wasm-sdk@0.1.0-alpha.1

# Update tag to previous version
npm dist-tag add dash-wasm-sdk@0.1.0-alpha.0 alpha
```

#### For Stable Releases
```bash
# Deprecate problematic version
npm deprecate dash-wasm-sdk@0.1.0 "Critical bug, use 0.0.9 instead"

# Update latest tag
npm dist-tag add dash-wasm-sdk@0.0.9 latest
```

### GitHub Release Rollback
```bash
# Delete release and tag
gh release delete wasm-sdk-v0.1.0-alpha.1
git tag -d wasm-sdk-v0.1.0-alpha.1
git push origin :refs/tags/wasm-sdk-v0.1.0-alpha.1
```

### Communication Template
```markdown
## URGENT: dash-wasm-sdk Rollback Notice

**Version Affected**: 0.1.0-alpha.1
**Issue**: [Brief description of critical issue]
**Action Taken**: Package rolled back to 0.1.0-alpha.0

**Immediate Action Required**:
- If you installed 0.1.0-alpha.1, please downgrade:
  `npm install dash-wasm-sdk@0.1.0-alpha.0`

**Issue Tracking**: https://github.com/dashpay/platform/issues/XXX

We apologize for the inconvenience and are working on a fix.
```

## Monitoring and Support

### Post-Release Monitoring
- Monitor GitHub issues for bug reports
- Track npm download statistics
- Monitor CDN usage and error rates
- Watch for security vulnerability reports

### Support Channels
- **GitHub Issues**: Primary support and bug reports
- **GitHub Discussions**: Community questions and feedback
- **Discord**: Real-time community support (if applicable)
- **Documentation**: Keep examples and guides up to date

### Success Metrics
- Package download count
- Issue report frequency and resolution time
- Community engagement and feedback
- Performance benchmarks maintenance