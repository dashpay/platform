/**
 * Identity Manager - Dash Platform WASM SDK Sample Application
 * Demonstrates identity management operations using the WASM SDK
 */

import init, { 
    WasmSdk, 
    WasmSdkBuilder, 
    identity_fetch,
    get_identity_balance,
    get_identity_keys,
    prefetch_trusted_quorums_mainnet,
    prefetch_trusted_quorums_testnet
} from '../../pkg/wasm_sdk.js';

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

class IdentityManager {
    constructor() {
        this.sdk = null;
        this.currentNetwork = 'testnet';
        this.isInitialized = false;
        this.currentIdentity = null;
        
        // Sample identity for testing (testnet)
        this.sampleIdentityId = '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk';
        
        // Bind methods
        this.initializeSDK = this.initializeSDK.bind(this);
        this.lookupIdentity = this.lookupIdentity.bind(this);
        this.checkBalance = this.checkBalance.bind(this);
        this.viewKeys = this.viewKeys.bind(this);
        this.createIdentity = this.createIdentity.bind(this);
    }

    async init() {
        try {
            await this.initializeEventListeners();
            await this.initializeSDK();
            this.logOperation('System', 'Application initialized successfully');
        } catch (error) {
            this.showError('Failed to initialize application: ' + error.message);
            this.logOperation('System', `Initialization failed: ${error.message}`, 'error');
        }
    }

    initializeEventListeners() {
        // Network selection
        const networkSelect = document.getElementById('networkSelect');
        networkSelect.addEventListener('change', async (e) => {
            this.currentNetwork = e.target.value;
            await this.initializeSDK();
        });

        // Identity lookup
        const lookupBtn = document.getElementById('lookupBtn');
        lookupBtn.addEventListener('click', this.lookupIdentity);

        // Enter key support for identity input
        const identityInput = document.getElementById('identityIdInput');
        identityInput.addEventListener('keypress', (e) => {
            if (e.key === 'Enter') {
                this.lookupIdentity();
            }
        });

        // Balance checking
        const checkBalanceBtn = document.getElementById('checkBalanceBtn');
        checkBalanceBtn.addEventListener('click', this.checkBalance);

        // View keys
        const viewKeysBtn = document.getElementById('viewKeysBtn');
        viewKeysBtn.addEventListener('click', this.viewKeys);

        // Export data
        const exportDataBtn = document.getElementById('exportDataBtn');
        exportDataBtn.addEventListener('click', () => this.exportData());

        // Identity creation
        const createIdentityBtn = document.getElementById('createIdentityBtn');
        createIdentityBtn.addEventListener('click', this.createIdentity);

        // Form validation for identity creation
        const assetLockProof = document.getElementById('assetLockProof');
        const assetLockPrivateKey = document.getElementById('assetLockPrivateKey');
        const publicKeysJson = document.getElementById('publicKeysJson');

        [assetLockProof, assetLockPrivateKey, publicKeysJson].forEach(input => {
            input.addEventListener('input', this.validateCreationForm.bind(this));
        });
    }

    async initializeSDK() {
        try {
            this.updateStatus('connecting', 'Initializing SDK...');
            this.logOperation('SDK', `Initializing for ${this.currentNetwork} network`);

            // Initialize WASM module
            await init();

            // Prefetch trusted quorums (required for WASM)
            this.updateStatus('connecting', 'Prefetching trusted quorums...');
            if (this.currentNetwork === 'mainnet') {
                await prefetch_trusted_quorums_mainnet();
            } else {
                await prefetch_trusted_quorums_testnet();
            }

            // Create trusted SDK builder (WASM only supports trusted mode)
            let builder;
            if (this.currentNetwork === 'mainnet') {
                builder = WasmSdkBuilder.new_mainnet_trusted();
            } else {
                builder = WasmSdkBuilder.new_testnet_trusted();
            }

            // Build SDK instance
            this.sdk = builder.build();
            this.isInitialized = true;

            this.updateStatus('connected', `Connected to ${this.currentNetwork} (trusted mode)`);
            this.logOperation('SDK', `Successfully connected to ${this.currentNetwork}`, 'success');

            // Update network info in UI
            document.getElementById('currentNetwork').textContent = this.currentNetwork.toUpperCase();
            
            // Try to get network status
            await this.updateNetworkInfo();

        } catch (error) {
            this.updateStatus('error', 'Connection failed');
            this.logOperation('SDK', `Connection failed: ${error.message}`, 'error');
            throw error;
        }
    }

