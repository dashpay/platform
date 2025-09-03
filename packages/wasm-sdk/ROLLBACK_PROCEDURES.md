# Rollback Procedures for Dash WASM SDK

This document outlines procedures for rolling back problematic releases of the Dash WASM SDK.

## 🚨 When to Rollback

Initiate a rollback when:

- **Critical Security Vulnerability**: Newly discovered security issues in the released version
- **Breaking Changes**: Unintended breaking changes causing widespread application failures
- **Performance Regression**: Significant performance degradation affecting user experience
- **WASM Loading Issues**: WebAssembly initialization or loading failures in browsers
- **Data Corruption**: Issues that could lead to data loss or corruption
- **CDN Distribution Problems**: Package not accessible via major CDNs

## ⚡ Emergency Rollback (< 2 hours)

### 1. Immediate Response

```bash
# Clone the repository
git clone https://github.com/dashpay/platform.git
cd platform

# Switch to the last known good version
git checkout wasm-sdk-v<LAST_GOOD_VERSION>

# Navigate to WASM SDK
cd packages/wasm-sdk
```

### 2. Emergency Republish

```bash
# Build the last good version
./build-optimized.sh

# Emergency publish (requires npm token)
cd pkg
npm version <ROLLBACK_VERSION> --no-git-tag-version
npm publish --access public --tag latest

# Example:
# npm version 0.0.9-rollback.1 --no-git-tag-version
# npm publish --access public --tag latest
```

### 3. Notify Stakeholders

**GitHub Issue Template:**
```markdown
## 🚨 EMERGENCY ROLLBACK - WASM SDK v<PROBLEMATIC_VERSION>

**Status**: ROLLED BACK to v<ROLLBACK_VERSION>

**Issue**: [Brief description of the problem]

**Impact**: [Description of user impact]

**Actions Taken**:
- [ ] Rolled back npm package to v<ROLLBACK_VERSION>
- [ ] Updated CDN references
- [ ] Created emergency patch
- [ ] Notified community

**Next Steps**:
- [ ] Investigate root cause
- [ ] Prepare patch release
- [ ] Update documentation
```

**Community Notification:**
- Post to GitHub Discussions
- Update README with rollback notice
- Send notification to developer channels

## 🔄 Planned Rollback (4-8 hours)

### 1. Prepare Rollback Environment

```bash
# Create rollback branch
git checkout -b rollback/wasm-sdk-v<PROBLEMATIC_VERSION>

# Revert to last good commit
git revert --no-commit <PROBLEMATIC_COMMIT_HASH>
git commit -m "Rollback: Revert problematic changes in v<PROBLEMATIC_VERSION>"
```

### 2. Update Version Strategy

Choose appropriate versioning:
- **Patch rollback**: `0.1.1 → 0.1.0-rollback.1` (recommended)
- **Minor rollback**: `0.2.0 → 0.1.9-rollback.1`
- **Republish previous**: Republish exact previous version with `--force`

### 3. Comprehensive Testing

```bash
# Run full test suite
npm test

# Test CDN functionality
curl -f "https://unpkg.com/@dashevo/dash-wasm-sdk@latest/package.json"

# Test browser compatibility
# (Manual testing with test HTML file)
```

### 4. Execute Rollback

```bash
# Build rollback version
./build-optimized.sh

# Update package version
cd pkg
npm version <ROLLBACK_VERSION>

# Publish rollback
npm publish --access public

# Tag the rollback
git tag -a "wasm-sdk-v<ROLLBACK_VERSION>" -m "Rollback from v<PROBLEMATIC_VERSION>"
git push origin --tags
```

## 📋 Rollback Checklist

### Pre-Rollback Assessment

- [ ] **Severity Assessment**: Determine if issue warrants rollback
- [ ] **Impact Analysis**: Document affected users and applications
- [ ] **Alternative Solutions**: Consider hotfix vs full rollback
- [ ] **Stakeholder Approval**: Get approval from project maintainers

### Rollback Execution

- [ ] **Backup Current State**: Create branch with current problematic version
- [ ] **Identify Last Good Version**: Verify which version to rollback to
- [ ] **Test Rollback Version**: Ensure rollback version builds and functions
- [ ] **Update Documentation**: Prepare rollback announcements

### Post-Rollback Tasks

- [ ] **Verify npm Package**: Confirm rollback version is live
- [ ] **Test CDN Availability**: Verify CDN propagation (unpkg, jsDelivr)  
- [ ] **Update Documentation**: Add rollback notice to README
- [ ] **Community Notification**: Inform users via GitHub, discussions
- [ ] **Monitor Feedback**: Watch for issues with rollback version

## 🛠 Rollback Scripts

### Automated Rollback Script

