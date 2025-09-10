# 💰 WASM SDK Funded Testing Infrastructure

## ⚠️ CRITICAL WARNING: REAL FUND USAGE

**This testing infrastructure uses ACTUAL TESTNET FUNDS. Each test operation consumes real testnet credits and DASH. Use with extreme caution and proper configuration.**

---

## 🎯 Overview

The funded testing infrastructure enables comprehensive integration testing of WASM SDK state transitions and operations using real testnet funding. It provides:

- **Real Identity Creation**: Actual blockchain asset locks and identity registration
- **Live Document Operations**: Document creation, updates, and deletion with real credit consumption
- **Authentic State Transitions**: Full end-to-end testing with network submission
- **Credit Usage Monitoring**: Comprehensive tracking and reporting of fund consumption
- **Safety Mechanisms**: Multiple layers of protection against fund misuse

## 📁 Architecture

```
funded/
├── 📄 README.md                    # This documentation
├── 🔧 .env.example                 # Environment configuration template
├── 📦 package.json                 # Dependencies and scripts
├── 🚀 .github/workflows/           # CI/CD for funded tests
│
├── 🛠️ utils/                       # Core Infrastructure
│   ├── wasm-faucet-client.js       # Faucet integration and funding logic
│   ├── identity-pool.js            # Pre-funded identity pool management
│   ├── credit-tracker.js           # Usage monitoring and reporting
│   ├── validate-funded-config.js   # Configuration validation
│   └── check-faucet-balance.js     # Faucet health monitoring
│
├── 🧪 integration/                 # Integration Tests
│   ├── identity-operations.test.js # Real identity creation and funding
│   └── document-operations.test.js # Real document CRUD operations
│
├── 🎭 e2e/                         # End-to-End Tests
│   └── complete-workflows.test.js  # Full user journeys
│
└── 📊 logs/                        # Usage Tracking
    ├── credit-usage.log            # Detailed operation logs
    ├── daily-usage.json            # Historical usage data
    └── usage-reports/              # Generated reports
```

## 🚀 Quick Start

### 1. Environment Setup

```bash
# Copy configuration template
cp funded/.env.example funded/.env

# Configure your testnet faucet (REQUIRED)
# Edit funded/.env with your faucet details:
FAUCET_1_ADDRESS=your-testnet-faucet-address
FAUCET_1_PRIVATE_KEY=your-testnet-faucet-private-key
```

### 2. Validation

```bash
# Validate configuration
npm run validate-config

# Check faucet balance
npm run check-faucet
```

### 3. Running Funded Tests

```bash
# Dry run (validate without funding)
npm run test:dry-run

# Run low-tier tests (<0.5 DASH each)
npm run test:low

# Run medium-tier tests (<2 DASH each) 
npm run test:medium

# Run high-tier tests (<5 DASH each)
npm run test:high
```

## 💡 Funding Tiers

### 🟢 Low Tier (`--tier low`)
- **Per Operation**: Up to 50M credits (~0.5 DASH)
- **Per Suite**: Up to 200M credits (~2 DASH)
- **Daily Budget**: Up to 1B credits (~10 DASH)
- **Use Case**: Basic testing, CI/CD, development

### 🟡 Medium Tier (`--tier medium`)
- **Per Operation**: Up to 200M credits (~2 DASH)  
- **Per Suite**: Up to 1B credits (~10 DASH)
- **Daily Budget**: Up to 5B credits (~50 DASH)
- **Use Case**: Comprehensive testing, batch operations

### 🔴 High Tier (`--tier high`)
- **Per Operation**: Up to 500M credits (~5 DASH)
- **Per Suite**: Up to 2B credits (~20 DASH) 
- **Daily Budget**: Up to 10B credits (~100 DASH)
- **Use Case**: Performance testing, load testing, stress testing

## 🛡️ Safety Mechanisms

### Multi-Layer Protection

1. **Environment Enforcement**
   - Testnet-only operation (mainnet blocked)
   - Explicit enable flag required (`ENABLE_FUNDED_TESTS=true`)
   - Network validation on every operation

2. **Usage Limits**
   - Per-operation credit limits
   - Per-test-suite budgets
   - Daily usage caps
   - Emergency stop thresholds