    async updateNetworkInfo() {
        try {
            // Note: Some network status methods might not be available in current WASM SDK
            // This is a placeholder for when they become available
            document.getElementById('blockHeight').textContent = 'Available via network queries';
        } catch (error) {
            console.log('Network info not available:', error.message);
        }
    }

    updateStatus(status, text) {
        const statusIndicator = document.getElementById('statusIndicator');
        const statusText = document.getElementById('statusText');
        const statusDot = statusIndicator.querySelector('.status-dot');
        const connectionStatus = document.getElementById('connectionStatus');

        statusText.textContent = text;
        connectionStatus.textContent = text;

        // Remove existing status classes
        statusDot.classList.remove('connected', 'connecting', 'error');
        
        // Add new status class
        statusDot.classList.add(status === 'connected' ? 'connected' : 
                                 status === 'connecting' ? 'connecting' : 'error');
    }

    async lookupIdentity() {
        if (!this.isInitialized) {
            this.showError('SDK not initialized. Please wait for connection.');
            return;
        }

        const identityIdInput = document.getElementById('identityIdInput');
        const identityId = identityIdInput.value.trim();

        if (!identityId) {
            this.showError('Please enter an identity ID');
            return;
        }

        if (!this.isValidIdentityId(identityId)) {
            this.showError('Invalid identity ID format. Should be Base58 encoded.');
            return;
        }

        try {
            this.showLoading('Looking up identity...');
            this.hideError();
            this.logOperation('Identity', `Looking up ID: ${identityId}`);

            // Fetch identity from network
            const identity = await identity_fetch(this.sdk, identityId);
            
            if (identity) {
                this.currentIdentity = { id: identityId, data: identity };
                this.displayIdentityResults(identity);
                this.logOperation('Identity', 'Identity found successfully', 'success');
            } else {
                this.showError('Identity not found');
                this.logOperation('Identity', 'Identity not found', 'error');
            }

        } catch (error) {
            this.showError('Failed to lookup identity: ' + error.message);
            this.logOperation('Identity', `Lookup failed: ${error.message}`, 'error');
        } finally {
            this.hideLoading();
        }
    }

    displayIdentityResults(identity) {
        const resultsContainer = document.getElementById('identityResults');
        const identityData = document.getElementById('identityData');

        // Create formatted web display instead of raw JSON
        identityData.innerHTML = this.createIdentityDisplay(identity);

        resultsContainer.style.display = 'block';

        // Scroll to results
        resultsContainer.scrollIntoView({ behavior: 'smooth', block: 'start' });
    }

    createIdentityDisplay(identity) {
        const data = identity.toJSON();
        
        return `
            <table class="info-table">
                <tr>
                    <td class="label">ID:</td>
                    <td class="value"><code>${data.id}</code></td>
                </tr>
                <tr>
                    <td class="label">Balance:</td>
                    <td class="value"><strong>${(data.balance || 0).toLocaleString()}</strong> credits</td>
                </tr>
                <tr>
                    <td class="label">Revision:</td>
                    <td class="value">${data.revision !== undefined ? data.revision : 'N/A'}</td>
                </tr>
                <tr>
                    <td class="label">Public Keys:</td>
                    <td class="value">${data.publicKeys?.length || 0}</td>
                </tr>
            </table>

            ${data.publicKeys && data.publicKeys.length > 0 ? `
                <table class="keys-table">
                    <thead>
                        <tr>
                            <th>ID</th>
                            <th>Type</th>
                            <th>Purpose</th>
                            <th>Security</th>
                            <th>Status</th>
                        </tr>
                    </thead>
                    <tbody>
                        ${data.publicKeys.map(key => `
                            <tr>
                                <td>${key.id}</td>
                                <td>${mapKeyType(key.type)}</td>
                                <td>${mapKeyPurpose(key.purpose)}</td>
                                <td>${mapSecurityLevel(key.securityLevel)}</td>
                                <td>${key.disabledAt ? 'Disabled' : 'Active'}</td>
                            </tr>
                        `).join('')}
                    </tbody>
                </table>
            ` : ''}
        `;
    }

    formatIdentityData(identity) {
        // Get complete identity data like CLI does
        const data = identity.toJSON();
        
        // Add human-readable key information
        if (data.publicKeys) {
            data.publicKeys = data.publicKeys.map(key => ({
                ...key,
                typeLabel: mapKeyType(key.type),
                purposeLabel: mapKeyPurpose(key.purpose), 
                securityLevelLabel: mapSecurityLevel(key.securityLevel)
            }));
        }
        
        // Add summary metadata
        data.summary = {
            balanceInCredits: data.balance || 0,
            balanceInDash: this.creditsToOash(data.balance || 0),
            publicKeyCount: data.publicKeys?.length || 0,
            network: this.currentNetwork
        };
        
        return JSON.stringify(data, null, 2);
    }

