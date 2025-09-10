#!/usr/bin/env node
// token-pricing.test.mjs - Tests for token pricing query functions

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
import init from '../pkg/dash_wasm_sdk.js';
import { WasmSDK } from '../src-js/index.js';

// Initialize WASM
console.log('Initializing WASM SDK...');
const wasmPath = join(__dirname, '../pkg/dash_wasm_sdk_bg.wasm');
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

console.log('\nToken Pricing Query Tests\n');

// Test values
const TOKEN_CONTRACT_2 = 'H7FRpZJqZK933r9CzZMsCuf1BM34NT5P2wSJyjDkprqy';

// Initialize SDK - prefetch quorums for trusted mode
console.log('Prefetching trusted quorums...');
try {
    await wasmSdk.prefetch_trusted_quorums_testnet();
    console.log('Quorums prefetched successfully');
} catch (error) {
    console.log('Warning: Could not prefetch quorums:', error.message);
}

// Use trusted builder as required for WASM
const builder = wasmSdk.WasmSdkBuilder.new_testnet_trusted();
const sdk = await builder.build();

// Token Pricing Tests
describe('Token Pricing Helper Functions');

await test('calculate_token_id_from_contract - calculate token ID from contract and position', async () => {
    const contractId = TOKEN_CONTRACT_2;
    const position = 0;
    
    const tokenId = wasmSdk.calculate_token_id_from_contract(contractId, position);
    console.log(`   Contract: ${contractId}`);
    console.log(`   Position: ${position}`);
    console.log(`   Calculated Token ID: ${tokenId}`);
    
    // Token ID should be a valid base58 string
    if (!tokenId || typeof tokenId !== 'string' || tokenId.length === 0) {
        throw new Error('Invalid token ID returned');
    }
});

await test('get_token_price_by_contract - fetch token price using contract ID and position', async () => {
    try {
        const contractId = TOKEN_CONTRACT_2;
        const position = 0;
        
        const priceInfo = await wasmSdk.get_token_price_by_contract(sdk, contractId, position);
        console.log(`   Contract: ${contractId}`);
        console.log(`   Position: ${position}`);
        console.log(`   Price Info: ${JSON.stringify(priceInfo)}`);
        
        // Verify response structure
        if (!priceInfo || !priceInfo.tokenId) {
            throw new Error('Invalid price info structure');
        }
        
        console.log(`   Token ID: ${priceInfo.tokenId}`);
        console.log(`   Current Price: ${priceInfo.currentPrice}`);
        console.log(`   Base Price: ${priceInfo.basePrice}`);
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   Expected network error (offline)');
        } else if (error.message.includes('No pricing schedule found') || error.message.includes('Token not found')) {
            console.log('   Token pricing not set or token does not exist');
        } else {
            throw error;
        }
    }
});

// Summary
console.log('\n=== Test Summary ===');
console.log(`Passed: ${passed}`);
console.log(`Failed: ${failed}`);
console.log(`Total: ${passed + failed}`);

process.exit(failed > 0 ? 1 : 0);