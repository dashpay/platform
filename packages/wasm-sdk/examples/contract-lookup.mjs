#!/usr/bin/env node

/**
 * Contract Lookup CLI with Proof Verification Control
 * 
 * Comprehensive contract analysis tool showing contract details, document types, and documents.
 * Uses the modern JavaScript wrapper with configurable proof verification.
 * 
 * Usage: node examples/contract-lookup.mjs [contract-id] [document-type] [--no-proofs]
 */

import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
import { webcrypto } from 'crypto';

// Set up Node.js environment for WASM
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
if (!global.crypto) global.crypto = webcrypto;

// Pre-load WASM for Node.js
import init from '../pkg/wasm_sdk.js';

// Load environment configuration
function loadEnv() {
    try {
        const envPath = join(dirname(fileURLToPath(import.meta.url)), '../.env');
        const envFile = readFileSync(envPath, 'utf8');
        const env = {};
        
        for (const line of envFile.split('\n')) {
            const trimmed = line.trim();
            if (trimmed && !trimmed.startsWith('#')) {
                const [key, ...valueParts] = trimmed.split('=');
                if (key && valueParts.length > 0) {
                    let value = valueParts.join('=');
                    if (value.startsWith('"') && value.endsWith('"')) {
                        value = value.slice(1, -1);
                    }
                    env[key] = value;
                }
            }
        }
        return env;
    } catch (error) {
        console.log('⚠️ Could not load .env file, using defaults');
        return {};
    }
}

// Import JavaScript wrapper (the correct approach)
import { WasmSDK } from '../src-js/index.js';

