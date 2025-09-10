/**
 * Jest Setup File for WASM SDK Tests
 * Configures global test environment, mocks, and utilities
 */

import { webcrypto } from 'crypto';
import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

// Get directory paths
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Set up globals for WASM compatibility
if (!global.crypto) {
    Object.defineProperty(global, 'crypto', {
        value: webcrypto,
        writable: true,
        configurable: true
    });
}

// Mock browser globals for Node.js testing
global.window = global.window || {};
global.document = global.document || {
    createElement: () => ({ style: {} }),
    getElementById: () => ({ textContent: '', style: {} }),
    addEventListener: () => {}
};

// Test utilities and constants
global.TEST_CONFIG = {
    // Test networks
    TESTNET: 'testnet',
    MAINNET: 'mainnet',
    
    // Test timeouts
    QUICK_TIMEOUT: 10000,   // 10 seconds for quick operations
    STANDARD_TIMEOUT: 30000, // 30 seconds for standard operations
    SLOW_TIMEOUT: 60000,    // 60 seconds for slow operations
    
    // Test data
    SAMPLE_IDENTITY_ID: '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk',
    SAMPLE_CONTRACT_ID: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec', // DPNS
    SAMPLE_USERNAME: 'alice',
    
    // Contract IDs
    DPNS_TESTNET: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
    DASHPAY_TESTNET: '8cvMFwa2YbEsNNoc1PXfTacy2PVq2SzQoQYQNuS7IZ5Y',
    WITHDRAWALS_TESTNET: 'HjdgVJn9W2umtLUZsjKF5mUyexo2zqiCkJkkPq1mGw5f'
};

// Global test utilities
global.sleep = (ms) => new Promise(resolve => setTimeout(resolve, ms));

global.withTimeout = async (promise, timeout = TEST_CONFIG.STANDARD_TIMEOUT) => {
    return Promise.race([
        promise,
        new Promise((_, reject) => 
            setTimeout(() => reject(new Error(`Operation timed out after ${timeout}ms`)), timeout)
        )
    ]);
};

// WASM initialization helper
global.initializeWasm = async () => {
    try {
        const wasmPath = join(__dirname, '../pkg/dash_wasm_sdk_bg.wasm');
        const wasmBuffer = readFileSync(wasmPath);
        const init = (await import('../pkg/dash_wasm_sdk.js')).default;
        await init(wasmBuffer);
        return true;
    } catch (error) {
        console.warn('WASM initialization failed:', error.message);
        return false;
    }
};

// SDK factory helper
global.createTestSDK = async (options = {}) => {
    const { WasmSDK } = await import('../src-js/index.js');
    
    const defaultOptions = {
        network: TEST_CONFIG.TESTNET,
        proofs: false, // Disable proofs for faster testing
        debug: false,
        transport: {
            timeout: TEST_CONFIG.STANDARD_TIMEOUT,
            retries: 3
        }
    };
    
    const sdk = new WasmSDK({ ...defaultOptions, ...options });
    await sdk.initialize();
    return sdk;
};

// Test result validators
global.expectValidIdentity = (identity) => {
    expect(identity).toBeDefined();
    expect(identity).toHaveProperty('id');
    expect(typeof identity.id).toBe('string');
    expect(identity.id).toMatch(/^[A-Za-z0-9]{44,}$/); // Base58 format
};

global.expectValidContract = (contract) => {
    expect(contract).toBeDefined();
    expect(contract).toHaveProperty('documents');
    expect(typeof contract.documents).toBe('object');
};

global.expectValidDocument = (document) => {
    expect(document).toBeDefined();
    expect(document).toHaveProperty('id');
    expect(document).toHaveProperty('ownerId');
    expect(typeof document.id).toBe('string');
    expect(typeof document.ownerId).toBe('string');
};

// Performance measurement utilities
global.measurePerformance = async (operation, name = 'operation') => {
    const start = performance.now();
    const result = await operation();
    const duration = performance.now() - start;
    
    console.log(`⏱️ ${name} took ${duration.toFixed(2)}ms`);
    
    return {
        result,
        duration,
        name
    };
};

// Network test utilities
global.testNetworkConnectivity = async () => {
    try {
        const response = await fetch('https://52.12.176.90:1443/health');
        return response.ok;
    } catch {
        return false;
    }
};

// Error matchers for better test assertions
expect.extend({
    toBeValidBase58String(received) {
        const pass = typeof received === 'string' && /^[A-Za-z0-9]{44,}$/.test(received);
        return {
            message: () =>
                `expected ${received} to be a valid Base58 string`,
            pass,
        };
    },
    
    toHaveValidQueryResult(received) {
        const pass = Array.isArray(received) && received.every(item => 
            typeof item === 'object' && item !== null
        );
        return {
            message: () =>
                `expected ${received} to be a valid query result array`,
            pass,
        };
    },
    
    toCompleteWithinTime(received, expectedTime) {
        const pass = received < expectedTime;
        return {
            message: () =>
                `expected operation to complete within ${expectedTime}ms, but took ${received}ms`,
            pass,
        };
    }
});

// Cleanup after each test
afterEach(() => {
    // Clear any mocks
    jest.clearAllMocks();
    
    // Force garbage collection if available
    if (global.gc) {
        global.gc();
    }
});

console.log('🧪 Jest setup complete - WASM SDK test environment ready');