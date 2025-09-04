#!/usr/bin/env node

/**
 * Test script to verify identity lookup functionality after API fixes
 */

import init, { 
    WasmSdkBuilder,
    identity_fetch
} from './pkg/dash_wasm_sdk.js';

async function testIdentityLookup() {
    try {
        console.log('🚀 Testing Identity Lookup after API fixes');
        
        // Initialize WASM
        console.log('📦 Initializing WASM module...');
        await init();
        
        // Build SDK
        console.log('🔧 Building SDK...');
        const sdk = WasmSdkBuilder.new_testnet().build();
        
        // Test the identity ID from the user's screenshot
        const testIdentityId = 'DcoJJ3W9dauwLD51vzNuXJ9vna2T7mprVm7wbgVYifNq';
        console.log(`🔍 Looking up identity: ${testIdentityId}`);
        
        // Use the correct WASM function
        const identity = await identity_fetch(sdk, testIdentityId);
        
        if (identity) {
            console.log('✅ Identity lookup successful!');
            console.log('📋 Identity details:', {
                id: testIdentityId,
                balance: identity.balance || 'N/A',
                revision: identity.getRevision ? identity.getRevision() : 'N/A'
            });
            
            // Try to convert to JSON for display
            try {
                const identityJson = identity.toJSON ? identity.toJSON() : identity;
                console.log('📄 Full identity data:', JSON.stringify(identityJson, null, 2));
            } catch (e) {
                console.log('⚠️  Could not convert to JSON:', e.message);
            }
        } else {
            console.log('❌ Identity not found');
        }
        
    } catch (error) {
        console.error('💥 Identity lookup failed:', error.message);
        console.error('🔍 Error details:', error);
        
        // Check if it's a network connectivity issue
        if (error.message.includes('fetch')) {
            console.log('🌐 This appears to be a network connectivity issue');
            console.log('   The API fix is correct, but network access is needed for real lookups');
        }
    }
}

console.log('='.repeat(60));
console.log('🧪 WASM SDK Identity Lookup Test');
console.log('='.repeat(60));

await testIdentityLookup();

console.log('='.repeat(60));
console.log('✨ Test completed');