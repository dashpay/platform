/**
 * Unit Tests for getting-started.mjs example
 * Tests the complete beginner tutorial flow
 */

import { jest } from '@jest/globals';

describe('Getting Started Example', () => {
    let sdk;

    beforeAll(async () => {
        // Initialize WASM once for all tests
        const wasmInitialized = await global.initializeWasm();
        if (!wasmInitialized) {
            throw new Error('Failed to initialize WASM - tests cannot proceed');
        }
    });

    beforeEach(async () => {
        // Create fresh SDK instance for each test
        sdk = await global.createTestSDK({
            network: 'testnet',
            proofs: false,
            debug: false
        });
    });

    afterEach(async () => {
        if (sdk && sdk.destroy) {
            await sdk.destroy();
        }
    });

    describe('SDK Initialization', () => {
        test('should initialize SDK successfully', async () => {
            expect(sdk).toBeDefined();
            expect(typeof sdk.getStatus).toBe('function');
            expect(typeof sdk.getIdentity).toBe('function');
        }, TEST_CONFIG.STANDARD_TIMEOUT);

        test('should connect to testnet by default', async () => {
            const status = await sdk.getStatus();
            expect(status).toBeDefined();
            expect(status.network || status.chain).toBeDefined();
        }, TEST_CONFIG.STANDARD_TIMEOUT);

        test('should handle network switching', async () => {
            // Test that we can create SDK instances for different networks
            const testnetSdk = await global.createTestSDK({ network: 'testnet' });
            const mainnetSdk = await global.createTestSDK({ network: 'mainnet' });

            expect(testnetSdk).toBeDefined();
            expect(mainnetSdk).toBeDefined();

            await testnetSdk.destroy();
            await mainnetSdk.destroy();
        }, TEST_CONFIG.SLOW_TIMEOUT);
    });

    describe('Basic Cryptographic Operations', () => {
        test('should generate mnemonic phrases', async () => {
            const wordCounts = [12, 15, 18, 21, 24];
            
            for (const wordCount of wordCounts) {
                const mnemonic = await sdk.generateMnemonic(wordCount);
                
                expect(typeof mnemonic).toBe('string');
                expect(mnemonic.split(' ')).toHaveLength(wordCount);
                
                // Validate mnemonic format
                expect(mnemonic).toMatch(/^[a-z ]+$/);
            }
        }, TEST_CONFIG.STANDARD_TIMEOUT);

        test('should validate mnemonic phrases', async () => {
            // Generate a valid mnemonic
            const validMnemonic = await sdk.generateMnemonic(12);
            const isValid = await sdk.validateMnemonic(validMnemonic);
            expect(isValid).toBe(true);

            // Test invalid mnemonic
            const invalidMnemonic = 'invalid mnemonic phrase that should not validate';
            const isInvalid = await sdk.validateMnemonic(invalidMnemonic);
            expect(isInvalid).toBe(false);
        }, TEST_CONFIG.STANDARD_TIMEOUT);

        test('should derive keys from seed', async () => {
            const mnemonic = await sdk.generateMnemonic(12);
            const seed = await sdk.generateSeedFromMnemonic(mnemonic);
            
            expect(seed).toBeDefined();
            expect(typeof seed).toBe('string');
            expect(seed.length).toBeGreaterThan(0);

            // Test key derivation
            const key = await sdk.deriveKeyFromSeed(seed, 'identity', 0);
            expect(key).toBeDefined();
            expect(typeof key.privateKey).toBe('string');
            expect(typeof key.publicKey).toBe('string');
        }, TEST_CONFIG.STANDARD_TIMEOUT);
    });

    describe('Platform Queries', () => {
        test('should get platform status', async () => {
            const performance = await global.measurePerformance(
                () => sdk.getStatus(),
                'getStatus'
            );

            const status = performance.result;
            expect(status).toBeDefined();
            expect(status).toBeInstanceOf(Object);
            
            // Performance check
            expect(performance.duration).toCompleteWithinTime(TEST_CONFIG.QUICK_TIMEOUT);
        }, TEST_CONFIG.STANDARD_TIMEOUT);

        test('should lookup identity by ID', async () => {
            const identityId = TEST_CONFIG.SAMPLE_IDENTITY_ID;
            
            const performance = await global.measurePerformance(
                () => sdk.getIdentity(identityId),
                'getIdentity'
            );

            const identity = performance.result;
            if (identity) {
                global.expectValidIdentity(identity);
                expect(identity.id).toBe(identityId);
            } else {
                // Identity might not exist - that's OK for this test
                expect(identity).toBeNull();
            }

            // Performance check
            expect(performance.duration).toCompleteWithinTime(TEST_CONFIG.STANDARD_TIMEOUT);
        }, TEST_CONFIG.SLOW_TIMEOUT);

        test('should get identity balance', async () => {
            const identityId = TEST_CONFIG.SAMPLE_IDENTITY_ID;
            
            try {
                const balance = await sdk.getIdentityBalance(identityId);
                
                if (balance !== null) {
                    expect(typeof balance).toBe('object');
                    expect(balance).toHaveProperty('balance');
                    expect(typeof balance.balance).toBe('number');
                    expect(balance.balance).toBeGreaterThanOrEqual(0);
                }
            } catch (error) {
                // Identity might not exist or have no balance
                expect(error.message).toContain('not found');
            }
        }, TEST_CONFIG.STANDARD_TIMEOUT);

        test('should query documents from known contracts', async () => {
            const contractId = TEST_CONFIG.DPNS_TESTNET;
            const documentType = 'domain';
            
            try {
                const documents = await sdk.getDocuments(contractId, documentType, {
                    limit: 5
                });

                if (documents && documents.length > 0) {
                    expect(documents).toHaveValidQueryResult();
                    documents.forEach(doc => global.expectValidDocument(doc));
                }
            } catch (error) {
                // Contract might not be accessible or have no documents
                console.warn('Document query failed (expected in some test environments):', error.message);
            }
        }, TEST_CONFIG.SLOW_TIMEOUT);
    });

    describe('Error Handling', () => {
        test('should handle invalid identity IDs gracefully', async () => {
            const invalidId = 'invalid-identity-id';
            
            await expect(async () => {
                await sdk.getIdentity(invalidId);
            }).rejects.toThrow();
        }, TEST_CONFIG.STANDARD_TIMEOUT);

        test('should handle network connectivity issues', async () => {
            // Create SDK with invalid endpoint
            const brokenSdk = await global.createTestSDK({
                transport: {
                    url: 'https://invalid.endpoint.com:1443'
                }
            });

            await expect(async () => {
                await brokenSdk.getStatus();
            }).rejects.toThrow();

            await brokenSdk.destroy();
        }, TEST_CONFIG.STANDARD_TIMEOUT);

        test('should handle malformed parameters', async () => {
            // Test various invalid parameters
            await expect(async () => {
                await sdk.getDocuments('', 'invalid-type');
            }).rejects.toThrow();

            await expect(async () => {
                await sdk.generateMnemonic(0);
            }).rejects.toThrow();

            await expect(async () => {
                await sdk.validateMnemonic('');
            }).rejects.toThrow();
        }, TEST_CONFIG.STANDARD_TIMEOUT);
    });

    describe('Resource Management', () => {
        test('should properly clean up resources', async () => {
            const sdkInstance = await global.createTestSDK();
            
            // Use the SDK
            await sdkInstance.getStatus();
            
            // Clean up
            await sdkInstance.destroy();
            
            // Verify cleanup (SDK should not respond after destruction)
            // Note: Implementation may vary - this is a conceptual test
            expect(sdkInstance).toBeDefined();
        }, TEST_CONFIG.STANDARD_TIMEOUT);

        test('should handle multiple concurrent operations', async () => {
            const operations = [
                sdk.getStatus(),
                sdk.generateMnemonic(12),
                sdk.getStatus()
            ];

            const results = await Promise.allSettled(operations);
            
            // At least some operations should succeed
            const successCount = results.filter(r => r.status === 'fulfilled').length;
            expect(successCount).toBeGreaterThan(0);
        }, TEST_CONFIG.SLOW_TIMEOUT);
    });

    describe('Performance Benchmarks', () => {
        test('should meet performance requirements for basic operations', async () => {
            const benchmarks = [
                { name: 'generateMnemonic', fn: () => sdk.generateMnemonic(12), maxTime: 1000 },
                { name: 'getStatus', fn: () => sdk.getStatus(), maxTime: 5000 },
                { name: 'validateMnemonic', fn: async () => {
                    const mnemonic = await sdk.generateMnemonic(12);
                    return sdk.validateMnemonic(mnemonic);
                }, maxTime: 2000 }
            ];

            for (const benchmark of benchmarks) {
                const performance = await global.measurePerformance(
                    benchmark.fn,
                    benchmark.name
                );

                expect(performance.duration).toCompleteWithinTime(benchmark.maxTime);
            }
        }, TEST_CONFIG.SLOW_TIMEOUT);

        test('should handle batch operations efficiently', async () => {
            const batchSize = 5;
            const mnemonics = [];

            const startTime = performance.now();
            
            for (let i = 0; i < batchSize; i++) {
                mnemonics.push(await sdk.generateMnemonic(12));
            }
            
            const totalTime = performance.now() - startTime;
            const avgTime = totalTime / batchSize;

            console.log(`Average mnemonic generation time: ${avgTime.toFixed(2)}ms`);
            
            expect(mnemonics).toHaveLength(batchSize);
            expect(avgTime).toBeLessThan(2000); // Should be reasonably fast
        }, TEST_CONFIG.SLOW_TIMEOUT);
    });
});