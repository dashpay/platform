const fs = require('fs');
const path = require('path');

// Load the WASM file
const wasmPath = path.join(__dirname, 'pkg', 'wasm_sdk_bg.wasm');
const wasmBuffer = fs.readFileSync(wasmPath);

// Create a simple fetch implementation for Node.js
global.fetch = require('node-fetch');
global.Headers = fetch.Headers;
global.Request = fetch.Request;
global.Response = fetch.Response;

// Load the JS bindings
const wasm = require('./pkg/wasm_sdk.js');

async function testBalance() {
    try {
        console.log('Initializing WASM module...');
        await wasm.default(wasmBuffer);
        
        console.log('Building SDK for mainnet...');
        const sdk = await wasm.WasmSdkBuilder.new_mainnet().build();
        
        const identityId = '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk';
        console.log(`\nFetching balance for identity: ${identityId}`);
        
        // Create fetch options
        const options = new wasm.FetchOptions();
        options.prove = true;
        options.timeout = 30000;
        options.retries = 3;
        
        try {
            // Try to fetch the identity
            console.log('Fetching identity with proof...');
            const identity = await wasm.fetchIdentity(sdk, identityId, options);
            
            const balance = identity.balance();
            console.log(`\n✓ Success!`);
            console.log(`Balance: ${balance} credits`);
            console.log(`Balance in DASH: ${balance / 100000000} DASH`);
            console.log(`Revision: ${identity.revision()}`);
            console.log(`Public keys: ${identity.publicKeysCount()}`);
            
        } catch (error) {
            console.error('Error fetching identity:', error.message);
            
            // Try direct balance fetch
            console.log('\nTrying direct balance fetch...');
            const balance = await wasm.fetchIdentityBalance(sdk, identityId, options);
            console.log(`Balance: ${balance} credits`);
            console.log(`Balance in DASH: ${balance / 100000000} DASH`);
        }
        
    } catch (error) {
        console.error('Error:', error.message);
        console.error('Stack:', error.stack);
    }
}

testBalance();