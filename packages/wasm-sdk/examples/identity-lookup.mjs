#!/usr/bin/env node

/**
 * Simple Identity Lookup CLI - No Proof Verification
 * 
 * Uses identity_fetch_unproved for cleaner output without massive proof data.
 * 
 * Usage: node examples/identity-lookup-simple.mjs <identity-id>
 */

import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
import { webcrypto } from 'crypto';

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

// Set up environment
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
if (!global.crypto) global.crypto = webcrypto;

// Import WASM SDK
import init, { 
    WasmSdkBuilder, 
    identity_fetch_unproved,
    get_identity_balance,
    prefetch_trusted_quorums_testnet,
    prefetch_trusted_quorums_mainnet
} from '../pkg/wasm_sdk.js';

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
    console.log('🔍 Simple Identity Lookup (No Proofs)');
    console.log('='.repeat(40));
    
    // Load environment configuration
    const env = loadEnv();
    
    // Use command line arg or default from .env
    const identityId = process.argv[2] || env.IDENTITY_ID;
    const network = env.NETWORK || 'testnet';
    
    if (!identityId) {
        console.log('Usage: node examples/identity-lookup-simple.mjs [identity-id]');
        console.log('');
        console.log('If no identity ID provided, will use IDENTITY_ID from .env file');
        console.log('Current .env configuration:');
        console.log(`  NETWORK=${network}`);
        console.log(`  IDENTITY_ID=${env.IDENTITY_ID || 'not set'}`);
        process.exit(1);
    }
    
    console.log(`🌐 Network: ${network.toUpperCase()}`);
    console.log(`🎯 Identity: ${identityId}`);
    
    try {
        console.log('📦 Loading WASM...');
        const wasmPath = join(__dirname, '../pkg/wasm_sdk_bg.wasm');
        await init(readFileSync(wasmPath));
        
        console.log('🔒 Setting up trusted SDK...');
        
        // Use network from .env
        let sdk;
        if (network === 'mainnet') {
            await prefetch_trusted_quorums_mainnet();
            sdk = WasmSdkBuilder.new_mainnet_trusted().build();
        } else {
            await prefetch_trusted_quorums_testnet();
            sdk = WasmSdkBuilder.new_testnet_trusted().build();
        }
        
        console.log(`🔍 Looking up: ${identityId}`);
        
        // Use unproved version for cleaner output
        const identity = await identity_fetch_unproved(sdk, identityId);
        
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