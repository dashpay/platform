// Shared Worker for WASM SDK
let sdk = null;
let isInitialized = false;
let initializationPromise = null;
const connections = new Set();

// Import the WASM SDK
importScripts('./pkg/wasm_sdk.js');

// Import with correct variable name
const wasm_bindgen = self.wasm_bindgen;

async function initializeSdk() {
    if (isInitialized) return sdk;
    
    if (initializationPromise) {
        return initializationPromise;
    }
    
    initializationPromise = (async () => {
        try {
            postMessage({ type: 'progress', percent: 10, text: 'Loading WASM module...' });
            
            await wasm_bindgen('./pkg/wasm_sdk_bg.wasm');
            
            postMessage({ type: 'progress', percent: 50, text: 'Initializing SDK...' });
            
            // Use the same initialization as index.html
            sdk = wasm_bindgen.WasmSdkBuilder.new_testnet().build();
            
            postMessage({ type: 'progress', percent: 90, text: 'Finalizing...' });
            
            isInitialized = true;
            
            postMessage({ type: 'progress', percent: 100, text: 'Ready!' });
            postMessage({ type: 'initialized', success: true });
            
            return sdk;
        } catch (error) {
            postMessage({ type: 'initialized', success: false, error: error.message });
            throw error;
        }
    })();
    
    return initializationPromise;
}

// Broadcast message to all connected ports
function postMessage(message) {
    connections.forEach(port => {
        port.postMessage(message);
    });
}

// Handle connections
self.onconnect = function(e) {
    const port = e.ports[0];
    connections.add(port);
    
    port.onmessage = async function(e) {
        const { type, id, method, args } = e.data;
        
        try {
            switch (type) {
                case 'init':
                    await initializeSdk();
                    port.postMessage({ type: 'initComplete', id });
                    break;
                    
                case 'checkStatus':
                    port.postMessage({ 
                        type: 'status', 
                        id, 
                        initialized: isInitialized 
                    });
                    break;
                    
                case 'execute':
                    if (!sdk) {
                        await initializeSdk();
                    }
                    
                    // Map method names to correct WASM functions
                    let result;
                    switch (method) {
                        case 'identity_fetch':
                            result = await wasm_bindgen.identity_fetch(sdk, ...args);
                            break;
                        case 'get_identity_balance':
                            result = await wasm_bindgen.get_identity_balance(sdk, ...args);
                            break;
                        case 'get_identity_keys':
                            result = await wasm_bindgen.get_identity_keys(sdk, ...args);
                            break;
                        case 'get_data_contract':
                            result = await wasm_bindgen.data_contract_fetch(sdk, ...args);
                            break;
                        case 'get_documents':
                            result = await wasm_bindgen.get_documents(sdk, ...args);
                            break;
                        default:
                            // Try to call the method directly if it exists
                            if (typeof wasm_bindgen[method] === 'function') {
                                result = await wasm_bindgen[method](sdk, ...args);
                            } else if (typeof sdk[method] === 'function') {
                                result = await sdk[method](...args);
                            } else {
                                throw new Error(`Method ${method} not found`);
                            }
                    }
                    
                    port.postMessage({ 
                        type: 'result', 
                        id, 
                        result 
                    });
                    break;
                    
                default:
                    port.postMessage({ 
                        type: 'error', 
                        id, 
                        error: 'Unknown message type' 
                    });
            }
        } catch (error) {
            port.postMessage({ 
                type: 'error', 
                id, 
                error: error.message 
            });
        }
    };
    
    // Remove port when disconnected
    port.onmessageerror = () => {
        connections.delete(port);
    };
    
    // Send initial status
    port.postMessage({ 
        type: 'connected', 
        initialized: isInitialized 
    });
};