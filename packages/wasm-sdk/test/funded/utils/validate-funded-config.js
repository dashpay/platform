#!/usr/bin/env node

/**
 * Validate Funded Test Configuration
 * Checks all required environment variables and safety settings
 */

require('dotenv').config();

console.log('🔍 Validating Funded Test Configuration');
console.log('======================================');

let passed = 0;
let failed = 0;
let warnings = 0;

function check(name, condition, errorMessage = null) {
    if (condition) {
        console.log(`✅ ${name}`);
        passed++;
    } else {
        console.log(`❌ ${name}: ${errorMessage || 'Check failed'}`);
        failed++;
    }
}

function warn(name, condition, warningMessage) {
    if (condition) {
        console.log(`⚠️ ${name}: ${warningMessage}`);
        warnings++;
    }
}

// Critical Configuration Checks
console.log('\n🔧 Critical Configuration');
console.log('-'.repeat(30));

check('ENABLE_FUNDED_TESTS set', 
    process.env.ENABLE_FUNDED_TESTS === 'true',
    'Set ENABLE_FUNDED_TESTS=true to enable funded tests');

check('Network is testnet/devnet',
    ['testnet', 'devnet', 'regtest'].includes(process.env.NETWORK),
    `Network ${process.env.NETWORK} not allowed for funded tests`);

check('Primary faucet address configured',
    process.env.FAUCET_1_ADDRESS && process.env.FAUCET_1_ADDRESS.length > 30,
    'FAUCET_1_ADDRESS missing or too short');

check('Primary faucet private key configured',
    process.env.FAUCET_1_PRIVATE_KEY && process.env.FAUCET_1_PRIVATE_KEY.length > 40,
    'FAUCET_1_PRIVATE_KEY missing or too short');

// Safety Limits
console.log('\n🛡️ Safety Limits');
console.log('-'.repeat(20));

const maxPerTest = parseInt(process.env.MAX_CREDITS_PER_TEST) || 0;
const maxPerSuite = parseInt(process.env.MAX_CREDITS_PER_SUITE) || 0;
const maxDaily = parseInt(process.env.MAX_DAILY_USAGE) || 0;

check('Per-test limit configured',
    maxPerTest > 0 && maxPerTest <= 500000000,
    `Invalid MAX_CREDITS_PER_TEST: ${maxPerTest} (should be 1-500M)`);

check('Per-suite limit configured',
    maxPerSuite > 0 && maxPerSuite <= 2000000000,
    `Invalid MAX_CREDITS_PER_SUITE: ${maxPerSuite} (should be 1-2000M)`);

check('Daily limit configured',
    maxDaily > 0 && maxDaily <= 10000000000,
    `Invalid MAX_DAILY_USAGE: ${maxDaily} (should be 1-10000M)`);

// Pool Configuration
console.log('\n🏊 Identity Pool Configuration');
console.log('-'.repeat(35));

const poolSize = parseInt(process.env.IDENTITY_POOL_SIZE) || 0;
const minBalance = parseInt(process.env.MIN_IDENTITY_BALANCE) || 0;
const initialCredits = parseInt(process.env.INITIAL_IDENTITY_CREDITS) || 0;

check('Pool size reasonable',
    poolSize > 0 && poolSize <= 50,
    `Invalid IDENTITY_POOL_SIZE: ${poolSize} (should be 1-50)`);

check('Minimum balance set',
    minBalance > 0 && minBalance <= 100000000,
    `Invalid MIN_IDENTITY_BALANCE: ${minBalance} (should be 1-100M)`);

check('Initial credits reasonable',
    initialCredits >= minBalance && initialCredits <= 200000000,
    `Invalid INITIAL_IDENTITY_CREDITS: ${initialCredits} (should be ${minBalance}-200M)`);

// Optional but Recommended
console.log('\n💡 Optional Configuration');
console.log('-'.repeat(30));

warn('Backup faucet not configured',
    !process.env.FAUCET_2_ADDRESS,
    'Consider configuring FAUCET_2_ADDRESS for redundancy');

warn('Storage not enabled',
    process.env.FAUCET_WALLET_USE_STORAGE !== 'true',
    'Consider enabling FAUCET_WALLET_USE_STORAGE for faster sync');

warn('Sync optimization not set',
    !process.env.SKIP_SYNC_BEFORE_HEIGHT,
    'Consider setting SKIP_SYNC_BEFORE_HEIGHT for faster wallet sync');

// Security Warnings
console.log('\n🚨 Security Warnings');
console.log('-'.repeat(25));

warn('Mainnet protection',
    process.env.NETWORK === 'mainnet',
    'CRITICAL: Mainnet detected! Funded tests should never run on mainnet');

warn('Production environment',
    process.env.NODE_ENV === 'production' && !process.env.ALLOW_FUNDED_PRODUCTION,
    'Running funded tests in production environment');

warn('Debug mode enabled',
    process.env.DEBUG_FUNDED_TESTS === 'true',
    'Debug mode will log detailed transaction information');

// Estimated Costs
console.log('\n💰 Estimated Costs');
console.log('-'.repeat(20));

const tierCosts = {
    low: { perTest: 50000000, perSuite: 200000000, daily: 1000000000 },
    medium: { perTest: 200000000, perSuite: 1000000000, daily: 5000000000 },
    high: { perTest: 500000000, perSuite: 2000000000, daily: 10000000000 }
};

const currentTier = process.env.FUNDING_TIER || 'low';
const costs = tierCosts[currentTier] || tierCosts.low;

console.log(`Current Tier: ${currentTier}`);
console.log(`Estimated per test: ${costs.perTest} credits (~${(costs.perTest / 100000000).toFixed(1)} DASH)`);
console.log(`Estimated per suite: ${costs.perSuite} credits (~${(costs.perSuite / 100000000).toFixed(1)} DASH)`);
console.log(`Daily budget: ${costs.daily} credits (~${(costs.daily / 100000000).toFixed(1)} DASH)`);

// Final Summary
console.log('\n📊 Configuration Summary');
console.log('========================');
console.log(`✅ Passed: ${passed}`);
console.log(`❌ Failed: ${failed}`);
console.log(`⚠️ Warnings: ${warnings}`);

if (failed === 0) {
    console.log('\n🎉 Configuration validation passed!');
    console.log('');
    console.log('Ready to run funded tests:');
    console.log('  npm run test:dry-run     # Validate without funding');
    console.log('  npm run test:low         # Run low-tier funded tests');
    console.log('  npm run check-faucet     # Check faucet balance');
    console.log('  npm run pool-status      # Check identity pool');
    process.exit(0);
} else {
    console.log('\n❌ Configuration validation failed!');
    console.log('Please fix the errors above before running funded tests.');
    process.exit(1);
}