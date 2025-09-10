/**
 * WASM SDK Faucet Client
 * Adapts platform-test-suite funding mechanisms for WASM SDK testing
 */

const Dash = require('dash');

class WasmFaucetClient {
    constructor(options = {}) {
        this.network = options.network || process.env.NETWORK || 'testnet';
        this.debug = options.debug || false;
        
        // Multi-faucet support for parallel testing
        this.workerId = process.env.MOCHA_WORKER_ID || 1;
        this.faucetConfig = this.selectFaucet();
        
        // Safety limits
        this.maxFundingPerOperation = options.maxFundingPerOperation || 500000000; // 5 DASH max
        this.dailyUsageLimit = options.dailyUsageLimit || 5000000000; // 50 DASH daily
        this.currentDailyUsage = 0;
        
        // Client instances
        this.faucetClient = null;
        this.isInitialized = false;
        
        // Usage tracking
        this.fundingHistory = [];
        this.createdIdentities = new Map(); // Track identities for cleanup
    }

    /**
     * Select faucet based on worker ID for parallel testing
     */
    selectFaucet() {
        const faucetNumber = ((this.workerId - 1) % 2) + 1; // Alternate between FAUCET_1 and FAUCET_2
        
        const config = {
            address: process.env[`FAUCET_${faucetNumber}_ADDRESS`],
            privateKey: process.env[`FAUCET_${faucetNumber}_PRIVATE_KEY`],
            workerId: this.workerId,
            faucetId: faucetNumber
        };

        if (!config.address || !config.privateKey) {
            throw new Error(`Faucet ${faucetNumber} not configured. Please set FAUCET_${faucetNumber}_ADDRESS and FAUCET_${faucetNumber}_PRIVATE_KEY`);
        }

        if (this.debug) {
            console.log(`🚰 Using Faucet ${faucetNumber} for worker ${this.workerId}`);
            console.log(`   Address: ${config.address.substring(0, 20)}...`);
        }

        return config;
    }

    /**
     * Initialize faucet client
     */
    async initialize() {
        if (this.isInitialized) return;

        try {
            this.validateEnvironment();
            
            const clientOptions = {
                network: this.network,
                timeout: 30000,
                wallet: {
                    privateKey: this.faucetConfig.privateKey,
                    waitForInstantLockTimeout: 120000
                }
            };

            // Add storage options if enabled
            if (process.env.FAUCET_WALLET_USE_STORAGE === 'true') {
                const { NodeForage } = require('nodeforage');
                clientOptions.wallet.adapter = new NodeForage({
                    dir: process.env.FAUCET_WALLET_STORAGE_DIR || process.cwd(),
                    name: `wasm-faucet-${this.faucetConfig.faucetId}-${this.faucetConfig.address}`
                });
            }

            // Add sync optimization
            if (process.env.SKIP_SYNC_BEFORE_HEIGHT) {
                clientOptions.wallet.unsafeOptions = {
                    skipSynchronizationBeforeHeight: parseInt(process.env.SKIP_SYNC_BEFORE_HEIGHT)
                };
            }

            this.faucetClient = new Dash.Client(clientOptions);

            // Wait for wallet sync
            if (this.debug) {
                console.log('🔄 Synchronizing faucet wallet...');
            }
            
            await this.faucetClient.getWalletAccount();
            this.isInitialized = true;

            if (this.debug) {
                const account = await this.faucetClient.getWalletAccount();
                const balance = account.getTotalBalance();
                console.log(`✅ Faucet initialized - Balance: ${balance} satoshis (${(balance / 1e8).toFixed(4)} DASH)`);
            }

        } catch (error) {
            throw new Error(`Failed to initialize faucet client: ${error.message}`);
        }
    }

