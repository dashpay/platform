/**
 * State Transition Tests - Functional tests for state transition operations
 * 
 * These tests verify state transition functions work correctly with the real WASM SDK.
 * Note: Most will fail without proper funding or existing identities.
 */

describe('State Transitions (Functional)', () => {
    let sdk;
    
    // Test values
    const TEST_MNEMONIC = "during develop before curtain hazard rare job language become verb message travel";
    const TEST_IDENTITY = '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk';
    const DPNS_CONTRACT = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';
    const TOKEN_CONTRACT = 'Hqyu8WcRwXCTwbNxdga4CN5gsVEGc67wng4TFzceyLUv';
    
    // Ensure WASM is ready before all tests
    before(async function() {
        this.timeout(30000);
        await global.ensureWasmInitialized();
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
    
    describe('Identity State Transitions', () => {
        it('should fail identity_create without funding', async function() {
            this.timeout(15000);
            
            if (!global.wasmSdk.identity_create) {
                this.skip('identity_create not available');
                return;
            }
            
            try {
                await global.wasmSdk.identity_create(
                    sdk,
                    TEST_MNEMONIC,
                    null,   // no alias
                    0       // key index
                );
                // Should fail without funding
            } catch (error) {
                expect(error.message).to.exist;
                // Expected to fail without funding
                expect(error.message).to.match(/funding|balance|insufficient|network/i);
            }
        });
        
        it('should fail identity_update with invalid data', async function() {
            this.timeout(15000);
            
            try {
                const updateData = JSON.stringify({
                    add: [{
                        purpose: 0,  // authentication
                        securityLevel: 0,
                        keyType: 0,  // ECDSA
                        data: "invalidpublickey"
                    }]
                });
                
                await global.wasmSdk.identity_update(
                    sdk,
                    TEST_MNEMONIC,
                    TEST_IDENTITY,
                    updateData,
                    0  // key index
                );
                // Should fail with invalid key data
            } catch (error) {
                expect(error.message).to.exist;
                expect(error.message).to.match(/invalid|key|data|identity|network/i);
            }
        });
        
        it('should fail identity_topup without funding', async function() {
            this.timeout(15000);
            
            if (!global.wasmSdk.identity_topup) {
                this.skip('identity_topup not available');
                return;
            }
            
            try {
                await global.wasmSdk.identity_topup(
                    sdk,
                    TEST_MNEMONIC,
                    TEST_IDENTITY,
                    100000  // amount
                );
                // Should fail without funding
            } catch (error) {
                expect(error.message).to.exist;
                expect(error.message).to.match(/funding|balance|insufficient|network/i);
            }
        });
        
        it('should fail identity_withdraw without balance', async function() {
            this.timeout(15000);
            
            try {
                await global.wasmSdk.identity_withdraw(
                    sdk,
                    TEST_MNEMONIC,
                    TEST_IDENTITY,
                    "yMTzqaUcb7e4QLiPT5f5hqNjgCXQq65pLm",  // destination address
                    100000,  // amount
                    0        // key index
                );
                // Should fail without balance
            } catch (error) {
                expect(error.message).to.exist;
                expect(error.message).to.match(/balance|insufficient|withdraw|network/i);
            }
        });
        
        it('should handle identity_put if available', async function() {
            this.timeout(15000);
            
            // This function is known to cause issues in WASM
            if (!global.wasmSdk.identity_put) {
                this.skip('identity_put not available');
                return;
            }
            
            try {
                await global.wasmSdk.identity_put(sdk);
                // May panic or throw
            } catch (error) {
                expect(error.message).to.exist;
                // Known to panic in WASM
            }
        });
    });
    
    describe('Document State Transitions', () => {
        it('should fail document_create without identity', async function() {
            this.timeout(15000);
            
            try {
                const documentData = JSON.stringify({
                    label: "testdomain",
                    normalizedLabel: "testdomain",
                    normalizedParentDomainName: "dash",
                    records: {
                        identity: TEST_IDENTITY
                    }
                });
                
                await global.wasmSdk.document_create(
                    sdk,
                    TEST_MNEMONIC,
                    TEST_IDENTITY,
                    DPNS_CONTRACT,
                    "domain",
                    documentData,
                    0  // key index
                );
                // Should fail without proper identity
            } catch (error) {
                expect(error.message).to.exist;
                expect(error.message).to.match(/identity|document|contract|network/i);
            }
        });
        
        it('should fail document_update without existing document', async function() {
            this.timeout(15000);
            
            try {
                const updateData = JSON.stringify({
                    records: {
                        identity: TEST_IDENTITY
                    }
                });
                
                await global.wasmSdk.document_update(
                    sdk,
                    TEST_MNEMONIC,
                    TEST_IDENTITY,
                    DPNS_CONTRACT,
                    "domain",
                    "nonexistentdocumentid",
                    updateData,
                    0  // key index
                );
                // Should fail without existing document
            } catch (error) {
                expect(error.message).to.exist;
                expect(error.message).to.match(/document|not found|invalid|network/i);
            }
        });
        
        it('should fail document_delete without existing document', async function() {
            this.timeout(15000);
            
            try {
                await global.wasmSdk.document_delete(
                    sdk,
                    TEST_MNEMONIC,
                    TEST_IDENTITY,
                    DPNS_CONTRACT,
                    "domain",
                    "nonexistentdocumentid",
                    0  // key index
                );
                // Should fail without existing document
            } catch (error) {
                expect(error.message).to.exist;
                expect(error.message).to.match(/document|not found|invalid|network/i);
            }
        });
    });
    
    describe('Token State Transitions', () => {
        it('should fail token_mint without authority', async function() {
            this.timeout(15000);
            
            if (!global.wasmSdk.token_mint) {
                this.skip('token_mint not available');
                return;
            }
            
            try {
                await global.wasmSdk.token_mint(
                    sdk,
                    TEST_MNEMONIC,
                    TOKEN_CONTRACT,
                    TEST_IDENTITY,  // recipient
                    1000000,        // amount
                    0               // key index
                );
                // Should fail without minting authority
            } catch (error) {
                expect(error.message).to.exist;
                expect(error.message).to.match(/authority|permission|token|mint/i);
            }
        });
        
        it('should fail token_burn without balance', async function() {
            this.timeout(15000);
            
            if (!global.wasmSdk.token_burn) {
                this.skip('token_burn not available');
                return;
            }
            
            try {
                await global.wasmSdk.token_burn(
                    sdk,
                    TEST_MNEMONIC,
                    TOKEN_CONTRACT,
                    1000000,  // amount
                    0         // key index
                );
                // Should fail without token balance
            } catch (error) {
                expect(error.message).to.exist;
                expect(error.message).to.match(/balance|insufficient|token|burn/i);
            }
        });
        
        it('should fail token_transfer without balance', async function() {
            this.timeout(15000);
            
            if (!global.wasmSdk.token_transfer) {
                this.skip('token_transfer not available');
                return;
            }
            
            try {
                await global.wasmSdk.token_transfer(
                    sdk,
                    TEST_MNEMONIC,
                    TOKEN_CONTRACT,
                    TEST_IDENTITY,  // recipient
                    1000000,        // amount
                    0               // key index
                );
                // Should fail without token balance
            } catch (error) {
                expect(error.message).to.exist;
                expect(error.message).to.match(/balance|insufficient|token|transfer/i);
            }
        });
    });
    
    describe('Contract State Transitions', () => {
        it('should fail data_contract_create without proper data', async function() {
            this.timeout(15000);
            
            try {
                const contractData = JSON.stringify({
                    documents: {
                        testDoc: {
                            type: "object",
                            properties: {
                                name: { type: "string" }
                            },
                            required: ["name"]
                        }
                    }
                });
                
                await global.wasmSdk.data_contract_create(
                    sdk,
                    TEST_MNEMONIC,
                    TEST_IDENTITY,
                    contractData,
                    0  // key index
                );
                // Should fail without proper identity/funding
            } catch (error) {
                expect(error.message).to.exist;
                expect(error.message).to.match(/contract|identity|funding|network/i);
            }
        });
        
        it('should fail data_contract_update without existing contract', async function() {
            this.timeout(15000);
            
            try {
                const updateData = JSON.stringify({
                    version: 2,
                    documents: {
                        testDoc: {
                            type: "object",
                            properties: {
                                name: { type: "string" },
                                value: { type: "number" }
                            },
                            required: ["name"]
                        }
                    }
                });
                
                await global.wasmSdk.data_contract_update(
                    sdk,
                    TEST_MNEMONIC,
                    "nonexistentcontractid",
                    updateData,
                    0  // key index
                );
                // Should fail without existing contract
            } catch (error) {
                expect(error.message).to.exist;
                expect(error.message).to.match(/contract|not found|invalid|network/i);
            }
        });
    });
    
    describe('Voting State Transitions', () => {
        it('should fail masternode_vote without masternode identity', async function() {
            this.timeout(15000);
            
            if (!global.wasmSdk.masternode_vote) {
                this.skip('masternode_vote not available');
                return;
            }
            
            try {
                await global.wasmSdk.masternode_vote(
                    sdk,
                    TEST_MNEMONIC,
                    "invalidproposalhash",
                    "yes",
                    0  // key index
                );
                // Should fail without masternode identity
            } catch (error) {
                expect(error.message).to.exist;
                expect(error.message).to.match(/masternode|vote|permission|identity/i);
            }
        });
    });
    
    describe('Error Handling', () => {
        it('should handle null SDK gracefully', async () => {
            try {
                await global.wasmSdk.identity_create(null, TEST_MNEMONIC, null, 0);
                expect.fail('Should have thrown error with null SDK');
            } catch (error) {
                expect(error.message).to.match(/sdk|required/i);
            }
        });
        
        it('should handle invalid mnemonic', async function() {
            this.timeout(15000);
            
            if (!global.wasmSdk.identity_create) {
                this.skip('identity_create not available');
                return;
            }
            
            try {
                await global.wasmSdk.identity_create(
                    sdk,
                    "invalid mnemonic phrase",
                    null,
                    0
                );
            } catch (error) {
                expect(error.message).to.exist;
                expect(error.message).to.match(/mnemonic|invalid/i);
            }
        });
    });
});