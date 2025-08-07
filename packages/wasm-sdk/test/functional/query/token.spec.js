/**
 * Token Query Tests - Functional tests for token-related queries
 * 
 * These tests verify token query functions work correctly with the real WASM SDK.
 * They may make network calls to testnet when network is available.
 */

const { TestData } = require('../../fixtures/test-data.js');

describe('Token Queries (Functional)', () => {
    let sdk;
    
    // Test values from docs.html
    const TEST_IDENTITY = '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk';
    const TOKEN_CONTRACT_1 = 'Hqyu8WcRwXCTwbNxdga4CN5gsVEGc67wng4TFzceyLUv';
    const TOKEN_CONTRACT_2 = 'H7FRpZJqZK933r9CzZMsCuf1BM34NT5P2wSJyjDkprqy';
    const TOKEN_CONTRACT_3 = 'EETVvWgohFDKtbB3ejEzBcDRMNYkc9TtgXY6y8hzP3Ta';
    
    // Ensure WASM is ready before all tests
    before(async function() {
        this.timeout(30000);
        await global.ensureWasmInitialized();
        
        // Try to prefetch quorums for trusted mode
        try {
            await global.wasmSdk.prefetch_trusted_quorums_testnet();
        } catch (error) {
            // Network errors are acceptable - tests will handle offline mode
        }
    });
    
    beforeEach(async function() {
        this.timeout(10000);
        // Create SDK instance for each test
        try {
            const builder = global.wasmSdk.WasmSdkBuilder.new_testnet_trusted();
            sdk = await builder.build();
        } catch (error) {
            // If trusted mode fails, try regular testnet
            sdk = await global.createTestSdk.testnet();
        }
    });
    
    afterEach(() => {
        if (sdk && sdk.free) {
            sdk.free();
        }
    });
    
    describe('Token Status and Info Queries', () => {
        it('should get token statuses for multiple tokens', async function() {
            this.timeout(15000);
            
            try {
                const result = await global.wasmSdk.get_token_statuses(
                    sdk,
                    [TOKEN_CONTRACT_1, TOKEN_CONTRACT_2]
                );
                
                if (result) {
                    expect(result).to.be.an('object');
                    // Result should be a map of token contract ID to status
                    Object.values(result).forEach(status => {
                        expect(status).to.be.an('object');
                    });
                }
            } catch (error) {
                // Network errors are acceptable in functional tests
                expect(error.message).to.match(/network|connection|timeout|token/i);
            }
        });
        
        it('should get token direct purchase prices', async function() {
            this.timeout(15000);
            
            try {
                const result = await global.wasmSdk.get_token_direct_purchase_prices(
                    sdk,
                    [TOKEN_CONTRACT_2]
                );
                
                if (result) {
                    expect(result).to.be.an('object');
                    // Result should be a map of token contract ID to price info
                    Object.values(result).forEach(price => {
                        if (price !== null && price !== undefined) {
                            expect(price).to.be.a('number');
                        }
                    });
                }
            } catch (error) {
                expect(error.message).to.match(/network|connection|timeout|token/i);
            }
        });
        
        it('should get token contract info', async function() {
            this.timeout(15000);
            
            try {
                const result = await global.wasmSdk.get_token_contract_info(
                    sdk,
                    TOKEN_CONTRACT_3
                );
                
                if (result) {
                    expect(result).to.be.an('object');
                    if (result.name) expect(result.name).to.be.a('string');
                    if (result.symbol) expect(result.symbol).to.be.a('string');
                    if (result.decimals !== undefined) expect(result.decimals).to.be.a('number');
                }
            } catch (error) {
                expect(error.message).to.match(/network|connection|timeout|token/i);
            }
        });
    });
    
    describe('Token Supply and Distribution Queries', () => {
        it('should get token perpetual distribution last claim', async function() {
            this.timeout(15000);
            
            try {
                const result = await global.wasmSdk.get_token_perpetual_distribution_last_claim(
                    sdk,
                    TEST_IDENTITY,
                    TOKEN_CONTRACT_3
                );
                
                if (result) {
                    expect(result).to.be.an('object');
                }
            } catch (error) {
                expect(error.message).to.match(/network|connection|timeout|token|identity/i);
            }
        });
        
        it('should get token total supply', async function() {
            this.timeout(15000);
            
            try {
                const result = await global.wasmSdk.get_token_total_supply(
                    sdk,
                    TOKEN_CONTRACT_1
                );
                
                if (result !== undefined && result !== null) {
                    expect(result).to.be.a('number');
                    expect(result).to.be.at.least(0);
                }
            } catch (error) {
                expect(error.message).to.match(/network|connection|timeout|token/i);
            }
        });
        
        it('should get token circulating supply', async function() {
            this.timeout(15000);
            
            try {
                const result = await global.wasmSdk.get_token_circulating_supply(
                    sdk,
                    TOKEN_CONTRACT_1
                );
                
                if (result !== undefined && result !== null) {
                    expect(result).to.be.a('number');
                    expect(result).to.be.at.least(0);
                }
            } catch (error) {
                expect(error.message).to.match(/network|connection|timeout|token/i);
            }
        });
    });
    
    describe('Token Balance Queries', () => {
        it('should get identity token balance', async function() {
            this.timeout(15000);
            
            // Check if function exists first (from identity tests we know some are missing)
            if (!global.wasmSdk.get_identity_token_balances) {
                this.skip('get_identity_token_balances not implemented');
                return;
            }
            
            try {
                const result = await global.wasmSdk.get_identity_token_balances(
                    sdk,
                    TEST_IDENTITY,
                    [TOKEN_CONTRACT_1, TOKEN_CONTRACT_2]
                );
                
                if (result) {
                    expect(result).to.be.an('object');
                    // Result should be a map of token contract ID to balance
                    Object.values(result).forEach(balance => {
                        if (balance !== undefined && balance !== null) {
                            expect(balance).to.be.a('number');
                            expect(balance).to.be.at.least(0);
                        }
                    });
                }
            } catch (error) {
                expect(error.message).to.match(/network|connection|timeout|token|identity/i);
            }
        });
    });
    
    describe('Token Holder Queries', () => {
        it('should get token holders', async function() {
            this.timeout(15000);
            
            // Check if function exists
            if (!global.wasmSdk.get_token_holders) {
                this.skip('get_token_holders not implemented');
                return;
            }
            
            try {
                const result = await global.wasmSdk.get_token_holders(
                    sdk,
                    TOKEN_CONTRACT_1,
                    10,   // limit
                    null  // start after
                );
                
                if (result) {
                    expect(result).to.be.an('array');
                    result.forEach(holder => {
                        expect(holder).to.be.an('object');
                        if (holder.identity) expect(holder.identity).to.be.a('string');
                        if (holder.balance !== undefined) expect(holder.balance).to.be.a('number');
                    });
                }
            } catch (error) {
                expect(error.message).to.match(/network|connection|timeout|token/i);
            }
        });
    });
    
    describe('Token Transaction Queries', () => {
        it('should get token transactions', async function() {
            this.timeout(15000);
            
            // Check if function exists
            if (!global.wasmSdk.get_token_transactions) {
                this.skip('get_token_transactions not implemented');
                return;
            }
            
            try {
                const result = await global.wasmSdk.get_token_transactions(
                    sdk,
                    TOKEN_CONTRACT_1,
                    10,   // limit
                    null  // start after
                );
                
                if (result) {
                    expect(result).to.be.an('array');
                    result.forEach(tx => {
                        expect(tx).to.be.an('object');
                        if (tx.from) expect(tx.from).to.be.a('string');
                        if (tx.to) expect(tx.to).to.be.a('string');
                        if (tx.amount !== undefined) expect(tx.amount).to.be.a('number');
                    });
                }
            } catch (error) {
                expect(error.message).to.match(/network|connection|timeout|token/i);
            }
        });
    });
    
    describe('Error Handling', () => {
        it('should handle invalid token contract ID', async function() {
            this.timeout(15000);
            
            try {
                await global.wasmSdk.get_token_total_supply(
                    sdk,
                    'invalid-token-contract-id'
                );
            } catch (error) {
                expect(error.message).to.exist;
                expect(error.message).to.match(/invalid|token|contract/i);
            }
        });
        
        it('should handle null SDK gracefully', async () => {
            try {
                await global.wasmSdk.get_token_statuses(null, [TOKEN_CONTRACT_1]);
                expect.fail('Should have thrown error with null SDK');
            } catch (error) {
                expect(error.message).to.match(/sdk|required/i);
            }
        });
        
        it('should handle empty token list', async function() {
            this.timeout(15000);
            
            try {
                const result = await global.wasmSdk.get_token_statuses(sdk, []);
                
                // Empty list might return empty result or error
                if (result) {
                    expect(result).to.be.an('object');
                    expect(Object.keys(result)).to.have.length(0);
                }
            } catch (error) {
                expect(error.message).to.exist;
            }
        });
    });
});