    /**
     * Validate environment and safety checks
     */
    validateEnvironment() {
        // Ensure testnet only
        if (this.network !== 'testnet' && this.network !== 'devnet' && this.network !== 'regtest') {
            throw new Error(`Funded tests only supported on testnet/devnet/regtest, got: ${this.network}`);
        }

        // Check daily usage limits
        if (this.currentDailyUsage >= this.dailyUsageLimit) {
            throw new Error(`Daily funding limit exceeded: ${this.currentDailyUsage}/${this.dailyUsageLimit} satoshis`);
        }

        // Verify faucet configuration
        if (!this.faucetConfig.address || !this.faucetConfig.privateKey) {
            throw new Error('Faucet configuration incomplete');
        }
    }

    /**
     * Fund a specific address with credits
     */
    async fundAddress(address, amount) {
        await this.initialize();
        this.validateFundingRequest(amount);

        try {
            if (this.debug) {
                console.log(`💰 Funding ${address} with ${amount} satoshis (${(amount / 1e8).toFixed(4)} DASH)`);
            }

            const account = await this.faucetClient.getWalletAccount();
            
            // Create and broadcast funding transaction
            const transaction = await account.createTransaction({
                recipient: address,
                satoshis: amount
            });

            if (this.debug) {
                console.log(`📤 Broadcasting funding transaction: ${transaction.id}`);
            }

            // Wait for transaction confirmation
            await this.waitForTransactionConfirmation(transaction);

            // Track usage
            this.recordFunding(address, amount, transaction.id);

            if (this.debug) {
                console.log(`✅ Funding completed: ${transaction.id}`);
            }

            return {
                txId: transaction.id,
                amount,
                address,
                timestamp: Date.now()
            };

        } catch (error) {
            throw new Error(`Funding failed: ${error.message}`);
        }
    }

    /**
     * Create a funded identity for testing
     */
    async createFundedIdentity(creditsAmount = 100000000) { // 1 DASH in credits default
        await this.initialize();

        try {
            // Calculate required satoshis (credits are 1000:1 ratio with satoshis)
            const requiredSatoshis = Math.ceil(creditsAmount / 1000) + 10000000; // Add buffer for fees
            this.validateFundingRequest(requiredSatoshis);

            if (this.debug) {
                console.log(`🏗️ Creating funded identity with ${creditsAmount} credits (${requiredSatoshis} satoshis required)`);
            }

            const account = await this.faucetClient.getWalletAccount();
            
            // Create identity asset lock transaction
            const {
                transaction,
                privateKey,
                outputIndex
            } = await this.faucetClient.platform.identities.utils.createAssetLockTransaction(requiredSatoshis);

            // Create asset lock proof
            const assetLockProof = await this.faucetClient.platform.identities.utils.createAssetLockProof(
                transaction,
                outputIndex
            );

            // Create and broadcast identity
            const identity = await this.faucetClient.platform.identities.register(
                assetLockProof,
                privateKey
            );

            // Track for cleanup
            this.createdIdentities.set(identity.getId().toString(), {
                id: identity.getId().toString(),
                privateKey: privateKey.toString(),
                assetLockProof,
                creditsAmount,
                createdAt: Date.now(),
                txId: transaction.id
            });

            // Record usage
            this.recordFunding('identity-creation', requiredSatoshis, transaction.id);

            if (this.debug) {
                console.log(`✅ Identity created: ${identity.getId().toString()}`);
                console.log(`   Private Key: ${privateKey.toString().substring(0, 20)}...`);
                console.log(`   Credits: ${creditsAmount}`);
            }

            return {
                identity,
                identityId: identity.getId().toString(),
                privateKey: privateKey.toString(),
                assetLockProof,
                creditsAmount,
                txId: transaction.id
            };

        } catch (error) {
            throw new Error(`Identity creation failed: ${error.message}`);
        }
    }

