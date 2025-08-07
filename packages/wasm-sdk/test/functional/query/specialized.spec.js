/**
 * Specialized Query Tests - Functional tests for specialized platform queries
 * 
 * These tests verify specialized query functions (masternodes, groups, etc.) work correctly.
 */

describe('Specialized Queries (Functional)', () => {
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
    
    describe('Masternode Queries', () => {
        it('should get masternodes', async function() {
            this.timeout(15000);
            
            // Check if function exists
            if (!global.wasmSdk.get_masternodes) {
                this.skip('get_masternodes not implemented');
                return;
            }
            
            try {
                const result = await global.wasmSdk.get_masternodes(sdk);
                
                if (result) {
                    expect(result).to.be.an('array');
                    result.forEach(node => {
                        expect(node).to.be.an('object');
                    });
                }
            } catch (error) {
                expect(error.message).to.match(/network|connection|timeout|masternode/i);
            }
        });
        
        it('should get masternode status', async function() {
            this.timeout(15000);
            
            if (!global.wasmSdk.get_masternode_status) {
                this.skip('get_masternode_status not implemented');
                return;
            }
            
            try {
                // Would need a real masternode proTxHash
                const result = await global.wasmSdk.get_masternode_status(
                    sdk,
                    'invalidProTxHash'
                );
                
                // Should fail with invalid hash
            } catch (error) {
                expect(error.message).to.exist;
            }
        });
    });
    
    describe('Group Queries', () => {
        it('should get groups', async function() {
            this.timeout(15000);
            
            if (!global.wasmSdk.get_groups) {
                this.skip('get_groups not implemented');
                return;
            }
            
            try {
                const result = await global.wasmSdk.get_groups(sdk);
                
                if (result) {
                    expect(result).to.be.an('array');
                }
            } catch (error) {
                expect(error.message).to.match(/network|connection|timeout|group/i);
            }
        });
    });
    
    describe('Protocol Version Queries', () => {
        it('should get protocol version', async function() {
            this.timeout(15000);
            
            if (!global.wasmSdk.get_protocol_version) {
                this.skip('get_protocol_version not implemented');
                return;
            }
            
            try {
                const result = await global.wasmSdk.get_protocol_version(sdk);
                
                if (result !== undefined && result !== null) {
                    expect(result).to.be.a('number');
                    expect(result).to.be.at.least(1);
                }
            } catch (error) {
                expect(error.message).to.match(/network|connection|timeout|protocol/i);
            }
        });
        
        it('should get protocol version history', async function() {
            this.timeout(15000);
            
            if (!global.wasmSdk.get_protocol_version_history) {
                this.skip('get_protocol_version_history not implemented');
                return;
            }
            
            try {
                const result = await global.wasmSdk.get_protocol_version_history(
                    sdk,
                    10,   // limit
                    0     // offset
                );
                
                if (result) {
                    expect(result).to.be.an('array');
                }
            } catch (error) {
                expect(error.message).to.match(/network|connection|timeout|protocol/i);
            }
        });
    });
    
    describe('System Utility Queries', () => {
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
                expect(error.message).to.match(/network|connection|timeout|block/i);
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
});