```bash
#!/bin/bash
# rollback.sh - Automated rollback script

set -e

PROBLEMATIC_VERSION=$1
ROLLBACK_VERSION=$2
LAST_GOOD_COMMIT=$3

if [ -z "$PROBLEMATIC_VERSION" ] || [ -z "$ROLLBACK_VERSION" ] || [ -z "$LAST_GOOD_COMMIT" ]; then
    echo "Usage: ./rollback.sh <problematic_version> <rollback_version> <last_good_commit>"
    exit 1
fi

echo "🔄 Starting rollback from v$PROBLEMATIC_VERSION to v$ROLLBACK_VERSION"

# Create rollback branch
git checkout -b "rollback/v$PROBLEMATIC_VERSION"

# Revert to last good commit
git reset --hard "$LAST_GOOD_COMMIT"

# Build rollback version
echo "📦 Building rollback version..."
./build-optimized.sh

# Update package version
cd pkg
npm version "$ROLLBACK_VERSION" --no-git-tag-version

echo "✅ Rollback prepared. Review and publish manually with:"
echo "   cd pkg && npm publish --access public"
echo "   git tag -a 'wasm-sdk-v$ROLLBACK_VERSION' -m 'Rollback from v$PROBLEMATIC_VERSION'"
```

### CDN Verification Script

```bash
#!/bin/bash
# verify-cdn.sh - Verify CDN rollback propagation

VERSION=$1
if [ -z "$VERSION" ]; then
    echo "Usage: ./verify-cdn.sh <version>"
    exit 1
fi

echo "🌐 Verifying CDN availability for v$VERSION"

# Test unpkg
echo "Testing unpkg..."
if curl -f "https://unpkg.com/@dashevo/dash-wasm-sdk@$VERSION/package.json" > /dev/null 2>&1; then
    echo "✅ unpkg: Available"
else
    echo "❌ unpkg: Not available"
fi

# Test jsDelivr
echo "Testing jsDelivr..."
if curl -f "https://cdn.jsdelivr.net/npm/@dashevo/dash-wasm-sdk@$VERSION/package.json" > /dev/null 2>&1; then
    echo "✅ jsDelivr: Available"
else
    echo "❌ jsDelivr: Not available"
fi
```

## 🔍 Investigation Procedures

### Post-Rollback Investigation

1. **Preserve Evidence**
   ```bash
   # Create forensic branch
   git checkout -b forensic/v<PROBLEMATIC_VERSION>-analysis
   
   # Document the issue
   echo "Issue analysis for v<PROBLEMATIC_VERSION>" > INCIDENT_REPORT.md
   ```

2. **Analyze Root Cause**
   - Review commit changes
   - Analyze test failures
   - Check build differences
   - Review dependency changes

3. **Prevention Measures**
   - Update tests to catch the issue
   - Improve CI/CD validation
   - Add regression tests
   - Update review procedures

## 🚀 Recovery Planning

### Hotfix Strategy

```bash
# Create hotfix branch
git checkout -b hotfix/v<FIXED_VERSION>

# Apply minimal fix
# ... make changes ...

# Test thoroughly
npm test
./build-optimized.sh
cd pkg && npm pack && cd ..

# Release hotfix
cd pkg
npm version <FIXED_VERSION>
npm publish --access public
```

### Communication Templates

**Rollback Announcement:**
```markdown
## ⚠️ WASM SDK v<VERSION> Rollback Notice

We've identified an issue with WASM SDK v<PROBLEMATIC_VERSION> and have rolled back to v<ROLLBACK_VERSION>.

**What happened**: [Brief description]

**Action required**: 
- CDN users: No action needed (automatically updated)
- npm users: `npm install @dashevo/dash-wasm-sdk@latest`

**Timeline**: Hotfix expected within [timeframe]

We apologize for any inconvenience.
```

## 📞 Emergency Contacts

When executing rollbacks, notify:

1. **Project Maintainers**: [Contact information]
2. **DevOps Team**: [Contact information]
3. **Community Managers**: [Contact information]
4. **Security Team**: [Contact information] (for security rollbacks)

## 🔒 Security Rollbacks

For security-related rollbacks:

1. **Do not disclose details** until fix is available
2. **Coordinate with security team** before any action
3. **Consider coordinated disclosure** timeline
4. **Prepare security advisory** for post-fix communication

## 📖 Additional Resources

- [npm unpublish policy](https://www.npmjs.com/policies/unpublish)
- [Semantic Versioning](https://semver.org/)
- [GitHub Release Management](https://docs.github.com/en/repositories/releasing-projects-on-github)
- [CDN Cache Invalidation](https://developers.cloudflare.com/cache/how-to/purge-cache/)

---

**Note**: Always test rollback procedures in a staging environment when possible. Keep this document updated as procedures evolve.