    async checkBalance() {
        if (!this.currentIdentity) {
            this.showError('No identity loaded. Please lookup an identity first.');
            return;
        }

        try {
            this.showLoading('Checking balance...');
            this.logOperation('Balance', `Checking balance for ${this.currentIdentity.id}`);

            // Get identity balance
            const balanceResult = await get_identity_balance(this.sdk, this.currentIdentity.id);
            
            if (balanceResult !== null && balanceResult !== undefined) {
                this.displayBalance(balanceResult);
                this.logOperation('Balance', 'Balance retrieved successfully', 'success');
            } else {
                this.showError('Could not retrieve balance information');
                this.logOperation('Balance', 'Balance retrieval failed', 'error');
            }

        } catch (error) {
            this.showError('Failed to check balance: ' + error.message);
            this.logOperation('Balance', `Balance check failed: ${error.message}`, 'error');
        } finally {
            this.hideLoading();
        }
    }

    displayBalance(balance) {
        const balanceSection = document.getElementById('balanceSection');
        const currentBalance = document.getElementById('currentBalance');
        const balanceInDash = document.getElementById('balanceInDash');
        const identityRevision = document.getElementById('identityRevision');

        currentBalance.textContent = `${balance.toLocaleString()} credits`;
        balanceInDash.textContent = `${this.creditsToOash(balance)} DASH`;
        identityRevision.textContent = this.currentIdentity.data.revision || 'Unknown';

        balanceSection.style.display = 'block';
        balanceSection.scrollIntoView({ behavior: 'smooth', block: 'start' });
    }

    async viewKeys() {
        if (!this.currentIdentity) {
            this.showError('No identity loaded. Please lookup an identity first.');
            return;
        }

        try {
            this.showLoading('Retrieving public keys...');
            this.logOperation('Keys', `Retrieving keys for ${this.currentIdentity.id}`);

            // Get identity keys
            const keysResult = await get_identity_keys(
                this.sdk,
                this.currentIdentity.id,
                'all',
                null, // specificKeyIds
                null, // searchPurposeMap  
                null, // limit
                null  // offset
            );

            if (keysResult && keysResult.length > 0) {
                this.displayKeys(keysResult);
                this.logOperation('Keys', `Retrieved ${keysResult.length} public keys`, 'success');
            } else {
                // Fallback to identity data keys if direct key query fails
                const identityKeys = this.currentIdentity.data.publicKeys || [];
                if (identityKeys.length > 0) {
                    this.displayKeys(identityKeys);
                    this.logOperation('Keys', `Retrieved ${identityKeys.length} keys from identity data`, 'success');
                } else {
                    this.showError('No public keys found for this identity');
                    this.logOperation('Keys', 'No public keys found', 'error');
                }
            }

        } catch (error) {
            // Fallback to identity data if API call fails
            const identityKeys = this.currentIdentity.data.publicKeys || [];
            if (identityKeys.length > 0) {
                this.displayKeys(identityKeys);
                this.logOperation('Keys', `Retrieved ${identityKeys.length} keys from identity data (fallback)`, 'success');
            } else {
                this.showError('Failed to retrieve keys: ' + error.message);
                this.logOperation('Keys', `Key retrieval failed: ${error.message}`, 'error');
            }
        } finally {
            this.hideLoading();
        }
    }

    displayKeys(keys) {
        const keysSection = document.getElementById('keysSection');
        const keysResults = document.getElementById('keysResults');

        keysResults.innerHTML = '';

        keys.forEach((key, index) => {
            const keyElement = this.createKeyElement(key, index);
            keysResults.appendChild(keyElement);
        });

        keysSection.style.display = 'block';
        keysSection.scrollIntoView({ behavior: 'smooth', block: 'start' });
    }

