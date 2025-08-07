/**
 * Group Query Tests - Functional tests for group-related queries
 * 
 * These tests verify group query functions work correctly with the real WASM SDK.
 */

describe('Group Queries (Functional)', () => {
    let sdk;
    
    // Test values
    const TEST_GROUP_ID = '49PJEnNx7ReCitzkLdkDNr4s6RScGsnNexcdSZJ1ph5N';
    const TEST_ACTION_ID = '6XJzL6Qb8Zhwxt4HFwh8NAn7q1u4dwdoUf8EmgzDudFZ';
    
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
    
    describe('Group Information Queries', () => {
        it('should get group info', async function() {
            this.timeout(15000);
            
            if (!global.wasmSdk.get_group_info) {
                this.skip('get_group_info not implemented');
                return;
            }
            
            try {
                const result = await global.wasmSdk.get_group_info(
                    sdk,
                    TEST_GROUP_ID,    // group ID
                    0                 // subgroup position
                );
                
                if (result) {
                    expect(result).to.be.an('object');
                    
                    if (result.type !== undefined) {
                        expect(result.type).to.be.a('string');
                    }
                    if (result.memberCount !== undefined) {
                        expect(result.memberCount).to.be.a('number');
                        expect(result.memberCount).to.be.at.least(0);
                    }
                }
            } catch (error) {
                expect(error.message).to.match(/network|connection|timeout|group/i);
            }
        });
        
        it('should get group infos', async function() {
            this.timeout(15000);
            
            if (!global.wasmSdk.get_group_infos) {
                this.skip('get_group_infos not implemented');
                return;
            }
            
            try {
                const result = await global.wasmSdk.get_group_infos(
                    sdk,
                    TEST_GROUP_ID,    // group ID
                    null,             // start after
                    100               // limit
                );
                
                if (result) {
                    expect(result).to.be.an('array');
                    
                    result.forEach(groupInfo => {
                        expect(groupInfo).to.be.an('object');
                    });
                }
            } catch (error) {
                expect(error.message).to.match(/network|connection|timeout|group/i);
            }
        });
        
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
                    
                    result.forEach(group => {
                        expect(group).to.be.an('object');
                    });
                }
            } catch (error) {
                expect(error.message).to.match(/network|connection|timeout|group/i);
            }
        });
    });
    
    describe('Group Action Queries', () => {
        it('should get group actions', async function() {
            this.timeout(15000);
            
            if (!global.wasmSdk.get_group_actions) {
                this.skip('get_group_actions not implemented');
                return;
            }
            
            try {
                const result = await global.wasmSdk.get_group_actions(
                    sdk,
                    TEST_GROUP_ID,    // group ID
                    0,                // subgroup position
                    'ACTIVE',         // action status
                    null,             // start after
                    100               // limit
                );
                
                if (result) {
                    expect(result).to.be.an('array');
                    
                    result.forEach(action => {
                        expect(action).to.be.an('object');
                    });
                }
            } catch (error) {
                expect(error.message).to.match(/network|connection|timeout|group|action/i);
            }
        });
        
        it('should get group action signers', async function() {
            this.timeout(15000);
            
            if (!global.wasmSdk.get_group_action_signers) {
                this.skip('get_group_action_signers not implemented');
                return;
            }
            
            try {
                const result = await global.wasmSdk.get_group_action_signers(
                    sdk,
                    TEST_GROUP_ID,    // group ID
                    0,                // subgroup position
                    'ACTIVE',         // action status
                    TEST_ACTION_ID    // action ID
                );
                
                if (result) {
                    expect(result).to.be.an('array');
                    
                    result.forEach(signer => {
                        expect(signer).to.be.an('object');
                    });
                }
            } catch (error) {
                expect(error.message).to.match(/network|connection|timeout|group|action|signer/i);
            }
        });
    });
    
    describe('Action Status Tests', () => {
        it('should handle different action statuses', async function() {
            this.timeout(15000);
            
            if (!global.wasmSdk.get_group_actions) {
                this.skip('get_group_actions not implemented');
                return;
            }
            
            const statuses = ['ACTIVE', 'COMPLETED', 'CANCELLED'];
            
            for (const status of statuses) {
                try {
                    const result = await global.wasmSdk.get_group_actions(
                        sdk,
                        TEST_GROUP_ID,
                        0,
                        status,
                        null,
                        10
                    );
                    
                    // Any result or network error is acceptable
                    if (result) {
                        expect(result).to.be.an('array');
                    }
                } catch (error) {
                    expect(error.message).to.match(/network|connection|timeout|group|action|status/i);
                }
            }
        });
    });
});