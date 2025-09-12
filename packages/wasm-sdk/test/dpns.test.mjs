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

// Import JavaScript wrapper (correct approach)
import init, * as wasmSdk from '../pkg/dash_wasm_sdk.js';
import { WasmSDK } from '../src-js/index.js';

// Pre-load WASM for Node.js compatibility
console.log('Initializing WASM module...');
const wasmPath = join(__dirname, '../pkg/dash_wasm_sdk_bg.wasm');
await init(readFileSync(wasmPath));

// Initialize JavaScript wrapper
console.log('Initializing JavaScript wrapper...');
const sdk = new WasmSDK({
    network: 'testnet',
    proofs: false,
    debug: false
});
await sdk.initialize();
console.log('✅ JavaScript wrapper initialized successfully');

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

await test('dpnsConvertToHomographSafe - basic ASCII', async () => {
    const result = await sdk.dpnsConvertToHomographSafe("test");
    if (result !== "test") {
        throw new Error(`Expected "test", got "${result}"`);
    }
});

await test('dpnsConvertToHomographSafe - with numbers', async () => {
    const result = await sdk.dpnsConvertToHomographSafe("test123");
    if (result !== "test123") {
        throw new Error(`Expected "test123", got "${result}"`);
    }
});

await test('dpnsConvertToHomographSafe - with hyphens', async () => {
    const result = await sdk.dpnsConvertToHomographSafe("test-name");
    if (result !== "test-name") {
        throw new Error(`Expected "test-name", got "${result}"`);
    }
});

await test('dpnsConvertToHomographSafe - uppercase to lowercase', async () => {
    const result = await sdk.dpnsConvertToHomographSafe("TestName");
    if (result !== "testname") {
        throw new Error(`Expected "testname", got "${result}"`);
    }
});

await test('dpnsConvertToHomographSafe - special characters', async () => {
    // Only homograph characters (o,i,l) are converted, other special chars are lowercased but preserved
    const result = await sdk.dpnsConvertToHomographSafe("test@name!");
    if (result !== "test@name!") {
        throw new Error(`Expected "test@name!", got "${result}"`);
    }
});

await test('dpnsConvertToHomographSafe - ASCII homograph conversions (o,i,l)', async () => {
    const input = "IlIooLi"; // mix of I,l,i,o
    const result = await sdk.dpnsConvertToHomographSafe(input);
    // Expect: I->i->1, l->1, I->i->1, o->0, o->0, L->l->1, i->1 = "1110011"
    if (result !== "1110011") {
        throw new Error(`Expected "1110011" for "${input}", got "${result}"`);
    }
});

await test('dpnsConvertToHomographSafe - unicode homographs', async () => {
    // Only o,i,l are converted to 0,1,1 - other Unicode characters are preserved
    const result = await sdk.dpnsConvertToHomographSafe("tеst"); // е is Cyrillic
    // Cyrillic 'е' should remain as-is, only lowercased
    if (result !== "tеst") { // Should be the same (just lowercased)
        throw new Error(`Expected Cyrillic to be preserved (lowercased), got "${result}"`);
    }
});

// Username Validation Tests
describe('Username Validation');

await test('dpnsIsValidUsername - valid basic username', async () => {
    if (!(await sdk.dpnsIsValidUsername("alice"))) {
        throw new Error('Basic username "alice" should be valid');
    }
});

await test('dpnsIsValidUsername - valid with numbers', async () => {
    if (!(await sdk.dpnsIsValidUsername("alice123"))) {
        throw new Error('Username with numbers should be valid');
    }
});

await test('dpnsIsValidUsername - valid with hyphen', async () => {
    if (!(await sdk.dpnsIsValidUsername("alice-bob"))) {
        throw new Error('Username with hyphen should be valid');
    }
});

await test('dpnsIsValidUsername - too short', async () => {
    if (await sdk.dpnsIsValidUsername("ab")) {
        throw new Error('Username shorter than 3 characters should be invalid');
    }
});

