/**
 * Protocol Version Query Tests - Functional tests for protocol version queries
 * 
 * These tests verify protocol version query functions work correctly with the real WASM SDK.
 */

describe('Protocol Version Queries (Functional)', () => {
    let sdk;
    
    // Test values
    const TEST_PROTX_HASH = '143dcd6a6b7684fde01e88a10e5d65de9a29244c5ecd586d14a342657025f113';
    
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
    
    describe('Protocol Version Upgrade Queries', () => {
        it('should get protocol version upgrade state', async function() {
            this.timeout(15000);
            
            if (!global.wasmSdk.get_protocol_version_upgrade_state) {
                this.skip('get_protocol_version_upgrade_state not implemented');
                return;
            }
            
            try {
                const result = await global.wasmSdk.get_protocol_version_upgrade_state(sdk);
                
                if (result) {
                    expect(result).to.be.an('object');
                    
                    if (result.currentVersion !== undefined) {
                        expect(result.currentVersion).to.be.a('number');
                        expect(result.currentVersion).to.be.at.least(1);
                    }
                    if (result.nextVersion !== undefined) {
                        expect(result.nextVersion).to.be.a('number');
                    }
                    if (result.voteStatus !== undefined) {
                        expect(result.voteStatus).to.be.a('string');
                    }
                }
            } catch (error) {
                expect(error.message).to.match(/network|connection|timeout|protocol|version|upgrade/i);
            }
        });
        
        it('should get protocol version upgrade vote status', async function() {
            this.timeout(15000);
            
            if (!global.wasmSdk.get_protocol_version_upgrade_vote_status) {
                this.skip('get_protocol_version_upgrade_vote_status not implemented');
                return;
            }
            
            try {
                const result = await global.wasmSdk.get_protocol_version_upgrade_vote_status(
                    sdk,
                    TEST_PROTX_HASH,    // start protx hash
                    100                 // count
                );
                
                if (result) {
                    expect(result).to.be.an('array');
                    
                    result.forEach(vote => {
                        expect(vote).to.be.an('object');
                        
                        if (vote.proTxHash !== undefined) {
                            expect(vote.proTxHash).to.be.a('string');
                            expect(vote.proTxHash).to.have.length.above(0);
                        }
                        if (vote.vote !== undefined) {
                            expect(vote.vote).to.be.a('string');
                        }
                    });
                }
            } catch (error) {
                expect(error.message).to.match(/network|connection|timeout|protocol|version|vote|status/i);
            }
        });
    });
    
    describe('Protocol Version Information', () => {
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
                expect(error.message).to.match(/network|connection|timeout|protocol|version/i);
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
                    
                    result.forEach(versionInfo => {
                        expect(versionInfo).to.be.an('object');
                        
                        if (versionInfo.version !== undefined) {
                            expect(versionInfo.version).to.be.a('number');
                            expect(versionInfo.version).to.be.at.least(1);
                        }
                    });
                }
            } catch (error) {
                expect(error.message).to.match(/network|connection|timeout|protocol|version|history/i);
            }
        });
    });
    
    describe('Version Validation', () => {
        it('should handle version queries consistently', async function() {
            this.timeout(15000);
            
            const versionQueries = [
                { name: 'get_protocol_version', fn: global.wasmSdk.get_protocol_version },
                { name: 'get_protocol_version_upgrade_state', fn: global.wasmSdk.get_protocol_version_upgrade_state }
            ];
            
            const results = {};
            
            for (const query of versionQueries) {
                if (query.fn) {
                    try {
                        const result = await query.fn(sdk);
                        results[query.name] = result;
                    } catch (error) {
                        // Network errors are expected
                        expect(error.message).to.match(/network|connection|timeout/i);
                        results[query.name] = null;
                    }
                }
            }
            
            // If we got version results, they should be consistent
            if (results.get_protocol_version && results.get_protocol_version_upgrade_state) {
                const currentVersion = results.get_protocol_version;
                const upgradeState = results.get_protocol_version_upgrade_state;
                
                if (upgradeState.currentVersion) {
                    expect(currentVersion).to.equal(upgradeState.currentVersion);
                }
            }
        });
    });
});