    createKeyElement(key, index) {
        const keyDiv = document.createElement('div');
        keyDiv.className = 'key-item';

        const keyHeader = document.createElement('div');
        keyHeader.className = 'key-header';
        
        const keyId = document.createElement('span');
        keyId.className = 'key-id';
        keyId.textContent = `Key #${key.id !== undefined ? key.id : index}`;
        
        const keyType = document.createElement('span');
        keyType.className = 'key-type';
        keyType.textContent = mapKeyType(key.type);
        
        keyHeader.appendChild(keyId);
        keyHeader.appendChild(keyType);

        const keyDetails = document.createElement('div');
        keyDetails.className = 'key-details';

        // Key details with proper mapping
        const details = [
            { label: 'Purpose', value: mapKeyPurpose(key.purpose) },
            { label: 'Security Level', value: mapSecurityLevel(key.securityLevel) },
            { label: 'Read Only', value: key.readOnly ? 'Yes' : 'No' },
            { label: 'Disabled At', value: key.disabledAt || 'Active' },
            { label: 'Contract Bounds', value: key.contractBounds || 'None' }
        ];

        details.forEach(detail => {
            const detailDiv = document.createElement('div');
            detailDiv.className = 'key-detail';
            
            const label = document.createElement('span');
            label.className = 'label';
            label.textContent = detail.label + ':';
            
            const value = document.createElement('span');
            value.textContent = detail.value;
            
            detailDiv.appendChild(label);
            detailDiv.appendChild(value);
            keyDetails.appendChild(detailDiv);
        });

        // Key data (hex representation)
        if (key.data) {
            const keyDataDiv = document.createElement('div');
            keyDataDiv.className = 'key-data';
            keyDataDiv.textContent = this.formatKeyData(key.data);
            keyDiv.appendChild(keyDataDiv);
        }

        keyDiv.appendChild(keyHeader);
        keyDiv.appendChild(keyDetails);

        return keyDiv;
    }

    async createIdentity() {
        if (!this.isInitialized) {
            this.showError('SDK not initialized. Please wait for connection.');
            return;
        }

        const assetLockProof = document.getElementById('assetLockProof').value.trim();
        const assetLockPrivateKey = document.getElementById('assetLockPrivateKey').value.trim();
        const publicKeysJson = document.getElementById('publicKeysJson').value.trim();

        if (!this.validateCreationInputs(assetLockProof, assetLockPrivateKey, publicKeysJson)) {
            return;
        }

        try {
            this.showLoading('Creating identity...');
            this.logOperation('Creation', 'Starting identity creation process');

            // Parse public keys JSON
            const publicKeys = JSON.parse(publicKeysJson);

            // Create identity using the SDK
            const result = await this.sdk.identity_create(
                assetLockProof,
                assetLockPrivateKey,
                JSON.stringify(publicKeys)
            );

            if (result) {
                this.displayCreationResults(result);
                this.logOperation('Creation', 'Identity created successfully', 'success');
            } else {
                this.showError('Identity creation failed - no result returned');
                this.logOperation('Creation', 'Identity creation failed - no result', 'error');
            }

        } catch (error) {
            this.showError('Failed to create identity: ' + error.message);
            this.logOperation('Creation', `Identity creation failed: ${error.message}`, 'error');
        } finally {
            this.hideLoading();
        }
    }

    displayCreationResults(result) {
        const resultsContainer = document.getElementById('creationResults');
        const creationData = document.getElementById('creationData');

        creationData.textContent = JSON.stringify(result, null, 2);
        resultsContainer.style.display = 'block';
        resultsContainer.scrollIntoView({ behavior: 'smooth', block: 'start' });
    }

    // Utility Methods

    isValidIdentityId(id) {
        // Basic validation for Base58 format (simplified)
        return /^[1-9A-HJ-NP-Za-km-z]{44,}$/.test(id);
    }

    creditsToOash(credits) {
        return (credits / 100000000).toFixed(8);
    }

    getKeyTypeName(type) {
        const types = {
            0: 'ECDSA_SECP256K1',
            1: 'BLS12_381',
            2: 'ECDSA_HASH160',
            3: 'BIP13_SCRIPT_HASH'
        };
        return types[type] || `Unknown (${type})`;
    }

    getKeyPurpose(purpose) {
        const purposes = {
            0: 'AUTHENTICATION',
            1: 'ENCRYPTION', 
            2: 'DECRYPTION',
            3: 'WITHDRAW'
        };
        return purposes[purpose] || `Unknown (${purpose})`;
    }

    formatKeyData(data) {
        if (data instanceof Uint8Array) {
            return Array.from(data).map(b => b.toString(16).padStart(2, '0')).join('');
        }
        return data.toString();
    }

    validateCreationForm() {
        const assetLockProof = document.getElementById('assetLockProof').value.trim();
        const assetLockPrivateKey = document.getElementById('assetLockPrivateKey').value.trim();
        const publicKeysJson = document.getElementById('publicKeysJson').value.trim();
        const createBtn = document.getElementById('createIdentityBtn');

        const isValid = assetLockProof && assetLockPrivateKey && publicKeysJson;
        createBtn.disabled = !isValid;
    }

