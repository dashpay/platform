/**
 * Identity Pool Manager
 * Manages a pool of pre-funded identities for efficient WASM SDK testing
 */

const WasmFaucetClient = require('./wasm-faucet-client');

class IdentityPool {
    constructor(options = {}) {
        this.poolSize = options.poolSize || 20;
        this.minBalance = options.minBalance || 10000000; // 10M credits minimum
        this.refundThreshold = options.refundThreshold || 5000000; // 5M credits refund threshold
        this.initialCredits = options.initialCredits || 50000000; // 50M credits per identity
        
        this.debug = options.debug || false;
        this.network = options.network || 'testnet';
        
        // Pool management
        this.availableIdentities = new Map();
        this.inUseIdentities = new Map();
        this.faucetClient = new WasmFaucetClient({ 
            network: this.network, 
            debug: this.debug 
        });
        
        // Pool statistics
        this.stats = {
            totalCreated: 0,
            totalFunded: 0,
            totalCreditsDistributed: 0,
            poolInitializedAt: null,
            lastRefundAt: null
        };

        this.isInitialized = false;
    }

    /**
     * Initialize the identity pool
     */
    async initialize() {
        if (this.isInitialized) return;

        try {
            if (this.debug) {
                console.log('🏊 Initializing identity pool...');
                console.log(`   Pool Size: ${this.poolSize} identities`);
                console.log(`   Initial Credits: ${this.initialCredits} per identity`);
                console.log(`   Network: ${this.network}`);
            }

            await this.faucetClient.initialize();

            // Check if we can afford to create the pool
            const totalRequiredSatoshis = this.poolSize * (Math.ceil(this.initialCredits / 1000) + 10000000);
            const canFund = await this.faucetClient.canFund(totalRequiredSatoshis);
            
            if (!canFund.canFund) {
                throw new Error(`Cannot create identity pool: ${canFund.reason}`);
            }

            // Pre-create identities (but only if needed)
            const existingPool = this.loadExistingPool();
            if (existingPool.size >= Math.floor(this.poolSize / 2)) {
                if (this.debug) {
                    console.log(`📦 Using existing pool: ${existingPool.size} identities available`);
                }
                this.availableIdentities = existingPool;
            } else {
                await this.createInitialPool();
            }

            this.stats.poolInitializedAt = Date.now();
            this.isInitialized = true;

            if (this.debug) {
                console.log(`✅ Identity pool ready: ${this.availableIdentities.size} available`);
            }

        } catch (error) {
            throw new Error(`Identity pool initialization failed: ${error.message}`);
        }
    }

    /**
     * Create initial pool of identities
     */
    async createInitialPool() {
        const identitiesNeeded = Math.min(this.poolSize, 5); // Start with smaller batch
        
        if (this.debug) {
            console.log(`🏗️ Creating ${identitiesNeeded} funded identities...`);
        }

        const creationPromises = [];
        for (let i = 0; i < identitiesNeeded; i++) {
            creationPromises.push(this.createPoolIdentity(i));
        }

        const results = await Promise.allSettled(creationPromises);
        
        let successCount = 0;
        let failureCount = 0;
        
        results.forEach((result, index) => {
            if (result.status === 'fulfilled') {
                const { identityId, privateKey, creditsAmount } = result.value;
                this.availableIdentities.set(identityId, {
                    identityId,
                    privateKey,
                    creditsAmount,
                    createdAt: Date.now(),
                    lastUsed: null,
                    usageCount: 0
                });
                successCount++;
                this.stats.totalCreated++;
            } else {
                failureCount++;
                console.warn(`⚠️ Failed to create identity ${index}: ${result.reason}`);
            }
        });

        if (successCount === 0) {
            throw new Error('Failed to create any identities for the pool');
        }

        if (this.debug) {
            console.log(`✅ Pool creation completed: ${successCount} success, ${failureCount} failed`);
        }

        this.savePool();
    }

    /**
     * Create a single identity for the pool
     */
    async createPoolIdentity(index) {
        try {
            const result = await this.faucetClient.createFundedIdentity(this.initialCredits);
            
            if (this.debug) {
                console.log(`   Identity ${index + 1}: ${result.identityId} (${this.initialCredits} credits)`);
            }

            return result;
        } catch (error) {
            throw new Error(`Pool identity ${index} creation failed: ${error.message}`);
        }
    }

    /**
     * Get an available identity from the pool
     */
    async getAvailableIdentity(requiredCredits = 1000000) { // 1M credits default
        await this.initialize();

        // Find suitable identity
        for (const [identityId, identityInfo] of this.availableIdentities.entries()) {
            if (identityInfo.creditsAmount >= requiredCredits) {
                // Move to in-use pool
                this.inUseIdentities.set(identityId, identityInfo);
                this.availableIdentities.delete(identityId);
                
                // Update usage stats
                identityInfo.lastUsed = Date.now();
                identityInfo.usageCount++;
                
                if (this.debug) {
                    console.log(`🎯 Allocated identity: ${identityId} (${identityInfo.creditsAmount} credits available)`);
                }
                
                return identityInfo;
            }
        }

        // No suitable identity found - create a new one
        if (this.debug) {
            console.log(`🏗️ Creating new identity for ${requiredCredits} credits requirement`);
        }
        
        const creditsToCreate = Math.max(requiredCredits * 2, this.initialCredits); // Create with buffer
        const result = await this.faucetClient.createFundedIdentity(creditsToCreate);
        
        const identityInfo = {
            identityId: result.identityId,
            privateKey: result.privateKey,
            creditsAmount: result.creditsAmount,
            createdAt: Date.now(),
            lastUsed: Date.now(),
            usageCount: 1
        };

        this.inUseIdentities.set(result.identityId, identityInfo);
        this.stats.totalCreated++;

        return identityInfo;
    }

