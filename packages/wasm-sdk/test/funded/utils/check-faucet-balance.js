#!/usr/bin/env node

/**
 * Check Faucet Balance Utility
 * Validates faucet wallet balance and funding capacity
 */

require('dotenv').config();
const WasmFaucetClient = require('./wasm-faucet-client');

async function checkFaucetBalance() {
    console.log('🚰 Checking Faucet Balance');
    console.log('=========================');

    try {
        // Initialize faucet client
        const faucet = new WasmFaucetClient({ debug: true });
        await faucet.initialize();

        // Get balance
        const balance = await faucet.getFaucetBalance();
        const balanceInDash = balance / 1e8;

        console.log(`💰 Faucet Balance: ${balance} satoshis (${balanceInDash.toFixed(6)} DASH)`);

        // Get funding stats
        const stats = faucet.getFundingStats();
        
        console.log('\n📊 Funding Statistics:');
        console.log(`   Worker ID: ${stats.workerId}`);
        console.log(`   Faucet ID: ${stats.faucetId}`);
        console.log(`   Daily Usage: ${stats.totalUsage} / ${stats.dailyLimit} satoshis`);
        console.log(`   Usage Percentage: ${stats.usagePercentage}%`);
        console.log(`   Remaining Budget: ${stats.remainingBudget} satoshis`);

        // Calculate funding capacity
        console.log('\n🎯 Funding Capacity Analysis:');
        
        const tierCosts = {
            low: 50000000,      // 50M credits = 50K satoshis
            medium: 200000000,  // 200M credits = 200K satoshis 
            high: 500000000     // 500M credits = 500K satoshis
        };

        Object.entries(tierCosts).forEach(([tier, costInCredits]) => {
            const costInSatoshis = costInCredits / 1000; // Credits to satoshis conversion
            const operationsSupported = Math.floor(balance / costInSatoshis);
            console.log(`   ${tier.toUpperCase()} tier: ${operationsSupported} operations (${costInSatoshis} satoshis each)`);
        });

        // Health check
        console.log('\n🏥 Health Check:');
        
        if (balance < 10000000) { // Less than 0.1 DASH
            console.log('🚨 CRITICAL: Faucet balance very low');
        } else if (balance < 100000000) { // Less than 1 DASH
            console.log('⚠️ WARNING: Faucet balance low');
        } else {
            console.log('✅ Faucet balance healthy');
        }

        // Check if can fund common operations
        const canFundLow = await faucet.canFund(50000); // Low tier operation
        const canFundMedium = await faucet.canFund(200000); // Medium tier operation
        const canFundHigh = await faucet.canFund(500000); // High tier operation

        console.log('\n🎯 Funding Readiness:');
        console.log(`   Low Tier: ${canFundLow.canFund ? '✅' : '❌'} ${canFundLow.canFund ? '' : canFundLow.reason}`);
        console.log(`   Medium Tier: ${canFundMedium.canFund ? '✅' : '❌'} ${canFundMedium.canFund ? '' : canFundMedium.reason}`);
        console.log(`   High Tier: ${canFundHigh.canFund ? '✅' : '❌'} ${canFundHigh.canFund ? '' : canFundHigh.reason}`);

        // Cleanup
        await faucet.cleanup();

        console.log('\n🎉 Faucet check completed successfully');
        return 0;

    } catch (error) {
        console.error(`❌ Faucet check failed: ${error.message}`);
        
        console.log('\n🔧 Troubleshooting:');
        console.log('1. Verify funded/.env configuration');
        console.log('2. Check FAUCET_1_ADDRESS and FAUCET_1_PRIVATE_KEY');
        console.log('3. Ensure network connectivity');
        console.log('4. Confirm faucet wallet has sufficient balance');
        
        return 1;
    }
}

checkFaucetBalance()
    .then(code => process.exit(code))
    .catch(error => {
        console.error('💥 Check crashed:', error.message);
        process.exit(1);
    });