async function main() {
    console.log('🔍 Contract Lookup CLI');
    console.log('='.repeat(30));
    
    // Load environment configuration
    const env = loadEnv();
    
    // Parse command line arguments first
    const args = process.argv.slice(2);
    const debugMode = args.includes('--debug'); // Debug disabled by default
    
    // Control debug output - set environment variable for WASM module
    if (debugMode) {
        process.env.WASM_DEBUG = 'true';
        process.env.RUST_LOG = 'debug';
    } else {
        process.env.WASM_DEBUG = 'false';  
        process.env.RUST_LOG = 'error';
    }
    
    const contractId = args.find(arg => !arg.startsWith('--')) || 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec'; // DPNS contract
    let documentType = args[1] && !args[1].startsWith('--') ? args[1] : null;
    const useProofs = !args.includes('--no-proofs'); // Default to proofs enabled
    const network = env.NETWORK || 'testnet';

    // Contract information and document types
    const knownContracts = {
        'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec': {
            name: 'DPNS',
            types: ['domain', 'preorder'],
            default: 'domain'
        },
        'Bwr4WHCPz5rFVAD87RqTs3izo4zpzwsEdKPWUT1NS1C7': {
            name: 'DashPay', 
            types: ['contactInfo', 'profile'],
            default: 'profile'  // profile has more documents
        }
    };

    // Auto-detect document type if not provided
    if (!documentType) {
        const contractInfo = knownContracts[contractId];
        if (contractInfo) {
            documentType = contractInfo.default;
            console.log(`🔍 Auto-detected document type: ${documentType} (${contractInfo.name} contract)`);
        } else {
            documentType = 'domain'; // fallback
            console.log(`🔍 Auto-detected document type: ${documentType} (unknown contract)`);
        }
    }
    
    if (args.includes('--help') || args.includes('-h')) {
        console.log('Usage: node examples/contract-lookup.mjs [contract-id] [document-type] [--no-proofs] [--debug]');
        console.log('');
        console.log('Arguments:');
        console.log('  contract-id     Data contract ID (default: DPNS contract)');
        console.log('  document-type   Document type (default: auto-detected)');
        console.log('');
        console.log('Options:');
        console.log('  --no-proofs    Disable proof verification (default: proofs enabled)');
        console.log('  --debug        Enable debug output (default: disabled)');
        console.log('  --help, -h     Show this help message');
        console.log('');
        console.log('Examples:');
        console.log('  node examples/document-lookup.mjs                    # DPNS domains with proofs');
        console.log('  node examples/document-lookup.mjs --no-proofs        # DPNS domains without proofs');
        console.log('  node examples/document-lookup.mjs <contract> <type>  # Custom contract/type');
        process.exit(0);
    }
    
    console.log(`🌐 Network: ${network.toUpperCase()}`);
    console.log(`📄 Contract: ${contractId}`);
    console.log(`📋 Document Type: ${documentType}`);
    console.log(`🔒 Proofs: ${useProofs ? 'ENABLED' : 'DISABLED'} (default: enabled)`);
    if (debugMode) {
        console.log(`🐛 Debug: ENABLED`);
    }
    
    try {
        // Pre-load WASM for Node.js compatibility
        console.log('📦 Pre-loading WASM for Node.js...');
        const wasmPath = join(__dirname, '../pkg/wasm_sdk_bg.wasm');
        await init(readFileSync(wasmPath));
        
        console.log('📦 Initializing JavaScript wrapper...');
        
        // Use JavaScript wrapper (the correct approach)  
        const sdk = new WasmSDK({
            network: network,
            proofs: useProofs, // Use the proof setting from command line
            debug: debugMode, // Use debug mode from command line
            transport: {
                timeout: 60000,
                retries: 5
            }
        });
        
        await sdk.initialize();
        console.log(`✅ SDK initialized successfully (proofs: ${useProofs ? 'enabled' : 'disabled'})`);
        
        console.log(`🔍 Analyzing contract: ${contractId}`);
        
        // First, get contract information to discover document types
        console.log('📄 Fetching contract details...');
        const contract = await sdk.getDataContract(contractId);
        
        if (!contract) {
            console.log('❌ Contract not found');
            process.exit(1);
        }
        
        const contractData = typeof contract.toJSON === 'function' ? contract.toJSON() : contract;
        
        console.log('\n📊 Contract Information:');
        console.log(`   📄 Contract ID: ${contractId}`);
        console.log(`   👤 Owner ID: ${contractData.ownerId || contractData.$ownerId || 'N/A'}`);
        console.log(`   🔄 Version: ${contractData.version || contractData.$version || 'N/A'}`);
        
        // Debug contract data structure
        console.log('🔍 Contract data structure:');
        console.log('   Available fields:', Object.keys(contractData));
        
        // Get all document types from contract (check different possible locations)
        let documentTypes = [];
        if (contractData.documents) {
            documentTypes = Object.keys(contractData.documents);
        } else if (contractData.documentTypes) {
            documentTypes = Object.keys(contractData.documentTypes);
        } else if (contractData.schema && contractData.schema.documents) {
            documentTypes = Object.keys(contractData.schema.documents);
        } else if (contractData.data && contractData.data.documentSchemas) {
            documentTypes = Object.keys(contractData.data.documentSchemas);
        }
        
        console.log(`   📋 Document Types Found: ${documentTypes.length}`);
        console.log(`   📝 Document Types: ${documentTypes.join(', ') || 'None'}`);
        
        // If no document types found, show contract structure for debugging
        if (documentTypes.length === 0) {
            console.log('\n🔍 Full contract structure:');
            console.log(JSON.stringify(contractData, null, 2).substring(0, 500) + '...');
        }
        
        if (documentTypes.length === 0) {
            console.log('❌ No document types found in contract');
            return;
        }
        
        // Query all document types (collect data silently)
        console.log('\n🔍 Retrieving all documents from contract...');
        
        const results = {
            contract: {
                id: contractId,
                ownerId: contractData.ownerId || contractData.$ownerId || null,
                version: contractData.version || contractData.$version || null,
                documentTypes: documentTypes.length,
                totalDocuments: 0
            },
            documents: {}
        };
        
        for (const docType of documentTypes) {
            console.log(`   📄 Fetching ${docType} documents...`);
            
            try {
                // JavaScript wrapper now returns structured JSON response with complete WASM SDK data
                const response = await sdk.getDocuments(contractId, docType, { getAllDocuments: true });
                
                results.documents[docType] = {
                    count: response.totalCount,
                    documents: response.documents.map(doc => doc.$id || doc.id || doc.identifier)
                };
                
                results.contract.totalDocuments += response.totalCount;
                console.log(`   ✅ ${response.totalCount} ${docType} documents`);
                
            } catch (error) {
                console.log(`   ❌ Error querying ${docType}: ${error.message}`);
                results.documents[docType] = {
                    count: 0,
                    documents: [],
                    error: error.message
                };
            }
        }
        
        // Output clean JSON results
        console.log('\n📄 Contract Analysis Results:');
        console.log('='.repeat(50));
        console.log(JSON.stringify(results, null, 2));
        
        // Show clean summary table
        console.log('\n📊 Document Summary Table:');
        console.log('='.repeat(50));
        console.log(`Contract: ${contractId}`);
        console.log(`Owner: ${results.contract.ownerId || 'N/A'}`);
        console.log(`Version: ${results.contract.version || 'N/A'}`);
        console.log(`Document Types: ${results.contract.documentTypes}`);
        console.log(`Total Documents: ${results.contract.totalDocuments}`);
        console.log('');
        console.log('Document Type Breakdown:');
        Object.entries(results.documents).forEach(([type, data]) => {
            if (data.error) {
                console.log(`  ${type}: ERROR - ${data.error}`);
            } else {
                console.log(`  ${type}: ${data.count} documents`);
            }
        });
        
        if (results.contract.totalDocuments > 0) {
            console.log(`\n✅ SUCCESS! Retrieved ${results.contract.totalDocuments} total documents`);
        } else {
            console.log('\n⚠️ No documents found in contract');
        }
        
        // Clean up
        await sdk.destroy();
        console.log('\n🎉 Document lookup completed successfully!');
        
    } catch (error) {
        console.log(`❌ Document lookup failed: ${error.message}`);
        
        if (error.message.includes('Non-trusted mode')) {
            console.log('🔧 Trusted mode error - JavaScript wrapper issue');
        } else if (error.message.includes('fetch')) {
            console.log('🌐 Network connectivity issue');
        } else if (error.message.includes('not found')) {
            console.log('📄 Contract or document type may not exist');
        }
        
        console.log('\nFor debugging:');
        console.log('1. Verify contract ID exists on testnet');
        console.log('2. Check document type is correct');
        console.log('3. Try with --no-proofs for faster testing');
        
        process.exit(1);
    }
}

await main();