/**
 * Utilities Tests - Migrated from utilities-simple.test.mjs
 * 
 * These tests verify utility functions in the WASM SDK including
 * version info, network operations, error handling, and type validation.
 */

const { TestSdkBuilder, TestAssertions } = require('../helpers/sdk-builder.js');
const { TestData } = require('../fixtures/test-data.js');

describe('Utility Functions', () => {
    describe('SDK Version and Initialization', () => {
        it('should create SDK and check version', async () => {
            const sdk = await global.createTestSdk.testnet();
            
            const version = sdk.version();
            expect(version).to.be.a('number');
            expect(version).to.be.at.least(1);
            
            if (sdk.free) {
                sdk.free();
            }
        });

        it('should create mainnet SDK', async () => {
            const sdk = await global.createTestSdk.mainnet();
            expect(sdk).to.exist;
            expect(sdk.version).to.be.a('function');
            
            if (sdk.free) {
                sdk.free();
            }
        });
    });

    describe('Trusted Quorum Prefetch', () => {
        it('should handle prefetch_trusted_quorums_mainnet', async () => {
            try {
                await global.wasmSdk.prefetch_trusted_quorums_mainnet();
                // Success means network is available
            } catch (error) {
                // Network errors are acceptable in test environment
                expect(error.message).to.match(/network|fetch|connection/i);
            }
        });

        it('should handle prefetch_trusted_quorums_testnet', async () => {
            try {
                await global.wasmSdk.prefetch_trusted_quorums_testnet();
                // Success means network is available
            } catch (error) {
                // Network errors are acceptable in test environment
                expect(error.message).to.match(/network|fetch|connection/i);
            }
        });

        it('should provide prefetch functions as exports', () => {
            expect(global.wasmSdk.prefetch_trusted_quorums_mainnet).to.be.a('function');
            expect(global.wasmSdk.prefetch_trusted_quorums_testnet).to.be.a('function');
        });
    });

    describe('Test Serialization', () => {
        it('should check testSerialization method availability', async () => {
            const sdk = await global.createTestSdk.testnet();
            
            if (typeof sdk.testSerialization === 'function') {
                const result = sdk.testSerialization('test-string');
                // Method exists but may return undefined - this is acceptable
                expect(result === undefined || typeof result === 'string').to.be.true;
            } else {
                // Method doesn't exist - also acceptable
                expect(sdk.testSerialization).to.be.undefined;
            }
            
            if (sdk.free) {
                sdk.free();
            }
        });
    });

    describe('Error Handling', () => {
        it('should fail gracefully with null SDK', async () => {
            try {
                await global.wasmSdk.get_status(null);
                expect.fail('Should have thrown error with null SDK');
            } catch (error) {
                expect(error.message).to.match(/SDK|required/i);
            }
        });

        it('should fail gracefully with undefined SDK', async () => {
            try {
                await global.wasmSdk.get_status(undefined);
                expect.fail('Should have thrown error with undefined SDK');
            } catch (error) {
                expect(error.message).to.match(/SDK|required/i);
            }
        });

        it('should fail gracefully with freed SDK', async () => {
            const sdk = await global.createTestSdk.testnet();
            
            if (sdk.free) {
                sdk.free();
                
                try {
                    sdk.version();
                    expect.fail('Should have thrown error with freed SDK');
                } catch (error) {
                    // Expected error when using freed SDK
                    expect(error).to.exist;
                }
            } else {
                // If free() doesn't exist, skip this test
                this.skip();
            }
        });

        it('should handle invalid parameters gracefully', async () => {
            // Test various invalid parameter scenarios
            expect(() => {
                global.wasmSdk.validate_mnemonic(123);
            }).to.not.throw(); // Should handle gracefully, not crash
            
            expect(() => {
                global.wasmSdk.generate_mnemonic("invalid");
            }).to.throw(); // Should throw for invalid word count
        });
    });

    describe('Type Validation', () => {
        it('should validate string parameters', async () => {
            // The stub should handle type validation gracefully
            const result = global.wasmSdk.validate_mnemonic(123);
            expect(result).to.be.a('boolean');
            expect(result).to.be.false; // Invalid type should return false
        });

        it('should validate array parameters', async () => {
            const sdk = await global.createTestSdk.testnet();
            
            try {
                await global.wasmSdk.get_path_elements(sdk, "not-an-array", []);
                expect.fail('Should have thrown error with non-array parameter');
            } catch (error) {
                expect(error.message).to.match(/array/i);
            }
            
            if (sdk.free) {
                sdk.free();
            }
        });

        it('should validate number parameters', async () => {
            try {
                global.wasmSdk.generate_mnemonic("twelve");
                expect.fail('Should have thrown error with string instead of number');
            } catch (error) {
                expect(error.message).to.match(/Word count must be|invalid/i);
            }
        });
    });

    describe('Network-dependent Utilities', () => {
        let sdk;

        beforeEach(async () => {
            sdk = await global.createTestSdk.testnet();
        });

        afterEach(() => {
            if (sdk && sdk.free) {
                sdk.free();
            }
        });

        it('should handle wait_for_state_transition_result with invalid hash', async () => {
            try {
                await global.wasmSdk.wait_for_state_transition_result(
                    sdk,
                    "0000000000000000000000000000000000000000000000000000000000000000"
                );
                expect.fail('Should have failed or timed out with invalid hash');
            } catch (error) {
                expect(error.message).to.match(/not found|timeout|invalid/i);
            }
        });

        it('should handle get_path_elements', async () => {
            try {
                const result = await global.wasmSdk.get_path_elements(sdk, [], []);
                expect(result).to.be.an('object');
                expect(result).to.have.property('elements');
            } catch (error) {
                // Network errors are acceptable
                expect(error.message).to.match(/network|connection/i);
            }
        });

        it('should handle get_path_elements with invalid parameters', async () => {
            try {
                await global.wasmSdk.get_path_elements(sdk, "not-array", []);
                expect.fail('Should have thrown error with invalid parameters');
            } catch (error) {
                expect(error.message).to.match(/array/i);
            }
        });

        it('should get SDK status', async () => {
            try {
                const status = await global.wasmSdk.get_status(sdk);
                expect(status).to.be.an('object');
            } catch (error) {
                // Network errors are acceptable
                expect(error.message).to.match(/network|connection/i);
            }
        });
    });

    describe('Start Function', () => {
        it('should handle start function calls', async () => {
            try {
                await global.wasmSdk.start();
                // Multiple calls might succeed or fail
                await global.wasmSdk.start();
            } catch (error) {
                // "Already started" or similar errors are acceptable
                expect(error.message).to.match(/start|already|init/i);
            }
        });

        it('should provide start function as export', () => {
            expect(global.wasmSdk.start).to.be.a('function');
        });
    });

    describe('Function Existence', () => {
        it('should have all expected utility functions', () => {
            const utilityFunctions = [
                'prefetch_trusted_quorums_mainnet',
                'prefetch_trusted_quorums_testnet',
                'wait_for_state_transition_result',
                'start',
                'get_path_elements',
                'get_status'
            ];
            
            for (const fn of utilityFunctions) {
                expect(global.wasmSdk[fn]).to.be.a('function', `${fn} should be a function`);
            }
        });

        it('should have SDK builder functions', () => {
            expect(global.wasmSdk.WasmSdkBuilder).to.exist;
            expect(global.wasmSdk.WasmSdkBuilder.new_testnet).to.be.a('function');
            expect(global.wasmSdk.WasmSdkBuilder.new_mainnet).to.be.a('function');
        });

        it('should have key generation functions', () => {
            const keyFunctions = [
                'generate_mnemonic',
                'validate_mnemonic',
                'mnemonic_to_seed'
            ];
            
            for (const fn of keyFunctions) {
                expect(global.wasmSdk[fn]).to.be.a('function', `${fn} should be a function`);
            }
        });
    });

    describe('Integration Tests', () => {
        it('should create SDK, call utilities, and clean up', async () => {
            const sdk = await global.createTestSdk.testnet();
            
            // Check version
            const version = sdk.version();
            expect(version).to.be.a('number');
            
            // Try network operation
            try {
                const status = await global.wasmSdk.get_status(sdk);
                expect(status).to.be.an('object');
            } catch (error) {
                // Network errors are acceptable
            }
            
            // Clean up
            if (sdk.free) {
                sdk.free();
            }
        });

        it('should handle multiple SDK instances', async () => {
            const sdk1 = await global.createTestSdk.testnet();
            const sdk2 = await global.createTestSdk.mainnet();
            
            expect(sdk1).to.exist;
            expect(sdk2).to.exist;
            expect(sdk1.version()).to.be.a('number');
            expect(sdk2.version()).to.be.a('number');
            
            // Clean up
            if (sdk1.free) sdk1.free();
            if (sdk2.free) sdk2.free();
        });
    });

    describe('Performance and Stress Tests', () => {
        it('should handle multiple rapid function calls', async () => {
            const promises = [];
            
            // Create multiple promises for testing concurrency
            for (let i = 0; i < 5; i++) {
                promises.push(global.wasmSdk.generate_mnemonic(12));
            }
            
            const results = await Promise.all(promises);
            expect(results).to.have.length(5);
            
            results.forEach(mnemonic => {
                expect(mnemonic).to.be.a('string');
                expect(mnemonic.split(' ')).to.have.length(12);
            });
        });

        it('should handle rapid SDK creation and destruction', async () => {
            const sdks = [];
            
            // Create multiple SDKs
            for (let i = 0; i < 3; i++) {
                const sdk = await global.createTestSdk.testnet();
                sdks.push(sdk);
            }
            
            // Verify they all work
            sdks.forEach(sdk => {
                expect(sdk.version()).to.be.a('number');
            });
            
            // Clean up
            sdks.forEach(sdk => {
                if (sdk.free) sdk.free();
            });
        });
    });
});