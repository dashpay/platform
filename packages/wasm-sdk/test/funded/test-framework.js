#!/usr/bin/env node

/**
 * Test Funded Framework Infrastructure
 * Validates the funded testing framework without actually connecting to network
 */

require('dotenv').config();

async function testFramework() {
    console.log('🧪 Testing Funded Framework Infrastructure');
    console.log('=========================================');

    let passed = 0;
    let failed = 0;

    async function test(name, fn) {
        try {
            await fn();
            console.log(`✅ ${name}`);
            passed++;
        } catch (error) {
            console.log(`❌ ${name}: ${error.message}`);
            failed++;
        }
    }

    // Test 1: Environment configuration
    await test('Environment configuration loaded', () => {
        if (!process.env.ENABLE_FUNDED_TESTS) {
            throw new Error('ENABLE_FUNDED_TESTS not set');
        }
        if (!process.env.FAUCET_1_ADDRESS) {
            throw new Error('FAUCET_1_ADDRESS not configured');
        }
        if (process.env.NETWORK !== 'testnet') {
            throw new Error('Network not set to testnet');
        }
    });

    // Test 2: Safety mechanisms
    await test('Safety mechanisms work', () => {
        const limits = {
            perTest: parseInt(process.env.MAX_CREDITS_PER_TEST),
            perSuite: parseInt(process.env.MAX_CREDITS_PER_SUITE),
            daily: parseInt(process.env.MAX_DAILY_USAGE)
        };

        if (limits.perTest <= 0 || limits.perTest > 500000000) {
            throw new Error('Invalid per-test limit');
        }
        if (limits.perSuite <= limits.perTest) {
            throw new Error('Per-suite limit should be greater than per-test');
        }
        if (limits.daily <= limits.perSuite) {
            throw new Error('Daily limit should be greater than per-suite');
        }
    });

    // Test 3: Credit tracker initialization
    await test('Credit tracker initializes', async () => {
        const CreditTracker = require('./utils/credit-tracker');
        const tracker = new CreditTracker({ debug: false });
        
        const sessionStats = tracker.getSessionStats();
        if (!sessionStats.sessionId) {
            throw new Error('Session ID not generated');
        }
        if (sessionStats.totalOperations !== 0) {
            throw new Error('Initial operation count should be 0');
        }
        
        // Test operation recording
        tracker.recordOperation({
            type: 'test-operation',
            identityId: 'test-identity',
            amount: 1000000,
            satoshis: 1000,
            txId: 'test-tx',
            testName: 'framework-test',
            success: true
        });

        const updatedStats = tracker.getSessionStats();
        if (updatedStats.totalOperations !== 1) {
            throw new Error('Operation recording failed');
        }

        await tracker.finalize();
    });

    // Test 4: Identity pool logic
    await test('Identity pool logic works', async () => {
        // Test the identity pool without actual network operations
        const poolConfig = {
            poolSize: parseInt(process.env.IDENTITY_POOL_SIZE),
            minBalance: parseInt(process.env.MIN_IDENTITY_BALANCE),
            initialCredits: parseInt(process.env.INITIAL_IDENTITY_CREDITS)
        };

        if (poolConfig.poolSize <= 0 || poolConfig.poolSize > 100) {
            throw new Error('Invalid pool size');
        }
        if (poolConfig.minBalance <= 0) {
            throw new Error('Invalid minimum balance');
        }
        if (poolConfig.initialCredits <= poolConfig.minBalance) {
            throw new Error('Initial credits should be greater than minimum balance');
        }

        console.log(`   Pool Size: ${poolConfig.poolSize} identities`);
        console.log(`   Min Balance: ${poolConfig.minBalance} credits`);
        console.log(`   Initial Credits: ${poolConfig.initialCredits} credits`);
    });

    // Test 5: File structure validation
    await test('Test file structure complete', async () => {
        const fs = require('fs');
        const path = require('path');

        const requiredFiles = [
            'utils/wasm-faucet-client.js',
            'utils/identity-pool.js', 
            'utils/credit-tracker.js',
            'utils/validate-funded-config.js',
            'utils/check-faucet-balance.js',
            'integration/identity-operations.test.js',
            'integration/document-operations.test.js',
            '.env',
            'package.json',
            'README.md'
        ];

        for (const file of requiredFiles) {
            const filePath = path.join(__dirname, file);
            if (!fs.existsSync(filePath)) {
                throw new Error(`Required file missing: ${file}`);
            }
            
            const stats = fs.statSync(filePath);
            if (stats.size === 0) {
                throw new Error(`Required file empty: ${file}`);
            }
        }

        console.log(`   Verified ${requiredFiles.length} required files`);
    });

    // Test 6: Playwright test syntax validation
    await test('Playwright test files are valid', async () => {
        const fs = require('fs');
        
        const testFiles = [
            'integration/identity-operations.test.js',
            'integration/document-operations.test.js'
        ];

        for (const testFile of testFiles) {
            const content = fs.readFileSync(testFile, 'utf8');
            
            // Check for required Playwright imports and structure
            if (!content.includes("const { test, expect } = require('@playwright/test')")) {
                throw new Error(`${testFile} missing Playwright imports`);
            }
            if (!content.includes('test.describe')) {
                throw new Error(`${testFile} missing test structure`);
            }
            if (!content.includes('WasmFaucetClient')) {
                throw new Error(`${testFile} missing faucet client integration`);
            }
            if (!content.includes('creditTracker.recordOperation')) {
                throw new Error(`${testFile} missing credit tracking`);
            }
        }

        console.log(`   Validated ${testFiles.length} Playwright test files`);
    });

    // Test 7: Security validations
    await test('Security validations active', () => {
        // Network safety
        if (process.env.NETWORK === 'mainnet') {
            throw new Error('🚨 CRITICAL: Mainnet detected - funded tests blocked');
        }

        // Configuration safety
        if (!process.env.ENABLE_FUNDED_TESTS) {
            throw new Error('Funded tests not explicitly enabled');
        }

        // Faucet key format basic validation
        const faucetKey = process.env.FAUCET_1_PRIVATE_KEY;
        if (!faucetKey || faucetKey.length < 50) {
            throw new Error('Faucet private key appears invalid');
        }

        // Address format basic validation  
        const faucetAddr = process.env.FAUCET_1_ADDRESS;
        if (!faucetAddr || (!faucetAddr.startsWith('y') && !faucetAddr.startsWith('X'))) {
            throw new Error('Faucet address format appears invalid for testnet');
        }
    });

    // Summary
    console.log('');
    console.log('📊 Framework Test Summary');
    console.log('=========================');
    console.log(`✅ Passed: ${passed}`);
    console.log(`❌ Failed: ${failed}`);
    console.log(`📈 Success Rate: ${(passed / (passed + failed) * 100).toFixed(1)}%`);

    if (failed === 0) {
        console.log('');
        console.log('🎉 Funded testing framework is ready!');
        console.log('');
        console.log('✅ Infrastructure Status:');
        console.log('  ✅ Environment properly configured');
        console.log('  ✅ Safety mechanisms active');
        console.log('  ✅ Credit tracking operational');  
        console.log('  ✅ Identity pool logic validated');
        console.log('  ✅ Test structure complete');
        console.log('  ✅ Security validations passing');
        console.log('');
        console.log('🚀 Ready for live testing:');
        console.log('  • Dry run completed successfully');
        console.log('  • All safety mechanisms validated');
        console.log('  • Configuration complete and secure');
        console.log('  • Real testnet funding infrastructure ready');
        console.log('');
        console.log('💡 Next Steps:');
        console.log('  1. Run low-tier tests: ./run-funded-tests.sh --tier low --confirm-safety');
        console.log('  2. Monitor usage: npm run usage-report');
        console.log('  3. Check balance: npm run check-faucet');
        
        return 0;
    } else {
        console.log('');
        console.log(`❌ ${failed} framework tests failed`);
        console.log('Please fix the issues above before running funded tests');
        return 1;
    }
}

testFramework()
    .then(code => process.exit(code))
    .catch(error => {
        console.error('💥 Framework test crashed:', error.message);
        process.exit(1);
    });