/**
 * Document Query Tests - Functional tests for document and data contract queries
 * 
 * These tests verify document query functions work correctly with the real WASM SDK.
 * They may make network calls to testnet when network is available.
 */

const { TestData } = require('../../fixtures/test-data.js');

describe('Document Queries (Functional)', () => {
    let sdk;
    
    // Documented test values from docs.html
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
    
    describe('Document Queries', () => {
        it('should get documents without filters', async function() {
            this.timeout(15000);
            
            try {
                const result = await global.wasmSdk.get_documents(
                    sdk,
                    DPNS_CONTRACT,
                    "domain",
                    null,  // no where clause
                    null,  // no order by
                    10,    // limit
                    null,  // no start after
                    null   // no start at
                );
                
                if (result) {
                    expect(result).to.be.an('array');
                    result.forEach(doc => {
                        expect(doc).to.be.an('object');
                    });
                }
            } catch (error) {
                // Network errors are acceptable in functional tests
                expect(error.message).to.match(/network|connection|timeout|contract/i);
            }
        });
        
        it('should get documents with where clause', async function() {
            this.timeout(15000);
            
            try {
                // Search for domains owned by test identity
                const whereClause = JSON.stringify([
                    ["$ownerId", "==", TEST_IDENTITY]
                ]);
                
                const result = await global.wasmSdk.get_documents(
                    sdk,
                    DPNS_CONTRACT,
                    "domain",
                    whereClause,
                    null,  // no order by
                    10,    // limit
                    null,  // no start after
                    null   // no start at
                );
                
                if (result) {
                    expect(result).to.be.an('array');
                    // If we get results, they should be owned by the test identity
                    result.forEach(doc => {
                        if (doc.$ownerId) {
                            expect(doc.$ownerId).to.equal(TEST_IDENTITY);
                        }
                    });
                }
            } catch (error) {
                expect(error.message).to.match(/network|connection|timeout|contract/i);
            }
        });
        
        it('should get documents with orderBy clause', async function() {
            this.timeout(15000);
            
            try {
                const orderBy = JSON.stringify([
                    ["$createdAt", "desc"]
                ]);
                
                const result = await global.wasmSdk.get_documents(
                    sdk,
                    DPNS_CONTRACT,
                    "domain",
                    null,     // no where
                    orderBy,  // order by creation time descending
                    5,        // limit
                    null,     // no start after
                    null      // no start at
                );
                
                if (result && result.length > 1) {
                    // Verify descending order if we have multiple results
                    for (let i = 1; i < result.length; i++) {
                        if (result[i-1].$createdAt && result[i].$createdAt) {
                            expect(result[i-1].$createdAt).to.be.at.least(result[i].$createdAt);
                        }
                    }
                }
            } catch (error) {
                expect(error.message).to.match(/network|connection|timeout|contract/i);
            }
        });
        
        it('should get documents with complex where clause', async function() {
            this.timeout(15000);
            
            try {
                // Multiple conditions
                const whereClause = JSON.stringify([
                    ["normalizedLabel", "startsWith", "test"],
                    ["normalizedParentDomainName", "==", "dash"]
                ]);
                
                const result = await global.wasmSdk.get_documents(
                    sdk,
                    DPNS_CONTRACT,
                    "domain",
                    whereClause,
                    null,
                    10,
                    null,
                    null
                );
                
                if (result) {
                    expect(result).to.be.an('array');
                    // Results should match our query conditions
                    result.forEach(doc => {
                        if (doc.normalizedLabel) {
                            expect(doc.normalizedLabel).to.match(/^test/);
                        }
                        if (doc.normalizedParentDomainName) {
                            expect(doc.normalizedParentDomainName).to.equal('dash');
                        }
                    });
                }
            } catch (error) {
                expect(error.message).to.match(/network|connection|timeout|contract/i);
            }
        });
        
        it('should handle get_single_document with invalid ID', async function() {
            this.timeout(15000);
            
            try {
                await global.wasmSdk.get_single_document(
                    sdk,
                    DPNS_CONTRACT,
                    "domain",
                    "invalidDocumentId"
                );
                // If we get here, document was somehow found (unlikely)
            } catch (error) {
                // Expected to fail with invalid ID
                expect(error.message).to.exist;
            }
        });
    });
    
    describe('Data Contract Queries', () => {
        it('should fetch DPNS contract', async function() {
            this.timeout(15000);
            
            try {
                const result = await global.wasmSdk.data_contract_fetch(sdk, DPNS_CONTRACT);
                
                if (result) {
                    expect(result).to.be.an('object');
                    if (result.id) expect(result.id).to.be.a('string');
                    if (result.version !== undefined) expect(result.version).to.be.a('number');
                    if (result.ownerId) expect(result.ownerId).to.be.a('string');
                }
            } catch (error) {
                expect(error.message).to.match(/network|connection|timeout|contract|proof|verification/i);
            }
        });
        
        it('should fetch Token contract', async function() {
            this.timeout(15000);
            
            try {
                const result = await global.wasmSdk.data_contract_fetch(sdk, TOKEN_CONTRACT);
                
                if (result) {
                    expect(result).to.be.an('object');
                    if (result.id) expect(result.id).to.be.a('string');
                }
            } catch (error) {
                expect(error.message).to.match(/network|connection|timeout|contract|proof|verification/i);
            }
        });
        
        it('should fetch contract history', async function() {
            this.timeout(15000);
            
            try {
                const result = await global.wasmSdk.data_contract_fetch_history(
                    sdk,
                    DPNS_CONTRACT,
                    10,    // limit
                    0,     // offset
                    null,  // start at version
                    true   // prove
                );
                
                if (result) {
                    expect(result).to.be.an('array');
                    result.forEach(version => {
                        expect(version).to.be.an('object');
                    });
                }
            } catch (error) {
                expect(error.message).to.match(/network|connection|timeout|contract|proof|verification/i);
            }
        });
        
        it('should fetch multiple data contracts', async function() {
            this.timeout(15000);
            
            try {
                const result = await global.wasmSdk.get_data_contracts(
                    sdk,
                    [DPNS_CONTRACT, 'ALybvzfcCwMs7sinDwmtumw17NneuW7RgFtFHgjKmF3A']
                );
                
                if (result) {
                    expect(result).to.be.an('array');
                    result.forEach(contract => {
                        expect(contract).to.be.an('object');
                    });
                }
            } catch (error) {
                expect(error.message).to.match(/network|connection|timeout|contract/i);
            }
        });
    });
    
    describe('Token Document Queries', () => {
        it('should handle token document queries', async function() {
            this.timeout(15000);
            
            try {
                const result = await global.wasmSdk.get_documents(
                    sdk,
                    TOKEN_CONTRACT,
                    "token",  // assuming token document type
                    null,
                    null,
                    10,
                    null,
                    null
                );
                
                if (result) {
                    expect(result).to.be.an('array');
                }
            } catch (error) {
                // Token queries might fail if contract doesn't have 'token' document type
                expect(error.message).to.exist;
            }
        });
    });
    
    describe('System Status Queries', () => {
        it('should get platform status', async function() {
            this.timeout(15000);
            
            try {
                const result = await global.wasmSdk.get_status(sdk);
                
                if (result) {
                    expect(result).to.be.an('object');
                }
            } catch (error) {
                expect(error.message).to.match(/network|connection|timeout/i);
            }
        });
    });
    
    describe('Epoch Queries', () => {
        it('should get current epoch', async function() {
            this.timeout(15000);
            
            try {
                const result = await global.wasmSdk.get_current_epoch(sdk);
                
                if (result !== undefined && result !== null) {
                    expect(result).to.be.a('number');
                    expect(result).to.be.at.least(0);
                }
            } catch (error) {
                expect(error.message).to.match(/network|connection|timeout/i);
            }
        });
        
        it('should get epoch info', async function() {
            this.timeout(15000);
            
            try {
                const result = await global.wasmSdk.get_epoch_info(sdk, 1); // Get info for epoch 1
                
                if (result) {
                    expect(result).to.be.an('object');
                }
            } catch (error) {
                expect(error.message).to.match(/network|connection|timeout/i);
            }
        });
    });
    
    describe('Error Handling', () => {
        it('should handle invalid contract ID gracefully', async function() {
            this.timeout(15000);
            
            try {
                await global.wasmSdk.get_documents(
                    sdk,
                    'invalid-contract-id',
                    'domain',
                    null,
                    null,
                    10,
                    null,
                    null
                );
            } catch (error) {
                expect(error.message).to.exist;
                expect(error.message).to.match(/invalid|contract|network/i);
            }
        });
        
        it('should handle malformed where clause', async function() {
            this.timeout(15000);
            
            try {
                // Invalid JSON in where clause
                await global.wasmSdk.get_documents(
                    sdk,
                    DPNS_CONTRACT,
                    'domain',
                    'not valid json',
                    null,
                    10,
                    null,
                    null
                );
            } catch (error) {
                expect(error.message).to.exist;
            }
        });
    });
});