3. **Monitoring and Alerts**
   - Real-time usage tracking
   - Anomaly detection
   - Usage pattern analysis
   - Automatic reporting

4. **Resource Management**
   - Identity pool management
   - Automatic fund recovery
   - Resource cleanup on shutdown
   - Transaction confirmation waiting

### Emergency Controls

```bash
# Emergency stop all tests
kill -9 $(ps aux | grep 'funded-tests' | awk '{print $2}')

# Check current usage
npm run usage-report

# Validate remaining faucet balance
npm run check-faucet
```

## 🧪 Test Categories

### Real Identity Operations
- ✅ **Identity Creation**: Actual blockchain asset locks (150M-400M satoshis)
- ✅ **Identity Topup**: Additional credit allocation to existing identities
- ✅ **Balance Verification**: Real credit balance queries and validation
- ✅ **Error Testing**: Insufficient balance scenarios with real constraints

### Live Document Operations
- ✅ **Document Creation**: Real document creation consuming platform credits
- ✅ **Document Updates**: Revision-based updates with actual fee consumption
- ✅ **Batch Operations**: Multiple document operations with cost optimization
- ✅ **Permission Testing**: Real access control and identity verification

### State Transition Testing
- ✅ **Network Submission**: Actual state transition broadcasting
- ✅ **Fee Analysis**: Real network fee consumption measurement
- ✅ **Proof Generation**: Authentic cryptographic proof creation
- ✅ **Error Recovery**: Network failure handling with fund preservation

## 📊 Usage Monitoring

### Real-Time Tracking

The system tracks every credit consumed:

```javascript
// Example tracking record
{
  timestamp: 1694676234567,
  type: 'identity-creation',
  identityId: '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk',
  amount: 100000000,        // Credits consumed
  satoshis: 110000000,      // Actual satoshis spent (including fees)
  txId: 'abc123...',        // Blockchain transaction ID
  testName: 'should create identity with 100M credits',
  success: true,
  metadata: {
    duration: 2847,
    networkConfirmation: true
  }
}
```

### Reporting

```bash
# Generate usage report
npm run usage-report

# Check pool status
npm run pool-status

# View detailed logs
tail -f funded/logs/credit-usage.log
```

## 🔧 Configuration Reference

### Required Environment Variables

```bash
# Safety and Network
NETWORK=testnet                     # MUST be testnet/devnet/regtest
ENABLE_FUNDED_TESTS=true           # REQUIRED safety flag

# Primary Faucet (REQUIRED)
FAUCET_1_ADDRESS=your-address       # Testnet address with funds
FAUCET_1_PRIVATE_KEY=your-key       # Private key for faucet wallet

# Safety Limits (REQUIRED)
MAX_CREDITS_PER_TEST=50000000      # 50M credits per operation  
MAX_CREDITS_PER_SUITE=500000000    # 500M credits per test suite
MAX_DAILY_USAGE=2000000000         # 2B credits daily limit
```

### Optional Configuration

```bash
# Backup Faucet (Recommended)
FAUCET_2_ADDRESS=backup-address
FAUCET_2_PRIVATE_KEY=backup-key

# Identity Pool
IDENTITY_POOL_SIZE=25              # Pre-funded identities
MIN_IDENTITY_BALANCE=10000000      # Minimum credits per identity
INITIAL_IDENTITY_CREDITS=50000000  # Initial funding per identity

# Performance Optimization
SKIP_SYNC_BEFORE_HEIGHT=1800000    # Speed up wallet sync
FAUCET_WALLET_USE_STORAGE=true     # Cache wallet state
PARALLEL_FUNDED_WORKERS=3          # Concurrent test workers
```

## 🚨 Safety Guidelines

### Before Running Tests

1. **✅ Verify Network**: Ensure `NETWORK=testnet`
2. **✅ Check Balance**: Run `npm run check-faucet`
3. **✅ Validate Config**: Run `npm run validate-config`
4. **✅ Start Small**: Begin with `--tier low`

### During Test Execution

- **Monitor Usage**: Track credit consumption in real-time
- **Watch for Alerts**: Pay attention to warning messages
- **Emergency Stop**: Use Ctrl+C or kill process if needed
- **Check Logs**: Monitor `funded/logs/credit-usage.log`

