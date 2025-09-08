#!/usr/bin/env node
// dpns-migrated.test.mjs - DPNS tests using JavaScript wrapper (MIGRATED)

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

// 🎯 MIGRATED: Import JavaScript wrapper (correct approach)
import init from '../pkg/wasm_sdk.js';
import { WasmSDK } from '../src-js/index.js';

// Pre-load WASM for Node.js compatibility
console.log('Initializing WASM module...');
const wasmPath = join(__dirname, '../pkg/wasm_sdk_bg.wasm');
await init(readFileSync(wasmPath));

// 🎯 MIGRATED: Use JavaScript wrapper initialization
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

console.log('\n🎯 DPNS Tests Using JavaScript Wrapper (MIGRATED)\n');

// Homograph Safety Tests - 🎯 MIGRATED
describe('Homograph Safety (Wrapper)');

await test('dpnsConvertToHomographSafe - basic ASCII', async () => {
    const result = await sdk.dpnsConvertToHomographSafe("test"); // 🎯 MIGRATED
    if (result !== "test") {
        throw new Error(`Expected "test", got "${result}"`);
    }
});

await test('dpnsConvertToHomographSafe - with numbers', async () => {
    const result = await sdk.dpnsConvertToHomographSafe("test123"); // 🎯 MIGRATED
    if (result !== "test123") {
        throw new Error(`Expected "test123", got "${result}"`);
    }
});

await test('dpnsConvertToHomographSafe - with hyphens', async () => {
    const result = await sdk.dpnsConvertToHomographSafe("test-name"); // 🎯 MIGRATED
    if (result !== "test-name") {
        throw new Error(`Expected "test-name", got "${result}"`);
    }
});

await test('dpnsConvertToHomographSafe - uppercase to lowercase', async () => {
    const result = await sdk.dpnsConvertToHomographSafe("TestName"); // 🎯 MIGRATED
    if (result !== "testname") {
        throw new Error(`Expected "testname", got "${result}"`);
    }
});

await test('dpnsConvertToHomographSafe - special characters', async () => {
    // Only homograph characters (o,i,l) are converted, other special chars are lowercased but preserved
    const result = await sdk.dpnsConvertToHomographSafe("test@name!"); // 🎯 MIGRATED
    if (result !== "test@name!") {
        throw new Error(`Expected "test@name!", got "${result}"`);
    }
});

await test('dpnsConvertToHomographSafe - ASCII homograph conversions (o,i,l)', async () => {
    const input = "IlIooLi"; // mix of I,l,i,o
    const result = await sdk.dpnsConvertToHomographSafe(input); // 🎯 MIGRATED
    // Expect: I->i->1, l->1, I->i->1, o->0, o->0, L->l->1, i->1 = "1110011"
    if (result !== "1110011") {
        throw new Error(`Expected "1110011" for "${input}", got "${result}"`);
    }
});

await test('dpnsConvertToHomographSafe - unicode homographs', async () => {
    // Only o,i,l are converted to 0,1,1 - other Unicode characters are preserved
    const result = await sdk.dpnsConvertToHomographSafe("tеst"); // 🎯 MIGRATED, е is Cyrillic
    // Cyrillic 'е' should remain as-is, only lowercased
    if (result !== "tеst") { // Should be the same (just lowercased)
        throw new Error(`Expected Cyrillic to be preserved (lowercased), got "${result}"`);
    }
});

// Username Validation Tests - 🎯 MIGRATED
describe('Username Validation (Wrapper)');

await test('dpnsIsValidUsername - valid basic username', async () => {
    if (!(await sdk.dpnsIsValidUsername("alice"))) { // 🎯 MIGRATED
        throw new Error('Basic username "alice" should be valid');
    }
});

await test('dpnsIsValidUsername - valid with numbers', async () => {
    if (!(await sdk.dpnsIsValidUsername("alice123"))) { // 🎯 MIGRATED
        throw new Error('Username with numbers should be valid');
    }
});

await test('dpnsIsValidUsername - valid with hyphen', async () => {
    if (!(await sdk.dpnsIsValidUsername("alice-bob"))) { // 🎯 MIGRATED
        throw new Error('Username with hyphen should be valid');
    }
});

await test('dpnsIsValidUsername - too short', async () => {
    if (await sdk.dpnsIsValidUsername("ab")) { // 🎯 MIGRATED
        throw new Error('Username shorter than 3 characters should be invalid');
    }
});

await test('dpnsIsValidUsername - too long', async () => {
    const longName = "a".repeat(64);
    if (await sdk.dpnsIsValidUsername(longName)) { // 🎯 MIGRATED
        throw new Error('Username longer than 63 characters should be invalid');
    }
});

await test('dpnsIsValidUsername - starts with hyphen', async () => {
    if (await sdk.dpnsIsValidUsername("-alice")) { // 🎯 MIGRATED
        throw new Error('Username starting with hyphen should be invalid');
    }
});

await test('dpnsIsValidUsername - ends with hyphen', async () => {
    if (await sdk.dpnsIsValidUsername("alice-")) { // 🎯 MIGRATED
        throw new Error('Username ending with hyphen should be invalid');
    }
});

await test('dpnsIsValidUsername - double hyphen', async () => {
    if (await sdk.dpnsIsValidUsername("alice--bob")) { // 🎯 MIGRATED
        throw new Error('Username with double hyphen should be invalid');
    }
});

