#!/usr/bin/env node

import { Worker } from 'worker_threads';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
import { readFileSync } from 'fs';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Load WASM module in main thread first
const wasmBuffer = readFileSync(join(__dirname, '../pkg/dash_wasm_sdk_bg.wasm'));

// Test runner
async function runTest() {
    console.log('Testing DashPay Contact Keys (DIP15) Implementation...\n');
    
    const worker = new Worker(join(__dirname, '../shared-sdk-worker.js'), {
        workerData: { wasmBuffer }
    });
    
    let wasmSdk = null;
    let testsPassed = 0;
    let testsFailed = 0;
    
    // Helper to call SDK methods
    const callSdk = (method, ...args) => {
        return new Promise((resolve, reject) => {
            const id = Date.now() + Math.random();
            worker.postMessage({ id, method, args });
            
            const handler = (msg) => {
                if (msg.id === id) {
                    worker.off('message', handler);
                    if (msg.error) {
                        reject(new Error(msg.error));
                    } else {
                        resolve(msg.result);
                    }
                }
            };
            
            worker.on('message', handler);
        });
    };
    
    // Test helper
    const test = async (name, fn) => {
        try {
            await fn();
            console.log(`✓ ${name}`);
            testsPassed++;
        } catch (error) {
            console.error(`✗ ${name}:`, error.message);
            testsFailed++;
        }
    };
    
    // Wait for worker to be ready
    await new Promise(resolve => {
        worker.once('message', (msg) => {
            if (msg.ready) resolve();
        });
    });
    
    // Test constants
    const testMnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    const testPassphrase = "";
    
    // Test identity IDs (from docs.html)
    const senderIdentity = "5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk";
    const receiverIdentity = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec";
    
    // DIP15 Tests
    console.log('=== DIP15 DashPay Contact Keys Tests ===\n');
    
    await test('DIP15 base path for mainnet', async () => {
        const path = "m/9'/5'/15'/0'";
        const result = await callSdk('derive_key_from_seed_with_path', testMnemonic, testPassphrase, path, 'mainnet');
        if (!result) throw new Error('No result');
        if (!result.private_key) throw new Error('No private key');
        if (!result.public_key) throw new Error('No public key');
        if (!result.address) throw new Error('No address');
        console.log(`  Base path address: ${result.address}`);
    });
    
    await test('DIP15 base path for testnet', async () => {
        const path = "m/9'/1'/15'/0'";
        const result = await callSdk('derive_key_from_seed_with_path', testMnemonic, testPassphrase, path, 'testnet');
        if (!result) throw new Error('No result');
        if (!result.private_key) throw new Error('No private key');
        if (!result.public_key) throw new Error('No public key');
        if (!result.address) throw new Error('No address');
        console.log(`  Base path address: ${result.address}`);
    });
    
    await test('DIP15 multiple accounts', async () => {
        for (let account = 0; account < 3; account++) {
            const path = `m/9'/5'/15'/${account}'`;
            const result = await callSdk('derive_key_from_seed_with_path', testMnemonic, testPassphrase, path, 'mainnet');
            if (!result) throw new Error(`No result for account ${account}`);
            console.log(`  Account ${account} address: ${result.address}`);
        }
    });
    
    await test('DIP15 conceptual path structure', async () => {
        // Test the base path that would be extended with identity IDs
        const basePath = "m/9'/5'/15'/0'";
        const result = await callSdk('derive_key_from_seed_with_path', testMnemonic, testPassphrase, basePath, 'mainnet');
        
        // Log what the full path would look like with identity IDs
        const conceptualPath = `${basePath}/${senderIdentity}/${receiverIdentity}/0`;
        console.log(`  Base path: ${basePath}`);
        console.log(`  Conceptual full path: ${conceptualPath}`);
        console.log(`  Note: Full 256-bit identity ID paths require DIP14 implementation`);
        
        if (!result || !result.address) throw new Error('Failed to derive base path');
    });
    
    await test('DIP15 vs DIP13 key isolation', async () => {
        // DIP15 key
        const dip15Path = "m/9'/5'/15'/0'";
        const dip15Key = await callSdk('derive_key_from_seed_with_path', testMnemonic, testPassphrase, dip15Path, 'mainnet');
        
        // DIP13 identity key
        const dip13Path = "m/9'/5'/5'/0'/0'/0'/0'";
        const dip13Key = await callSdk('derive_key_from_seed_with_path', testMnemonic, testPassphrase, dip13Path, 'mainnet');
        
        if (!dip15Key || !dip13Key) throw new Error('Failed to derive keys');
        if (dip15Key.address === dip13Key.address) throw new Error('DIP15 and DIP13 keys should be different');
        
        console.log(`  DIP15 address: ${dip15Key.address}`);
        console.log(`  DIP13 address: ${dip13Key.address}`);
    });
    
    await test('DIP15 deterministic derivation', async () => {
        // Same path should always produce same key
        const path = "m/9'/1'/15'/0'";
        const result1 = await callSdk('derive_key_from_seed_with_path', testMnemonic, testPassphrase, path, 'testnet');
        const result2 = await callSdk('derive_key_from_seed_with_path', testMnemonic, testPassphrase, path, 'testnet');
        
        if (!result1 || !result2) throw new Error('Failed to derive keys');
        if (result1.address !== result2.address) throw new Error('Same path should produce same address');
        if (result1.private_key !== result2.private_key) throw new Error('Same path should produce same private key');
    });
    
    // Summary
    console.log('\n=== DIP15 Test Summary ===');
    console.log(`Passed: ${testsPassed}`);
    console.log(`Failed: ${testsFailed}`);
    console.log(`Total: ${testsPassed + testsFailed}`);
    
    console.log('\n=== DIP15 Implementation Notes ===');
    console.log('1. Current implementation derives base DIP15 paths (m/9\'/coin\'/15\'/account\')');
    console.log('2. Full DIP15 paths include 256-bit identity IDs as per DIP14');
    console.log('3. The WASM SDK can derive the base path, but extending with 256-bit IDs requires additional work');
    console.log('4. UI shows conceptual full path for educational purposes');
    console.log('5. Each user pair gets unique deterministic addresses for privacy');
    
    // Cleanup
    await worker.terminate();
    process.exit(testsFailed > 0 ? 1 : 0);
}

runTest().catch(console.error);