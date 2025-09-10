#!/usr/bin/env node

/**
 * Security Validation Test Suite
 * Tests critical security fixes and input validation
 */

import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
import { webcrypto } from 'crypto';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Set up Node.js environment for WASM
if (!global.crypto) global.crypto = webcrypto;

// Import the components to test
import { DataSanitizer, WasmSDKError, WasmConfigurationError } from '../src-js/error-handler.js';
import { InputValidator } from '../src-js/input-validator.js';
import { ConfigManager } from '../src-js/config-manager.js';

console.log('🔒 Security Validation Test Suite');
console.log('='.repeat(50));

/**
 * Test data sanitization
 */
async function testDataSanitization() {
    console.log('\n🧹 Testing Data Sanitization...');
    
    const tests = [
        {
            name: 'Sensitive field names',
            input: {
                identityId: '4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF',
                mnemonic: 'abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about',
                privateKey: '1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef',
                normalData: 'this should not be redacted'
            },
            expectedRedacted: ['mnemonic', 'privateKey']
        },
        {
            name: 'Sensitive string values',
            input: {
                data: '1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef', // Looks like private key
                words: 'abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about',
                normal: 'regular string data'
            },
            expectedRedacted: ['data', 'words']
        },
        {
            name: 'Nested objects',
            input: {
                config: {
                    network: 'testnet',
                    credentials: {
                        mnemonic: 'secret words here',
                        apiKey: 'secret-api-key'
                    }
                },
                metadata: {
                    version: '1.0.0'
                }
            },
            expectedRedacted: ['config.credentials.mnemonic']
        }
    ];
    
    for (const test of tests) {
        console.log(`  Testing: ${test.name}`);
        const sanitized = DataSanitizer.sanitizeContext(test.input);
        
        // Check that sensitive fields are redacted
        test.expectedRedacted.forEach(path => {
            const pathParts = path.split('.');
            let current = sanitized;
            let found = true;
            
            for (const part of pathParts) {
                if (current && typeof current === 'object' && part in current) {
                    current = current[part];
                } else {
                    found = false;
                    break;
                }
            }
            
            if (found && current !== '[SANITIZED]' && !current.includes('[SANITIZED]')) {
                throw new Error(`Failed to sanitize ${path}: ${current}`);
            }
        });
        
        console.log(`    ✅ ${test.name} passed`);
    }
    
    // Test error message sanitization
    console.log('  Testing error message sanitization...');
    const sensitiveMessage = 'Error with private key: 1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef';
    const sanitizedMessage = DataSanitizer.sanitizeErrorMessage(sensitiveMessage);
    
    if (sensitiveMessage === sanitizedMessage) {
        throw new Error('Error message was not sanitized');
    }
    
    console.log('    ✅ Error message sanitization passed');
    
    // Test WasmSDKError auto-sanitization
    console.log('  Testing WasmSDKError auto-sanitization...');
    const error = new WasmSDKError('Test error', 'TEST_ERROR', {
        mnemonic: 'secret words',
        identityId: '4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF',
        privateKey: 'abcdef123456'
    });
    
    const errorJson = error.toJSON();
    if (errorJson.context.mnemonic !== '[SANITIZED]' || errorJson.context.privateKey !== '[SANITIZED]') {
        throw new Error('WasmSDKError did not auto-sanitize context');
    }
    
    // But identityId should remain (it's not in sensitive fields)
    if (errorJson.context.identityId !== '4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF') {
        throw new Error('WasmSDKError incorrectly sanitized non-sensitive data');
    }
    
    console.log('    ✅ WasmSDKError auto-sanitization passed');
}

/**
 * Test input validation
 */