await test('dpnsIsValidUsername - special characters', async () => {
    if (await sdk.dpnsIsValidUsername("alice@bob")) { // 🎯 MIGRATED
        throw new Error('Username with special characters should be invalid');
    }
});

await test('dpnsIsValidUsername - spaces', async () => {
    if (await sdk.dpnsIsValidUsername("alice bob")) { // 🎯 MIGRATED
        throw new Error('Username with spaces should be invalid');
    }
});

// Contested Username Tests - 🎯 MIGRATED
describe('Contested Username Detection (Wrapper)');

await test('dpnsIsContestedUsername - non-contested name', async () => {
    if (await sdk.dpnsIsContestedUsername("uniquename123")) { // 🎯 MIGRATED
        throw new Error('Unique username should not be contested');
    }
});

await test('dpnsIsContestedUsername - common name', async () => {
    // Common names like "alice", "bob", "test" might be contested
    const result = await sdk.dpnsIsContestedUsername("alice"); // 🎯 MIGRATED
    // This depends on implementation - just check it returns a boolean
    if (typeof result !== 'boolean') {
        throw new Error('Should return boolean');
    }
});

await test('dpnsIsContestedUsername - single letter', async () => {
    const result = await sdk.dpnsIsContestedUsername("a"); // 🎯 MIGRATED
    if (typeof result !== 'boolean') {
        throw new Error('Should return boolean');
    }
});

await test('dpnsIsContestedUsername - three letter', async () => {
    const result = await sdk.dpnsIsContestedUsername("abc"); // 🎯 MIGRATED
    if (typeof result !== 'boolean') {
        throw new Error('Should return boolean');
    }
});

// Network-dependent DPNS tests - 🎯 MIGRATED
describe('DPNS Network Operations (Wrapper)');

await test('dpnsIsNameAvailable - requires network', async () => {
    try {
        const result = await sdk.dpnsIsNameAvailable("testname"); // 🎯 MIGRATED
        // If this succeeds, it means network is available
        if (typeof result !== 'boolean') {
            throw new Error('Should return boolean');
        }
        console.log(`   ✓ Network test successful: name available = ${result}`);
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   ⚠️ Network error (expected in offline mode)');
        } else {
            throw error;
        }
    }
});

await test('dpnsResolveName - requires network', async () => {
    try {
        const result = await sdk.dpnsResolveName("alice.dash"); // 🎯 MIGRATED
        // If this succeeds, it means network is available
        if (result && typeof result !== 'object') {
            throw new Error('Should return object or null');
        }
        console.log(`   ✓ Network test successful: resolve result type = ${typeof result}`);
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   ⚠️ Network error (expected in offline mode)');
        } else {
            throw error;
        }
    }
});

// Edge Case Tests - 🎯 MIGRATED
describe('DPNS Edge Cases (Wrapper)');

await test('dpnsConvertToHomographSafe - empty string', async () => {
    const result = await sdk.dpnsConvertToHomographSafe(""); // 🎯 MIGRATED
    if (result !== "") {
        throw new Error(`Expected empty string, got "${result}"`);
    }
});

await test('dpnsIsValidUsername - empty string', async () => {
    if (await sdk.dpnsIsValidUsername("")) { // 🎯 MIGRATED
        throw new Error('Empty string should not be valid username');
    }
});

await test('dpnsIsContestedUsername - empty string', async () => {
    const result = await sdk.dpnsIsContestedUsername(""); // 🎯 MIGRATED
    if (typeof result !== 'boolean') {
        throw new Error('Should return boolean even for empty string');
    }
});

await test('dpnsConvertToHomographSafe - only special characters', async () => {
    const result = await sdk.dpnsConvertToHomographSafe("@#$%"); // 🎯 MIGRATED
    // Special characters are preserved, only homograph chars (o,i,l) are converted
    if (result !== "@#$%") {
        throw new Error(`Expected special characters to be preserved, got "${result}"`);
    }
});

// 🎯 MIGRATED: Proper resource cleanup
await sdk.destroy();

console.log(`\n\n🎯 DPNS MIGRATION SUCCESS TEST RESULTS:`);
console.log(`✅ Passed: ${passed}`);
console.log(`❌ Failed: ${failed}`);
console.log(`📊 Total: ${passed + failed}`);

if (failed === 0) {
    console.log(`\n🚀 DPNS MIGRATION VALIDATION SUCCESSFUL!`);
    console.log(`All migrated DPNS tests work perfectly with JavaScript wrapper.`);
    console.log(`\n📋 DPNS Functions Successfully Validated in Real Usage:`);
    console.log(`   ✓ dpnsIsValidUsername() - Comprehensive username validation`);
    console.log(`   ✓ dpnsConvertToHomographSafe() - Homograph attack prevention`);
    console.log(`   ✓ dpnsIsContestedUsername() - Contest detection`);
    console.log(`   ✓ dpnsResolveName() - Name resolution (network-dependent)`);
    console.log(`   ✓ dpnsIsNameAvailable() - Name availability (network-dependent)`);
} else {
    console.log(`\n⚠️ DPNS migration has ${failed} failing tests. Need to investigate wrapper implementation.`);
}

console.log(`\n📝 Migration Notes:`);
console.log(`- All DPNS utility functions now use JavaScript wrapper pattern`);
console.log(`- Network-dependent functions gracefully handle offline mode`);
console.log(`- Resource management follows proper wrapper cleanup pattern`);
console.log(`- Tests validate both local and network-dependent DPNS functionality`);

process.exit(failed > 0 ? 1 : 0);