# Vanilla JavaScript Integration Example

This guide demonstrates how to integrate the Dash Platform WASM SDK into a vanilla JavaScript application using the WasmSDK wrapper class.

## Table of Contents

- [Installation](#installation)
- [Basic Setup](#basic-setup)
- [Configuration](#configuration)
- [Identity Operations](#identity-operations)
- [DPNS Operations](#dpns-operations)
- [Data Contract Operations](#data-contract-operations)
- [Token Operations](#token-operations)
- [Wallet Operations](#wallet-operations)
- [Error Handling](#error-handling)
- [Troubleshooting](#troubleshooting)
- [Complete Example](#complete-example)

## Installation

### Browser Environment

```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Dash Platform WASM SDK Example</title>
</head>
<body>
    <div id="app">
        <h1>Dash Platform WASM SDK</h1>
        <div id="results"></div>
        <button onclick="runExample()">Run Example</button>
    </div>

    <!-- Load the SDK as ES module -->
    <script type="module">
        import { WasmSDK } from './path/to/wasm-sdk/src-js/index.js';
        window.WasmSDK = WasmSDK;
    </script>
    
    <script type="module" src="app.js"></script>
</body>
</html>
```

### Node.js Environment

```bash
npm install @dashevo/wasm-sdk
```

```javascript
import { WasmSDK } from '@dashevo/wasm-sdk';
```

## Basic Setup

### Initialize the SDK

```javascript
// app.js
import { WasmSDK, WasmSDKError, WasmInitializationError, WasmOperationError } from './path/to/wasm-sdk/src-js/index.js';

class DashPlatformApp {
    constructor() {
        this.sdk = null;
        this.resultContainer = document.getElementById('results');
        this.setupSDK();
    }

    async setupSDK() {
        try {
            // Configure SDK for testnet
            this.sdk = new WasmSDK({
                network: 'testnet',
                transport: {
                    url: 'https://52.12.176.90:1443/',
                    timeout: 30000,
                    retries: 3
                },
                proofs: true,
                settings: {
                    connect_timeout_ms: 10000,
                    timeout_ms: 30000,
                    retries: 3,
                    ban_failed_address: true
                }
            });

            // Initialize the SDK
            await this.sdk.initialize();
            
            this.logResult('✅ SDK initialized successfully');
            
            // Optional: Get SDK version
            const version = this.sdk.getVersion();
            this.logResult(`📋 SDK Version: ${version}`);

        } catch (error) {
            this.handleError('SDK Initialization', error);
        }
    }

    logResult(message) {
        const p = document.createElement('p');
        p.textContent = `${new Date().toLocaleTimeString()}: ${message}`;
        this.resultContainer.appendChild(p);
        console.log(message);
    }

    handleError(operation, error) {
        let errorMessage = `❌ ${operation} failed: ${error.message}`;
        
        if (error instanceof WasmInitializationError) {
            errorMessage += ' (Initialization Error)';
        } else if (error instanceof WasmOperationError) {
            errorMessage += ' (Operation Error)';
        }

        this.logResult(errorMessage);
        console.error(`${operation} error:`, error);
    }
}

// Global app instance
let app = null;

// Initialize app when DOM is loaded
document.addEventListener('DOMContentLoaded', () => {
    app = new DashPlatformApp();
});
```

## Configuration

### Environment-Specific Configuration

```javascript
class ConfigManager {
    static getConfig(environment = 'testnet') {
        const configs = {
            testnet: {
                network: 'testnet',
                transport: {
                    url: 'https://52.12.176.90:1443/',
                    timeout: 30000,
                    retries: 3
                },
                proofs: true
            },
            mainnet: {
                network: 'mainnet',
                transport: {
                    url: 'https://your-mainnet-endpoint:1443/',
                    timeout: 30000,
                    retries: 3
                },
                proofs: true
            },
            local: {
                network: 'testnet',
                transport: {
                    url: 'http://localhost:1443/',
                    timeout: 30000,
                    retries: 3
                },
                proofs: false // Disable proofs for local development
            }
        };

        return configs[environment] || configs.testnet;
    }

    static createSDK(environment = 'testnet') {
        const config = this.getConfig(environment);
        return new WasmSDK(config);
    }
}

// Usage
const sdk = ConfigManager.createSDK('testnet');
```

## Identity Operations

### Fetch Identity Information

```javascript
async function fetchIdentityExample() {
    try {
        const identityId = 'your-identity-id-here';
        
        // Fetch with proof (default)
        const identity = await app.sdk.getIdentity(identityId);
        app.logResult(`👤 Identity fetched: ${JSON.stringify(identity, null, 2)}`);

        // Fetch identity keys
        const keys = await app.sdk.getIdentityKeys(identityId);
        app.logResult(`🔑 Identity keys: ${JSON.stringify(keys, null, 2)}`);

        // Get identity balance
        const balance = await app.sdk.getIdentityBalance(identityId);
        app.logResult(`💰 Identity balance: ${balance} credits`);

        // Get identity nonce
        const nonce = await app.sdk.getIdentityNonce(identityId);
        app.logResult(`🔢 Identity nonce: ${nonce}`);

    } catch (error) {
        app.handleError('Identity Operations', error);
    }
}
```

### Create New Identity

```javascript
async function createIdentityExample() {
    try {
        // This requires an asset lock transaction - typically done on the backend
        const assetLockProof = 'your-asset-lock-proof-hex';
        const assetLockPrivateKey = 'your-asset-lock-private-key';
        const publicKeysJson = JSON.stringify([
            {
                "id": 0,
                "purpose": 0,
                "securityLevel": 0,
                "keyType": 0,
                "readOnly": false,
                "data": "your-public-key-bytes"
            }
        ]);

        const newIdentity = await app.sdk.createIdentity(
            assetLockProof,
            assetLockPrivateKey,
            publicKeysJson
        );

        app.logResult(`🆕 New identity created: ${JSON.stringify(newIdentity, null, 2)}`);
        
    } catch (error) {
        app.handleError('Identity Creation', error);
    }
}
```

## DPNS Operations

### Username Validation and Registration

```javascript
async function dpnsExample() {
    try {
        const username = 'test-username';

        // Validate username format
        const isValid = app.sdk.isDpnsUsernameValid(username);
        app.logResult(`✅ Username "${username}" is valid: ${isValid}`);

        // Check if username is contested
        const isContested = app.sdk.isDpnsUsernameContested(username);
        app.logResult(`⚠️ Username "${username}" is contested: ${isContested}`);

        // Convert to homograph-safe format
        const safeName = app.sdk.dpnsConvertToHomographSafe(username);
        app.logResult(`🔒 Homograph-safe name: "${safeName}"`);

        // Check name availability
        const isAvailable = await app.sdk.isDpnsNameAvailable(username);
        app.logResult(`📋 Username "${username}" is available: ${isAvailable}`);

        // Resolve existing name (if available)
        if (!isAvailable) {
            const resolved = await app.sdk.resolveDpnsName(username);
            app.logResult(`🔍 Resolved name: ${JSON.stringify(resolved, null, 2)}`);
        }

    } catch (error) {
        app.handleError('DPNS Operations', error);
    }
}
```

## Data Contract Operations

### Working with Data Contracts

```javascript
async function dataContractExample() {
    try {
        const contractId = 'your-data-contract-id';

        // Fetch data contract
        const contract = await app.sdk.getDataContract(contractId);
        app.logResult(`📄 Data contract: ${JSON.stringify(contract, null, 2)}`);

        // Get contract history
        const history = await app.sdk.getDataContractHistory(contractId, {
            limit: 10,
            prove: true
        });
        app.logResult(`📚 Contract history: ${JSON.stringify(history, null, 2)}`);

        // Fetch multiple contracts
        const contracts = await app.sdk.getDataContracts([contractId]);
        app.logResult(`📄 Multiple contracts: ${JSON.stringify(contracts, null, 2)}`);

    } catch (error) {
        app.handleError('Data Contract Operations', error);
    }
}
```

## Token Operations

### Token Calculations and Queries

```javascript
async function tokenExample() {
    try {
        const contractId = 'your-token-contract-id';
        const tokenPosition = 0;

        // Calculate token ID
        const tokenId = app.sdk.calculateTokenId(contractId, tokenPosition);
        app.logResult(`🪙 Token ID: ${tokenId}`);

        // Get token price
        const price = await app.sdk.getTokenPriceByContract(contractId, tokenPosition);
        app.logResult(`💲 Token price: ${JSON.stringify(price, null, 2)}`);

        // Get identity token balances
        const identityId = 'your-identity-id';
        const balances = await app.sdk.getIdentityTokenBalances(identityId, [tokenId]);
        app.logResult(`💰 Token balances: ${JSON.stringify(balances, null, 2)}`);

    } catch (error) {
        app.handleError('Token Operations', error);
    }
}
```

## Wallet Operations

### Key Derivation

```javascript
async function walletExample() {
    try {
        const mnemonic = 'your twelve word mnemonic phrase here for wallet operations testing';
        const passphrase = null; // Optional passphrase
        const network = 'testnet';

        // Derive standard key
        const path = "m/44'/5'/0'/0/0";
        const derivedKey = app.sdk.deriveKey(mnemonic, passphrase, path, network);
        app.logResult(`🔑 Derived key: ${JSON.stringify(derivedKey, null, 2)}`);

        // Derive DashPay contact key
        const senderIdentityId = 'sender-identity-id';
        const receiverIdentityId = 'receiver-identity-id';
        const account = 0;
        const addressIndex = 0;

        const contactKey = app.sdk.deriveDashPayContactKey(
            mnemonic,
            passphrase,
            senderIdentityId,
            receiverIdentityId,
            account,
            addressIndex,
            network
        );
        app.logResult(`📱 DashPay contact key: ${JSON.stringify(contactKey, null, 2)}`);

    } catch (error) {
        app.handleError('Wallet Operations', error);
    }
}
```

## Error Handling

### Comprehensive Error Management

```javascript
class ErrorHandler {
    static handle(operation, error) {
        // Log structured error information
        const errorInfo = {
            operation,
            timestamp: new Date().toISOString(),
            errorType: error.constructor.name,
            message: error.message,
            code: error.code,
            context: error.context
        };

        console.error('Detailed error info:', errorInfo);

        // Handle different error types
        if (error instanceof WasmInitializationError) {
            return this.handleInitializationError(error);
        } else if (error instanceof WasmOperationError) {
            return this.handleOperationError(error);
        } else {
            return this.handleGenericError(error);
        }
    }

    static handleInitializationError(error) {
        const suggestions = [
            '• Check network connectivity',
            '• Verify WASM files are properly loaded',
            '• Ensure configuration is correct',
            '• Try reinitializing the SDK'
        ];

        return {
            userMessage: 'Failed to initialize SDK',
            suggestions,
            category: 'initialization'
        };
    }

    static handleOperationError(error) {
        const suggestions = [];
        
        if (error.context?.errorCategory === 'network') {
            suggestions.push('• Check network connectivity', '• Verify endpoint URL');
        } else if (error.context?.errorCategory === 'validation') {
            suggestions.push('• Check input parameters', '• Verify data format');
        } else if (error.context?.errorCategory === 'timeout') {
            suggestions.push('• Increase timeout settings', '• Check network stability');
        } else {
            suggestions.push('• Check SDK documentation', '• Verify parameters');
        }

        return {
            userMessage: `Operation failed: ${error.context?.operation || 'Unknown'}`,
            suggestions,
            category: error.context?.errorCategory || 'unknown'
        };
    }

    static handleGenericError(error) {
        return {
            userMessage: 'An unexpected error occurred',
            suggestions: ['• Check console for details', '• Try again later'],
            category: 'generic'
        };
    }
}

// Enhanced error handling in app
class DashPlatformApp {
    // ... previous code ...

    handleError(operation, error) {
        const errorResult = ErrorHandler.handle(operation, error);
        
        this.logResult(`❌ ${errorResult.userMessage}`);
        errorResult.suggestions.forEach(suggestion => {
            this.logResult(`   ${suggestion}`);
        });
        
        // Optional: Show user-friendly notifications
        this.showNotification(errorResult.userMessage, 'error');
    }

    showNotification(message, type) {
        // Implement user-friendly notifications
        const notification = document.createElement('div');
        notification.className = `notification ${type}`;
        notification.textContent = message;
        document.body.appendChild(notification);

        setTimeout(() => {
            document.body.removeChild(notification);
        }, 5000);
    }
}
```

## Troubleshooting

### Common Issues and Solutions

#### 1. WASM Module Loading Issues

```javascript
// Solution: Check WASM file availability
async function checkWasmAvailability() {
    try {
        const response = await fetch('./path/to/dash_wasm_sdk_bg.wasm');
        if (!response.ok) {
            throw new Error(`WASM file not found: ${response.status}`);
        }
        app.logResult('✅ WASM file is accessible');
    } catch (error) {
        app.logResult('❌ WASM file check failed: ' + error.message);
    }
}
```

#### 2. Network Configuration Issues

```javascript
// Solution: Test network connectivity
async function testNetworkConnectivity() {
    try {
        const config = app.sdk.getConfig();
        const response = await fetch(config.transport.url, { 
            method: 'HEAD',
            mode: 'cors'
        });
        
        app.logResult(`✅ Network connectivity OK: ${response.status}`);
    } catch (error) {
        app.logResult('❌ Network connectivity failed: ' + error.message);
        app.logResult('   • Check CORS settings');
        app.logResult('   • Verify endpoint URL');
        app.logResult('   • Check firewall settings');
    }
}
```

#### 3. Memory Management

```javascript
// Solution: Proper cleanup
window.addEventListener('beforeunload', () => {
    if (app.sdk) {
        app.sdk.destroy();
        app.logResult('🧹 SDK resources cleaned up');
    }
});
```

#### 4. Browser Compatibility

```javascript
// Solution: Feature detection
function checkBrowserCompatibility() {
    const requiredFeatures = [
        'WebAssembly',
        'fetch',
        'Promise',
        'import' // ES modules
    ];

    const missing = requiredFeatures.filter(feature => {
        if (feature === 'WebAssembly') return typeof WebAssembly === 'undefined';
        if (feature === 'import') return typeof import === 'undefined';
        return typeof window[feature] === 'undefined';
    });

    if (missing.length > 0) {
        app.logResult(`❌ Missing browser features: ${missing.join(', ')}`);
        return false;
    }

    app.logResult('✅ Browser compatibility check passed');
    return true;
}
```

## Complete Example

### Full Application Implementation

```javascript
// complete-example.js
import { WasmSDK, WasmSDKError, WasmInitializationError, WasmOperationError } from './path/to/wasm-sdk/src-js/index.js';

class CompleteDashPlatformExample {
    constructor() {
        this.sdk = null;
        this.resultContainer = document.getElementById('results');
        this.isInitialized = false;
        
        this.init();
    }

    async init() {
        try {
            await this.checkBrowserSupport();
            await this.initializeSDK();
            this.setupEventListeners();
            
            this.logResult('🚀 Application ready!');
        } catch (error) {
            this.handleError('Application Initialization', error);
        }
    }

    async checkBrowserSupport() {
        if (typeof WebAssembly === 'undefined') {
            throw new Error('WebAssembly not supported in this browser');
        }
        
        this.logResult('✅ Browser support verified');
    }

    async initializeSDK() {
        this.sdk = new WasmSDK({
            network: 'testnet',
            transport: {
                url: 'https://52.12.176.90:1443/',
                timeout: 30000,
                retries: 3
            },
            proofs: true
        });

        await this.sdk.initialize();
        this.isInitialized = true;
        
        this.logResult('✅ SDK initialized successfully');
    }

    setupEventListeners() {
        // Add event listeners for UI interactions
        document.getElementById('fetch-identity').addEventListener('click', () => {
            this.runIdentityExample();
        });

        document.getElementById('dpns-example').addEventListener('click', () => {
            this.runDpnsExample();
        });

        document.getElementById('token-example').addEventListener('click', () => {
            this.runTokenExample();
        });

        // Cleanup on page unload
        window.addEventListener('beforeunload', () => {
            if (this.sdk) {
                this.sdk.destroy();
            }
        });
    }

    async runIdentityExample() {
        try {
            const identityId = prompt('Enter Identity ID:');
            if (!identityId) return;

            this.logResult(`🔍 Fetching identity: ${identityId}`);
            
            const identity = await this.sdk.getIdentity(identityId);
            this.logResult(`✅ Identity: ${JSON.stringify(identity, null, 2)}`);
            
        } catch (error) {
            this.handleError('Identity Example', error);
        }
    }

    async runDpnsExample() {
        try {
            const username = prompt('Enter username to check:');
            if (!username) return;

            this.logResult(`🔍 Checking username: ${username}`);
            
            const isValid = this.sdk.isDpnsUsernameValid(username);
            const isAvailable = await this.sdk.isDpnsNameAvailable(username);
            
            this.logResult(`✅ Valid: ${isValid}, Available: ${isAvailable}`);
            
        } catch (error) {
            this.handleError('DPNS Example', error);
        }
    }

    async runTokenExample() {
        try {
            const contractId = prompt('Enter contract ID:');
            if (!contractId) return;

            this.logResult(`🔍 Calculating token ID for contract: ${contractId}`);
            
            const tokenId = this.sdk.calculateTokenId(contractId, 0);
            this.logResult(`✅ Token ID: ${tokenId}`);
            
        } catch (error) {
            this.handleError('Token Example', error);
        }
    }

    logResult(message) {
        const timestamp = new Date().toLocaleTimeString();
        const logEntry = `${timestamp}: ${message}`;
        
        const p = document.createElement('p');
        p.textContent = logEntry;
        this.resultContainer.appendChild(p);
        
        console.log(logEntry);
        
        // Auto-scroll to bottom
        this.resultContainer.scrollTop = this.resultContainer.scrollHeight;
    }

    handleError(operation, error) {
        const errorMsg = `❌ ${operation} failed: ${error.message}`;
        this.logResult(errorMsg);
        
        // Show detailed error in console
        console.error(`Detailed error for ${operation}:`, error);
        
        // Provide user guidance
        if (error instanceof WasmInitializationError) {
            this.logResult('💡 Try refreshing the page and check your network connection');
        } else if (error instanceof WasmOperationError) {
            this.logResult('💡 Check the input parameters and try again');
        }
    }
}

// Initialize when DOM is ready
document.addEventListener('DOMContentLoaded', () => {
    new CompleteDashPlatformExample();
});

// Make available globally for debugging
window.DashPlatformExample = CompleteDashPlatformExample;
```

### HTML Interface

```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Dash Platform WASM SDK - Complete Example</title>
    <style>
        body {
            font-family: Arial, sans-serif;
            max-width: 1200px;
            margin: 0 auto;
            padding: 20px;
            background-color: #f5f5f5;
        }
        
        .container {
            background: white;
            padding: 20px;
            border-radius: 8px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
        }
        
        .controls {
            margin-bottom: 20px;
        }
        
        button {
            background: #008de4;
            color: white;
            border: none;
            padding: 10px 20px;
            margin: 5px;
            border-radius: 5px;
            cursor: pointer;
        }
        
        button:hover {
            background: #006bb3;
        }
        
        #results {
            background: #f8f9fa;
            padding: 15px;
            border-radius: 5px;
            height: 400px;
            overflow-y: auto;
            font-family: 'Courier New', monospace;
            font-size: 12px;
            border: 1px solid #ddd;
        }
        
        #results p {
            margin: 5px 0;
            padding: 3px 0;
            border-bottom: 1px solid #eee;
        }
        
        .notification {
            position: fixed;
            top: 20px;
            right: 20px;
            padding: 15px;
            border-radius: 5px;
            color: white;
            z-index: 1000;
        }
        
        .notification.error {
            background: #dc3545;
        }
        
        .notification.success {
            background: #28a745;
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>🚀 Dash Platform WASM SDK</h1>
        <p>Complete vanilla JavaScript integration example</p>
        
        <div class="controls">
            <button id="fetch-identity">Fetch Identity</button>
            <button id="dpns-example">DPNS Example</button>
            <button id="token-example">Token Example</button>
            <button onclick="document.getElementById('results').innerHTML = ''">Clear Results</button>
        </div>
        
        <div id="results"></div>
    </div>
    
    <script type="module" src="complete-example.js"></script>
</body>
</html>
```

## Performance Tips

1. **SDK Initialization**: Initialize once and reuse the SDK instance
2. **Error Handling**: Always wrap operations in try-catch blocks
3. **Memory Management**: Call `destroy()` when done to free WASM resources
4. **Proof Validation**: Disable proofs for faster operations when security isn't critical
5. **Network Optimization**: Use appropriate timeout and retry settings

## Next Steps

- Explore [React Integration Example](./react-example.md)
- Learn about [Vue.js Integration](./vue-example.md)
- Check out [Angular Integration](./angular-example.md)
- Review the [API Reference](../AI_REFERENCE.md)

## Support

For issues and questions:
- Check the [Troubleshooting](#troubleshooting) section
- Review the [API Reference](../AI_REFERENCE.md)
- Visit the [GitHub repository](https://github.com/dashevo/platform)