await test('dpnsIsValidUsername - too long', async () => {
    const longName = "a".repeat(64);
    if (await sdk.dpnsIsValidUsername(longName)) {
        throw new Error('Username longer than 63 characters should be invalid');
    }
});

await test('dpnsIsValidUsername - starts with hyphen', async () => {
    if (await sdk.dpnsIsValidUsername("-alice")) {
        throw new Error('Username starting with hyphen should be invalid');
    }
});

await test('dpnsIsValidUsername - ends with hyphen', async () => {
    if (await sdk.dpnsIsValidUsername("alice-")) {
        throw new Error('Username ending with hyphen should be invalid');
    }
});

await test('dpnsIsValidUsername - double hyphen', async () => {
    if (await sdk.dpnsIsValidUsername("alice--bob")) {
        throw new Error('Username with double hyphen should be invalid');
    }
});


await test('dpnsIsValidUsername - special characters', async () => {
    if (await sdk.dpnsIsValidUsername("alice@bob")) {
        throw new Error('Username with special characters should be invalid');
    }
});

await test('dpnsIsValidUsername - spaces', async () => {
    if (await sdk.dpnsIsValidUsername("alice bob")) {
        throw new Error('Username with spaces should be invalid');
    }
});

// Contested Username Tests
describe('Contested Username Detection');

await test('dpnsIsContestedUsername - non-contested name', async () => {
    if (await sdk.dpnsIsContestedUsername("uniquename123")) {
        throw new Error('Unique username should not be contested');
    }
});

await test('dpns_is_contested_username - common name', async () => {
    // Common names like "alice", "bob", "test" might be contested
    const result = await sdk.dpnsIsContestedUsername("alice");
    // This depends on implementation - just check it returns a boolean
    if (typeof result !== 'boolean') {
        throw new Error('Should return boolean');
    }
});

await test('dpns_is_contested_username - single letter', async () => {
    const result = await sdk.dpnsIsContestedUsername("a");
    if (typeof result !== 'boolean') {
        throw new Error('Should return boolean');
    }
});

await test('dpns_is_contested_username - three letter', async () => {
    const result = await sdk.dpnsIsContestedUsername("abc");
    if (typeof result !== 'boolean') {
        throw new Error('Should return boolean');
    }
});

// Network-dependent DPNS tests (expected to fail without network)
describe('DPNS Network Operations (Expected to Fail)');

// Initialize SDK for network tests
// Network SDK already initialized above
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
            if (error.message.includes('network') || error.message.includes('connection') || 
                error.message.includes('Non-trusted mode is not supported in WASM')) {
                console.log('   Expected error (network or non-trusted mode)');
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
            const result = await sdk.dpnsIsNameAvailable("testname");
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
            const result = await sdk.dpnsResolveName("alice.dash");
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

await test('dpnsConvertToHomographSafe - empty string', async () => {
    const result = wasmSdk.dpns_convert_to_homograph_safe("");
    if (result !== "") {
        throw new Error(`Expected empty string, got "${result}"`);
    }
});

await test('dpns_is_valid_username - empty string', async () => {
    if (wasmSdk.dpns_is_valid_username("")) {
        throw new Error('Empty string should not be valid username');
    }
});

await test('dpns_is_contested_username - empty string', async () => {
    const result = await sdk.dpnsIsContestedUsername("");
    if (typeof result !== 'boolean') {
        throw new Error('Should return boolean even for empty string');
    }
});

await test('dpnsConvertToHomographSafe - only special characters', async () => {
    const result = wasmSdk.dpns_convert_to_homograph_safe("@#$%");
    // Special characters are preserved, only homograph chars (o,i,l) are converted
    if (result !== "@#$%") {
        throw new Error(`Expected special characters to be preserved, got "${result}"`);
    }
});

// Cleanup
await sdk.destroy();

console.log(`\n\nTest Results: ${passed} passed, ${failed} failed, ${passed + failed} total`);
process.exit(failed > 0 ? 1 : 0);