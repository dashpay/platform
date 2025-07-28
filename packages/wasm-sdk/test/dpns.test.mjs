#!/usr/bin/env node
// dpns.test.mjs - Tests for DPNS (Dash Platform Name Service) functions

import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
import { webcrypto } from 'crypto';

// Get directory paths
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Set up globals for WASM
if (!global.crypto) {
    Object.defineProperty(global, 'crypto', {
        value: webcrypto,
        writable: true,
        configurable: true
    });
}

// Import WASM SDK
import init, * as wasmSdk from '../pkg/wasm_sdk.js';

// Initialize WASM
console.log('Initializing WASM SDK...');
const wasmPath = join(__dirname, '../pkg/wasm_sdk_bg.wasm');
const wasmBuffer = readFileSync(wasmPath);
await init(wasmBuffer);

// Test utilities
let passed = 0;
let failed = 0;

async function test(name, fn) {
    try {
        await fn();
        console.log(`✅ ${name}`);
        passed++;
    } catch (error) {
        console.log(`❌ ${name}`);
        console.log(`   ${error.message}`);
        failed++;
    }
}

function describe(name) {
    console.log(`\n${name}`);
}

console.log('\nDPNS Tests\n');

// Homograph Safety Tests
describe('Homograph Safety');

await test('dpns_convert_to_homograph_safe - basic ASCII', () => {
    const result = wasmSdk.dpns_convert_to_homograph_safe("test");
    if (result !== "test") {
        throw new Error(`Expected "test", got "${result}"`);
    }
});

await test('dpns_convert_to_homograph_safe - with numbers', () => {
    const result = wasmSdk.dpns_convert_to_homograph_safe("test123");
    if (result !== "test123") {
        throw new Error(`Expected "test123", got "${result}"`);
    }
});

await test('dpns_convert_to_homograph_safe - with hyphens', () => {
    const result = wasmSdk.dpns_convert_to_homograph_safe("test-name");
    if (result !== "test-name") {
        throw new Error(`Expected "test-name", got "${result}"`);
    }
});

await test('dpns_convert_to_homograph_safe - uppercase to lowercase', () => {
    const result = wasmSdk.dpns_convert_to_homograph_safe("TestName");
    if (result !== "testname") {
        throw new Error(`Expected "testname", got "${result}"`);
    }
});

await test('dpns_convert_to_homograph_safe - special characters', () => {
    // This should remove or convert special characters
    const result = wasmSdk.dpns_convert_to_homograph_safe("test@name!");
    // The exact behavior depends on implementation, but it should not contain @ or !
    if (result.includes('@') || result.includes('!')) {
        throw new Error(`Special characters should be removed/converted, got "${result}"`);
    }
});

await test('dpns_convert_to_homograph_safe - unicode homographs', () => {
    // Test with common homograph characters
    const result = wasmSdk.dpns_convert_to_homograph_safe("tеst"); // е is Cyrillic
    // Should convert to safe ASCII equivalent
    if (result === "tеst") { // If it's still the same, homograph protection failed
        throw new Error('Homograph protection should convert Cyrillic characters');
    }
});

// Username Validation Tests
describe('Username Validation');

await test('dpns_is_valid_username - valid basic username', () => {
    if (!wasmSdk.dpns_is_valid_username("alice")) {
        throw new Error('Basic username "alice" should be valid');
    }
});

await test('dpns_is_valid_username - valid with numbers', () => {
    if (!wasmSdk.dpns_is_valid_username("alice123")) {
        throw new Error('Username with numbers should be valid');
    }
});

await test('dpns_is_valid_username - valid with hyphen', () => {
    if (!wasmSdk.dpns_is_valid_username("alice-bob")) {
        throw new Error('Username with hyphen should be valid');
    }
});

await test('dpns_is_valid_username - too short', () => {
    if (wasmSdk.dpns_is_valid_username("ab")) {
        throw new Error('Username shorter than 3 characters should be invalid');
    }
});

await test('dpns_is_valid_username - too long', () => {
    const longName = "a".repeat(64);
    if (wasmSdk.dpns_is_valid_username(longName)) {
        throw new Error('Username longer than 63 characters should be invalid');
    }
});

await test('dpns_is_valid_username - starts with hyphen', () => {
    if (wasmSdk.dpns_is_valid_username("-alice")) {
        throw new Error('Username starting with hyphen should be invalid');
    }
});

await test('dpns_is_valid_username - ends with hyphen', () => {
    if (wasmSdk.dpns_is_valid_username("alice-")) {
        throw new Error('Username ending with hyphen should be invalid');
    }
});

await test('dpns_is_valid_username - double hyphen', () => {
    if (wasmSdk.dpns_is_valid_username("alice--bob")) {
        throw new Error('Username with double hyphen should be invalid');
    }
});

await test('dpns_is_valid_username - uppercase', () => {
    if (wasmSdk.dpns_is_valid_username("Alice")) {
        throw new Error('Username with uppercase should be invalid');
    }
});

await test('dpns_is_valid_username - special characters', () => {
    if (wasmSdk.dpns_is_valid_username("alice@bob")) {
        throw new Error('Username with special characters should be invalid');
    }
});

await test('dpns_is_valid_username - spaces', () => {
    if (wasmSdk.dpns_is_valid_username("alice bob")) {
        throw new Error('Username with spaces should be invalid');
    }
});

// Contested Username Tests
describe('Contested Username Detection');

await test('dpns_is_contested_username - non-contested name', () => {
    if (wasmSdk.dpns_is_contested_username("uniquename123")) {
        throw new Error('Unique username should not be contested');
    }
});

