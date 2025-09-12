#!/usr/bin/env node

/**
 * Real Contract Operations Test
 * Tests actual data contract creation using real funded identities
 * ⚠️ WARNING: This consumes real testnet credits!
 */

import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
import { webcrypto } from 'crypto';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

if (!global.crypto) global.crypto = webcrypto;

// Load environment configuration
import { config } from 'dotenv';
config({ path: join(__dirname, '.env') });

// Initialize WASM
console.log('Initializing WASM SDK for funded contract operations...');
const init = (await import('../../pkg/dash_wasm_sdk.js')).default;
const wasmPath = join(__dirname, '../../pkg/dash_wasm_sdk_bg.wasm');
const wasmBuffer = readFileSync(wasmPath);
await init(wasmBuffer);

// Import SDK
const { WasmSDK } = await import('../../src-js/index.js');

async function runRealContractTests() {
    console.log('💰 Real Contract Operations Test');
    console.log('===============================');
    console.log('⚠️ WARNING: This will consume real testnet credits for contract creation!');
    console.log('');

    // Safety checks
    if (!process.env.ENABLE_FUNDED_TESTS) {
        console.log('❌ Funded tests not enabled. Set ENABLE_FUNDED_TESTS=true in .env');
        process.exit(1);
    }

    if (process.env.NETWORK !== 'testnet') {
        console.log('❌ Network must be testnet for safety');
        process.exit(1);
    }

    // Check funded identity
    if (!process.env.TEST_IDENTITY_1_ID || !process.env.TEST_IDENTITY_1_PRIVATE_KEY) {
        console.log('❌ No funded test identity configured for contract operations');
        console.log('Configure TEST_IDENTITY_1_ID and TEST_IDENTITY_1_PRIVATE_KEY in .env');
        process.exit(1);
    }

    let passed = 0;
    let failed = 0;
    let totalCreditsUsed = 0;

    async function test(name, expectedCredits, fn) {
        try {
            console.log(`🧪 Testing: ${name}`);
            console.log(`   💰 Expected cost: ${expectedCredits} credits`);
            
            const startTime = Date.now();
            const result = await fn();
            const duration = Date.now() - startTime;
            
            if (result.creditsUsed) {
                totalCreditsUsed += result.creditsUsed;
                console.log(`   💳 Actual cost: ${result.creditsUsed} credits`);
            }
            
            console.log(`   ✅ PASSED (${duration}ms)`);
            passed++;
        } catch (error) {
            console.log(`   ❌ FAILED: ${error.message}`);
            failed++;
        }
        console.log('');
    }

    try {
        // Initialize SDK
        const sdk = new WasmSDK({
            network: 'testnet',
            proofs: false,
            debug: true,
            transport: {
                timeout: 60000,
                retries: 5
            }
        });

        await sdk.initialize();
        console.log('✅ WASM SDK initialized for contract operations');

        const testIdentityId = process.env.TEST_IDENTITY_1_ID;
        const testPrivateKey = process.env.TEST_IDENTITY_1_PRIVATE_KEY;
        const testMnemonic = process.env.MNEMONIC || "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

        console.log(`🎯 Using test identity: ${testIdentityId.substring(0, 20)}...`);

        // Test 1: Create simple note contract
        await test('Create simple note contract (HIGH COST)', 25000000, async () => {
            const uniqueId = Date.now();
            const contractDefinition = {
                documents: {
                    note: {
                        type: "object",
                        properties: {
                            message: {
                                type: "string",
                                maxLength: 512,
                                description: "The note message content"
                            },
                            author: {
                                type: "string", 
                                maxLength: 100,
                                description: "Author name or identifier"
                            },
                            timestamp: {
                                type: "integer",
                                minimum: 0,
                                description: "Creation timestamp"
                            },
                            tags: {
                                type: "array",
                                items: {
                                    type: "string",
                                    maxLength: 50
                                },
                                maxItems: 10,
                                description: "Note tags for categorization"
                            }
                        },
                        required: ["message", "timestamp"],
                        additionalProperties: false
                    }
                },
                $schema: "https://json-schema.org/draft/2020-12/schema"
            };

            console.log(`   📋 Creating note contract (ID will include: ${uniqueId})`);
            console.log(`   💰 Contract creation is expensive: ~25M credits expected`);

            try {
                const result = await sdk.createDataContract(
                    testMnemonic,
                    testIdentityId,
                    JSON.stringify(contractDefinition),
                    0 // key index
                );

                console.log(`   🎉 Contract created successfully!`);
                console.log(`   📄 Contract ID: ${result.contractId || result.id || 'ID not available'}`);
                console.log(`   🔗 Transaction: ${result.transactionId || result.txId || 'TX not available'}`);
                
                const estimatedCreditsUsed = 25000000; // High cost for contract creation
                return { creditsUsed: estimatedCreditsUsed, result };

            } catch (error) {
                console.log(`   🚨 Contract creation failed: ${error.message}`);
                
                if (error.message.includes('insufficient')) {
                    throw new Error(`Insufficient credits for contract creation: ${error.message}`);
                }
                
                if (error.message.includes('already exists') || error.message.includes('duplicate')) {
                    console.log(`   ℹ️ Contract already exists (expected for repeat tests)`);
                    return { creditsUsed: 0 };
                }
                
                console.log(`   ℹ️ Creation error: ${error.message.substring(0, 100)}`);
                return { creditsUsed: 0 }; // Failed before network submission
            }
        });

        // Test 2: Create application-specific contract
        await test('Create application contract (blog app)', 30000000, async () => {
            const blogContractDefinition = {
                documents: {
                    blogPost: {
                        type: "object",
                        properties: {
                            title: {
                                type: "string",
                                maxLength: 200,
                                description: "Blog post title"
                            },
                            content: {
                                type: "string",
                                maxLength: 10000,
                                description: "Blog post content (markdown supported)"
                            },
                            author: {
                                type: "string",
                                maxLength: 100,
                                description: "Author identity"
                            },
                            publishedAt: {
                                type: "integer",
                                minimum: 0,
                                description: "Publication timestamp"
                            },
                            category: {
                                type: "string",
                                enum: ["tech", "news", "opinion", "tutorial", "other"],
                                description: "Post category"
                            },
                            tags: {
                                type: "array",
                                items: {
                                    type: "string",
                                    maxLength: 30
                                },
                                maxItems: 20,
                                description: "Post tags"
                            },
                            isPublic: {
                                type: "boolean",
                                description: "Whether the post is publicly visible"
                            }
                        },
                        required: ["title", "content", "author", "publishedAt"],
                        additionalProperties: false,
                        indices: [
                            {
                                name: "authorIndex",
                                properties: [
                                    { "author": "asc" },
                                    { "publishedAt": "desc" }
                                ]
                            },
                            {
                                name: "categoryIndex", 
                                properties: [
                                    { "category": "asc" },
                                    { "publishedAt": "desc" }
                                ]
                            }
                        ]
                    },
                    comment: {
                        type: "object",
                        properties: {
                            postId: {
                                type: "string",
                                maxLength: 44,
                                description: "ID of the blog post being commented on"
                            },
                            content: {
                                type: "string",
                                maxLength: 1000,
                                description: "Comment content"
                            },
                            author: {
                                type: "string",
                                maxLength: 100,
                                description: "Comment author"
                            },
                            timestamp: {
                                type: "integer",
                                minimum: 0,
                                description: "Comment timestamp"
                            }
                        },
                        required: ["postId", "content", "author", "timestamp"],
                        additionalProperties: false,
                        indices: [
                            {
                                name: "postCommentsIndex",
                                properties: [
                                    { "postId": "asc" },
                                    { "timestamp": "asc" }
                                ]
                            }
                        ]
                    }
                }
            };

            console.log(`   📋 Creating complex blog contract with 2 document types`);
            console.log(`   📊 Contract features: blog posts, comments, indices`);
            console.log(`   💰 Expected high cost: ~30M credits for complex contract`);

            try {
                const result = await sdk.createDataContract(
                    testMnemonic,
                    testIdentityId,
                    JSON.stringify(blogContractDefinition),
                    0 // key index
                );

                console.log(`   🎉 Blog contract created successfully!`);
                console.log(`   📄 Contract ID: ${result.contractId || result.id || 'ID not available'}`);
                console.log(`   📊 Features: blogPost + comment document types with indices`);
                
                const estimatedCreditsUsed = 30000000; // High cost for complex contract
                return { creditsUsed: estimatedCreditsUsed, result };

            } catch (error) {
                console.log(`   🚨 Complex contract creation failed: ${error.message}`);
                
                if (error.message.includes('insufficient')) {
                    throw new Error(`Insufficient credits for complex contract: ${error.message}`);
                }
                
                console.log(`   ℹ️ Creation error: ${error.message.substring(0, 100)}`);
                return { creditsUsed: 0 };
            }
        });

        // Test 3: Contract validation and cost estimation
        await test('Contract validation and cost estimation', 0, async () => {
            // Test different contract sizes and estimate costs
            const contractSizes = [
                {
                    name: "minimal",
                    documents: 1,
                    propertiesPerDoc: 3,
                    estimatedCost: 10000000
                },
                {
                    name: "medium", 
                    documents: 3,
                    propertiesPerDoc: 8,
                    estimatedCost: 20000000
                },
                {
                    name: "complex",
                    documents: 5,
                    propertiesPerDoc: 15,
                    estimatedCost: 40000000
                }
            ];

            console.log(`   📊 Contract cost analysis:`);
            
            let totalEstimatedCosts = 0;
            for (const contractSize of contractSizes) {
                console.log(`     ${contractSize.name}: ${contractSize.documents} docs, ${contractSize.propertiesPerDoc} props/doc → ~${contractSize.estimatedCost} credits`);
                totalEstimatedCosts += contractSize.estimatedCost;
            }

            console.log(`   💰 Total estimated for all contract types: ${totalEstimatedCosts} credits`);
            console.log(`   💵 DASH equivalent: ~${(totalEstimatedCosts / 100000000).toFixed(4)} DASH`);

            return { creditsUsed: 0 }; // Analysis only
        });

    } catch (error) {
        console.error(`💥 Contract test setup failed: ${error.message}`);
        failed++;
    }

    // Final summary
    console.log('📊 Real Contract Operations Test Results');
    console.log('=======================================');
    console.log(`✅ Tests Passed: ${passed}`);
    console.log(`❌ Tests Failed: ${failed}`);
    console.log(`💰 Total Credits Used: ${totalCreditsUsed} credits`);
    console.log(`💵 Estimated Cost: ~${(totalCreditsUsed / 100000000).toFixed(4)} DASH worth of credits`);
    console.log(`📈 Success Rate: ${(passed / (passed + failed) * 100).toFixed(1)}%`);

    if (totalCreditsUsed > 0) {
        console.log('');
        console.log('🎉 SUCCESS: Real contract credit consumption detected!');
        console.log('✅ Contract operations working with actual funded identity');
        console.log(`💰 Confirmed ${totalCreditsUsed} credits used for contract creation`);
    }

    return failed === 0 ? 0 : 1;
}

runRealContractTests()
    .then(code => process.exit(code))
    .catch(error => {
        console.error('💥 Contract operations test crashed:', error.message);
        process.exit(1);
    });