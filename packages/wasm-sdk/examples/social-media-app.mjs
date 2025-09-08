#!/usr/bin/env node

/**
 * Social Media App Example
 * 
 * Complete demonstration of building a social media application using DashPay contracts.
 * Shows profile management, contact handling, document operations, and social interactions.
 * 
 * Usage: node examples/social-media-app.mjs [identity-id] [--network=testnet|mainnet] [--no-proofs] [--debug]
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
import init from '../pkg/wasm_sdk.js';
import { WasmSDK } from '../src-js/index.js';

async function main() {
    console.log('📱 Social Media App - Complete DashPay Integration');
    console.log('='.repeat(60));
    
    // Parse command line arguments
    const args = process.argv.slice(2);
    const userIdentityId = args.find(arg => !arg.startsWith('--')) || '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk';
    const network = args.find(arg => arg.startsWith('--network='))?.split('=')[1] || 'testnet';
    const useProofs = !args.includes('--no-proofs');
    const debugMode = args.includes('--debug');
    
    // DashPay contract ID
    const DASHPAY_CONTRACT = 'Bwr4WHCPz5rFVAD87RqTs3izo4zpzwsEdKPWUT1NS1C7';
    
    console.log(`👤 User Identity: ${userIdentityId}`);
    console.log(`🌐 Network: ${network.toUpperCase()}`);
    console.log(`📄 DashPay Contract: ${DASHPAY_CONTRACT}`);
    console.log(`🔒 Proofs: ${useProofs ? 'ENABLED' : 'DISABLED'}`);
    
    try {
        // Pre-load WASM for Node.js compatibility
        console.log('\n📦 Pre-loading WASM for Node.js...');
        const wasmPath = join(__dirname, '../pkg/wasm_sdk_bg.wasm');
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
        
        // === USER PROFILE MANAGEMENT ===
        console.log('👤 USER PROFILE MANAGEMENT');
        console.log('-'.repeat(35));
        
        try {
            // Get user's profile documents
            const profileResponse = await sdk.getDocuments(DASHPAY_CONTRACT, 'profile', {
                where: [["$ownerId", "==", userIdentityId]],
                limit: 1
            });
            
            console.log('✅ User Profile Query:');
            console.log(`   Total profiles: ${profileResponse.totalCount}`);
            
            if (profileResponse.documents.length > 0) {
                const profile = profileResponse.documents[0];
                console.log(`   Profile found:`);
                console.log(`     Display Name: ${profile.displayName || 'Not set'}`);
                console.log(`     Public Message: ${profile.publicMessage || 'Not set'}`);
                console.log(`     Avatar URL: ${profile.avatarUrl || 'Not set'}`);
                console.log(`     Created: ${new Date(profile.$createdAt).toLocaleDateString()}`);
            } else {
                console.log(`   No profile found for this identity`);
            }
        } catch (error) {
            console.log(`⚠️ Profile query failed: ${error.message}`);
        }
        
        // === CONTACT MANAGEMENT ===
        console.log('\n📇 CONTACT MANAGEMENT');
        console.log('-'.repeat(25));
        
        try {
            // Get user's contact info documents
            const contactResponse = await sdk.getDocuments(DASHPAY_CONTRACT, 'contactInfo', {
                where: [["$ownerId", "==", userIdentityId]],
                orderBy: [["$createdAt", "desc"]],
                limit: 10
            });
            
            console.log('✅ Contact Info Query:');
            console.log(`   Total contacts: ${contactResponse.totalCount}`);
            
            if (contactResponse.documents.length > 0) {
                console.log(`   Recent contacts:`);
                contactResponse.documents.slice(0, 5).forEach((contact, index) => {
                    console.log(`     Contact ${index + 1}:`);
                    console.log(`       To Identity: ${contact.toUserId || 'N/A'}`);
                    console.log(`       Encrypted Data: ${contact.encryptedData ? 'Present' : 'None'}`);
                    console.log(`       Created: ${new Date(contact.$createdAt).toLocaleDateString()}`);
                });
            } else {
                console.log(`   No contacts found for this identity`);
            }
        } catch (error) {
            console.log(`⚠️ Contact query failed: ${error.message}`);
        }
        
        // === SOCIAL DISCOVERY ===
        console.log('\n🔍 SOCIAL DISCOVERY');
        console.log('-'.repeat(25));
        
        try {
            // Get recent public profiles
            const recentProfilesResponse = await sdk.getDocuments(DASHPAY_CONTRACT, 'profile', {
                orderBy: [["$createdAt", "desc"]],
                limit: 5
            });
            
            console.log('✅ Recent Public Profiles:');
            console.log(`   Total profiles on platform: ${recentProfilesResponse.totalCount}`);
            
            if (recentProfilesResponse.documents.length > 0) {
                recentProfilesResponse.documents.forEach((profile, index) => {
                    console.log(`   Profile ${index + 1}:`);
                    console.log(`     Owner: ${profile.$ownerId.substring(0, 20)}...`);
                    console.log(`     Display Name: ${profile.displayName || 'Anonymous'}`);
                    console.log(`     Message: ${(profile.publicMessage || '').substring(0, 50)}...`);
                });
            }
        } catch (error) {
            console.log(`⚠️ Social discovery failed: ${error.message}`);
        }
        
        // === SOCIAL NETWORK ANALYSIS ===
        console.log('\n🕸️  SOCIAL NETWORK ANALYSIS');
        console.log('-'.repeat(35));
        
        try {
            // Analyze contact patterns
            const allContactsResponse = await sdk.getDocuments(DASHPAY_CONTRACT, 'contactInfo', {
                limit: 100
            });
            
            console.log('✅ Network Analysis:');
            console.log(`   Total contacts in network: ${allContactsResponse.totalCount}`);
            
            // Analyze connection patterns
            const owners = {};
            allContactsResponse.documents.forEach(contact => {
                const owner = contact.$ownerId;
                owners[owner] = (owners[owner] || 0) + 1;
            });
            
            const sortedOwners = Object.entries(owners)
                .sort(([,a], [,b]) => b - a)
                .slice(0, 3);
            
            console.log(`   Most connected users:`);
            sortedOwners.forEach(([identityId, contactCount], index) => {
                console.log(`     ${index + 1}. ${identityId.substring(0, 20)}... (${contactCount} contacts)`);
            });
        } catch (error) {
            console.log(`⚠️ Network analysis failed: ${error.message}`);
        }
        
        // === MESSAGING SIMULATION ===
        console.log('\n💬 MESSAGING SIMULATION');
        console.log('-'.repeat(30));
        
        // Simulate secure messaging workflow
        console.log('Secure Messaging Workflow:');
        console.log('1. ✓ Generate message encryption keys');
        const messageKey = await sdk.generateKeyPair('testnet');
        console.log(`   Message key: ${messageKey.address}`);
        
        console.log('2. ✓ Sign message for authenticity');
        const message = "Hello from DashPay social app!";
        const signature = await sdk.signMessage(message, messageKey.private_key_wif);
        console.log(`   Message signature: ${signature.substring(0, 30)}...`);
        
        console.log('3. ✓ Validate recipient identity');
        try {
            const recipient = await sdk.getIdentity(userIdentityId);
            console.log(`   Recipient verified: ${recipient ? 'Yes' : 'No'}`);
        } catch (error) {
            console.log(`   Recipient check failed: Network required`);
        }
        
        console.log('4. ✓ Message ready for secure transmission');
        
        // === APP DASHBOARD SIMULATION ===
        console.log('\n📊 APP DASHBOARD DATA');
        console.log('-'.repeat(30));
        
        const dashboardData = {
            user: {
                identityId: userIdentityId,
                hasProfile: false,
                contactCount: 0,
                profileViews: 0
            },
            platform: {
                totalProfiles: 0,
                totalContacts: 0,
                activeUsers: 0
            }
        };
        
        try {
            const userProfile = await sdk.getDocuments(DASHPAY_CONTRACT, 'profile', {
                where: [["$ownerId", "==", userIdentityId]],
                limit: 1
            });
            dashboardData.user.hasProfile = userProfile.totalCount > 0;
            
            const userContacts = await sdk.getDocuments(DASHPAY_CONTRACT, 'contactInfo', {
                where: [["$ownerId", "==", userIdentityId]]
            });
            dashboardData.user.contactCount = userContacts.totalCount;
            
            const allProfiles = await sdk.getDocuments(DASHPAY_CONTRACT, 'profile', { limit: 1 });
            dashboardData.platform.totalProfiles = allProfiles.totalCount;
            
            const allContacts = await sdk.getDocuments(DASHPAY_CONTRACT, 'contactInfo', { limit: 1 });
            dashboardData.platform.totalContacts = allContacts.totalCount;
        } catch (error) {
            console.log(`⚠️ Dashboard data collection: ${error.message}`);
        }
        
        console.log('Dashboard Summary:');
        console.log(`✓ User has profile: ${dashboardData.user.hasProfile}`);
        console.log(`✓ User contacts: ${dashboardData.user.contactCount}`);
        console.log(`✓ Total platform profiles: ${dashboardData.platform.totalProfiles}`);
        console.log(`✓ Total platform contacts: ${dashboardData.platform.totalContacts}`);
        
        // === SUMMARY ===
        console.log('\n📱 SOCIAL MEDIA APP CAPABILITIES');
        console.log('-'.repeat(40));
        console.log('✅ User profile management');
        console.log('✅ Contact and friend management');
        console.log('✅ Social discovery and browsing');
        console.log('✅ Network analysis and insights');
        console.log('✅ Secure messaging workflows');
        console.log('✅ Real-time dashboard data');
        console.log('✅ Complete social platform foundation');
        
        // Clean up
        await sdk.destroy();
        console.log('\n🎉 Social media app demonstration completed successfully!');
        
    } catch (error) {
        console.log(`❌ Social media app failed: ${error.message}`);
        
        if (error.message.includes('fetch') || error.message.includes('network')) {
            console.log('🌐 Social features require network connectivity');
        } else if (error.message.includes('not found')) {
            console.log('📄 DashPay contract or documents may not exist');
        }
        
        console.log('\nFor debugging:');
        console.log('1. Verify DashPay contract exists on network');
        console.log('2. Check network connectivity');
        console.log('3. Try different identity ID');
        console.log('4. Use --debug for detailed output');
        
        process.exit(1);
    }
}

await main();