async function testInputValidation() {
    console.log('\n🛡️ Testing Input Validation...');
    
    // Test identity ID validation
    console.log('  Testing identity ID validation...');
    
    const validIdentityId = '4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF';
    const invalidIdentityIds = [
        '', // Empty
        'invalid-id', // Wrong format
        '4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF', // Too long
        '123', // Too short
        null, // Null
        undefined, // Undefined
        123, // Wrong type
    ];
    
    // Valid ID should pass
    try {
        InputValidator.validateIdentityId(validIdentityId);
        console.log('    ✅ Valid identity ID accepted');
    } catch (error) {
        throw new Error(`Valid identity ID rejected: ${error.message}`);
    }
    
    // Invalid IDs should fail
    for (const invalidId of invalidIdentityIds) {
        try {
            InputValidator.validateIdentityId(invalidId);
            throw new Error(`Invalid identity ID was accepted: ${invalidId}`);
        } catch (error) {
            if (!(error instanceof WasmConfigurationError)) {
                throw new Error(`Wrong error type for invalid identity ID: ${error.constructor.name}`);
            }
            // Expected to throw
        }
    }
    console.log('    ✅ Invalid identity IDs properly rejected');
    
    // Test network validation
    console.log('  Testing network validation...');
    
    InputValidator.validateNetwork('testnet');
    InputValidator.validateNetwork('mainnet');
    
    const invalidNetworks = ['invalid', '', 'TESTNET', 'test-net', null, undefined, 123];
    for (const invalidNetwork of invalidNetworks) {
        try {
            InputValidator.validateNetwork(invalidNetwork);
            throw new Error(`Invalid network was accepted: ${invalidNetwork}`);
        } catch (error) {
            if (!(error instanceof WasmConfigurationError)) {
                throw new Error(`Wrong error type for invalid network: ${error.constructor.name}`);
            }
        }
    }
    console.log('    ✅ Network validation passed');
    
    // Test JSON validation
    console.log('  Testing JSON validation...');
    
    // First test valid JSON
    console.log('    Testing valid JSON input...');
    const validJson = '{"field": "value", "number": 123}';
    let parsed;
    try {
        parsed = InputValidator.validateJsonString(validJson, 'testJson');
        if (parsed.field !== 'value' || parsed.number !== 123) {
            throw new Error('JSON validation failed to parse correctly');
        }
        console.log('    ✅ Valid JSON accepted');
    } catch (error) {
        console.error('    ❌ Valid JSON rejected:', error.message);
        throw error;
    }
    
    const invalidJsonInputs = [
        { json: '{"invalid": json}', desc: 'Malformed JSON' },
        { json: '{"__proto__": "evil"}', desc: 'Prototype pollution attempt' },
        { json: 'a'.repeat(1500), desc: 'Oversized JSON string' },
        { json: 123, desc: 'Wrong data type' }
    ];
    
    let securityBlocksCount = 0;
    for (const {json, desc} of invalidJsonInputs) {
        try {
            InputValidator.validateJsonString(json, 'testJson', { maxSize: 1000 });
            throw new Error(`${desc} was accepted - SECURITY VULNERABILITY!`);
        } catch (error) {
            if (!(error instanceof WasmConfigurationError)) {
                throw new Error(`Wrong error type for ${desc}: ${error.constructor.name}`);
            }
            // Expected to throw - this is correct security behavior
            console.log(`    🛡️ ${desc} properly blocked`);
            securityBlocksCount++;
        }
    }
    
    if (securityBlocksCount !== invalidJsonInputs.length) {
        throw new Error(`Security validation incomplete: ${securityBlocksCount}/${invalidJsonInputs.length} threats blocked`);
    }
    
    console.log('    ✅ JSON validation passed - All security threats blocked');
    
    // Test URL validation
    console.log('  Testing URL validation...');
    
    const validUrls = [
        'https://example.com',
        'https://seed-1.testnet.networks.dash.org:1443/',
        'https://dash.example.com:8080/path?query=value'
    ];
    
    for (const validUrl of validUrls) {
        try {
            InputValidator.validateUrl(validUrl, 'testUrl');
        } catch (error) {
            throw new Error(`Valid URL rejected: ${validUrl} - ${error.message}`);
        }
    }
    
    const invalidUrls = [
        'http://insecure.com', // HTTP not allowed
        'https://', // Invalid URL
        'ftp://example.com', // Wrong protocol
        '', // Empty
        'https://example.com:99999', // Invalid port
        null, // Null
        'a'.repeat(3000), // Too long
    ];
    
    for (const invalidUrl of invalidUrls) {
        try {
            InputValidator.validateUrl(invalidUrl, 'testUrl');
            throw new Error(`Invalid URL was accepted: ${invalidUrl}`);
        } catch (error) {
            if (!(error instanceof WasmConfigurationError)) {
                throw new Error(`Wrong error type for invalid URL: ${error.constructor.name}`);
            }
        }
    }
    console.log('    ✅ URL validation passed');
    
    // Test where clause validation
    console.log('  Testing where clause validation...');
    
    const validWhereClauses = [
        [['field', '=', 'value']],
        [['age', '>', 18], ['name', 'startsWith', 'A']],
        '[[\"field\", \"=\", \"value\"]]' // JSON string
    ];
    
    for (const validWhere of validWhereClauses) {
        try {
            const result = InputValidator.validateWhereClause(validWhere, 'testWhere');
            if (!Array.isArray(result)) {
                throw new Error('Where clause validation should return array');
            }
        } catch (error) {
            throw new Error(`Valid where clause rejected: ${JSON.stringify(validWhere)} - ${error.message}`);
        }
    }
    
    const invalidWhereClauses = [
        [['field']], // Too few elements
        [['field', 'invalid_operator', 'value']], // Invalid operator
        [[123, '=', 'value']], // Field not string
        'invalid json', // Invalid JSON string
        123 // Wrong type
    ];
    
    for (const invalidWhere of invalidWhereClauses) {
        try {
            InputValidator.validateWhereClause(invalidWhere, 'testWhere');
            throw new Error(`Invalid where clause was accepted: ${JSON.stringify(invalidWhere)}`);
        } catch (error) {
            if (!(error instanceof WasmConfigurationError)) {
                throw new Error(`Wrong error type for invalid where clause: ${error.constructor.name}`);
            }
        }
    }
    console.log('    ✅ Where clause validation passed');
}

