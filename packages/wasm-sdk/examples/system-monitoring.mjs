#!/usr/bin/env node

/**
 * System Monitoring Example
 * 
 * Comprehensive demonstration of system status, epoch information, and platform monitoring.
 * Shows status queries, epoch operations, quorum information, and system health monitoring.
 * 
 * Usage: node examples/system-monitoring.mjs [--network=testnet|mainnet] [--no-proofs] [--debug]
 */

import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
import { webcrypto } from 'crypto';

// Set up Node.js environment for WASM
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
if (!global.crypto) global.crypto = webcrypto;

// Import JavaScript wrapper (the correct approach)
import init from '../pkg/dash_wasm_sdk.js';
import { WasmSDK } from '../src-js/index.js';

async function main() {
    console.log('⚙️ Comprehensive System Monitoring Example');
    console.log('='.repeat(50));
    
    // Parse command line arguments
    const args = process.argv.slice(2);
    const network = args.find(arg => arg.startsWith('--network='))?.split('=')[1] || 'testnet';
    const useProofs = !args.includes('--no-proofs');
    const debugMode = args.includes('--debug');
    
    console.log(`🌐 Network: ${network.toUpperCase()}`);
    console.log(`🔒 Proofs: ${useProofs ? 'ENABLED' : 'DISABLED'}`);
    if (debugMode) console.log(`🐛 Debug: ENABLED`);
    
    try {
        // Pre-load WASM for Node.js compatibility
        console.log('\n📦 Pre-loading WASM for Node.js...');
        const wasmPath = join(__dirname, '../pkg/dash_wasm_sdk_bg.wasm');
        await init(readFileSync(wasmPath));
        
        // Initialize JavaScript wrapper
        console.log('📦 Initializing JavaScript wrapper...');
        const sdk = new WasmSDK({
            network: network,
            proofs: useProofs,
            debug: debugMode
        });
        await sdk.initialize();
        console.log('✅ SDK initialized successfully\n');
        
        // === SYSTEM STATUS ===
        console.log('📊 SYSTEM STATUS');
        console.log('-'.repeat(20));
        
        try {
            const status = await sdk.getStatus();
            console.log('✅ Platform Status:');
            console.log(`   Version: ${status.version || 'N/A'}`);
            console.log(`   Network: ${status.network || 'N/A'}`);
            console.log(`   Chain Height: ${status.chain?.latestBlockHeight || 'N/A'}`);
            console.log(`   Core Height: ${status.chain?.coreChainLockedHeight || 'N/A'}`);
            console.log(`   Peers: ${status.network?.peersCount || 'N/A'}`);
            console.log(`   Listening: ${status.network?.listening || 'N/A'}`);
        } catch (error) {
            console.log(`⚠️ Status query failed: ${error.message}`);
        }
        
        // === EPOCH INFORMATION ===
        console.log('\n🕐 EPOCH INFORMATION');
        console.log('-'.repeat(25));
        
        try {
            const currentEpoch = await sdk.getCurrentEpoch();
            console.log(`✅ Current epoch: ${currentEpoch}`);
            
            // Get recent epochs
            const epochsInfo = await sdk.getEpochsInfo(Math.max(1, currentEpoch - 2), 3, true);
            console.log(`✅ Recent epochs: ${epochsInfo.length} retrieved`);
            
            // Get finalized epochs
            const finalizedEpochs = await sdk.getFinalizedEpochInfos(3, false);
            console.log(`✅ Finalized epochs: ${finalizedEpochs.length} retrieved`);
        } catch (error) {
            console.log(`⚠️ Epoch operations failed: ${error.message}`);
        }
        
        // === QUORUM INFORMATION ===
        console.log('\n🏛️  QUORUM INFORMATION');
        console.log('-'.repeat(30));
        
        try {
            const quorums = await sdk.getCurrentQuorumsInfo();
            console.log('✅ Current Quorums:');
            if (quorums && quorums.length) {
                console.log(`   Active quorums: ${quorums.length}`);
                quorums.slice(0, 3).forEach((quorum, index) => {
                    console.log(`   Quorum ${index + 1}: ${quorum.type || 'N/A'} (${quorum.membersCount || 'N/A'} members)`);
                });
            } else {
                console.log(`   Quorum data structure: ${typeof quorums}`);
            }
        } catch (error) {
            console.log(`⚠️ Quorum query failed: ${error.message}`);
        }
        
        // === PLATFORM METRICS ===
        console.log('\n💰 PLATFORM METRICS');
        console.log('-'.repeat(25));
        
        try {
            const totalCredits = await sdk.getTotalCreditsInPlatform();
            console.log(`✅ Total platform credits: ${totalCredits}`);
        } catch (error) {
            console.log(`⚠️ Platform credits query failed: ${error.message}`);
        }
        
        // === LOW-LEVEL PATH QUERIES ===
        console.log('\n🛤️  LOW-LEVEL PATH QUERIES');
        console.log('-'.repeat(35));
        
        try {
            // Test path elements (low-level GroveDB access)
            const pathResult = await sdk.getPathElements(['32'], []); // Identities path
            console.log(`✅ Path elements query completed: ${typeof pathResult}`);
        } catch (error) {
            console.log(`⚠️ Path elements failed: ${error.message}`);
        }
        
        // === PROTOCOL VERSION QUERIES ===
        console.log('\n🔄 PROTOCOL VERSION INFORMATION');
        console.log('-'.repeat(40));
        
        try {
            const upgradeState = await sdk.getProtocolVersionUpgradeState();
            console.log('✅ Protocol upgrade state:');
            console.log(`   Current version: ${upgradeState.currentVersion || 'N/A'}`);
            console.log(`   Pending upgrade: ${upgradeState.pendingUpgrade || 'None'}`);
        } catch (error) {
            console.log(`⚠️ Protocol version query failed: ${error.message}`);
        }
        
        try {
            const voteStatus = await sdk.getProtocolVersionUpgradeVoteStatus();
            console.log(`✅ Protocol upgrade vote status: ${typeof voteStatus}`);
        } catch (error) {
            console.log(`⚠️ Protocol vote status failed: ${error.message}`);
        }
        
        // === MONITORING DASHBOARD EXAMPLE ===
        console.log('\n📈 MONITORING DASHBOARD SIMULATION');
        console.log('-'.repeat(45));
        
        const monitoringData = {
            timestamp: new Date().toISOString(),
            network: network,
            proofs: useProofs
        };
        
        // Collect all available metrics
        try {
            monitoringData.status = await sdk.getStatus();
        } catch (e) { monitoringData.statusError = e.message; }
        
        try {
            monitoringData.currentEpoch = await sdk.getCurrentEpoch();
        } catch (e) { monitoringData.epochError = e.message; }
        
        try {
            monitoringData.totalCredits = await sdk.getTotalCreditsInPlatform();
        } catch (e) { monitoringData.creditsError = e.message; }
        
        console.log('Dashboard Data Collected:');
        console.log(`✓ Timestamp: ${monitoringData.timestamp}`);
        console.log(`✓ Status: ${monitoringData.status ? 'Available' : 'Error'}`);
        console.log(`✓ Current epoch: ${monitoringData.currentEpoch || 'Error'}`);
        console.log(`✓ Total credits: ${monitoringData.totalCredits || 'Error'}`);
        
        // === SUMMARY ===
        console.log('\n📊 MONITORING CAPABILITIES SUMMARY');
        console.log('-'.repeat(40));
        console.log('✅ Real-time platform status monitoring');
        console.log('✅ Epoch and blockchain information tracking');
        console.log('✅ Quorum and consensus monitoring');
        console.log('✅ Platform economics (total credits)');
        console.log('✅ Low-level state tree access');
        console.log('✅ Protocol version upgrade tracking');
        console.log('✅ Complete monitoring dashboard data collection');
        
        // Clean up
        await sdk.destroy();
        console.log('\n🎉 System monitoring demonstration completed successfully!');
        
    } catch (error) {
        console.log(`❌ System monitoring failed: ${error.message}`);
        
        if (error.message.includes('fetch') || error.message.includes('network')) {
            console.log('🌐 Network connectivity required for platform monitoring');
        }
        
        console.log('\nFor debugging:');
        console.log('1. Ensure network connectivity');
        console.log('2. Try with --no-proofs for faster queries');
        console.log('3. Use --debug for detailed output');
        console.log('4. Check if platform endpoint is accessible');
        
        process.exit(1);
    }
}

await main();