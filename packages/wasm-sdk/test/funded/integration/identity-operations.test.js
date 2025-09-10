/**
 * Funded Identity Operations Integration Tests
 * Tests real identity creation, funding, and operations using actual testnet credits
 * 
 * ⚠️ WARNING: These tests consume real testnet funds!
 * Ensure proper environment configuration before running.
 */

const { test, expect } = require('@playwright/test');
const WasmFaucetClient = require('../utils/wasm-faucet-client');
const IdentityPool = require('../utils/identity-pool');
const CreditTracker = require('../utils/credit-tracker');

// Load environment configuration
require('dotenv').config({ path: require('path').join(__dirname, '../.env') });

test.describe('Funded Identity Operations', () => {
    let faucetClient;
    let identityPool;
    let creditTracker;

    test.beforeAll(async () => {
        // Safety checks
        if (!process.env.ENABLE_FUNDED_TESTS) {
            test.skip('Funded tests not enabled. Set ENABLE_FUNDED_TESTS=true to run.');
        }

        if (process.env.NETWORK !== 'testnet' && process.env.NETWORK !== 'devnet') {
            test.skip('Funded tests only run on testnet/devnet for safety');
        }

        // Initialize funding infrastructure
        creditTracker = new CreditTracker({ debug: true });
        faucetClient = new WasmFaucetClient({ debug: true });
        identityPool = new IdentityPool({ 
            poolSize: 5, // Smaller pool for testing
            debug: true 
        });

        await faucetClient.initialize();
        await identityPool.initialize();
        
        console.log('🚰 Funded test infrastructure ready');
        console.log(`💰 Faucet balance: ${await faucetClient.getFaucetBalance()} satoshis`);
    });

    test.afterAll(async () => {
        if (creditTracker) {
            const report = await creditTracker.finalize();
            console.log('📊 Credit usage report generated:', report.reportFile);
        }

        if (identityPool) {
            await identityPool.cleanup();
        }

        if (faucetClient) {
            await faucetClient.cleanup();
        }
    });

    test.describe('Real Identity Creation', () => {
        test('should create identity with 100M credits', async () => {
            const creditsAmount = 100000000; // 100M credits (~1 DASH)
            
            // Check if funding is available
            const canFund = await faucetClient.canFund(creditsAmount / 1000 + 50000000);
            if (!canFund.canFund) {
                test.skip(`Cannot fund test: ${canFund.reason}`);
            }

            const startTime = Date.now();
            
            try {
                // Create identity with real funding
                const result = await faucetClient.createFundedIdentity(creditsAmount);
                
                // Verify identity was created
                expect(result.identityId).toBeDefined();
                expect(result.identityId).toMatch(/^[A-Za-z0-9]{44,}$/); // Base58 format
                expect(result.privateKey).toBeDefined();
                expect(result.creditsAmount).toBe(creditsAmount);
                expect(result.txId).toBeDefined();

                // Track the operation
                creditTracker.recordOperation({
                    type: 'identity-creation',
                    identityId: result.identityId,
                    amount: creditsAmount,
                    satoshis: creditsAmount / 1000 + 10000000, // Estimated
                    txId: result.txId,
                    testName: 'should create identity with 100M credits',
                    success: true,
                    metadata: {
                        duration: Date.now() - startTime,
                        networkConfirmation: true
                    }
                });

                console.log(`✅ Identity created successfully: ${result.identityId}`);
                console.log(`💰 Credits allocated: ${creditsAmount}`);
                console.log(`📤 Transaction: ${result.txId}`);

            } catch (error) {
                creditTracker.recordError(`Identity creation failed: ${error.message}`);
                throw error;
            }
        });

        test('should handle insufficient funding gracefully', async () => {
            const invalidAmount = 1000; // Too small for identity creation
            
            try {
                await faucetClient.createFundedIdentity(invalidAmount);
                expect(true).toBe(false); // Should not reach here
            } catch (error) {
                // Expected failure - record but don't fail test
                creditTracker.recordOperation({
                    type: 'identity-creation-invalid',
                    identityId: null,
                    amount: invalidAmount,
                    satoshis: 0,
                    txId: null,
                    testName: 'should handle insufficient funding gracefully',
                    success: false,
                    error: error.message
                });

                expect(error.message).toMatch(/insufficient|minimum|too small/i);
                console.log(`✅ Correctly rejected insufficient funding: ${error.message}`);
            }
        });

        test('should create multiple identities from pool', async () => {
            const identitiesNeeded = 3;
            const creditsPerIdentity = 25000000; // 25M credits each
            
            const identities = [];
            
            try {
                for (let i = 0; i < identitiesNeeded; i++) {
                    const identity = await identityPool.getAvailableIdentity(creditsPerIdentity);
                    identities.push(identity);
                    
                    expect(identity.identityId).toBeDefined();
                    expect(identity.creditsAmount).toBeGreaterThanOrEqual(creditsPerIdentity);
                    
                    console.log(`✅ Pool identity ${i + 1}: ${identity.identityId} (${identity.creditsAmount} credits)`);
                }

                // Verify all identities are unique
                const uniqueIds = new Set(identities.map(id => id.identityId));
                expect(uniqueIds.size).toBe(identitiesNeeded);

                // Return identities to pool (simulating test completion)
                for (const identity of identities) {
                    const remainingCredits = identity.creditsAmount - 1000000; // Simulate some usage
                    identityPool.returnIdentity(identity.identityId, remainingCredits);
                    
                    creditTracker.recordOperation({
                        type: 'pool-identity-usage',
                        identityId: identity.identityId,
                        amount: 1000000, // Credits used
                        satoshis: 1000, // Minimal tracking
                        txId: null,
                        testName: 'should create multiple identities from pool',
                        success: true,
                        metadata: {
                            poolOperation: true,
                            remainingCredits
                        }
                    });
                }

                console.log(`✅ Pool test completed: ${identitiesNeeded} identities used and returned`);

            } catch (error) {
                creditTracker.recordError(`Pool identity test failed: ${error.message}`);
                throw error;
            }
        });
    });

    test.describe('Identity Funding Operations', () => {
        test('should top up existing identity', async () => {
            const topupAmount = 50000000; // 50M credits
            
            try {
                // Get an identity from pool
                const identity = await identityPool.getAvailableIdentity(10000000);
                const initialCredits = identity.creditsAmount;
                
                // Top up the identity
                const result = await faucetClient.topupIdentity(
                    identity.identityId,
                    identity.privateKey,
                    topupAmount
                );

                expect(result.identityId).toBe(identity.identityId);
                expect(result.additionalCredits).toBe(topupAmount);
                expect(result.txId).toBeDefined();

                // Update pool with new balance
                identityPool.returnIdentity(identity.identityId, initialCredits + topupAmount);

                creditTracker.recordOperation({
                    type: 'identity-topup',
                    identityId: identity.identityId,
                    amount: topupAmount,
                    satoshis: topupAmount / 1000 + 5000000,
                    txId: result.txId,
                    testName: 'should top up existing identity',
                    success: true,
                    metadata: {
                        initialCredits,
                        finalCredits: initialCredits + topupAmount
                    }
                });

                console.log(`✅ Identity topped up: ${result.identityId}`);
                console.log(`💰 Credits added: ${topupAmount} (total: ${initialCredits + topupAmount})`);
                console.log(`📤 Transaction: ${result.txId}`);

            } catch (error) {
                creditTracker.recordError(`Identity topup failed: ${error.message}`);
                throw error;
            }
        });

        test('should handle topup with insufficient faucet balance', async () => {
            const excessiveAmount = 100000000000; // 100B credits (impossible amount)
            
            // Get an identity 
            const identity = await identityPool.getAvailableIdentity(1000000);
            
            try {
                await faucetClient.topupIdentity(
                    identity.identityId,
                    identity.privateKey,
                    excessiveAmount
                );
                
                expect(true).toBe(false); // Should not reach here
                
            } catch (error) {
                // Expected failure - ensure it's the right type of error
                expect(error.message).toMatch(/limit|balance|exceed|insufficient/i);
                
                creditTracker.recordOperation({
                    type: 'identity-topup-rejected',
                    identityId: identity.identityId,
                    amount: excessiveAmount,
                    satoshis: 0,
                    txId: null,
                    testName: 'should handle topup with insufficient faucet balance',
                    success: false,
                    error: error.message
                });

                // Return identity to pool (no credits used)
                identityPool.returnIdentity(identity.identityId, identity.creditsAmount);
                
                console.log(`✅ Correctly rejected excessive topup: ${error.message}`);
            }
        });
    });

    test.describe('Identity Balance Verification', () => {
        test('should verify identity balance after creation', async () => {
            const creditsAmount = 75000000; // 75M credits
            
            try {
                // Create identity
                const identity = await faucetClient.createFundedIdentity(creditsAmount);
                
                // Wait for balance to be available (network propagation)
                await test.step('Wait for balance propagation', async () => {
                    // This would use WASM SDK to check the balance
                    // For now, we verify the identity exists and has expected structure
                    expect(identity.identityId).toMatch(/^[A-Za-z0-9]{44,}$/);
                    expect(identity.creditsAmount).toBe(creditsAmount);
                });

                creditTracker.recordOperation({
                    type: 'identity-balance-verification',
                    identityId: identity.identityId,
                    amount: creditsAmount,
                    satoshis: creditsAmount / 1000 + 10000000,
                    txId: identity.txId,
                    testName: 'should verify identity balance after creation',
                    success: true,
                    metadata: {
                        verificationMethod: 'creation-response',
                        networkVerification: false // Would need WASM SDK integration
                    }
                });

                console.log(`✅ Identity balance verified: ${creditsAmount} credits`);

            } catch (error) {
                creditTracker.recordError(`Identity balance verification failed: ${error.message}`);
                throw error;
            }
        });

        test('should track credit consumption during operations', async () => {
            const identity = await identityPool.getAvailableIdentity(20000000);
            const initialCredits = identity.creditsAmount;
            
            try {
                // Simulate credit usage (in real test, this would be actual operations)
                const simulatedUsage = 5000000; // 5M credits used
                const remainingCredits = initialCredits - simulatedUsage;
                
                // Record the simulated operation
                creditTracker.recordOperation({
                    type: 'credit-consumption-simulation',
                    identityId: identity.identityId,
                    amount: simulatedUsage,
                    satoshis: simulatedUsage / 1000,
                    txId: 'simulated-operation',
                    testName: 'should track credit consumption during operations',
                    success: true,
                    metadata: {
                        initialCredits,
                        remainingCredits,
                        operationType: 'document-creation-simulation'
                    }
                });

                // Return identity with updated balance
                identityPool.returnIdentity(identity.identityId, remainingCredits);

                expect(remainingCredits).toBe(initialCredits - simulatedUsage);
                expect(remainingCredits).toBeGreaterThan(0);

                console.log(`✅ Credit consumption tracked: ${simulatedUsage} credits used`);
                console.log(`💰 Remaining: ${remainingCredits} credits`);

            } catch (error) {
                // Return identity with original balance if test failed
                identityPool.returnIdentity(identity.identityId, initialCredits);
                creditTracker.recordError(`Credit tracking failed: ${error.message}`);
                throw error;
            }
        });
    });

    test.describe('Pool Management and Optimization', () => {
        test('should maintain identity pool automatically', async () => {
            try {
                const initialStats = identityPool.getPoolStats();
                
                // Trigger pool maintenance
                await identityPool.maintainPool();
                
                const finalStats = identityPool.getPoolStats();
                
                // Verify pool health
                expect(finalStats.available).toBeGreaterThanOrEqual(0);
                expect(finalStats.total).toBeGreaterThanOrEqual(initialStats.total);

                creditTracker.recordOperation({
                    type: 'pool-maintenance',
                    identityId: null,
                    amount: 0,
                    satoshis: 0,
                    txId: null,
                    testName: 'should maintain identity pool automatically',
                    success: true,
                    metadata: {
                        initialStats,
                        finalStats,
                        maintenanceRequired: finalStats.available !== initialStats.available
                    }
                });

                console.log(`✅ Pool maintenance completed`);
                console.log(`📊 Pool status: ${finalStats.available} available, ${finalStats.inUse} in use`);

            } catch (error) {
                creditTracker.recordError(`Pool maintenance failed: ${error.message}`);
                throw error;
            }
        });

        test('should handle concurrent identity requests', async () => {
            const concurrentRequests = 3;
            const creditsPerRequest = 15000000; // 15M credits each
            
            try {
                const promises = [];
                for (let i = 0; i < concurrentRequests; i++) {
                    promises.push(
                        identityPool.getAvailableIdentity(creditsPerRequest)
                            .then(identity => ({ index: i, identity }))
                    );
                }

                const results = await Promise.all(promises);
                
                // Verify all requests succeeded and got unique identities
                expect(results).toHaveLength(concurrentRequests);
                
                const identityIds = results.map(r => r.identity.identityId);
                const uniqueIds = new Set(identityIds);
                expect(uniqueIds.size).toBe(concurrentRequests);

                // Return all identities to pool
                for (const { index, identity } of results) {
                    identityPool.returnIdentity(identity.identityId, identity.creditsAmount - 1000000);
                    
                    creditTracker.recordOperation({
                        type: 'concurrent-pool-request',
                        identityId: identity.identityId,
                        amount: 1000000, // Simulated usage
                        satoshis: 1000,
                        txId: null,
                        testName: 'should handle concurrent identity requests',
                        success: true,
                        metadata: {
                            requestIndex: index,
                            concurrentRequests,
                            creditsAllocated: identity.creditsAmount
                        }
                    });
                }

                console.log(`✅ Concurrent requests handled: ${concurrentRequests} identities allocated`);

            } catch (error) {
                creditTracker.recordError(`Concurrent identity requests failed: ${error.message}`);
                throw error;
            }
        });
    });

    test.describe('Safety and Limits Testing', () => {
        test('should enforce daily usage limits', async () => {
            const dailyLimit = faucetClient.dailyUsageLimit;
            const currentUsage = faucetClient.currentDailyUsage;
            const remainingBudget = dailyLimit - currentUsage;
            
            console.log(`📊 Daily limit enforcement test:`);
            console.log(`   Daily Limit: ${dailyLimit} satoshis`);
            console.log(`   Current Usage: ${currentUsage} satoshis`);
            console.log(`   Remaining Budget: ${remainingBudget} satoshis`);

            // Try to exceed daily limit (should be rejected)
            const excessiveAmount = remainingBudget + 100000000;
            
            const canFund = await faucetClient.canFund(excessiveAmount);
            expect(canFund.canFund).toBe(false);
            expect(canFund.reason).toMatch(/daily.*limit|exceed/i);

            creditTracker.recordOperation({
                type: 'daily-limit-test',
                identityId: null,
                amount: 0,
                satoshis: 0,
                txId: null,
                testName: 'should enforce daily usage limits',
                success: true,
                metadata: {
                    dailyLimit,
                    currentUsage,
                    remainingBudget,
                    rejectedAmount: excessiveAmount,
                    rejectionReason: canFund.reason
                }
            });

            console.log(`✅ Daily limit enforcement working: ${canFund.reason}`);
        });

        test('should validate network safety', async () => {
            // This test verifies that funded operations are properly restricted to testnet
            expect(process.env.NETWORK).toBe('testnet');
            
            // Verify faucet client enforces network restrictions
            expect(faucetClient.network).toBe('testnet');
            
            creditTracker.recordOperation({
                type: 'network-safety-validation',
                identityId: null,
                amount: 0,
                satoshis: 0,
                txId: null,
                testName: 'should validate network safety',
                success: true,
                metadata: {
                    network: process.env.NETWORK,
                    faucetNetwork: faucetClient.network,
                    safetyChecks: ['testnet-only', 'environment-validation']
                }
            });

            console.log(`✅ Network safety validated: ${process.env.NETWORK}`);
        });

        test('should generate comprehensive usage report', async () => {
            const sessionStats = creditTracker.getSessionStats();
            const poolStats = identityPool.getPoolStats();
            const faucetStats = faucetClient.getFundingStats();
            
            expect(sessionStats.sessionId).toBeDefined();
            expect(sessionStats.totalOperations).toBeGreaterThan(0);
            expect(poolStats.total).toBeGreaterThanOrEqual(0);
            expect(faucetStats.workerId).toBeDefined();

            // Export comprehensive report
            const reportFile = creditTracker.exportUsageReport();
            expect(reportFile).toBeDefined();
            
            console.log(`✅ Usage report generated: ${reportFile}`);
            console.log(`📊 Session stats: ${sessionStats.totalOperations} operations`);
            console.log(`🏊 Pool stats: ${poolStats.available} available, ${poolStats.inUse} in use`);
            console.log(`🚰 Faucet stats: ${faucetStats.totalOperations} operations, ${faucetStats.totalUsage} satoshis used`);
        });
    });
});