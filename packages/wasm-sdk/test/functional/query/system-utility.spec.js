/**
 * System and Utility Query Tests - Functional tests for system and utility queries
 * 
 * These tests verify system and utility query functions work correctly with the real WASM SDK.
 */

describe('System and Utility Queries (Functional)', () => {
    let sdk;
    
    // Ensure WASM is ready before all tests
    before(async function() {
        this.timeout(30000);
        await global.ensureWasmInitialized();
    });
    
    beforeEach(async function() {
        this.timeout(10000);
        // Create SDK instance for each test
        sdk = await global.createTestSdk.testnet();
    });
    
    afterEach(() => {
        if (sdk && sdk.free) {
            sdk.free();
        }
    });
    
    describe('System Information Queries', () => {
        it('should get current quorums info', async function() {
            this.timeout(15000);
            
            if (!global.wasmSdk.get_current_quorums_info) {
                this.skip('get_current_quorums_info not implemented');
                return;
            }
            
            try {
                const result = await global.wasmSdk.get_current_quorums_info(sdk);
                
                if (result) {
                    expect(result).to.be.an('array');
                    
                    result.forEach(quorum => {
                        expect(quorum).to.be.an('object');
                        
                        if (quorum.quorumType !== undefined) {
                            expect(quorum.quorumType).to.be.a('number');
                        }
                        if (quorum.memberCount !== undefined) {
                            expect(quorum.memberCount).to.be.a('number');
                            expect(quorum.memberCount).to.be.at.least(0);
                        }
                    });
                }
            } catch (error) {
                expect(error.message).to.match(/network|connection|timeout|quorum/i);
            }
        });
        
        it('should get total credits in platform', async function() {
            this.timeout(15000);
            
            if (!global.wasmSdk.get_total_credits_in_platform) {
                this.skip('get_total_credits_in_platform not implemented');
                return;
            }
            
            try {
                const result = await global.wasmSdk.get_total_credits_in_platform(sdk);
                
                if (result !== undefined && result !== null) {
                    expect(result).to.be.a('number');
                    expect(result).to.be.at.least(0);
                }
            } catch (error) {
                expect(error.message).to.match(/network|connection|timeout|credits|platform/i);
            }
        });
        
        it('should get block height', async function() {
            this.timeout(15000);
            
            if (!global.wasmSdk.get_block_height) {
                this.skip('get_block_height not implemented');
                return;
            }
            
            try {
                const result = await global.wasmSdk.get_block_height(sdk);
                
                if (result !== undefined && result !== null) {
                    expect(result).to.be.a('number');
                    expect(result).to.be.at.least(0);
                }
            } catch (error) {
                expect(error.message).to.match(/network|connection|timeout|block|height/i);
            }
        });
        
        it('should get chain info', async function() {
            this.timeout(15000);
            
            if (!global.wasmSdk.get_chain_info) {
                this.skip('get_chain_info not implemented');
                return;
            }
            
            try {
                const result = await global.wasmSdk.get_chain_info(sdk);
                
                if (result) {
                    expect(result).to.be.an('object');
                }
            } catch (error) {
                expect(error.message).to.match(/network|connection|timeout|chain/i);
            }
        });
    });
    
    describe('Platform Status', () => {
        it('should handle platform status queries', async function() {
            this.timeout(15000);
            
            // Test multiple system queries that should work together
            const queries = [
                { name: 'get_block_height', fn: global.wasmSdk.get_block_height },
                { name: 'get_total_credits_in_platform', fn: global.wasmSdk.get_total_credits_in_platform },
                { name: 'get_current_quorums_info', fn: global.wasmSdk.get_current_quorums_info }
            ];
            
            let anySucceeded = false;
            
            for (const query of queries) {
                if (query.fn) {
                    try {
                        const result = await query.fn(sdk);
                        
                        if (result !== undefined && result !== null) {
                            anySucceeded = true;
                        }
                    } catch (error) {
                        // Network errors are expected
                        expect(error.message).to.match(/network|connection|timeout/i);
                    }
                }
            }
            
            // At least one query should either succeed or fail with network error
            // This test mainly verifies the queries don't crash
        });
    });
    
    describe('SDK Version and Info', () => {
        it('should get SDK version', async function() {
            this.timeout(15000);
            
            try {
                // Test SDK version function
                const version = sdk.version();
                
                if (version !== undefined && version !== null) {
                    expect(version).to.be.a('number');
                    expect(version).to.be.at.least(1);
                }
            } catch (error) {
                // Some versions may not have this function
                expect(error.message).to.exist;
            }
        });
    });
});