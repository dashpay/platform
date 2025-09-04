/**
 * Token Transfer - Dash Platform WASM SDK Sample Application
 * Demonstrates token operations, portfolio management, and transfer capabilities
 */

import init, { WasmSdk, WasmSdkBuilder } from '../../pkg/dash_wasm_sdk.js';

class TokenTransfer {
    constructor() {
        this.sdk = null;
        this.currentNetwork = 'testnet';
        this.isInitialized = false;
        this.currentPortfolio = null;
        this.tokenCache = new Map();
        
        // Sample data for testing
        this.sampleIdentityId = '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk';
        this.sampleTokenContract = 'Hqyu8WcRwXCTwbNxdga4CN5gsVEGc67wng4TFzceyLUv';
        
        // Bind methods
        this.initializeSDK = this.initializeSDK.bind(this);
        this.loadPortfolio = this.loadPortfolio.bind(this);
        this.calculateTokenId = this.calculateTokenId.bind(this);
        this.getTokenInfo = this.getTokenInfo.bind(this);
        this.previewTransfer = this.previewTransfer.bind(this);
        this.executeTransfer = this.executeTransfer.bind(this);
        this.getPricing = this.getPricing.bind(this);
        this.checkBulkBalances = this.checkBulkBalances.bind(this);
    }