    /**
     * Top up an existing identity with additional credits
     */
    async topupIdentity(identityId, privateKey, additionalCredits) {
        await this.initialize();

        try {
            const requiredSatoshis = Math.ceil(additionalCredits / 1000) + 5000000; // Buffer for fees
            this.validateFundingRequest(requiredSatoshis);

            if (this.debug) {
                console.log(`⬆️ Topping up identity ${identityId} with ${additionalCredits} credits`);
            }

            // Create asset lock for topup
            const {
                transaction,
                outputIndex
            } = await this.faucetClient.platform.identities.utils.createAssetLockTransaction(requiredSatoshis);

            // Create topup transition
            const identityTopUpTransition = await this.faucetClient.platform.dpp.identity.createIdentityTopUpTransition(
                identityId,
                transaction,
                outputIndex,
                privateKey
            );

            // Broadcast topup
            await this.faucetClient.platform.broadcastStateTransition(identityTopUpTransition);

            // Record usage
            this.recordFunding(`topup-${identityId}`, requiredSatoshis, transaction.id);

            if (this.debug) {
                console.log(`✅ Identity topped up: ${identityId} (+${additionalCredits} credits)`);
            }

            return {
                identityId,
                additionalCredits,
                txId: transaction.id,
                timestamp: Date.now()
            };

        } catch (error) {
            throw new Error(`Identity topup failed: ${error.message}`);
        }
    }

    /**
     * Validate funding request against safety limits
     */
    validateFundingRequest(amount) {
        if (amount > this.maxFundingPerOperation) {
            throw new Error(`Funding request ${amount} exceeds maximum ${this.maxFundingPerOperation} satoshis`);
        }

        if (this.currentDailyUsage + amount > this.dailyUsageLimit) {
            throw new Error(`Funding would exceed daily limit: ${this.currentDailyUsage + amount}/${this.dailyUsageLimit}`);
        }
    }

    /**
     * Record funding usage for monitoring
     */
    recordFunding(recipient, amount, txId) {
        const record = {
            recipient,
            amount,
            txId,
            timestamp: Date.now(),
            workerId: this.workerId,
            faucetId: this.faucetConfig.faucetId
        };

        this.fundingHistory.push(record);
        this.currentDailyUsage += amount;

        if (this.debug) {
            console.log(`📊 Usage updated: ${this.currentDailyUsage}/${this.dailyUsageLimit} (${((this.currentDailyUsage/this.dailyUsageLimit)*100).toFixed(1)}%)`);
        }
    }

    /**
     * Wait for transaction confirmation
     */
    async waitForTransactionConfirmation(transaction) {
        const maxWaitTime = 120000; // 2 minutes
        const startTime = Date.now();

        while (Date.now() - startTime < maxWaitTime) {
            try {
                // Check if transaction is confirmed via InstantLock
                const account = await this.faucetClient.getWalletAccount();
                const txInfo = account.getTransaction(transaction.id);
                
                if (txInfo && txInfo.isInstantLocked) {
                    return true;
                }

                await new Promise(resolve => setTimeout(resolve, 1000));
            } catch (error) {
                // Continue waiting
                await new Promise(resolve => setTimeout(resolve, 2000));
            }
        }

        throw new Error('Transaction confirmation timeout');
    }

    /**
     * Get faucet balance
     */
    async getFaucetBalance() {
        await this.initialize();
        const account = await this.faucetClient.getWalletAccount();
        return account.getTotalBalance();
    }

    /**
     * Get funding statistics
     */
    getFundingStats() {
        return {
            totalOperations: this.fundingHistory.length,
            totalUsage: this.currentDailyUsage,
            dailyLimit: this.dailyUsageLimit,
            remainingBudget: this.dailyUsageLimit - this.currentDailyUsage,
            usagePercentage: (this.currentDailyUsage / this.dailyUsageLimit * 100).toFixed(1),
            createdIdentitiesCount: this.createdIdentities.size,
            workerId: this.workerId,
            faucetId: this.faucetConfig.faucetId
        };
    }

