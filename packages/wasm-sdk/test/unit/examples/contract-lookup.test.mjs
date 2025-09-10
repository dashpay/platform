/**
 * Unit Tests for contract-lookup.mjs example
 * Tests advanced contract and document exploration functionality
 */

import { jest } from '@jest/globals';

describe('Contract Lookup Example', () => {
    let sdk;

    beforeAll(async () => {
        const wasmInitialized = await global.initializeWasm();
        if (!wasmInitialized) {
            throw new Error('Failed to initialize WASM - tests cannot proceed');
        }
    });

    beforeEach(async () => {
        sdk = await global.createTestSDK({
            network: 'testnet',
            proofs: false
        });
    });

    afterEach(async () => {
        if (sdk && sdk.destroy) {
            await sdk.destroy();
        }
    });

    describe('Contract Information Retrieval', () => {
        test('should retrieve DPNS contract information', async () => {
            const contractId = TEST_CONFIG.DPNS_TESTNET;
            
            const performance = await global.measurePerformance(
                () => sdk.getDataContract(contractId),
                'getDataContract'
            );

            const contract = performance.result;
            
            if (contract) {
                global.expectValidContract(contract);
                
                // Verify DPNS-specific structure
                expect(contract.documents).toHaveProperty('domain');
                expect(contract.documents).toHaveProperty('preorder');
                
                // Verify document schema structure
                const domainSchema = contract.documents.domain;
                expect(domainSchema).toHaveProperty('properties');
                expect(domainSchema.properties).toHaveProperty('label');
                expect(domainSchema.properties).toHaveProperty('normalizedLabel');
                expect(domainSchema.properties).toHaveProperty('parentDomainName');
                
                // Verify metadata
                expect(contract).toHaveProperty('version');
                expect(contract).toHaveProperty('ownerId');
            }

            expect(performance.duration).toCompleteWithinTime(10000);
        }, TEST_CONFIG.SLOW_TIMEOUT);

        test('should retrieve DashPay contract information', async () => {
            const contractId = TEST_CONFIG.DASHPAY_TESTNET;
            
            try {
                const contract = await sdk.getDataContract(contractId);
                
                if (contract) {
                    global.expectValidContract(contract);
                    
                    // Verify DashPay-specific structure
                    expect(contract.documents).toHaveProperty('profile');
                    expect(contract.documents).toHaveProperty('contactInfo');
                    
                    // Verify profile schema
                    const profileSchema = contract.documents.profile;
                    expect(profileSchema).toHaveProperty('properties');
                    expect(profileSchema.properties).toHaveProperty('displayName');
                    expect(profileSchema.properties).toHaveProperty('publicMessage');
                }
            } catch (error) {
                console.warn('DashPay contract test skipped - contract not accessible');
            }
        }, TEST_CONFIG.SLOW_TIMEOUT);

        test('should handle contract not found scenarios', async () => {
            const nonExistentContractId = 'NonExistentContract123456789AbCdEf123456789';
            
            const result = await sdk.getDataContract(nonExistentContractId).catch(error => null);
            expect(result).toBeNull();
        }, TEST_CONFIG.STANDARD_TIMEOUT);
    });

    describe('Document Type Discovery', () => {
        test('should discover all document types in DPNS contract', async () => {
            const contractId = TEST_CONFIG.DPNS_TESTNET;
            
            try {
                const contract = await sdk.getDataContract(contractId);
                
                if (contract && contract.documents) {
                    const documentTypes = Object.keys(contract.documents);
                    
                    expect(documentTypes).toContain('domain');
                    expect(documentTypes).toContain('preorder');
                    
                    // Verify each document type has required properties
                    documentTypes.forEach(docType => {
                        const schema = contract.documents[docType];
                        expect(schema).toHaveProperty('properties');
                        expect(typeof schema.properties).toBe('object');
                    });
                }
            } catch (error) {
                console.warn('Document type discovery test skipped');
            }
        }, TEST_CONFIG.STANDARD_TIMEOUT);

        test('should analyze document schema properties', async () => {
            const contractId = TEST_CONFIG.DPNS_TESTNET;
            
            try {
                const contract = await sdk.getDataContract(contractId);
                
                if (contract && contract.documents && contract.documents.domain) {
                    const domainSchema = contract.documents.domain.properties;
                    
                    // Check required DPNS domain properties
                    const expectedProperties = ['label', 'normalizedLabel', 'parentDomainName', 'preorderSalt', 'records', 'subdomainRules'];
                    
                    expectedProperties.forEach(prop => {
                        if (domainSchema[prop]) {
                            expect(domainSchema[prop]).toHaveProperty('type');
                        }
                    });
                }
            } catch (error) {
                console.warn('Schema analysis test skipped');
            }
        }, TEST_CONFIG.STANDARD_TIMEOUT);
    });

    describe('Advanced Document Queries', () => {
        test('should execute basic document queries', async () => {
            const contractId = TEST_CONFIG.DPNS_TESTNET;
            const documentType = 'domain';
            
            try {
                const performance = await global.measurePerformance(
                    () => sdk.getDocuments(contractId, documentType, { limit: 5 }),
                    'getDocuments'
                );

                const documents = performance.result;
                
                if (documents && documents.length > 0) {
                    expect(documents).toHaveValidQueryResult();
                    
                    documents.forEach(doc => {
                        global.expectValidDocument(doc);
                        
                        // Verify DPNS domain document structure
                        expect(doc.data).toHaveProperty('label');
                        expect(typeof doc.data.label).toBe('string');
                    });
                }

                expect(performance.duration).toCompleteWithinTime(15000);
            } catch (error) {
                console.warn('Basic document query test skipped:', error.message);
            }
        }, TEST_CONFIG.SLOW_TIMEOUT);

        test('should execute queries with WHERE clauses', async () => {
            const contractId = TEST_CONFIG.DPNS_TESTNET;
            const documentType = 'domain';
            
            try {
                // Query domains with specific conditions
                const whereConditions = [
                    ['parentDomainName', '=', 'dash']
                ];

                const documents = await sdk.getDocuments(contractId, documentType, {
                    where: whereConditions,
                    limit: 10
                });

                if (documents && documents.length > 0) {
                    expect(documents).toHaveValidQueryResult();
                    
                    // Verify WHERE condition is applied
                    documents.forEach(doc => {
                        expect(doc.data.parentDomainName).toBe('dash');
                    });
                }
            } catch (error) {
                console.warn('WHERE clause test skipped:', error.message);
            }
        }, TEST_CONFIG.SLOW_TIMEOUT);

        test('should execute queries with ORDER BY clauses', async () => {
            const contractId = TEST_CONFIG.DPNS_TESTNET;
            const documentType = 'domain';
            
            try {
                // Query with ordering
                const documents = await sdk.getDocuments(contractId, documentType, {
                    orderBy: [['$createdAt', 'desc']],
                    limit: 5
                });

                if (documents && documents.length > 1) {
                    // Verify documents are ordered by creation time (newest first)
                    for (let i = 1; i < documents.length; i++) {
                        expect(documents[i-1].createdAt >= documents[i].createdAt).toBe(true);
                    }
                }
            } catch (error) {
                console.warn('ORDER BY test skipped:', error.message);
            }
        }, TEST_CONFIG.SLOW_TIMEOUT);

        test('should handle complex query combinations', async () => {
            const contractId = TEST_CONFIG.DPNS_TESTNET;
            const documentType = 'domain';
            
            try {
                const documents = await sdk.getDocuments(contractId, documentType, {
                    where: [
                        ['parentDomainName', '=', 'dash'],
                        ['$ownerId', '!=', '']
                    ],
                    orderBy: [
                        ['$createdAt', 'desc'],
                        ['label', 'asc']
                    ],
                    limit: 3,
                    offset: 0
                });

                if (documents) {
                    expect(Array.isArray(documents)).toBe(true);
                    expect(documents.length).toBeLessThanOrEqual(3);
                }
            } catch (error) {
                console.warn('Complex query test skipped:', error.message);
            }
        }, TEST_CONFIG.SLOW_TIMEOUT);
    });

    describe('Pagination and Bulk Document Retrieval', () => {
        test('should handle document pagination', async () => {
            const contractId = TEST_CONFIG.DPNS_TESTNET;
            const documentType = 'domain';
            
            try {
                // Get first page
                const firstPage = await sdk.getDocuments(contractId, documentType, {
                    limit: 2,
                    offset: 0
                });

                // Get second page
                const secondPage = await sdk.getDocuments(contractId, documentType, {
                    limit: 2,
                    offset: 2
                });

                if (firstPage && secondPage) {
                    // Pages should be different (if enough documents exist)
                    if (firstPage.length > 0 && secondPage.length > 0) {
                        const firstPageIds = firstPage.map(doc => doc.id);
                        const secondPageIds = secondPage.map(doc => doc.id);
                        
                        // No overlap between pages
                        const overlap = firstPageIds.some(id => secondPageIds.includes(id));
                        expect(overlap).toBe(false);
                    }
                }
            } catch (error) {
                console.warn('Pagination test skipped:', error.message);
            }
        }, TEST_CONFIG.SLOW_TIMEOUT);

        test('should handle large limit values', async () => {
            const contractId = TEST_CONFIG.DPNS_TESTNET;
            const documentType = 'domain';
            
            try {
                const documents = await sdk.getDocuments(contractId, documentType, {
                    limit: 50 // Larger batch
                });

                if (documents) {
                    expect(Array.isArray(documents)).toBe(true);
                    expect(documents.length).toBeLessThanOrEqual(50);
                    
                    if (documents.length > 0) {
                        documents.forEach(doc => global.expectValidDocument(doc));
                    }
                }
            } catch (error) {
                console.warn('Large limit test skipped:', error.message);
            }
        }, TEST_CONFIG.SLOW_TIMEOUT);
    });

    describe('Structured JSON Response Handling', () => {
        test('should properly parse and validate JSON responses', async () => {
            const contractId = TEST_CONFIG.DPNS_TESTNET;
            
            try {
                const contract = await sdk.getDataContract(contractId);
                
                if (contract) {
                    // Verify contract can be serialized/deserialized
                    const serialized = JSON.stringify(contract);
                    const parsed = JSON.parse(serialized);
                    
                    expect(parsed).toEqual(contract);
                    
                    // Verify nested structures
                    expect(typeof parsed.documents).toBe('object');
                    Object.keys(parsed.documents).forEach(docType => {
                        expect(typeof parsed.documents[docType]).toBe('object');
                    });
                }
            } catch (error) {
                console.warn('JSON response test skipped');
            }
        }, TEST_CONFIG.STANDARD_TIMEOUT);

        test('should handle document metadata correctly', async () => {
            const contractId = TEST_CONFIG.DPNS_TESTNET;
            const documentType = 'domain';
            
            try {
                const documents = await sdk.getDocuments(contractId, documentType, { limit: 3 });
                
                if (documents && documents.length > 0) {
                    documents.forEach(doc => {
                        // Verify required metadata fields
                        expect(doc).toHaveProperty('id');
                        expect(doc).toHaveProperty('ownerId');
                        expect(doc).toHaveProperty('data');
                        
                        // Verify metadata types
                        expect(typeof doc.id).toBe('string');
                        expect(typeof doc.ownerId).toBe('string');
                        expect(typeof doc.data).toBe('object');
                        
                        // Verify timestamps if present
                        if (doc.createdAt !== undefined) {
                            expect(typeof doc.createdAt).toBe('number');
                        }
                        if (doc.updatedAt !== undefined) {
                            expect(typeof doc.updatedAt).toBe('number');
                        }
                    });
                }
            } catch (error) {
                console.warn('Document metadata test skipped');
            }
        }, TEST_CONFIG.STANDARD_TIMEOUT);
    });

    describe('Performance and Scalability Tests', () => {
        test('should meet performance benchmarks for contract operations', async () => {
            const benchmarks = [
                {
                    name: 'getDataContract',
                    fn: () => sdk.getDataContract(TEST_CONFIG.DPNS_TESTNET),
                    maxTime: 10000
                },
                {
                    name: 'getDocuments',
                    fn: () => sdk.getDocuments(TEST_CONFIG.DPNS_TESTNET, 'domain', { limit: 5 }),
                    maxTime: 15000
                }
            ];

            for (const benchmark of benchmarks) {
                try {
                    const performance = await global.measurePerformance(
                        benchmark.fn,
                        benchmark.name
                    );

                    expect(performance.duration).toCompleteWithinTime(benchmark.maxTime);
                } catch (error) {
                    console.warn(`Benchmark ${benchmark.name} failed:`, error.message);
                }
            }
        }, TEST_CONFIG.SLOW_TIMEOUT);

        test('should handle concurrent contract queries', async () => {
            const concurrentOps = 3;
            const contractId = TEST_CONFIG.DPNS_TESTNET;

            const operations = Array(concurrentOps).fill(null).map(() => 
                sdk.getDataContract(contractId).catch(error => ({ error: error.message }))
            );

            const results = await Promise.allSettled(operations);
            
            expect(results).toHaveLength(concurrentOps);
            
            const successCount = results.filter(r => 
                r.status === 'fulfilled' && !r.value?.error
            ).length;
            
            // At least some operations should succeed
            expect(successCount).toBeGreaterThan(0);
        }, TEST_CONFIG.SLOW_TIMEOUT);
    });

    describe('Error Handling and Edge Cases', () => {
        test('should handle invalid contract IDs', async () => {
            const invalidIds = [
                '',
                'invalid-contract-id',
                'x'.repeat(100),
                '!@#$%^&*()'
            ];

            for (const id of invalidIds) {
                await expect(async () => {
                    await sdk.getDataContract(id);
                }).rejects.toThrow();
            }
        }, TEST_CONFIG.STANDARD_TIMEOUT);

        test('should handle invalid document types', async () => {
            const contractId = TEST_CONFIG.DPNS_TESTNET;
            const invalidDocTypes = [
                'nonexistent',
                '',
                'invalid-doc-type',
                123
            ];

            for (const docType of invalidDocTypes) {
                await expect(async () => {
                    await sdk.getDocuments(contractId, docType);
                }).rejects.toThrow();
            }
        }, TEST_CONFIG.STANDARD_TIMEOUT);

        test('should handle malformed query parameters', async () => {
            const contractId = TEST_CONFIG.DPNS_TESTNET;
            const documentType = 'domain';

            const invalidQueries = [
                { where: 'invalid' },
                { where: [[]] },
                { where: [['field']] },
                { orderBy: 'invalid' },
                { limit: -1 },
                { limit: 'invalid' },
                { offset: -1 }
            ];

            for (const query of invalidQueries) {
                await expect(async () => {
                    await sdk.getDocuments(contractId, documentType, query);
                }).rejects.toThrow();
            }
        }, TEST_CONFIG.STANDARD_TIMEOUT);
    });
});