    async init() {
        try {
            await this.initializeEventListeners();
            await this.initializeSDK();
            this.logOperation('System', 'Token Transfer application initialized successfully');
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

        // Portfolio loading
        const loadPortfolioBtn = document.getElementById('loadPortfolioBtn');
        loadPortfolioBtn.addEventListener('click', this.loadPortfolio);

        // Token ID calculation
        const calculateTokenIdBtn = document.getElementById('calculateTokenIdBtn');
        calculateTokenIdBtn.addEventListener('click', this.calculateTokenId);

        // Direct token info
        const getTokenInfoBtn = document.getElementById('getTokenInfoBtn');
        getTokenInfoBtn.addEventListener('click', this.getTokenInfo);

        // Transfer operations
        const previewTransferBtn = document.getElementById('previewTransferBtn');
        previewTransferBtn.addEventListener('click', this.previewTransfer);

        const executeTransferBtn = document.getElementById('executeTransferBtn');
        executeTransferBtn.addEventListener('click', this.executeTransfer);

        const cancelTransferBtn = document.getElementById('cancelTransferBtn');
        cancelTransferBtn.addEventListener('click', () => {
            document.getElementById('transferPreview').style.display = 'none';
        });

        // Pricing
        const getPricingBtn = document.getElementById('getPricingBtn');
        getPricingBtn.addEventListener('click', this.getPricing);

        // Bulk operations
        const bulkBalanceBtn = document.getElementById('bulkBalanceBtn');
        bulkBalanceBtn.addEventListener('click', this.checkBulkBalances);

        // Export bulk results
        const exportBulkBtn = document.getElementById('exportBulkBtn');
        exportBulkBtn.addEventListener('click', () => this.exportBulkResults());

        // Enter key support
        const portfolioInput = document.getElementById('portfolioIdentityId');
        portfolioInput.addEventListener('keypress', (e) => {
            if (e.key === 'Enter') {
                this.loadPortfolio();
            }
        });
    }

    async initializeSDK() {
        try {
            this.updateStatus('connecting', 'Initializing SDK...');
            this.logOperation('SDK', `Initializing for ${this.currentNetwork} network`);

            // Initialize WASM module
            await init();

            // Create SDK builder based on network
            let builder;
            if (this.currentNetwork === 'mainnet') {
                builder = WasmSdkBuilder.new_mainnet();
            } else {
                builder = WasmSdkBuilder.new_testnet();
            }

            // Build SDK instance
            this.sdk = builder.build();
            this.isInitialized = true;

            this.updateStatus('connected', `Connected to ${this.currentNetwork}`);
            this.logOperation('SDK', `Successfully connected to ${this.currentNetwork}`, 'success');

        } catch (error) {
            this.updateStatus('error', 'Connection failed');
            this.logOperation('SDK', `Connection failed: ${error.message}`, 'error');
            throw error;
        }
    }

    async loadPortfolio() {
        if (!this.isInitialized) {
            this.showError('SDK not initialized. Please wait for connection.');
            return;
        }

        const identityId = document.getElementById('portfolioIdentityId').value.trim();

        if (!identityId) {
            this.showError('Please enter an identity ID');
            return;
        }

        if (!this.isValidIdentityId(identityId)) {
            this.showError('Invalid identity ID format');
            return;
        }

        try {
            this.showLoadingOverlay('Loading token portfolio...');
            this.logOperation('Portfolio', `Loading portfolio for ${identityId}`);

            // Get known token contracts for this network
            const tokenContracts = this.getKnownTokenContracts();
            const portfolio = [];

            // Check balances for each known token
            for (const tokenContract of tokenContracts) {
                try {
                    // Calculate token ID
                    const tokenId = await this.sdk.calculate_token_id_from_contract(
                        tokenContract.contractId, 
                        tokenContract.position
                    );

                    // Get token balance
                    const balance = await this.sdk.get_identity_token_balances(
                        identityId,
                        [tokenId]
                    );

                    if (balance && balance[tokenId] && balance[tokenId] > 0) {
                        portfolio.push({
                            ...tokenContract,
                            tokenId,
                            balance: balance[tokenId],
                            balanceFormatted: this.formatTokenBalance(balance[tokenId], tokenContract.decimals)
                        });
                    }
                } catch (error) {
                    console.warn(`Failed to check balance for ${tokenContract.name}:`, error);
                }
            }

            this.currentPortfolio = { identityId, tokens: portfolio };
            this.displayPortfolio(portfolio);
            this.logOperation('Portfolio', `Portfolio loaded: ${portfolio.length} tokens with balances`, 'success');

        } catch (error) {
            this.showError('Failed to load portfolio: ' + error.message);
            this.logOperation('Portfolio', `Portfolio loading failed: ${error.message}`, 'error');
        } finally {
            this.hideLoadingOverlay();
        }
    }

    async calculateTokenId() {
        if (!this.isInitialized) {
            this.showError('SDK not initialized. Please wait for connection.');
            return;
        }

        const contractId = document.getElementById('tokenContractId').value.trim();
        const position = parseInt(document.getElementById('tokenPosition').value) || 0;

        if (!contractId) {
            this.showError('Please enter a contract ID');
            return;
        }

        try {
            this.logOperation('Token', `Calculating token ID for contract ${contractId}, position ${position}`);

            const tokenId = await this.sdk.calculate_token_id_from_contract(contractId, position);
            
            if (tokenId) {
                // Set the calculated ID in the direct lookup field
                document.getElementById('directTokenId').value = tokenId;
                
                // Automatically fetch token info
                await this.getTokenInfo();
                
                this.logOperation('Token', `Token ID calculated: ${tokenId}`, 'success');
            } else {
                this.showError('Failed to calculate token ID');
                this.logOperation('Token', 'Token ID calculation failed', 'error');
            }

        } catch (error) {
            this.showError('Failed to calculate token ID: ' + error.message);
            this.logOperation('Token', `Token ID calculation failed: ${error.message}`, 'error');
        }
    }

    async getTokenInfo() {
        if (!this.isInitialized) {
            this.showError('SDK not initialized. Please wait for connection.');
            return;
        }

        const tokenId = document.getElementById('directTokenId').value.trim();

        if (!tokenId) {
            this.showError('Please enter a token ID');
            return;
        }

        try {
            this.showLoadingOverlay('Fetching token information...');
            this.logOperation('Token', `Fetching info for token ${tokenId}`);

            // Get token information (this might require multiple API calls)
            const tokenInfo = await this.fetchTokenDetails(tokenId);
            
            this.displayTokenInfo(tokenInfo);
            this.logOperation('Token', 'Token information retrieved successfully', 'success');

        } catch (error) {
            this.showError('Failed to get token info: ' + error.message);
            this.logOperation('Token', `Token info retrieval failed: ${error.message}`, 'error');
        } finally {
            this.hideLoadingOverlay();
        }
    }

    async fetchTokenDetails(tokenId) {
        // This is a composite operation that might use multiple SDK methods
        const tokenInfo = {
            tokenId,
            fetchedAt: new Date().toISOString()
        };

        try {
            // Try to get token contract info
            const contractInfo = await this.sdk.get_token_contract_info(tokenId);
            if (contractInfo) {
                tokenInfo.contract = contractInfo;
            }
        } catch (error) {
            console.warn('Could not fetch token contract info:', error);
        }

        try {
            // Try to get total supply
            const supply = await this.sdk.get_token_total_supply(tokenId);
            if (supply !== null && supply !== undefined) {
                tokenInfo.totalSupply = supply;
            }
        } catch (error) {
            console.warn('Could not fetch token supply:', error);
        }

        return tokenInfo;
    }

    async previewTransfer() {
        const fromId = document.getElementById('fromIdentityId').value.trim();
        const toId = document.getElementById('toIdentityId').value.trim();
        const tokenId = document.getElementById('transferTokenId').value.trim();
        const amount = parseInt(document.getElementById('transferAmount').value) || 0;
        const privateKey = document.getElementById('transferPrivateKey').value.trim();

        if (!this.validateTransferInputs(fromId, toId, tokenId, amount, privateKey)) {
            return;
        }

        try {
            this.logOperation('Transfer', 'Creating transfer preview');

            // Get current balance to verify transfer is possible
            const balances = await this.sdk.get_identity_token_balances(fromId, [tokenId]);
            const currentBalance = balances[tokenId] || 0;

            const preview = {
                from: fromId,
                to: toId,
                tokenId,
                amount,
                currentBalance,
                remainingBalance: currentBalance - amount,
                transferable: currentBalance >= amount,
                timestamp: new Date().toISOString(),
                network: this.currentNetwork
            };

            this.displayTransferPreview(preview);
            this.logOperation('Transfer', `Transfer preview created for ${amount} tokens`, 'success');

        } catch (error) {
            this.showError('Failed to create transfer preview: ' + error.message);
            this.logOperation('Transfer', `Transfer preview failed: ${error.message}`, 'error');
        }
    }

    async executeTransfer() {
        try {
            this.showLoadingOverlay('Executing token transfer...');
            this.logOperation('Transfer', 'Executing token transfer');

            // This would normally create and submit a state transition
            // For demo purposes, we'll simulate the operation
            await this.simulateTransfer();

            this.logOperation('Transfer', 'Token transfer executed successfully', 'success');

        } catch (error) {
            this.showError('Failed to execute transfer: ' + error.message);
            this.logOperation('Transfer', `Transfer execution failed: ${error.message}`, 'error');
        } finally {
            this.hideLoadingOverlay();
        }
    }

    async simulateTransfer() {
        // Simulate transfer operation
        return new Promise((resolve) => {
            setTimeout(() => {
                const result = {
                    transactionId: 'simulated_' + Date.now(),
                    status: 'completed',
                    timestamp: new Date().toISOString(),
                    message: 'Transfer simulation completed successfully'
                };
                
                this.displayTransferResult(result);
                resolve(result);
            }, 2000);
        });
    }

    async getPricing() {
        if (!this.isInitialized) {
            this.showError('SDK not initialized. Please wait for connection.');
            return;
        }

        const contractId = document.getElementById('pricingContractId').value.trim();
        const position = parseInt(document.getElementById('pricingTokenPosition').value) || 0;

        if (!contractId) {
            this.showError('Please enter a contract ID');
            return;
        }

        try {
            this.showLoadingOverlay('Fetching pricing information...');
            this.logOperation('Pricing', `Getting pricing for contract ${contractId}, position ${position}`);

            // Get token pricing
            const pricing = await this.sdk.get_token_price_by_contract(contractId, position);
            
            if (pricing) {
                this.displayPricing(pricing);
                this.logOperation('Pricing', 'Pricing information retrieved successfully', 'success');
            } else {
                this.showError('No pricing information available');
                this.logOperation('Pricing', 'No pricing information available', 'error');
            }

        } catch (error) {
            this.showError('Failed to get pricing: ' + error.message);
            this.logOperation('Pricing', `Pricing retrieval failed: ${error.message}`, 'error');
        } finally {
            this.hideLoadingOverlay();
        }
    }

    async checkBulkBalances() {
        if (!this.isInitialized) {
            this.showError('SDK not initialized. Please wait for connection.');
            return;
        }

        const identityList = document.getElementById('identityList').value.trim();
        const tokenIdsInput = document.getElementById('bulkTokenIds').value.trim();

        if (!identityList) {
            this.showError('Please enter at least one identity ID');
            return;
        }

        const identityIds = identityList.split('\n').map(id => id.trim()).filter(id => id);
        const tokenIds = tokenIdsInput ? tokenIdsInput.split(',').map(id => id.trim()).filter(id => id) : [];

        if (identityIds.length === 0) {
            this.showError('No valid identity IDs provided');
            return;
        }

        try {
            this.showLoadingOverlay(`Checking balances for ${identityIds.length} identities...`);
            this.logOperation('Bulk', `Checking balances for ${identityIds.length} identities`);

            const bulkResults = [];

            for (const identityId of identityIds) {
                try {
                    let balanceData = {};

                    if (tokenIds.length > 0) {
                        // Check specific token balances
                        const tokenBalances = await this.sdk.get_identity_token_balances(identityId, tokenIds);
                        balanceData.tokenBalances = tokenBalances;
                    } else {
                        // Check general identity balance
                        const balance = await this.sdk.get_identity_balance(identityId);
                        balanceData.platformBalance = balance;
                    }

                    bulkResults.push({
                        identityId,
                        status: 'success',
                        data: balanceData
                    });
                    
                } catch (error) {
                    bulkResults.push({
                        identityId,
                        status: 'error',
                        error: error.message
                    });
                }
            }

            this.displayBulkResults(bulkResults);
            this.logOperation('Bulk', `Bulk balance check completed: ${bulkResults.length} results`, 'success');

        } catch (error) {
            this.showError('Bulk balance check failed: ' + error.message);
            this.logOperation('Bulk', `Bulk balance check failed: ${error.message}`, 'error');
        } finally {
            this.hideLoadingOverlay();
        }
    }

    // Display Methods

    displayPortfolio(portfolio) {
        const portfolioResults = document.getElementById('portfolioResults');
        const tokenList = document.getElementById('tokenList');
        const totalTokens = document.getElementById('totalTokens');
        const totalValue = document.getElementById('totalValue');

        totalTokens.textContent = `${portfolio.length} tokens`;
        totalValue.textContent = '$0.00 USD'; // Would calculate based on pricing data

        tokenList.innerHTML = '';

        if (portfolio.length === 0) {
            tokenList.innerHTML = `
                <div class="empty-state">
                    <div class="empty-icon">🪙</div>
                    <h4>No Token Balances Found</h4>
                    <p>This identity doesn't hold any of the known tokens, or all balances are zero.</p>
                </div>
            `;
        } else {
            portfolio.forEach(token => {
                const tokenElement = this.createTokenElement(token);
                tokenList.appendChild(tokenElement);
            });
        }

        portfolioResults.style.display = 'block';
        portfolioResults.scrollIntoView({ behavior: 'smooth', block: 'start' });
    }

    createTokenElement(token) {
        const tokenDiv = document.createElement('div');
        tokenDiv.className = 'token-item';

        tokenDiv.innerHTML = `
            <div class="token-header">
                <span class="token-name">${token.name}</span>
                <span class="token-symbol">${token.symbol}</span>
            </div>
            <div class="token-balance">${token.balanceFormatted}</div>
            <div class="token-meta">
                <div class="meta-item">
                    <span>Contract:</span>
                    <span class="monospace">${this.truncateId(token.contractId)}</span>
                </div>
                <div class="meta-item">
                    <span>Token ID:</span>
                    <span class="monospace">${this.truncateId(token.tokenId)}</span>
                </div>
                <div class="meta-item">
                    <span>Position:</span>
                    <span>${token.position}</span>
                </div>
                <div class="meta-item">
                    <span>Balance:</span>
                    <span>${token.balance} units</span>
                </div>
            </div>
            <div class="token-actions">
                <button class="btn btn-small" onclick="setTransferToken('${token.tokenId}', ${token.balance})">
                    Transfer
                </button>
                <button class="btn btn-small" onclick="viewTokenDetails('${token.tokenId}')">
                    Details
                </button>
            </div>
        `;

        return tokenDiv;
    }

    displayTokenInfo(tokenInfo) {
        const tokenInfoResults = document.getElementById('tokenInfoResults');
        const tokenDetails = document.getElementById('tokenDetails');

        tokenDetails.textContent = JSON.stringify(tokenInfo, null, 2);
        tokenInfoResults.style.display = 'block';
        tokenInfoResults.scrollIntoView({ behavior: 'smooth', block: 'start' });
    }

    displayTransferPreview(preview) {
        const transferPreview = document.getElementById('transferPreview');
        const previewDetails = document.getElementById('previewDetails');

        const previewText = `
Transfer Preview:
================

From Identity: ${preview.from}
To Identity:   ${preview.to}
Token ID:      ${preview.tokenId}
Amount:        ${preview.amount.toLocaleString()} units

Current Balance:   ${preview.currentBalance.toLocaleString()} units
Remaining Balance: ${preview.remainingBalance.toLocaleString()} units

Status: ${preview.transferable ? '✅ Transfer Possible' : '❌ Insufficient Balance'}
Network: ${preview.network.toUpperCase()}
Preview Time: ${new Date(preview.timestamp).toLocaleString()}

${!preview.transferable ? '\n⚠️ WARNING: Insufficient balance for this transfer!' : ''}
        `.trim();

        previewDetails.textContent = previewText;
        transferPreview.style.display = 'block';
        transferPreview.scrollIntoView({ behavior: 'smooth', block: 'start' });

        // Enable/disable execute button based on transferability
        const executeBtn = document.getElementById('executeTransferBtn');
        executeBtn.disabled = !preview.transferable;
    }

    displayTransferResult(result) {
        const transferResults = document.getElementById('transferResults');
        const transferData = document.getElementById('transferData');

        transferData.textContent = JSON.stringify(result, null, 2);
        transferResults.style.display = 'block';
        transferResults.scrollIntoView({ behavior: 'smooth', block: 'start' });

        // Hide preview
        document.getElementById('transferPreview').style.display = 'none';
    }

    displayPricing(pricing) {
        const pricingResults = document.getElementById('pricingResults');
        const currentPrice = document.getElementById('currentPrice');
        const totalSupply = document.getElementById('totalSupply');
        const marketCap = document.getElementById('marketCap');

        currentPrice.textContent = `${pricing.price || 'N/A'} credits`;
        totalSupply.textContent = (pricing.totalSupply || 0).toLocaleString();
        marketCap.textContent = pricing.marketCap ? `${pricing.marketCap.toLocaleString()} credits` : 'N/A';

        pricingResults.style.display = 'block';
        pricingResults.scrollIntoView({ behavior: 'smooth', block: 'start' });
    }

    displayBulkResults(results) {
        const bulkResults = document.getElementById('bulkResults');
        const bulkBalanceList = document.getElementById('bulkBalanceList');

        bulkBalanceList.innerHTML = '';

        results.forEach(result => {
            const resultDiv = document.createElement('div');
            resultDiv.className = 'bulk-balance-item';

            const identityDiv = document.createElement('div');
            identityDiv.className = 'bulk-identity';
            identityDiv.textContent = this.truncateId(result.identityId);

            const balanceDiv = document.createElement('div');
            balanceDiv.className = 'bulk-balance';

            const statusDiv = document.createElement('div');
            statusDiv.className = `bulk-status ${result.status}`;

            if (result.status === 'success') {
                if (result.data.tokenBalances) {
                    const tokenCount = Object.keys(result.data.tokenBalances).length;
                    balanceDiv.textContent = `${tokenCount} tokens`;
                } else if (result.data.platformBalance) {
                    balanceDiv.textContent = `${result.data.platformBalance.toLocaleString()} credits`;
                } else {
                    balanceDiv.textContent = 'No balances';
                }
                statusDiv.textContent = 'Success';
            } else {
                balanceDiv.textContent = 'Error';
                statusDiv.textContent = 'Failed';
                resultDiv.title = result.error;
            }

            resultDiv.appendChild(identityDiv);
            resultDiv.appendChild(balanceDiv);
            resultDiv.appendChild(statusDiv);

            bulkBalanceList.appendChild(resultDiv);
        });

        bulkResults.style.display = 'block';
        bulkResults.scrollIntoView({ behavior: 'smooth', block: 'start' });

        // Store results for export
        this.lastBulkResults = results;
    }

    // Utility Methods

    getKnownTokenContracts() {
        // Known token contracts for demo purposes
        const contracts = [
            {
                name: 'Sample Token A',
                symbol: 'STA',
                contractId: this.sampleTokenContract,
                position: 0,
                decimals: 8
            },
            {
                name: 'Sample Token B', 
                symbol: 'STB',
                contractId: this.sampleTokenContract,
                position: 1,
                decimals: 18
            }
        ];

        return contracts;
    }

    formatTokenBalance(balance, decimals = 8) {
        const divisor = Math.pow(10, decimals);
        const formatted = (balance / divisor).toFixed(decimals);
        return parseFloat(formatted).toString(); // Remove trailing zeros
    }

    validateTransferInputs(fromId, toId, tokenId, amount, privateKey) {
        if (!fromId) {
            this.showError('From Identity ID is required');
            return false;
        }

        if (!toId) {
            this.showError('To Identity ID is required');
            return false;
        }

        if (!tokenId) {
            this.showError('Token ID is required');
            return false;
        }

        if (!amount || amount <= 0) {
            this.showError('Transfer amount must be greater than 0');
            return false;
        }

        if (!privateKey) {
            this.showError('Private key is required for signing');
            return false;
        }

        if (!this.isValidIdentityId(fromId)) {
            this.showError('Invalid From Identity ID format');
            return false;
        }

        if (!this.isValidIdentityId(toId)) {
            this.showError('Invalid To Identity ID format');
            return false;
        }

        return true;
    }

    isValidIdentityId(id) {
        return /^[1-9A-HJ-NP-Za-km-z]{44,}$/.test(id);
    }

    truncateId(id) {
        if (!id || id.length <= 16) return id;
        return `${id.substring(0, 8)}...${id.substring(id.length - 8)}`;
    }

    exportBulkResults() {
        if (!this.lastBulkResults) {
            this.showError('No bulk results to export');
            return;
        }

        const csv = this.convertBulkResultsToCSV(this.lastBulkResults);
        this.downloadFile(csv, `bulk_balances_${Date.now()}.csv`, 'text/csv');
        this.logOperation('Export', `Exported bulk results (${this.lastBulkResults.length} entries)`, 'success');
    }

    convertBulkResultsToCSV(results) {
        const headers = ['Identity ID', 'Status', 'Platform Balance', 'Token Balances', 'Error'];
        const rows = [headers.join(',')];

        results.forEach(result => {
            const row = [
                `"${result.identityId}"`,
                `"${result.status}"`,
                `"${result.data?.platformBalance || ''}"`,
                `"${result.data?.tokenBalances ? JSON.stringify(result.data.tokenBalances) : ''}"`,
                `"${result.error || ''}"`
            ];
            rows.push(row.join(','));
        });

        return rows.join('\n');
    }

    downloadFile(content, filename, mimeType) {
        const blob = new Blob([content], { type: mimeType });
        const url = URL.createObjectURL(blob);
        
        const a = document.createElement('a');
        a.href = url;
        a.download = filename;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
    }

    // UI Helper Methods
    updateStatus(status, text) {
        const statusIndicator = document.getElementById('statusIndicator');
        const statusText = document.getElementById('statusText');
        const statusDot = statusIndicator.querySelector('.status-dot');

        statusText.textContent = text;

        // Remove existing status classes
        statusDot.classList.remove('connected', 'connecting', 'error');
        
        // Add new status class
        statusDot.classList.add(status === 'connected' ? 'connected' : 
                                 status === 'connecting' ? 'connecting' : 'error');
    }

    showLoadingOverlay(message) {
        const overlay = document.getElementById('loadingOverlay');
        const loadingText = document.getElementById('loadingText');
        loadingText.textContent = message;
        overlay.style.display = 'flex';
    }

    hideLoadingOverlay() {
        const overlay = document.getElementById('loadingOverlay');
        overlay.style.display = 'none';
    }

    showError(message) {
        const modal = document.getElementById('errorModal');
        const errorDetails = document.getElementById('errorDetails');
        errorDetails.textContent = message;
        modal.style.display = 'flex';
    }

    logOperation(category, message, type = 'info') {
        console.log(`[${category}] ${message}`);
        
        const transferLog = document.getElementById('transferLog');
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
        
        transferLog.appendChild(logEntry);
        transferLog.scrollTop = transferLog.scrollHeight;
    }
}

// Global functions for UI interactions
window.loadSamplePortfolio = function() {
    const input = document.getElementById('portfolioIdentityId');
    input.value = '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk';
};

window.clearPortfolio = function() {
    const input = document.getElementById('portfolioIdentityId');
    input.value = '';
    
    const portfolioResults = document.getElementById('portfolioResults');
    portfolioResults.style.display = 'none';
};

window.setTransferToken = function(tokenId, maxBalance) {
    document.getElementById('transferTokenId').value = tokenId;
    document.getElementById('transferAmount').max = maxBalance;
    
    // Scroll to transfer section
    document.querySelector('.transfer-section').scrollIntoView({ 
        behavior: 'smooth', 
        block: 'start' 
    });
};

window.viewTokenDetails = function(tokenId) {
    document.getElementById('directTokenId').value = tokenId;
    
    // Trigger token info fetch
    const app = window.tokenTransferApp;
    if (app) {
        app.getTokenInfo();
    }
};

window.closeErrorModal = function() {
    const modal = document.getElementById('errorModal');
    modal.style.display = 'none';
};

window.clearTransferLog = function() {
    const transferLog = document.getElementById('transferLog');
    transferLog.innerHTML = '<div class="log-entry"><span class="timestamp">[System]</span><span class="message">Log cleared</span></div>';
};

window.exportTransferLog = function() {
    const transferLog = document.getElementById('transferLog');
    const logEntries = Array.from(transferLog.querySelectorAll('.log-entry')).map(entry => {
        const timestamp = entry.querySelector('.timestamp').textContent;
        const message = entry.querySelector('.message').textContent;
        return `${timestamp} ${message}`;
    });

    const logData = logEntries.join('\n');
    const dataBlob = new Blob([logData], { type: 'text/plain' });
    const url = URL.createObjectURL(dataBlob);

    const a = document.createElement('a');
    a.href = url;
    a.download = `token_transfer_log_${Date.now()}.txt`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
};

// Initialize application when DOM is ready
document.addEventListener('DOMContentLoaded', () => {
    const app = new TokenTransfer();
    window.tokenTransferApp = app; // Make available globally for UI functions
    app.init();
});