    validateCreationInputs(assetLockProof, assetLockPrivateKey, publicKeysJson) {
        if (!assetLockProof) {
            this.showError('Asset lock proof is required');
            return false;
        }

        if (!assetLockPrivateKey) {
            this.showError('Asset lock private key is required');
            return false;
        }

        if (!publicKeysJson) {
            this.showError('Public keys JSON is required');
            return false;
        }

        try {
            JSON.parse(publicKeysJson);
        } catch (error) {
            this.showError('Invalid JSON format for public keys');
            return false;
        }

        return true;
    }

    exportData() {
        if (!this.currentIdentity) {
            this.showError('No identity data to export');
            return;
        }

        const exportData = {
            identity: this.currentIdentity,
            exportedAt: new Date().toISOString(),
            network: this.currentNetwork
        };

        const dataStr = JSON.stringify(exportData, null, 2);
        const dataBlob = new Blob([dataStr], { type: 'application/json' });
        const url = URL.createObjectURL(dataBlob);

        const a = document.createElement('a');
        a.href = url;
        a.download = `identity_${this.currentIdentity.id.substring(0, 8)}_${Date.now()}.json`;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);

        this.logOperation('Export', 'Identity data exported successfully', 'success');
    }

    logOperation(category, message, type = 'info') {
        const logDisplay = document.getElementById('operationsLog');
        const timestamp = new Date().toLocaleTimeString();
        
        const logEntry = document.createElement('div');
        logEntry.className = `log-entry ${type}`;
        
        const timestampSpan = document.createElement('span');
        timestampSpan.className = 'timestamp';
        timestampSpan.textContent = `[${timestamp}] [${category}]`;
        
        const messageSpan = document.createElement('span');
        messageSpan.className = 'message';
        messageSpan.textContent = message;
        
        logEntry.appendChild(timestampSpan);
        logEntry.appendChild(messageSpan);
        
        logDisplay.appendChild(logEntry);
        logDisplay.scrollTop = logDisplay.scrollHeight;
    }

    showError(message) {
        const errorElement = document.getElementById('errorMessage');
        errorElement.textContent = message;
        errorElement.style.display = 'block';
        
        // Hide after 10 seconds
        setTimeout(() => {
            errorElement.style.display = 'none';
        }, 10000);
    }

    hideError() {
        const errorElement = document.getElementById('errorMessage');
        errorElement.style.display = 'none';
    }

    showLoading(message) {
        const loadingElement = document.getElementById('loadingMessage');
        loadingElement.querySelector('span').textContent = message;
        loadingElement.style.display = 'flex';
    }

    hideLoading() {
        const loadingElement = document.getElementById('loadingMessage');
        loadingElement.style.display = 'none';
    }
}

// Global functions for UI interactions
window.loadSampleIdentity = function() {
    const input = document.getElementById('identityIdInput');
    input.value = '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk'; // Known testnet identity
};

window.clearInput = function() {
    const input = document.getElementById('identityIdInput');
    input.value = '';
};

window.generateSampleKeys = function() {
    const sampleKeys = [
        {
            id: 0,
            type: 0,
            purpose: 0,
            securityLevel: 0,
            data: "AkVuTKyF3YgKLAQlLEtaUL2HTditwGILfWUVqjzYnIgH",
            readOnly: false
        }
    ];
    
    const textarea = document.getElementById('publicKeysJson');
    textarea.value = JSON.stringify(sampleKeys, null, 2);
    
    // Trigger validation
    const event = new Event('input');
    textarea.dispatchEvent(event);
};

window.clearLog = function() {
    const logDisplay = document.getElementById('operationsLog');
    logDisplay.innerHTML = '<div class="log-entry"><span class="timestamp">[System]</span><span class="message">Log cleared</span></div>';
};

window.exportLog = function() {
    const logDisplay = document.getElementById('operationsLog');
    const logEntries = Array.from(logDisplay.querySelectorAll('.log-entry')).map(entry => {
        const timestamp = entry.querySelector('.timestamp').textContent;
        const message = entry.querySelector('.message').textContent;
        return `${timestamp} ${message}`;
    });

    const logData = logEntries.join('\n');
    const dataBlob = new Blob([logData], { type: 'text/plain' });
    const url = URL.createObjectURL(dataBlob);

    const a = document.createElement('a');
    a.href = url;
    a.download = `identity_manager_log_${Date.now()}.txt`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
};

// Initialize application when DOM is ready
document.addEventListener('DOMContentLoaded', () => {
    const app = new IdentityManager();
    app.init();
});