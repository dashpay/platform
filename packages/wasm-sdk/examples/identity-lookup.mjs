#!/usr/bin/env node

/**
 * Identity Lookup CLI with Proof Verification Control
 * 
 * Uses the modern JavaScript wrapper with configurable proof verification.
 * 
 * Usage: node examples/identity-lookup.mjs [identity-id] [--no-proofs]
 */

import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
import { webcrypto } from 'crypto';

// Set up Node.js environment for WASM
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
if (!global.crypto) global.crypto = webcrypto;

// Pre-load WASM for Node.js
import init from '../pkg/wasm_sdk.js';

// Load environment configuration
function loadEnv() {
    try {
        const envPath = join(dirname(fileURLToPath(import.meta.url)), '../.env');
        const envFile = readFileSync(envPath, 'utf8');
        const env = {};
        
        for (const line of envFile.split('\n')) {
            const trimmed = line.trim();
            if (trimmed && !trimmed.startsWith('#')) {
                const [key, ...valueParts] = trimmed.split('=');
                if (key && valueParts.length > 0) {
                    // Handle quoted values
                    let value = valueParts.join('=');
                    if (value.startsWith('"') && value.endsWith('"')) {
                        value = value.slice(1, -1);
                    }
                    env[key] = value;
                }
            }
        }
        return env;
    } catch (error) {
        console.log('⚠️ Could not load .env file, using defaults');
        return {};
    }
}

// (Already defined above)

// Import JavaScript wrapper (the correct approach)
import { WasmSDK } from '../src-js/index.js';

// Key type mapping (from Dash Platform specification)
function mapKeyType(type) {
    const keyTypes = {
        0: 'ECDSA_SECP256K1',
        1: 'BLS12_381',
        2: 'ECDSA_HASH160',
        3: 'BIP13_SCRIPT_HASH'
    };
    return keyTypes[type] || `Unknown Type (${type})`;
}

// Key purpose mapping (from Dash Platform specification)  
function mapKeyPurpose(purpose) {
    const keyPurposes = {
        0: 'AUTHENTICATION',
        1: 'ENCRYPTION', 
        2: 'DECRYPTION',
        3: 'WITHDRAW'
    };
    return keyPurposes[purpose] || `Unknown Purpose (${purpose})`;
}

// Security level mapping (from Dash Platform specification)
function mapSecurityLevel(level) {
    const securityLevels = {
        0: 'MASTER',
        1: 'CRITICAL', 
        2: 'HIGH',
        3: 'MEDIUM'
    };
    return securityLevels[level] || `Unknown Level (${level})`;
}

async function main() {
    console.log('🔍 Identity Lookup CLI');
    console.log('='.repeat(30));
    
    // Load environment configuration
    const env = loadEnv();
    
    // Parse command line arguments
    const args = process.argv.slice(2);
    const identityId = args.find(arg => !arg.startsWith('--')) || env.IDENTITY_ID;
    const useProofs = !args.includes('--no-proofs'); // Default to proofs enabled
    const network = env.NETWORK || 'testnet';
    
    if (!identityId) {
        console.log('Usage: node examples/identity-lookup.mjs [identity-id] [--no-proofs]');
        console.log('');
        console.log('Options:');
        console.log('  --no-proofs    Disable proof verification (default: proofs enabled)');
        console.log('');
        console.log('If no identity ID provided, will use IDENTITY_ID from .env file');
        console.log('Current .env configuration:');
        console.log(`  NETWORK=${network}`);
        console.log(`  IDENTITY_ID=${env.IDENTITY_ID || 'not set'}`);
        process.exit(1);
    }
    
    console.log(`🌐 Network: ${network.toUpperCase()}`);
    console.log(`🎯 Identity: ${identityId}`);
    console.log(`🔒 Proofs: ${useProofs ? 'ENABLED' : 'DISABLED'} (default: enabled)`);
    
    try {
        // Pre-load WASM for Node.js compatibility
        console.log('📦 Pre-loading WASM for Node.js...');
        const wasmPath = join(__dirname, '../pkg/wasm_sdk_bg.wasm');
        await init(readFileSync(wasmPath));
        
        console.log('📦 Initializing JavaScript wrapper...');
        
        // Use JavaScript wrapper (the correct approach)
        const sdk = new WasmSDK({
            network: network,
            proofs: useProofs,
            debug: false,
            transport: {
                timeout: 60000,
                retries: 5
            }
        });
        
        await sdk.initialize();
        console.log(`✅ SDK initialized successfully (proofs: ${useProofs ? 'enabled' : 'disabled'})`);
        
        console.log(`🔍 Looking up: ${identityId}`);
        
        // Use JavaScript wrapper method (handles proofs internally)
        const identity = await sdk.getIdentity(identityId);
        
        if (identity) {
            console.log('✅ SUCCESS! Identity found');
            
            // Get complete identity data
            const data = identity.toJSON();
            
            console.log('\n📋 Complete Identity Information:');
            console.log(JSON.stringify(data, null, 2));
            
            // Show formatted summary
            console.log('\n📊 Summary:');
            console.log(`   💰 Balance: ${data.balance || 'N/A'} credits`);
            console.log(`   🔄 Revision: ${data.revision !== undefined ? data.revision : 'N/A'}`);
            console.log(`   🔑 Public Keys: ${data.publicKeys?.length || 0}`);
            
            // Show detailed key information
            if (data.publicKeys && data.publicKeys.length > 0) {
                console.log('\n🔑 Public Key Details:');
                data.publicKeys.forEach((key, index) => {
                    console.log(`   Key ${index + 1}:`);
                    console.log(`     ID: ${key.id !== undefined ? key.id : 'N/A'}`);
                    console.log(`     Type: ${mapKeyType(key.type)}`);
                    console.log(`     Purpose: ${mapKeyPurpose(key.purpose)}`);
                    console.log(`     Security Level: ${mapSecurityLevel(key.securityLevel)}`);
                    if (key.data) {
                        console.log(`     Data: ${key.data.slice(0, 20)}...`);
                    }
                    console.log('');
                });
            }
        } else {
            console.log('❌ Identity not found');
        }
        
    } catch (error) {
        console.log(`❌ Failed: ${error.message}`);
        if (error.message.includes('Non-trusted mode')) {
            console.log('🔧 Trusted mode error still occurring');
        }
    }
}

await main();