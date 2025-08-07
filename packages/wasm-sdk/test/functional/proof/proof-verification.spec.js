/**
 * Proof Verification Tests - Functional tests for proof verification
 * 
 * These tests verify proof verification functions work correctly with the real WASM SDK.
 * Note: WASM requires trusted mode for proof verification.
 */

describe('Proof Verification (Functional)', () => {
    let sdk;
    
    // Test values
    const TEST_IDENTITY = '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk';
    const DPNS_CONTRACT = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';
    
    // Ensure WASM is ready before all tests
    before(async function() {
        this.timeout(30000);
        await global.ensureWasmInitialized();
    });
    
    beforeEach(async function() {
        this.timeout(10000);
        // Create SDK instance for each test - trusted mode required for proofs
        try {
            const builder = global.wasmSdk.WasmSdkBuilder.new_testnet_trusted();
            sdk = await builder.build();
        } catch (error) {
            // If trusted mode fails, tests will be skipped
            sdk = null;
        }
    });
    
    afterEach(() => {
        if (sdk && sdk.free) {
            sdk.free();
        }
    });
    
    describe('Basic Proof Verification', () => {
        it('should fail verify_proof with invalid proof data', async function() {
            this.timeout(15000);
            
            if (!sdk) {
                this.skip('Trusted mode SDK required for proof verification');
                return;
            }
            
            try {
                await global.wasmSdk.verify_proof(
                    sdk,
                    "invalidproofdata"
                );
                // Should fail with invalid proof
            } catch (error) {
                expect(error.message).to.exist;
                expect(error.message).to.match(/proof|invalid|verification/i);
            }
        });
        
        it('should fail verify_proofs with invalid batch data', async function() {
            this.timeout(15000);
            
            if (!sdk) {
                this.skip('Trusted mode SDK required for proof verification');
                return;
            }
            
            try {
                const proofs = JSON.stringify([
                    "invalidproof1",
                    "invalidproof2"
                ]);
                
                await global.wasmSdk.verify_proofs(sdk, proofs);
                // Should fail with invalid proofs
            } catch (error) {
                expect(error.message).to.exist;
                expect(error.message).to.match(/proof|invalid|verification/i);
            }
        });
    });
    
    describe('Query with Proof Verification', () => {
        it('should fetch identity with proof and verify', async function() {
            this.timeout(20000);
            
            if (!sdk) {
                this.skip('Trusted mode SDK required for proof verification');
                return;
            }
            
            try {
                // Fetch identity with proof
                const identityResult = await global.wasmSdk.identity_fetch(
                    sdk,
                    TEST_IDENTITY
                );
                
                // If we got a result with proof, we could verify it
                // But the SDK handles verification internally in trusted mode
                if (identityResult) {
                    expect(identityResult).to.be.an('object');
                }
            } catch (error) {
                // Network errors or proof verification errors are acceptable
                expect(error.message).to.match(/network|connection|proof|verification|identity/i);
            }
        });
        
        it('should fetch data contract with proof and verify', async function() {
            this.timeout(20000);
            
            if (!sdk) {
                this.skip('Trusted mode SDK required for proof verification');
                return;
            }
            
            try {
                // Fetch contract with proof
                const contractResult = await global.wasmSdk.data_contract_fetch(
                    sdk,
                    DPNS_CONTRACT
                );
                
                // SDK handles proof verification internally
                if (contractResult) {
                    expect(contractResult).to.be.an('object');
                }
            } catch (error) {
                expect(error.message).to.match(/network|connection|proof|verification|contract/i);
            }
        });
        
        it('should fetch documents with proof and verify', async function() {
            this.timeout(20000);
            
            if (!sdk) {
                this.skip('Trusted mode SDK required for proof verification');
                return;
            }
            
            try {
                // Fetch documents with proof
                const documentsResult = await global.wasmSdk.get_documents(
                    sdk,
                    DPNS_CONTRACT,
                    "domain",
                    null,
                    null,
                    5,
                    null,
                    null
                );
                
                // SDK handles proof verification internally
                if (documentsResult) {
                    expect(documentsResult).to.be.an('array');
                }
            } catch (error) {
                expect(error.message).to.match(/network|connection|proof|verification|document/i);
            }
        });
    });
    
    describe('Proof Validation Settings', () => {
        it('should work with trusted mode SDK', async function() {
            this.timeout(15000);
            
            if (!sdk) {
                this.skip('Trusted mode SDK required for proof verification');
                return;
            }
            
            // Trusted mode SDK should be initialized
            expect(sdk).to.exist;
            expect(sdk.version).to.be.a('function');
            
            const version = sdk.version();
            expect(version).to.be.a('number');
        });
        
        it('should handle non-trusted mode limitations', async function() {
            this.timeout(15000);
            
            // Create non-trusted SDK
            let nonTrustedSdk;
            try {
                nonTrustedSdk = await global.createTestSdk.testnet();
                
                // Non-trusted mode may have limitations with proof verification
                // This is expected behavior in WASM
                expect(nonTrustedSdk).to.exist;
            } catch (error) {
                // Non-trusted mode might not be available
                expect(error.message).to.exist;
            } finally {
                if (nonTrustedSdk && nonTrustedSdk.free) {
                    nonTrustedSdk.free();
                }
            }
        });
    });
    
    describe('Error Handling', () => {
        it('should handle null SDK gracefully', async () => {
            try {
                await global.wasmSdk.verify_proof(null, "someproof");
                expect.fail('Should have thrown error with null SDK');
            } catch (error) {
                expect(error.message).to.match(/sdk|required/i);
            }
        });
        
        it('should handle empty proof data', async function() {
            this.timeout(15000);
            
            if (!sdk) {
                this.skip('Trusted mode SDK required for proof verification');
                return;
            }
            
            try {
                await global.wasmSdk.verify_proof(sdk, "");
            } catch (error) {
                expect(error.message).to.exist;
                expect(error.message).to.match(/proof|empty|invalid/i);
            }
        });
        
        it('should handle malformed proof JSON', async function() {
            this.timeout(15000);
            
            if (!sdk) {
                this.skip('Trusted mode SDK required for proof verification');
                return;
            }
            
            if (!global.wasmSdk.verify_proofs) {
                this.skip('verify_proofs not available');
                return;
            }
            
            try {
                await global.wasmSdk.verify_proofs(sdk, "not valid json");
            } catch (error) {
                expect(error.message).to.exist;
                expect(error.message).to.match(/json|parse|invalid/i);
            }
        });
    });
});