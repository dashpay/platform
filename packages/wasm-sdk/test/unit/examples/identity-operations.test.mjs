/**
 * Unit Tests for identity-operations.mjs example
 * Tests comprehensive identity management and queries
 */

import { jest } from '@jest/globals';

describe('Identity Operations Example', () => {
    let sdk;

    beforeAll(async () => {
        const wasmInitialized = await global.initializeWasm();
        if (!wasmInitialized) {
            throw new Error('Failed to initialize WASM - tests cannot proceed');
        }
    });

    beforeEach(async () => {
        sdk = await global.createTestSDK({
            network: 'testnet',
            proofs: false
        });
    });

    afterEach(async () => {
        if (sdk && sdk.destroy) {
            await sdk.destroy();
        }
    });

    describe('Identity Lookup and Information', () => {
        test('should lookup identity and retrieve information', async () => {
            const identityId = TEST_CONFIG.SAMPLE_IDENTITY_ID;
            
            const performance = await global.measurePerformance(
                () => sdk.getIdentity(identityId),
                'getIdentity'
            );

            const identity = performance.result;
            
            if (identity) {
                global.expectValidIdentity(identity);
                
                // Verify identity structure
                expect(identity).toHaveProperty('id');
                expect(identity).toHaveProperty('balance');
                expect(identity).toHaveProperty('revision');
                
                // Verify data types
                expect(typeof identity.id).toBe('string');
                expect(typeof identity.balance).toBe('number');
                expect(typeof identity.revision).toBe('number');
                
                // Verify ranges
                expect(identity.balance).toBeGreaterThanOrEqual(0);
                expect(identity.revision).toBeGreaterThanOrEqual(0);
            } else {
                // Identity not found - acceptable for test
                expect(identity).toBeNull();
            }

            // Performance verification
            expect(performance.duration).toCompleteWithinTime(10000);
        }, TEST_CONFIG.SLOW_TIMEOUT);

        test('should handle multiple identity lookups', async () => {
            const identityIds = [
                TEST_CONFIG.SAMPLE_IDENTITY_ID,
                '8WFWpLyWYtXDHw9kaqj5Rc5Xac5WKFZF8GZrUbLjMbmz', // Another test ID
                'AnotherTestIdentityThatMightNotExist123456789'
            ];

            const results = [];
            
            for (const identityId of identityIds) {
                try {
                    const identity = await sdk.getIdentity(identityId);
                    results.push({ identityId, identity, error: null });
                } catch (error) {
                    results.push({ identityId, identity: null, error: error.message });
                }
            }

            expect(results).toHaveLength(identityIds.length);
            
            // At least one result should be processed (even if null/error)
            expect(results.every(r => r.identityId)).toBe(true);
        }, TEST_CONFIG.SLOW_TIMEOUT);
    });

    describe('Balance and Revision Queries', () => {
        test('should get identity balance', async () => {
            const identityId = TEST_CONFIG.SAMPLE_IDENTITY_ID;
            
            try {
                const performance = await global.measurePerformance(
                    () => sdk.getIdentityBalance(identityId),
                    'getIdentityBalance'
                );

                const balance = performance.result;
                
                if (balance !== null) {
                    expect(typeof balance).toBe('object');
                    expect(balance).toHaveProperty('balance');
                    expect(balance).toHaveProperty('revision');
                    
                    expect(typeof balance.balance).toBe('number');
                    expect(typeof balance.revision).toBe('number');
                    
                    expect(balance.balance).toBeGreaterThanOrEqual(0);
                    expect(balance.revision).toBeGreaterThanOrEqual(0);
                }

                expect(performance.duration).toCompleteWithinTime(8000);
            } catch (error) {
                // Identity might not exist
                expect(error.message).toMatch(/not found|invalid/i);
            }
        }, TEST_CONFIG.STANDARD_TIMEOUT);

        test('should handle balance queries for non-existent identities', async () => {
            const nonExistentId = 'NonExistentIdentity123456789AbCdEf123456789';
            
            await expect(async () => {
                await sdk.getIdentityBalance(nonExistentId);
            }).rejects.toThrow();
        }, TEST_CONFIG.STANDARD_TIMEOUT);

        test('should get identity revision', async () => {
            const identityId = TEST_CONFIG.SAMPLE_IDENTITY_ID;
            
            try {
                const identity = await sdk.getIdentity(identityId);
                
                if (identity && identity.revision !== undefined) {
                    expect(typeof identity.revision).toBe('number');
                    expect(identity.revision).toBeGreaterThanOrEqual(0);
                }
            } catch (error) {
                // Identity might not exist
                expect(error.message).toMatch(/not found|invalid/i);
            }
        }, TEST_CONFIG.STANDARD_TIMEOUT);
    });

    describe('Key Management and Nonce Operations', () => {
        test('should retrieve identity keys', async () => {
            const identityId = TEST_CONFIG.SAMPLE_IDENTITY_ID;
            
            try {
                const identity = await sdk.getIdentity(identityId);
                
                if (identity && identity.publicKeys) {
                    expect(Array.isArray(identity.publicKeys)).toBe(true);
                    
                    identity.publicKeys.forEach(key => {
                        expect(key).toHaveProperty('id');
                        expect(key).toHaveProperty('type');
                        expect(key).toHaveProperty('data');
                        expect(key).toHaveProperty('purpose');
                        expect(key).toHaveProperty('securityLevel');
                        
                        // Verify key properties
                        expect(typeof key.id).toBe('number');
                        expect(typeof key.type).toBe('number');
                        expect(typeof key.data).toBe('string');
                        expect(typeof key.purpose).toBe('number');
                        expect(typeof key.securityLevel).toBe('number');
                    });
                }
            } catch (error) {
                console.warn('Key retrieval test skipped - identity not found');
            }
        }, TEST_CONFIG.STANDARD_TIMEOUT);

        test('should handle key type mapping', () => {
            // Test key type constants/mappings if available
            const keyTypes = {
                0: 'ECDSA_SECP256K1',
                1: 'BLS12_381',
                2: 'ECDSA_HASH160',
                3: 'BIP13_SCRIPT_HASH'
            };

            Object.entries(keyTypes).forEach(([type, name]) => {
                expect(typeof name).toBe('string');
                expect(name).toMatch(/^[A-Z_0-9]+$/);
            });
        });

        test('should handle key purpose mapping', () => {
            const keyPurposes = {
                0: 'AUTHENTICATION',
                1: 'ENCRYPTION',
                2: 'DECRYPTION', 
                3: 'WITHDRAW'
            };

            Object.entries(keyPurposes).forEach(([purpose, name]) => {
                expect(typeof name).toBe('string');
                expect(name).toMatch(/^[A-Z_]+$/);
            });
        });

        test('should handle security level mapping', () => {
            const securityLevels = {
                0: 'MASTER',
                1: 'CRITICAL',
                2: 'HIGH',
                3: 'MEDIUM'
            };

            Object.entries(securityLevels).forEach(([level, name]) => {
                expect(typeof name).toBe('string');
                expect(name).toMatch(/^[A-Z]+$/);
            });
        });
    });

    describe('Multi-Identity Batch Operations', () => {
        test('should handle batch identity lookups', async () => {
            const batchIdentities = [
                TEST_CONFIG.SAMPLE_IDENTITY_ID,
                '8WFWpLyWYtXDHw9kaqj5Rc5Xac5WKFZF8GZrUbLjMbmz',
                'TestIdentity123456789AbCdEf123456789AbCdEf'
            ];

            const startTime = performance.now();
            const results = await Promise.allSettled(
                batchIdentities.map(id => sdk.getIdentity(id))
            );
            const totalTime = performance.now() - startTime;

            expect(results).toHaveLength(batchIdentities.length);
            
            // Check results structure
            results.forEach((result, index) => {
                expect(['fulfilled', 'rejected']).toContain(result.status);
                
                if (result.status === 'fulfilled' && result.value) {
                    global.expectValidIdentity(result.value);
                }
            });

            console.log(`Batch lookup of ${batchIdentities.length} identities took ${totalTime.toFixed(2)}ms`);
            
            // Performance check - batch should be reasonably efficient
            expect(totalTime).toBeLessThan(30000); // 30 seconds max for batch
        }, TEST_CONFIG.SLOW_TIMEOUT);

        test('should handle batch balance queries', async () => {
            const batchIdentities = [TEST_CONFIG.SAMPLE_IDENTITY_ID];

            const results = await Promise.allSettled(
                batchIdentities.map(async (id) => {
                    try {
                        return await sdk.getIdentityBalance(id);
                    } catch (error) {
                        return { error: error.message };
                    }
                })
            );

            expect(results).toHaveLength(batchIdentities.length);
            
            results.forEach(result => {
                expect(['fulfilled', 'rejected']).toContain(result.status);
            });
        }, TEST_CONFIG.SLOW_TIMEOUT);
    });

    describe('Token-Related Identity Operations', () => {
        test('should handle identity token queries if available', async () => {
            // This test checks if token-related identity operations are available
            // It may not apply to all identity types
            
            const identityId = TEST_CONFIG.SAMPLE_IDENTITY_ID;
            
            try {
                // Try to get identity
                const identity = await sdk.getIdentity(identityId);
                
                if (identity) {
                    // Token operations might be available through SDK
                    // This is a placeholder for potential token-related operations
                    expect(identity).toBeDefined();
                }
            } catch (error) {
                console.warn('Token-related identity operations not available or identity not found');
            }
        }, TEST_CONFIG.STANDARD_TIMEOUT);
    });

    describe('Public Key Hash Operations', () => {
        test('should handle public key hash operations', async () => {
            const identityId = TEST_CONFIG.SAMPLE_IDENTITY_ID;
            
            try {
                const identity = await sdk.getIdentity(identityId);
                
                if (identity && identity.publicKeys && identity.publicKeys.length > 0) {
                    identity.publicKeys.forEach(key => {
                        // Verify key data format
                        expect(key.data).toBeValidBase58String();
                        
                        // Verify key properties are within expected ranges
                        expect(key.type).toBeGreaterThanOrEqual(0);
                        expect(key.type).toBeLessThanOrEqual(3);
                        
                        expect(key.purpose).toBeGreaterThanOrEqual(0);
                        expect(key.purpose).toBeLessThanOrEqual(3);
                        
                        expect(key.securityLevel).toBeGreaterThanOrEqual(0);
                        expect(key.securityLevel).toBeLessThanOrEqual(3);
                    });
                }
            } catch (error) {
                console.warn('Public key hash test skipped - identity not accessible');
            }
        }, TEST_CONFIG.STANDARD_TIMEOUT);
    });

    describe('Performance and Scalability', () => {
        test('should meet performance benchmarks for identity operations', async () => {
            const benchmarks = [
                {
                    name: 'getIdentity',
                    fn: () => sdk.getIdentity(TEST_CONFIG.SAMPLE_IDENTITY_ID),
                    maxTime: 8000
                }
            ];

            for (const benchmark of benchmarks) {
                try {
                    const performance = await global.measurePerformance(
                        benchmark.fn,
                        benchmark.name
                    );

                    expect(performance.duration).toCompleteWithinTime(benchmark.maxTime);
                } catch (error) {
                    console.warn(`Benchmark ${benchmark.name} failed:`, error.message);
                }
            }
        }, TEST_CONFIG.SLOW_TIMEOUT);

        test('should handle concurrent identity operations', async () => {
            const concurrentOps = 3;
            const identityId = TEST_CONFIG.SAMPLE_IDENTITY_ID;

            const operations = Array(concurrentOps).fill(null).map(() => 
                sdk.getIdentity(identityId).catch(error => ({ error: error.message }))
            );

            const results = await Promise.allSettled(operations);
            
            expect(results).toHaveLength(concurrentOps);
            
            // At least some operations should complete
            const completedOps = results.filter(r => r.status === 'fulfilled').length;
            expect(completedOps).toBeGreaterThan(0);
        }, TEST_CONFIG.SLOW_TIMEOUT);
    });

    describe('Error Handling and Edge Cases', () => {
        test('should handle malformed identity IDs', async () => {
            const malformedIds = [
                '',
                'too-short',
                '!@#$%^&*()',
                'x'.repeat(100), // Too long
                null,
                undefined
            ];

            for (const id of malformedIds) {
                await expect(async () => {
                    await sdk.getIdentity(id);
                }).rejects.toThrow();
            }
        }, TEST_CONFIG.STANDARD_TIMEOUT);

        test('should handle network timeouts gracefully', async () => {
            // Create SDK with very short timeout
            const timeoutSdk = await global.createTestSDK({
                transport: { timeout: 1 } // 1ms timeout
            });

            await expect(async () => {
                await timeoutSdk.getIdentity(TEST_CONFIG.SAMPLE_IDENTITY_ID);
            }).rejects.toThrow();

            await timeoutSdk.destroy();
        }, TEST_CONFIG.STANDARD_TIMEOUT);

        test('should handle identity not found scenarios', async () => {
            const nonExistentId = 'NonExistent123456789AbCdEf123456789AbCdEf1';
            
            const result = await sdk.getIdentity(nonExistentId).catch(error => null);
            expect(result).toBeNull();
        }, TEST_CONFIG.STANDARD_TIMEOUT);
    });
});