/**
 * Test configuration security
 */
async function testConfigurationSecurity() {
    console.log('\n⚙️ Testing Configuration Security...');
    
    // Test that hardcoded endpoints are removed
    console.log('  Testing hardcoded endpoint removal...');
    
    const config = new ConfigManager();
    const configData = config.getConfig();
    
    // Should not have hardcoded endpoint arrays anymore
    if (configData.transport.urls || configData.transport.url) {
        throw new Error('Configuration still contains hardcoded endpoints');
    }
    
    // Should delegate endpoint discovery to WASM SDK
    if (config.hasCustomEndpoint()) {
        throw new Error('Should not have custom endpoint without user providing one');
    }
    
    console.log('    ✅ Hardcoded endpoints properly removed');
    
    // Test custom endpoint validation
    console.log('  Testing custom endpoint validation...');
    
    const validCustomEndpoint = 'https://custom.endpoint.com:1443/';
    try {
        const customConfig = new ConfigManager({
            transport: { url: validCustomEndpoint }
        });
        
        if (!customConfig.hasCustomEndpoint() || customConfig.getCustomEndpoint() !== validCustomEndpoint) {
            throw new Error('Custom endpoint not properly set');
        }
    } catch (error) {
        throw new Error(`Valid custom endpoint rejected: ${error.message}`);
    }
    
    const invalidCustomEndpoints = [
        'http://insecure.com', // HTTP
        'invalid-url', // Invalid format
        '', // Empty
    ];
    
    for (const invalidEndpoint of invalidCustomEndpoints) {
        try {
            new ConfigManager({
                transport: { url: invalidEndpoint }
            });
            throw new Error(`Invalid custom endpoint accepted: ${invalidEndpoint}`);
        } catch (error) {
            if (!(error instanceof WasmConfigurationError)) {
                console.error(`    ⚠️ Wrong error type for "${invalidEndpoint}":`, error.constructor.name, '-', error.message);
                throw new Error(`Wrong error type for invalid endpoint: ${error.constructor.name}`);
            }
            console.log(`    🛡️ Invalid endpoint "${invalidEndpoint}" properly blocked`);
        }
    }
    
    console.log('    ✅ Custom endpoint validation passed');
    
    // Test that configuration is simplified
    console.log('  Testing configuration simplification...');
    
    // Should not have complex nested schemas anymore
    const simpleConfig = new ConfigManager({
        network: 'testnet',
        proofs: false,
        debug: true,
        transport: {
            timeout: 45000,
            retries: 5
        }
    });
    
    const simplifiedConfig = simpleConfig.getConfig();
    
    // Check that all expected fields are present and properly set
    if (simplifiedConfig.network !== 'testnet' ||
        simplifiedConfig.proofs !== false ||
        simplifiedConfig.debug !== true ||
        simplifiedConfig.transport.timeout !== 45000 ||
        simplifiedConfig.transport.retries !== 5) {
        throw new Error('Simplified configuration not working correctly');
    }
    
    console.log('    ✅ Configuration simplification passed');
}

/**
 * Main test runner
 */
async function runSecurityTests() {
    try {
        console.log('🚀 Starting security validation tests...\n');
        
        await testDataSanitization();
        await testInputValidation();
        await testConfigurationSecurity();
        
        console.log('\n🎉 All Security Tests Passed! 🎉');
        console.log('✅ Data sanitization: Working correctly');
        console.log('✅ Input validation: Properly rejecting malicious input');
        console.log('✅ Configuration security: Hardcoded endpoints removed');
        console.log('✅ Error handling: Sensitive data automatically sanitized');
        console.log('✅ URL validation: HTTPS enforced, proper format checking');
        console.log('✅ JSON validation: Prototype pollution prevented');
        
        console.log('\n📊 Security Validation Summary:');
        console.log('- Sensitive data exposure: FIXED ✅');
        console.log('- Input validation vulnerabilities: FIXED ✅');
        console.log('- Hardcoded endpoint security risks: FIXED ✅');
        console.log('- Error message data leaks: FIXED ✅');
        console.log('- Configuration security: ENHANCED ✅');
        
        return true;
        
    } catch (error) {
        console.error('\n❌ Security Test Failed:', error.message);
        console.error('\n🔴 SECURITY VULNERABILITY DETECTED!');
        console.error('This indicates a critical security issue that must be fixed before production.');
        
        if (error.stack) {
            console.error('\nStack trace:', error.stack);
        }
        
        process.exit(1);
    }
}

// Run the tests
await runSecurityTests();