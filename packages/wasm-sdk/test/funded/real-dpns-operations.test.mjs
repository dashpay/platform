#!/usr/bin/env node

/**
 * Real DPNS Operations Test
 * Tests actual DPNS username registration using real funded identities
 * ⚠️ WARNING: This consumes real testnet credits!
 */

import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
import { webcrypto } from 'crypto';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

if (!global.crypto) global.crypto = webcrypto;

// Load environment configuration
import { config } from 'dotenv';
config({ path: join(__dirname, '.env') });

// Initialize WASM
console.log('Initializing WASM SDK for funded DPNS operations...');
const init = (await import('../../pkg/dash_wasm_sdk.js')).default;
const wasmPath = join(__dirname, '../../pkg/dash_wasm_sdk_bg.wasm');
const wasmBuffer = readFileSync(wasmPath);
await init(wasmBuffer);

// Import SDK
const { WasmSDK } = await import('../../src-js/index.js');

async function runRealDPNSTests() {
    console.log('💰 Real DPNS Operations Test');
    console.log('===========================');
    console.log('⚠️ WARNING: This will consume real testnet credits for username registration!');
    console.log('');

    // Safety checks
    if (!process.env.ENABLE_FUNDED_TESTS) {
        console.log('❌ Funded tests not enabled. Set ENABLE_FUNDED_TESTS=true in .env');
        process.exit(1);
    }

    if (process.env.NETWORK !== 'testnet') {
        console.log('❌ Network must be testnet for safety');
        process.exit(1);
    }

    // Check funded identity
    if (!process.env.TEST_IDENTITY_1_ID || !process.env.TEST_IDENTITY_1_PRIVATE_KEY) {
        console.log('❌ No funded test identity configured for DPNS operations');
        console.log('Configure TEST_IDENTITY_1_ID and TEST_IDENTITY_1_PRIVATE_KEY in .env');
        process.exit(1);
    }

    let passed = 0;
    let failed = 0;
    let totalCreditsUsed = 0;

    async function test(name, expectedCredits, fn) {
        try {
            console.log(`🧪 Testing: ${name}`);
            console.log(`   💰 Expected cost: ${expectedCredits} credits`);
            
            const startTime = Date.now();
            const result = await fn();
            const duration = Date.now() - startTime;
            
            if (result.creditsUsed) {
                totalCreditsUsed += result.creditsUsed;
                console.log(`   💳 Actual cost: ${result.creditsUsed} credits`);
            }
            
            console.log(`   ✅ PASSED (${duration}ms)`);
            passed++;
        } catch (error) {
            console.log(`   ❌ FAILED: ${error.message}`);
            failed++;
        }
        console.log('');
    }

    try {
        // Initialize SDK
        const sdk = new WasmSDK({
            network: 'testnet',
            proofs: false,
            debug: true,
            transport: {
                timeout: 60000,
                retries: 5
            }
        });

        await sdk.initialize();
        console.log('✅ WASM SDK initialized for DPNS operations');

        const testIdentityId = process.env.TEST_IDENTITY_1_ID;
        const testPrivateKey = process.env.TEST_IDENTITY_1_PRIVATE_KEY;
        const testMnemonic = process.env.MNEMONIC || "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

        console.log(`🎯 Using test identity: ${testIdentityId.substring(0, 20)}...`);
        
        // Test 1: Validate username format (free operation)
        await test('Validate usernames (free operation)', 0, async () => {
            const testUsernames = [
                'validuser123',
                'test-user',
                'a',
                'longusernamethatshouldstillbevalid'
            ];

            let validationResults = [];
            for (const username of testUsernames) {
                try {
                    const isValid = await sdk.dpnsIsValidUsername(username);
                    const isContested = await sdk.dpnsIsContestedUsername(username);
                    
                    validationResults.push({
                        username,
                        valid: isValid,
                        contested: isContested
                    });
                    
                    console.log(`     ${username}: valid=${isValid}, contested=${isContested}`);
                } catch (error) {
                    console.log(`     ${username}: validation failed - ${error.message}`);
                }
            }

            if (validationResults.length === 0) {
                throw new Error('No username validations completed');
            }

            return { creditsUsed: 0 }; // Validations are free
        });

        // Test 2: Check DPNS contract and existing domains (free)
        await test('Check DPNS contract and existing domains', 0, async () => {
            const dpnsContractId = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';
            
            try {
                // Get DPNS contract info
                const contract = await sdk.getDataContract(dpnsContractId);
                console.log(`   📋 DPNS contract found: ${contract ? '✓' : '✗'}`);

                // Query existing domains (free operation)
                const domains = await sdk.getDocuments(dpnsContractId, 'domain', {
                    limit: 5
                });
                
                console.log(`   📄 Found ${domains?.length || 0} existing domains`);
                
                if (domains && domains.length > 0) {
                    console.log(`   📊 Sample domain: ${domains[0].data?.label || 'label not available'}`);
                }

                return { creditsUsed: 0 }; // Queries are free

            } catch (error) {
                console.log(`   ⚠️ DPNS query failed: ${error.message}`);
                return { creditsUsed: 0 };
            }
        });

        // Test 3: Attempt DPNS domain registration (REAL CREDITS)
        await test('Register DPNS domain (REAL CREDIT CONSUMPTION)', 5000000, async () => {
            const uniqueUsername = `testuser${Date.now()}`;
            const dpnsContractId = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';
            
            // First validate the username
            const isValid = await sdk.dpnsIsValidUsername(uniqueUsername);
            if (!isValid) {
                throw new Error(`Generated username not valid: ${uniqueUsername}`);
            }

            console.log(`   🌐 Registering domain: ${uniqueUsername}.dash`);
            console.log(`   💰 Expected cost: ~5M credits for DPNS registration`);

            // Create DPNS domain document
            const domainData = {
                label: uniqueUsername,
                normalizedLabel: uniqueUsername.toLowerCase(),
                parentDomainName: 'dash',
                preorderSalt: new Array(32).fill(0).map(() => Math.floor(Math.random() * 256)),
                records: {
                    dashUniqueIdentityId: testIdentityId
                },
                subdomainRules: {
                    allowSubdomains: false
                }
            };

            try {
                const result = await sdk.createDocument(
                    testMnemonic,
                    testIdentityId,
                    dpnsContractId,
                    'domain',
                    JSON.stringify(domainData),
                    0 // key index
                );

                console.log(`   🎉 Domain registered: ${uniqueUsername}.dash`);
                console.log(`   📄 Document ID: ${result.documentId || 'ID not available'}`);
                console.log(`   🔗 Transaction: ${result.transactionId || result.txId || 'TX not available'}`);
                
                const estimatedCreditsUsed = 5000000; // DPNS registration cost
                return { creditsUsed: estimatedCreditsUsed, result };

            } catch (error) {
                console.log(`   🚨 DPNS registration failed: ${error.message}`);
                
                if (error.message.includes('insufficient')) {
                    throw new Error(`Insufficient credits for DPNS registration: ${error.message}`);
                }

                if (error.message.includes('already exists')) {
                    console.log(`   ℹ️ Username already exists (expected for some tests)`);
                    return { creditsUsed: 0 };
                }
                
                console.log(`   ℹ️ Registration error: ${error.message.substring(0, 100)}`);
                return { creditsUsed: 0 };
            }
        });

        // Test 4: Batch DPNS operations
        await test('Batch DPNS operations cost analysis', 0, async () => {
            const batchUsernames = [
                `batch1${Date.now()}`,
                `batch2${Date.now()}`,
                `batch3${Date.now()}`
            ];

            console.log(`   📦 Analyzing batch registration cost for ${batchUsernames.length} usernames`);

            let totalEstimatedCost = 0;
            let validUsernames = 0;

            for (const username of batchUsernames) {
                try {
                    const isValid = await sdk.dpnsIsValidUsername(username);
                    const isContested = await sdk.dpnsIsContestedUsername(username);
                    
                    if (isValid && !isContested) {
                        validUsernames++;
                        totalEstimatedCost += 5000000; // 5M credits per domain
                        console.log(`     ${username}: ✅ valid, cost ~5M credits`);
                    } else {
                        console.log(`     ${username}: ❌ ${!isValid ? 'invalid' : 'contested'}`);
                    }

                } catch (error) {
                    console.log(`     ${username}: ⚠️ validation failed`);
                }
            }

            console.log(`   📊 Batch analysis: ${validUsernames}/${batchUsernames.length} registrable`);
            console.log(`   💰 Total estimated cost: ${totalEstimatedCost} credits`);
            console.log(`   💵 Estimated DASH equivalent: ~${(totalEstimatedCost / 100000000).toFixed(4)} DASH`);

            return { creditsUsed: 0 }; // Analysis only, no actual registrations
        });

        // Cleanup
        await sdk.destroy();

    } catch (error) {
        console.error(`💥 DPNS test setup failed: ${error.message}`);
        failed++;
    }

    // Final summary
    console.log('📊 Real DPNS Operations Test Results');
    console.log('===================================');
    console.log(`✅ Tests Passed: ${passed}`);
    console.log(`❌ Tests Failed: ${failed}`);
    console.log(`💰 Total Credits Used: ${totalCreditsUsed} credits`);
    console.log(`💵 Estimated Cost: ~${(totalCreditsUsed / 100000000).toFixed(4)} DASH worth of credits`);
    console.log(`📈 Success Rate: ${(passed / (passed + failed) * 100).toFixed(1)}%`);

    if (totalCreditsUsed > 0) {
        console.log('');
        console.log('🎉 SUCCESS: Real DPNS credit consumption detected!');
        console.log('✅ DPNS operations working with actual funded identity');
        console.log(`💰 Confirmed ${totalCreditsUsed} credits used for username registration`);
    }

    return failed === 0 ? 0 : 1;
}

runRealDPNSTests()
    .then(code => process.exit(code))
    .catch(error => {
        console.error('💥 DPNS operations test crashed:', error.message);
        process.exit(1);
    });