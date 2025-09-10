#!/usr/bin/env node

/**
 * DPNS Management Example
 * 
 * Comprehensive demonstration of Dash Platform Name Service operations.
 * Shows username validation, homograph safety, contest detection, name resolution, and availability.
 * 
 * Usage: node examples/dpns-management.mjs [username] [--network=testnet|mainnet] [--no-proofs] [--debug]
 */

import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
import { webcrypto } from 'crypto';

// Set up Node.js environment for WASM
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
if (!global.crypto) global.crypto = webcrypto;

// Import JavaScript wrapper (the correct approach)
import init from '../pkg/dash_wasm_sdk.js';
import { WasmSDK } from '../src-js/index.js';

async function main() {
    console.log('🌐 Comprehensive DPNS Management Example');
    console.log('='.repeat(50));
    
    // Parse command line arguments
    const args = process.argv.slice(2);
    const testUsername = args.find(arg => !arg.startsWith('--')) || 'alice';
    const network = args.find(arg => arg.startsWith('--network='))?.split('=')[1] || 'testnet';
    const useProofs = !args.includes('--no-proofs');
    const debugMode = args.includes('--debug');
    
    console.log(`🎯 Username: ${testUsername}`);
    console.log(`🌐 Network: ${network.toUpperCase()}`);
    console.log(`🔒 Proofs: ${useProofs ? 'ENABLED' : 'DISABLED'}`);
    if (debugMode) console.log(`🐛 Debug: ENABLED`);
    
    try {
        // Pre-load WASM for Node.js compatibility
        console.log('\n📦 Pre-loading WASM for Node.js...');
        const wasmPath = join(__dirname, '../pkg/dash_wasm_sdk_bg.wasm');
        await init(readFileSync(wasmPath));
        
        // Initialize JavaScript wrapper
        console.log('📦 Initializing JavaScript wrapper...');
        const sdk = new WasmSDK({
            network: network,
            proofs: useProofs,
            debug: debugMode
        });
        await sdk.initialize();
        console.log('✅ SDK initialized successfully\n');
        
        // === USERNAME VALIDATION ===
        console.log('✅ USERNAME VALIDATION');
        console.log('-'.repeat(25));
        
        const testUsernames = [
            testUsername,
            'valid-username',
            'test123',
            'ab',           // too short
            'a'.repeat(64), // too long
            '-invalid',     // starts with hyphen
            'invalid-',     // ends with hyphen
            'alice--bob',   // double hyphen
            'alice@bob',    // special chars
            'alice bob'     // spaces
        ];
        
        for (const username of testUsernames) {
            const isValid = await sdk.dpnsIsValidUsername(username);
            const status = isValid ? '✅' : '❌';
            const reason = isValid ? 'valid' : 'invalid';
            console.log(`${status} "${username}": ${reason}`);
        }
        
        // === HOMOGRAPH SAFETY ===
        console.log('\n🛡️  HOMOGRAPH SAFETY CONVERSION');
        console.log('-'.repeat(40));
        
        const homographExamples = [
            'Alice',           // uppercase
            'BOB123',         // mixed case
            'IlIooLi',        // homograph characters
            'test-name',      // hyphens
            'user@domain',    // special chars
            'tеst',          // Cyrillic 'е'
            '',              // empty string
            '@#$%'           // only special chars
        ];
        
        for (const example of homographExamples) {
            const safe = await sdk.dpnsConvertToHomographSafe(example);
            console.log(`✓ "${example}" → "${safe}"`);
        }
        
        // === CONTEST DETECTION ===
        console.log('\n⚔️  CONTEST DETECTION');
        console.log('-'.repeat(25));
        
        const contestExamples = [
            'alice',
            'bob', 
            'test',
            'uniquename123456789',
            'a',               // single letter
            'abc',            // three letters
            testUsername
        ];
        
        for (const username of contestExamples) {
            const isContested = await sdk.dpnsIsContestedUsername(username);
            const status = isContested ? '⚔️ CONTESTED' : '✅ Not contested';
            console.log(`${status}: "${username}"`);
        }
        
        // === NAME RESOLUTION (Network Operations) ===
        console.log('\n🔍 NAME RESOLUTION & AVAILABILITY');
        console.log('-'.repeat(40));
        
        const nameExamples = [
            `${testUsername}.dash`,
            'alice.dash',
            'nonexistent.dash',
            'test.dash'
        ];
        
        console.log('Name Resolution:');
        for (const name of nameExamples) {
            try {
                const resolved = await sdk.dpnsResolveName(name);
                if (resolved) {
                    console.log(`✅ "${name}" resolves to identity: ${resolved.identityId || resolved.identity || 'N/A'}`);
                } else {
                    console.log(`⚠️ "${name}" not found`);
                }
            } catch (error) {
                console.log(`⚠️ "${name}" resolution failed: ${error.message.split(' ').slice(0, 5).join(' ')}...`);
            }
        }
        
        console.log('\nName Availability:');
        const availabilityExamples = [
            testUsername,
            'probablyavailable123456789',
            'alice',
            'test'
        ];
        
        for (const username of availabilityExamples) {
            try {
                const isAvailable = await sdk.dpnsIsNameAvailable(username);
                const status = isAvailable ? '✅ Available' : '❌ Taken';
                console.log(`${status}: "${username}"`);
            } catch (error) {
                console.log(`⚠️ "${username}" availability check failed: ${error.message.split(' ').slice(0, 5).join(' ')}...`);
            }
        }
        
        // === DPNS WORKFLOW EXAMPLES ===
        console.log('\n🚀 DPNS WORKFLOW EXAMPLES');
        console.log('-'.repeat(35));
        
        // Example 1: Username Registration Validation
        console.log('Example 1: Username Registration Validation');
        const proposedUsername = 'mynewusername';
        
        const step1 = await sdk.dpnsIsValidUsername(proposedUsername);
        console.log(`✓ Step 1 - Valid format: ${step1}`);
        
        const step2 = await sdk.dpnsConvertToHomographSafe(proposedUsername);
        console.log(`✓ Step 2 - Homograph-safe: "${step2}"`);
        
        const step3 = await sdk.dpnsIsContestedUsername(step2);
        console.log(`✓ Step 3 - Is contested: ${step3}`);
        
        try {
            const step4 = await sdk.dpnsIsNameAvailable(step2);
            console.log(`✓ Step 4 - Is available: ${step4}`);
            
            if (step1 && !step3 && step4) {
                console.log(`🎉 "${proposedUsername}" is ready for registration!`);
            } else {
                console.log(`⚠️ "${proposedUsername}" cannot be registered (validation failed)`);
            }
        } catch (error) {
            console.log(`⚠️ Step 4 - Availability check requires network connection`);
        }
        
        // Example 2: Bulk Username Validation
        console.log('\nExample 2: Bulk Username Validation');
        const bulkUsernames = ['alice', 'bob123', 'test-user', 'invalid@', 'toolongusernamethatexceedslimits'];
        
        console.log('Batch validation results:');
        for (const username of bulkUsernames) {
            const isValid = await sdk.dpnsIsValidUsername(username);
            const isContested = await sdk.dpnsIsContestedUsername(username);
            const safe = await sdk.dpnsConvertToHomographSafe(username);
            
            console.log(`  "${username}": valid=${isValid}, contested=${isContested}, safe="${safe}"`);
        }
        
        // Example 3: Domain Search Pattern
        console.log('\nExample 3: Domain Search Pattern');
        const searchTerms = ['alice', 'test', 'dash'];
        
        for (const term of searchTerms) {
            try {
                const fullDomain = `${term}.dash`;
                const resolved = await sdk.dpnsResolveName(fullDomain);
                console.log(`✓ Search "${term}": ${resolved ? 'Found' : 'Not found'}`);
            } catch (error) {
                console.log(`⚠️ Search "${term}": ${error.message.includes('network') ? 'Network required' : 'Error'}`);
            }
        }
        
        // === SUMMARY ===
        console.log('\n📊 DPNS OPERATIONS SUMMARY');
        console.log('-'.repeat(30));
        console.log(`✅ Username validation: ${testUsernames.length} examples tested`);
        console.log(`✅ Homograph safety: ${homographExamples.length} conversions demonstrated`);
        console.log(`✅ Contest detection: ${contestExamples.length} usernames checked`);
        console.log(`✅ Name resolution: ${nameExamples.length} domain lookups attempted`);
        console.log(`✅ Availability checking: ${availabilityExamples.length} usernames tested`);
        console.log(`✅ Practical workflows: 3 complete examples`);
        
        // Clean up
        await sdk.destroy();
        console.log('\n🎉 DPNS management demonstration completed successfully!');
        
    } catch (error) {
        console.log(`❌ DPNS operations failed: ${error.message}`);
        
        if (error.message.includes('fetch') || error.message.includes('network')) {
            console.log('🌐 Network operations require online connection');
        }
        
        console.log('\nFor debugging:');
        console.log('1. Check network connectivity for resolution/availability operations');
        console.log('2. Try with --no-proofs for faster testing');
        console.log('3. Use --debug for detailed output');
        console.log('4. Test different usernames');
        
        process.exit(1);
    }
}

await main();