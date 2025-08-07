/**
 * Helper utilities for creating and configuring SDK instances in tests
 */

/**
 * Test SDK Builder - provides easy access to configured SDK instances
 */
class TestSdkBuilder {
    /**
     * Create a testnet SDK instance
     */
    static async createTestnet() {
        if (!global.wasmSdk?.WasmSdkBuilder) {
            throw new Error('WASM SDK not initialized. Make sure setup.js is loaded.');
        }
        
        const builder = global.wasmSdk.WasmSdkBuilder.new_testnet();
        return await builder.build();
    }
    
    /**
     * Create a mainnet SDK instance  
     */
    static async createMainnet() {
        if (!global.wasmSdk?.WasmSdkBuilder) {
            throw new Error('WASM SDK not initialized. Make sure setup.js is loaded.');
        }
        
        const builder = global.wasmSdk.WasmSdkBuilder.new_mainnet();
        return await builder.build();
    }
    
    /**
     * Create SDK with custom configuration
     */
    static async createWithConfig(network = 'testnet', options = {}) {
        if (!global.wasmSdk?.WasmSdkBuilder) {
            throw new Error('WASM SDK not initialized. Make sure setup.js is loaded.');
        }
        
        const builderMethod = network === 'mainnet' ? 'new_mainnet' : 'new_testnet';
        const builder = global.wasmSdk.WasmSdkBuilder[builderMethod]();
        
        // Apply configuration options
        if (options.version) {
            builder.with_version(options.version);
        }
        
        if (options.settings) {
            builder.with_settings(options.settings);
        }
        
        return await builder.build();
    }
    
    /**
     * Create a mocked SDK instance (for unit tests)
     * This would return an SDK with network calls stubbed out
     */
    static async createMocked(network = 'testnet') {
        // TODO: Implement mocked SDK for unit tests
        // This would stub out network calls and return controlled responses
        const sdk = await this.createTestnet();
        
        // Add mocking capabilities here
        // Example: stub query methods to return test data
        
        return sdk;
    }
}

/**
 * Test assertion helpers
 */
const TestAssertions = {
    /**
     * Assert that a value looks like a valid identity ID
     */
    isValidIdentityId(value) {
        expect(value).to.be.a('string');
        expect(value).to.have.length.greaterThan(40);
        expect(value).to.match(/^[A-Za-z0-9]+$/);
    },
    
    /**
     * Assert that a value looks like a valid private key WIF
     */
    isValidWIF(value) {
        expect(value).to.be.a('string');
        expect(value).to.have.length.greaterThan(50);
        expect(value).to.match(/^[XL][A-Za-z0-9]+$/);
    },
    
    /**
     * Assert that a value looks like a valid public key
     */
    isValidPublicKey(value) {
        expect(value).to.be.a('string');
        expect(value).to.have.length.within(66, 130); // Compressed or uncompressed
        expect(value).to.match(/^[0-9a-fA-F]+$/);
    },
    
    /**
     * Assert that a value looks like a valid Dash address
     */
    isValidDashAddress(value, network = null) {
        expect(value).to.be.a('string');
        expect(value).to.have.length.within(25, 40);
        
        if (network === 'mainnet') {
            expect(value).to.match(/^X/);
        } else if (network === 'testnet') {
            expect(value).to.match(/^y/);
        }
    },
    
    /**
     * Assert that an error contains expected properties
     */
    isValidError(error, expectedMessage = null) {
        expect(error).to.be.an('error');
        if (expectedMessage) {
            expect(error.message).to.include(expectedMessage);
        }
    }
};

/**
 * Test data generators
 */
const TestGenerators = {
    /**
     * Generate a random test seed phrase
     */
    randomSeedPhrase() {
        // This would use the SDK's mnemonic generation
        // For now, return a known test seed
        return "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    },
    
    /**
     * Generate test document data
     */
    testDocument(type = 'note', data = {}) {
        const defaultData = {
            note: { message: 'Test message' },
            profile: { name: 'Test User' }
        };
        
        return {
            type,
            data: { ...defaultData[type], ...data }
        };
    },
    
    /**
     * Generate test contract schema
     */
    testContractSchema() {
        return {
            note: {
                type: "object",
                properties: {
                    message: {
                        type: "string",
                        maxLength: 100,
                        position: 0
                    }
                },
                required: ["message"],
                additionalProperties: false
            }
        };
    }
};

module.exports = {
    TestSdkBuilder,
    TestAssertions,
    TestGenerators
};