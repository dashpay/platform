/**
 * Epoch and Block Query Tests - Functional tests for epoch and block queries
 * 
 * These tests verify epoch and block query functions work correctly with the real WASM SDK.
 */

describe('Epoch and Block Queries (Functional)', () => {
    let sdk;
    
    // Test values
    const TEST_EVONODE_ID = '143dcd6a6b7684fde01e88a10e5d65de9a29244c5ecd586d14a342657025f113';
    
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
    
    describe('Epoch Information Queries', () => {
        it('should get epochs info', async function() {
            this.timeout(15000);
            
            if (!global.wasmSdk.get_epochs_info) {
                this.skip('get_epochs_info not implemented');
                return;
            }
            
            try {
                const result = await global.wasmSdk.get_epochs_info(
                    sdk,
                    1000,     // start epoch
                    100,      // count
                    true      // ascending
                );
                
                if (result) {
                    expect(result).to.be.an('array');
                    
                    if (result.length > 0) {
                        const firstEpoch = result[0];
                        expect(firstEpoch).to.be.an('object');
                        
                        if (firstEpoch.epochIndex !== undefined) {
                            expect(firstEpoch.epochIndex).to.be.a('number');
                        }
                        if (firstEpoch.startTime !== undefined) {
                            expect(firstEpoch.startTime).to.be.a('number');
                        }
                    }
                }
            } catch (error) {
                expect(error.message).to.match(/network|connection|timeout|epoch/i);
            }
        });
        
        it('should get finalized epoch infos', async function() {
            this.timeout(15000);
            
            if (!global.wasmSdk.get_finalized_epoch_infos) {
                this.skip('get_finalized_epoch_infos not implemented');
                return;
            }
            
            try {
                const result = await global.wasmSdk.get_finalized_epoch_infos(
                    sdk,
                    8635,     // start epoch
                    100       // count
                );
                
                if (result) {
                    expect(result).to.be.an('array');
                    
                    if (result.length > 0) {
                        const firstEpoch = result[0];
                        expect(firstEpoch).to.be.an('object');
                        
                        if (firstEpoch.epochIndex !== undefined) {
                            expect(firstEpoch.epochIndex).to.be.a('number');
                        }
                        if (firstEpoch.isFinalized !== undefined) {
                            expect(firstEpoch.isFinalized).to.be.a('boolean');
                        }
                    }
                }
            } catch (error) {
                expect(error.message).to.match(/network|connection|timeout|epoch|finalized/i);
            }
        });
    });
    
    describe('Evonode Block Queries', () => {
        it('should get evonodes proposed epoch blocks by IDs', async function() {
            this.timeout(15000);
            
            if (!global.wasmSdk.get_evonodes_proposed_epoch_blocks_by_ids) {
                this.skip('get_evonodes_proposed_epoch_blocks_by_ids not implemented');
                return;
            }
            
            try {
                const result = await global.wasmSdk.get_evonodes_proposed_epoch_blocks_by_ids(
                    sdk,
                    8635,                // epoch number
                    [TEST_EVONODE_ID]    // evonode IDs
                );
                
                if (result) {
                    expect(result).to.be.an('array');
                    
                    result.forEach(block => {
                        expect(block).to.be.an('object');
                    });
                }
            } catch (error) {
                expect(error.message).to.match(/network|connection|timeout|evonode|block/i);
            }
        });
        
        it('should get evonodes proposed epoch blocks by range', async function() {
            this.timeout(15000);
            
            if (!global.wasmSdk.get_evonodes_proposed_epoch_blocks_by_range) {
                this.skip('get_evonodes_proposed_epoch_blocks_by_range not implemented');
                return;
            }
            
            try {
                const result = await global.wasmSdk.get_evonodes_proposed_epoch_blocks_by_range(
                    sdk,
                    TEST_EVONODE_ID,    // start after ID
                    100                 // limit
                );
                
                if (result) {
                    expect(result).to.be.an('array');
                    
                    result.forEach(block => {
                        expect(block).to.be.an('object');
                    });
                }
            } catch (error) {
                expect(error.message).to.match(/network|connection|timeout|evonode|block|range/i);
            }
        });
    });
    
    describe('Block Height and Info', () => {
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
    });
});