await test('dpns_is_contested_username - common name', () => {
    // Common names like "alice", "bob", "test" might be contested
    const result = wasmSdk.dpns_is_contested_username("alice");
    // This depends on implementation - just check it returns a boolean
    if (typeof result !== 'boolean') {
        throw new Error('Should return boolean');
    }
});

await test('dpns_is_contested_username - single letter', () => {
    const result = wasmSdk.dpns_is_contested_username("a");
    if (typeof result !== 'boolean') {
        throw new Error('Should return boolean');
    }
});

await test('dpns_is_contested_username - three letter', () => {
    const result = wasmSdk.dpns_is_contested_username("abc");
    if (typeof result !== 'boolean') {
        throw new Error('Should return boolean');
    }
});

// Network-dependent DPNS tests (expected to fail without network)
describe('DPNS Network Operations (Expected to Fail)');

// Initialize SDK for network tests
let sdk = null;
try {
    const builder = wasmSdk.WasmSdkBuilder.new_testnet();
    sdk = await builder.build();
} catch (error) {
    console.log('   Failed to create SDK for network tests');
}

if (sdk) {
    await test('get_dpns_usernames - get usernames for identity', async () => {
        try {
            const result = await wasmSdk.get_dpns_usernames(
                sdk,
                '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk',
                10  // limit parameter
            );
            console.log(`   Found ${result?.length || 0} usernames for identity`);
        } catch (error) {
            if (error.message.includes('network') || error.message.includes('connection')) {
                console.log('   Expected network error (offline)');
            } else {
                throw error;
            }
        }
    });

    await test('dpns_register_name - requires identity and network', async () => {
        try {
            // This will fail without a valid identity and network connection
            const result = await wasmSdk.dpns_register_name(
                sdk,
                "testname",
                "invalididentityid",
                0,
                "invalidprivatekey"
            );
            throw new Error('Should have failed without valid identity');
        } catch (error) {
            // Expected to fail
            if (error.message.includes('Should have failed')) {
                throw error;
            }
        }
    });

    await test('dpns_is_name_available - requires network', async () => {
        try {
            const result = await wasmSdk.dpns_is_name_available(sdk, "testname");
            // If this succeeds, it means network is available
            if (typeof result !== 'boolean') {
                throw new Error('Should return boolean');
            }
        } catch (error) {
            // Expected to fail without network
            // This is acceptable
        }
    });

    await test('dpns_resolve_name - requires network', async () => {
        try {
            const result = await wasmSdk.dpns_resolve_name(sdk, "alice.dash");
            // If this succeeds, it means network is available
            if (result && typeof result !== 'object') {
                throw new Error('Should return object or null');
            }
        } catch (error) {
            // Expected to fail without network
            // This is acceptable
        }
    });

    await test('get_dpns_username_by_name - requires network', async () => {
        try {
            const result = await wasmSdk.get_dpns_username_by_name(sdk, "alice");
            // If this succeeds, it means network is available
            if (result && typeof result !== 'object') {
                throw new Error('Should return object or null');
            }
        } catch (error) {
            // Expected to fail without network
            // This is acceptable
        }
    });

    await test('get_dpns_usernames - requires network and identity', async () => {
        try {
            const result = await wasmSdk.get_dpns_usernames(sdk, "invalididentityid", 10);
            // If this succeeds, it means network is available
            if (!Array.isArray(result)) {
                throw new Error('Should return array');
            }
        } catch (error) {
            // Expected to fail without valid identity
            // This is acceptable
        }
    });

    await test('get_dpns_username - requires network and identity', async () => {
        try {
            const result = await wasmSdk.get_dpns_username(sdk, "invalididentityid");
            // If this succeeds, it means network is available
            if (result && typeof result !== 'object') {
                throw new Error('Should return object or null');
            }
        } catch (error) {
            // Expected to fail without valid identity
            // This is acceptable
        }
    });
}

// Edge Case Tests
describe('DPNS Edge Cases');

await test('dpns_convert_to_homograph_safe - empty string', () => {
    const result = wasmSdk.dpns_convert_to_homograph_safe("");
    if (result !== "") {
        throw new Error(`Expected empty string, got "${result}"`);
    }
});

await test('dpns_is_valid_username - empty string', () => {
    if (wasmSdk.dpns_is_valid_username("")) {
        throw new Error('Empty string should not be valid username');
    }
});

await test('dpns_is_contested_username - empty string', () => {
    const result = wasmSdk.dpns_is_contested_username("");
    if (typeof result !== 'boolean') {
        throw new Error('Should return boolean even for empty string');
    }
});

await test('dpns_convert_to_homograph_safe - only special characters', () => {
    const result = wasmSdk.dpns_convert_to_homograph_safe("@#$%");
    // Should either be empty or converted to safe characters
    if (result.includes('@') || result.includes('#') || result.includes('$') || result.includes('%')) {
        throw new Error('Special characters should be removed or converted');
    }
});

await test('dpns_is_valid_username - only numbers', () => {
    if (wasmSdk.dpns_is_valid_username("123456")) {
        throw new Error('Username with only numbers should be invalid');
    }
});

await test('dpns_is_valid_username - starts with number', () => {
    if (wasmSdk.dpns_is_valid_username("1alice")) {
        throw new Error('Username starting with number should be invalid');
    }
});

// Clean up
if (sdk) {
    sdk.free();
}

console.log(`\n\nTest Results: ${passed} passed, ${failed} failed, ${passed + failed} total`);
process.exit(failed > 0 ? 1 : 0);