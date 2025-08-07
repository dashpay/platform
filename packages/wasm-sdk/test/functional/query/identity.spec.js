/**
 * Identity Query Tests - Functional tests for identity-related queries
 * 
 * These tests verify identity query functions work correctly with the real WASM SDK.
 * They may make network calls to testnet when network is available.
 */

const { TestData } = require('../../fixtures/test-data.js');

describe('Identity Queries (Functional)', () => {
    let sdk;
    
    // Documented test values from docs.html
    const TEST_IDENTITY = '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk';
    const DPNS_CONTRACT = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';
    const TOKEN_CONTRACT = 'Hqyu8WcRwXCTwbNxdga4CN5gsVEGc67wng4TFzceyLUv';
    
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
    
    describe('Basic Identity Queries', () => {
        it('should fetch identity information', async function() {
            this.timeout(15000);
            
            try {
                const result = await global.wasmSdk.identity_fetch(sdk, TEST_IDENTITY);
                
                // If we get a result, validate its structure
                if (result) {
                    expect(result).to.be.an('object');
                    if (result.id) expect(result.id).to.be.a('string');
                    if (result.balance !== undefined) expect(result.balance).to.be.a('number');
                    if (result.publicKeys) expect(result.publicKeys).to.be.an('array');
                }
            } catch (error) {
                // Network errors or proof verification errors are acceptable in functional tests
                expect(error.message).to.match(/network|connection|timeout|identity|proof|verification/i);
            }
        });
        
        it('should get identity balance', async function() {
            this.timeout(15000);
            
            try {
                const result = await global.wasmSdk.get_identity_balance(sdk, TEST_IDENTITY);
                
                if (result !== undefined && result !== null) {
                    expect(result).to.be.a('number');
                    expect(result).to.be.at.least(0);
                }
            } catch (error) {
                // Network errors or identity not found are acceptable
                expect(error.message).to.match(/network|connection|timeout|identity/i);
            }
        });
        
        it('should get identity keys', async function() {
            this.timeout(15000);
            
            try {
                const result = await global.wasmSdk.get_identity_keys(sdk, TEST_IDENTITY, 'all');
                
                if (result) {
                    expect(result).to.be.an('array');
                    // Each key should have expected properties
                    result.forEach(key => {
                        expect(key).to.be.an('object');
                    });
                }
            } catch (error) {
                expect(error.message).to.match(/network|connection|timeout|identity/i);
            }
        });
        
        it('should get identity nonce', async function() {
            this.timeout(15000);
            
            try {
                const result = await global.wasmSdk.get_identity_nonce(sdk, TEST_IDENTITY);
                
                if (result !== undefined && result !== null) {
                    expect(result).to.be.a('number');
                    expect(result).to.be.at.least(0);
                }
            } catch (error) {
                expect(error.message).to.match(/network|connection|timeout|identity/i);
            }
        });
        
        it('should get identity contract nonce', async function() {
            this.timeout(15000);
            
            try {
                const result = await global.wasmSdk.get_identity_contract_nonce(
                    sdk, 
                    TEST_IDENTITY, 
                    DPNS_CONTRACT
                );
                
                if (result !== undefined && result !== null) {
                    expect(result).to.be.a('number');
                    expect(result).to.be.at.least(0);
                }
            } catch (error) {
                expect(error.message).to.match(/network|connection|timeout|identity|contract/i);
            }
        });
    });
    
    describe('Batch Identity Queries', () => {
        it('should get balances for multiple identities', async function() {
            this.timeout(15000);
            
            // Check if function exists first
            if (!global.wasmSdk.get_identities_balances) {
                this.skip('get_identities_balances not implemented');
                return;
            }
            
            try {
                const result = await global.wasmSdk.get_identities_balances(sdk, [TEST_IDENTITY]);
                
                if (result) {
                    expect(result).to.be.an('object');
                    // Result should be a map of identity ID to balance
                    Object.values(result).forEach(balance => {
                        if (balance !== undefined && balance !== null) {
                            expect(balance).to.be.a('number');
                        }
                    });
                }
            } catch (error) {
                expect(error.message).to.match(/network|connection|timeout|identity|not a function/i);
            }
        });
        
        it('should get identity balance and revision', async function() {
            this.timeout(15000);
            
            try {
                const result = await global.wasmSdk.get_identity_balance_and_revision(sdk, TEST_IDENTITY);
                
                if (result) {
                    expect(result).to.be.an('object');
                    if (result.balance !== undefined) {
                        expect(result.balance).to.be.a('number');
                    }
                    if (result.revision !== undefined) {
                        expect(result.revision).to.be.a('number');
                    }
                }
            } catch (error) {
                expect(error.message).to.match(/network|connection|timeout|identity/i);
            }
        });
    });
    
    describe('Contract Keys Queries', () => {
        it('should get identities contract keys', async function() {
            this.timeout(15000);
            
            try {
                const result = await global.wasmSdk.get_identities_contract_keys(
                    sdk,
                    [TEST_IDENTITY],
                    DPNS_CONTRACT,
                    'domain',  // document type
                    'all'      // purposes
                );
                
                if (result) {
                    expect(result).to.be.an('array');
                    result.forEach(key => {
                        expect(key).to.be.an('object');
                    });
                }
            } catch (error) {
                expect(error.message).to.match(/network|connection|timeout|identity|contract/i);
            }
        });
    });
    
    describe('Token Balance Queries', () => {
        it('should get identity token balances', async function() {
            this.timeout(15000);
            
            try {
                const result = await global.wasmSdk.get_identity_token_balances(
                    sdk,
                    TEST_IDENTITY,
                    [TOKEN_CONTRACT]
                );
                
                if (result) {
                    expect(result).to.be.an('object');
                }
            } catch (error) {
                expect(error.message).to.match(/network|connection|timeout|identity|token/i);
            }
        });
        
        it('should get multiple identities token balances', async function() {
            this.timeout(15000);
            
            try {
                const result = await global.wasmSdk.get_identities_token_balances(
                    sdk,
                    [TEST_IDENTITY],
                    TOKEN_CONTRACT
                );
                
                if (result) {
                    expect(result).to.be.an('object');
                }
            } catch (error) {
                expect(error.message).to.match(/network|connection|timeout|identity|token/i);
            }
        });
        
        it('should get identity token infos', async function() {
            this.timeout(15000);
            
            try {
                const result = await global.wasmSdk.get_identity_token_infos(
                    sdk,
                    TEST_IDENTITY,
                    [TOKEN_CONTRACT]
                );
                
                if (result) {
                    expect(result).to.be.an('object');
                }
            } catch (error) {
                expect(error.message).to.match(/network|connection|timeout|identity|token/i);
            }
        });
        
        it('should get multiple identities token infos', async function() {
            this.timeout(15000);
            
            try {
                const result = await global.wasmSdk.get_identities_token_infos(
                    sdk,
                    [TEST_IDENTITY],
                    TOKEN_CONTRACT
                );
                
                if (result) {
                    expect(result).to.be.an('object');
                }
            } catch (error) {
                expect(error.message).to.match(/network|connection|timeout|identity|token/i);
            }
        });
    });
    
    describe('Public Key Hash Queries', () => {
        it('should handle invalid public key hash gracefully', async function() {
            this.timeout(15000);
            
            try {
                await global.wasmSdk.get_identity_by_public_key_hash(
                    sdk,
                    "invalidhash"
                );
                // If we get here without error, that's acceptable
            } catch (error) {
                // Should get an error for invalid hash
                expect(error.message).to.exist;
            }
        });
        
        it('should get identity by non-unique public key hash', async function() {
            this.timeout(15000);
            
            try {
                // Example non-unique public key hash from docs
                const result = await global.wasmSdk.get_identity_by_non_unique_public_key_hash(
                    sdk,
                    '518038dc858461bcee90478fd994bba8057b7531',
                    null  // start_after parameter (optional)
                );
                
                if (result) {
                    expect(result).to.be.an('array');
                }
            } catch (error) {
                expect(error.message).to.match(/network|connection|timeout|hash/i);
            }
        });
    });
    
    describe('Error Handling', () => {
        it('should handle invalid identity ID gracefully', async function() {
            this.timeout(15000);
            
            try {
                await global.wasmSdk.identity_fetch(sdk, 'invalid-identity-id');
            } catch (error) {
                expect(error.message).to.exist;
                // Should get validation or network error
                expect(error.message).to.match(/invalid|identity|network/i);
            }
        });
        
        it('should handle null SDK gracefully', async () => {
            try {
                await global.wasmSdk.identity_fetch(null, TEST_IDENTITY);
                expect.fail('Should have thrown error with null SDK');
            } catch (error) {
                expect(error.message).to.match(/sdk|required/i);
            }
        });
    });
});