    /**
     * Return identity to the pool after use
     */
    returnIdentity(identityId, remainingCredits) {
        const identityInfo = this.inUseIdentities.get(identityId);
        if (!identityInfo) {
            console.warn(`⚠️ Attempted to return unknown identity: ${identityId}`);
            return;
        }

        // Update credits amount
        identityInfo.creditsAmount = remainingCredits;
        
        // Return to available pool if it still has enough credits
        if (remainingCredits >= this.minBalance) {
            this.availableIdentities.set(identityId, identityInfo);
            
            if (this.debug) {
                console.log(`🔄 Returned identity to pool: ${identityId} (${remainingCredits} credits remaining)`);
            }
        } else {
            if (this.debug) {
                console.log(`🗑️ Identity depleted: ${identityId} (${remainingCredits} credits < ${this.minBalance} minimum)`);
            }
        }

        this.inUseIdentities.delete(identityId);
        this.savePool();
    }

    /**
     * Refund low-balance identities
     */
    async refundLowBalanceIdentities() {
        const identitiesNeedingRefund = [];
        
        for (const [identityId, identityInfo] of this.availableIdentities.entries()) {
            if (identityInfo.creditsAmount < this.refundThreshold) {
                identitiesNeedingRefund.push({ identityId, identityInfo });
            }
        }

        if (identitiesNeedingRefund.length === 0) {
            if (this.debug) {
                console.log('✅ No identities need refunding');
            }
            return;
        }

        if (this.debug) {
            console.log(`💰 Refunding ${identitiesNeedingRefund.length} low-balance identities`);
        }

        const refundAmount = this.initialCredits - this.refundThreshold;
        
        for (const { identityId, identityInfo } of identitiesNeedingRefund) {
            try {
                await this.faucetClient.topupIdentity(
                    identityId,
                    identityInfo.privateKey,
                    refundAmount
                );
                
                identityInfo.creditsAmount += refundAmount;
                this.stats.totalFunded++;
                this.stats.totalCreditsDistributed += refundAmount;
                
                if (this.debug) {
                    console.log(`   ✅ Refunded ${identityId}: +${refundAmount} credits`);
                }
                
            } catch (error) {
                console.warn(`   ⚠️ Failed to refund ${identityId}: ${error.message}`);
            }
        }

        this.stats.lastRefundAt = Date.now();
        this.savePool();
    }

    /**
     * Get pool statistics
     */
    getPoolStats() {
        return {
            available: this.availableIdentities.size,
            inUse: this.inUseIdentities.size,
            total: this.availableIdentities.size + this.inUseIdentities.size,
            targetSize: this.poolSize,
            ...this.stats,
            faucetStats: this.faucetClient.getFundingStats()
        };
    }

    /**
     * Load existing pool from storage (placeholder for future persistence)
     */
    loadExistingPool() {
        // TODO: Implement pool persistence
        // For now, return empty Map
        return new Map();
    }

    /**
     * Save pool to storage (placeholder for future persistence) 
     */
    savePool() {
        // TODO: Implement pool persistence
        if (this.debug) {
            console.log(`💾 Pool state: ${this.availableIdentities.size} available, ${this.inUseIdentities.size} in use`);
        }
    }

    /**
     * Monitor pool health and auto-maintain
     */
    async maintainPool() {
        await this.initialize();

        if (this.debug) {
            console.log('🔧 Maintaining identity pool...');
        }

        // Refund low-balance identities
        await this.refundLowBalanceIdentities();

        // Create additional identities if pool is low
        const availableCount = this.availableIdentities.size;
        const minPoolSize = Math.floor(this.poolSize / 3);
        
        if (availableCount < minPoolSize) {
            const identitiesNeeded = minPoolSize - availableCount;
            
            if (this.debug) {
                console.log(`📈 Pool low (${availableCount}/${this.poolSize}), creating ${identitiesNeeded} identities`);
            }
            
            try {
                for (let i = 0; i < identitiesNeeded; i++) {
                    const result = await this.faucetClient.createFundedIdentity(this.initialCredits);
                    this.availableIdentities.set(result.identityId, {
                        identityId: result.identityId,
                        privateKey: result.privateKey,
                        creditsAmount: result.creditsAmount,
                        createdAt: Date.now(),
                        lastUsed: null,
                        usageCount: 0
                    });
                    this.stats.totalCreated++;
                }
                
                this.savePool();
            } catch (error) {
                console.warn(`⚠️ Pool maintenance failed: ${error.message}`);
            }
        }

        if (this.debug) {
            const stats = this.getPoolStats();
            console.log(`✅ Pool maintenance complete: ${stats.available} available, ${stats.inUse} in use`);
        }
    }

    /**
     * Cleanup the entire pool
     */
    async cleanup() {
        if (this.debug) {
            console.log('🧹 Cleaning up identity pool...');
        }

        const stats = this.getPoolStats();
        
        if (this.debug) {
            console.log(`📊 Pool cleanup summary:`);
            console.log(`   Total identities created: ${stats.totalCreated}`);
            console.log(`   Total credits distributed: ${stats.totalCreditsDistributed}`);
            console.log(`   Available identities: ${stats.available}`);
            console.log(`   In-use identities: ${stats.inUse}`);
        }

        // Cleanup faucet client
        await this.faucetClient.cleanup();

        // Clear pools
        this.availableIdentities.clear();
        this.inUseIdentities.clear();
        
        this.isInitialized = false;
    }
}

module.exports = IdentityPool;