    /**
     * Clean up created identities (for test teardown)
     */
    async cleanup() {
        if (this.debug && this.createdIdentities.size > 0) {
            console.log(`🧹 Cleaning up ${this.createdIdentities.size} created identities`);
        }

        // Note: Identity cleanup is complex as credits can't be easily recovered
        // For now, we just track them for monitoring
        const identityIds = Array.from(this.createdIdentities.keys());
        
        if (this.debug && identityIds.length > 0) {
            console.log(`📊 Created identities: ${identityIds.length}`);
            console.log(`💰 Total funding used: ${this.currentDailyUsage} satoshis`);
        }

        if (this.faucetClient) {
            await this.faucetClient.disconnect();
            this.faucetClient = null;
        }

        this.isInitialized = false;
    }

    /**
     * Export funding report for monitoring
     */
    exportFundingReport() {
        const report = {
            timestamp: new Date().toISOString(),
            network: this.network,
            workerId: this.workerId,
            faucetId: this.faucetConfig.faucetId,
            faucetAddress: this.faucetConfig.address,
            stats: this.getFundingStats(),
            fundingHistory: this.fundingHistory,
            createdIdentities: Array.from(this.createdIdentities.values()),
            environment: {
                nodeEnv: process.env.NODE_ENV,
                testEnv: process.env.TEST_ENVIRONMENT,
                skipSyncHeight: process.env.SKIP_SYNC_BEFORE_HEIGHT,
                useStorage: process.env.FAUCET_WALLET_USE_STORAGE
            }
        };

        return report;
    }

    /**
     * Safety check - verify we're on testnet
     */
    validateEnvironment() {
        if (this.network === 'mainnet') {
            throw new Error('🚨 DANGER: Funded tests are NOT allowed on mainnet!');
        }

        if (!['testnet', 'devnet', 'regtest'].includes(this.network)) {
            throw new Error(`Invalid network for funded tests: ${this.network}`);
        }

        // Additional safety check
        const isProduction = process.env.NODE_ENV === 'production' || 
                           process.env.CI === 'true' && process.env.ALLOW_FUNDED_CI !== 'true';
        
        if (isProduction && !process.env.ENABLE_FUNDED_TESTS) {
            throw new Error('Funded tests require explicit ENABLE_FUNDED_TESTS=true in production environments');
        }
    }

    /**
     * Emergency stop - prevents further funding
     */
    emergencyStop(reason) {
        console.error(`🚨 EMERGENCY STOP: ${reason}`);
        this.dailyUsageLimit = this.currentDailyUsage; // Prevent further funding
        
        if (this.faucetClient) {
            this.faucetClient.disconnect().catch(console.error);
        }
        
        throw new Error(`Emergency stop activated: ${reason}`);
    }

    /**
     * Check if funding is available
     */
    async canFund(amount) {
        if (!this.isInitialized) {
            await this.initialize();
        }

        try {
            // Check daily limits
            if (this.currentDailyUsage + amount > this.dailyUsageLimit) {
                return {
                    canFund: false,
                    reason: `Would exceed daily limit: ${this.currentDailyUsage + amount}/${this.dailyUsageLimit}`
                };
            }

            // Check per-operation limit
            if (amount > this.maxFundingPerOperation) {
                return {
                    canFund: false,
                    reason: `Amount ${amount} exceeds per-operation limit ${this.maxFundingPerOperation}`
                };
            }

            // Check faucet balance
            const faucetBalance = await this.getFaucetBalance();
            if (faucetBalance < amount + 10000000) { // Keep 0.1 DASH buffer
                return {
                    canFund: false,
                    reason: `Insufficient faucet balance: ${faucetBalance} < ${amount + 10000000} (with buffer)`
                };
            }

            return {
                canFund: true,
                faucetBalance,
                remainingDailyBudget: this.dailyUsageLimit - this.currentDailyUsage
            };

        } catch (error) {
            return {
                canFund: false,
                reason: `Balance check failed: ${error.message}`
            };
        }
    }
}

module.exports = WasmFaucetClient;