### After Test Completion

- **Review Reports**: Check generated usage reports
- **Verify Cleanup**: Ensure resources were properly cleaned up
- **Archive Results**: Save important logs and reports
- **Update Budget**: Adjust limits based on actual usage

## 💳 Cost Analysis

### Typical Operation Costs

| Operation Type | Credits | Satoshis | DASH Equivalent |
|----------------|---------|----------|-----------------|
| Identity Creation | 100M | ~110M | ~1.1 DASH |
| Document Creation | 2-5M | ~2-5K | ~0.00002-0.00005 DASH |
| Document Update | 1-3M | ~1-3K | ~0.00001-0.00003 DASH |
| Identity Topup | 50M | ~55M | ~0.55 DASH |
| DPNS Registration | 5-10M | ~5-10K | ~0.00005-0.0001 DASH |

### Budget Planning

**Daily Development Testing**: ~10 DASH
- 5-10 identity creations
- 50-100 document operations
- Error scenario testing
- Performance validation

**Weekly Integration Testing**: ~50 DASH
- Comprehensive workflow testing
- Batch operation validation
- Error recovery testing
- Performance benchmarking

**Monthly Regression Testing**: ~200 DASH
- Full test suite execution
- Load testing and optimization
- Security validation
- Platform compatibility testing

## 🛟 Troubleshooting

### Common Issues

#### Insufficient Faucet Balance
```
Error: Insufficient faucet balance: 45000000 < 50000000 (with buffer)
```
**Solution**: Fund the faucet wallet with more testnet DASH

#### Configuration Errors
```
Error: Faucet 1 not configured. Please set FAUCET_1_ADDRESS and FAUCET_1_PRIVATE_KEY
```
**Solution**: Complete the environment configuration in `funded/.env`

#### Network Issues
```
Error: Failed to initialize faucet client: fetch failed
```
**Solution**: Check internet connectivity and testnet node availability

#### Daily Limit Exceeded
```
Error: Daily funding limit exceeded: 2100000000/2000000000 satoshis
```
**Solution**: Wait for daily reset or increase `MAX_DAILY_USAGE` limit

### Recovery Procedures

#### Emergency Fund Recovery
```bash
# Stop all tests immediately
killall node

# Check remaining balances
npm run check-faucet
npm run pool-status

# Generate emergency report
npm run usage-report
```

#### Test Environment Reset
```bash
# Clear all cached data
rm -rf funded/storage/
rm -rf funded/logs/

# Restart with clean environment
npm run validate-config
npm run test:dry-run
```

## 🔮 Future Enhancements

### Planned Features
- **Fund Recovery**: Automated credit recovery from test identities
- **Advanced Analytics**: ML-based usage pattern analysis  
- **Multi-Network**: Support for devnet and regtest environments
- **Integration**: Direct WASM SDK integration for seamless operations
- **Optimization**: Smart batching and cost-efficient operation ordering

### Integration Roadmap
1. **Phase 1**: Basic funded operations (✅ Complete)
2. **Phase 2**: WASM SDK direct integration (🔄 Next)
3. **Phase 3**: Advanced analytics and optimization
4. **Phase 4**: Automated fund management and recovery

---

## ⚡ Quick Reference

### Commands
```bash
# Configuration and validation
npm run validate-config              # Validate environment setup
npm run check-faucet                # Check faucet balance
npm run test:dry-run                # Test configuration without funding

# Running tests
npm run test:low                    # Low-cost tests
npm run test:medium                 # Medium-cost tests  
npm run test:high                   # High-cost tests

# Monitoring and reporting
npm run pool-status                 # Check identity pool status
npm run usage-report               # Generate usage report
npm run cleanup                    # Clean up test resources
```

### Environment
```bash
# Required
NETWORK=testnet
ENABLE_FUNDED_TESTS=true
FAUCET_1_ADDRESS=your-testnet-address
FAUCET_1_PRIVATE_KEY=your-private-key

# Safety limits  
MAX_CREDITS_PER_TEST=50000000
MAX_DAILY_USAGE=2000000000
```

---

**🎯 Remember: Every operation consumes real funds. Always start with dry runs and low tiers!**

*Documentation version: 1.0.0*  
*Last updated: September 2025*