#!/usr/bin/env node
import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
import { webcrypto } from 'crypto';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

if (!global.crypto) {
    Object.defineProperty(global, 'crypto', {
        value: webcrypto,
        writable: true,
        configurable: true
    });
}

import init, * as wasmSdk from './pkg/wasm_sdk.js';

const wasmPath = join(__dirname, 'pkg/wasm_sdk_bg.wasm');
const wasmBuffer = readFileSync(wasmPath);
await init(wasmBuffer);

console.log('Testing address validation...\n');

// Test addresses from the failing test
const mainnetAddress = "XdRhagDMpNbHZSvgMXqkcCCWmrDYYty5Nh";
const testnetAddress = "yXdRhagDMpNbHZSvgMXqkcCCWmrDYYty5Nh";

console.log(`Mainnet address: ${mainnetAddress}`);
console.log(`Validate on mainnet: ${wasmSdk.validate_address(mainnetAddress, "mainnet")}`);
console.log(`Validate on testnet: ${wasmSdk.validate_address(mainnetAddress, "testnet")}`);

console.log(`\nTestnet address: ${testnetAddress}`);
console.log(`Validate on mainnet: ${wasmSdk.validate_address(testnetAddress, "mainnet")}`);
console.log(`Validate on testnet: ${wasmSdk.validate_address(testnetAddress, "testnet")}`);

// Generate a real address to test
console.log('\nGenerating real addresses...');
const mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

const mainnetKey = wasmSdk.derive_key_from_seed_with_path(
    mnemonic,
    undefined,
    "m/44'/5'/0'/0/0",
    "mainnet"
);

const testnetKey = wasmSdk.derive_key_from_seed_with_path(
    mnemonic,
    undefined,
    "m/44'/1'/0'/0/0",
    "testnet"
);

console.log(`\nGenerated mainnet address: ${mainnetKey.address}`);
console.log(`Validate on mainnet: ${wasmSdk.validate_address(mainnetKey.address, "mainnet")}`);

console.log(`\nGenerated testnet address: ${testnetKey.address}`);
console.log(`Validate on testnet: ${wasmSdk.validate_address(testnetKey.address, "testnet")}`);