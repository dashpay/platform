#!/usr/bin/env node
// run-all-tests.mjs - Comprehensive test runner for WASM SDK

import { spawn } from 'child_process';
import { readFileSync, writeFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Test files to run
const testFiles = [
    { name: 'SDK Initialization', file: 'sdk-init-simple.test.mjs' },
    { name: 'Key Generation', file: 'key-generation.test.mjs' },
    { name: 'DIP Derivation', file: 'dip-derivation.test.mjs' },
    { name: 'DPNS Functions', file: 'dpns.test.mjs' },
    { name: 'Utility Functions', file: 'utilities-simple.test.mjs' },
    { name: 'Identity Queries', file: 'identity-queries.test.mjs' },
    { name: 'Document Queries', file: 'document-queries.test.mjs' },
    { name: 'Specialized Queries', file: 'specialized-queries.test.mjs' },
    { name: 'Voting & Contested Resources', file: 'voting-contested-resources.test.mjs' },
    { name: 'Token Queries', file: 'token-queries.test.mjs' },
    { name: 'Group Queries', file: 'group-queries.test.mjs' },
    { name: 'Epoch & Block Queries', file: 'epoch-block-queries.test.mjs' },
    { name: 'Protocol & Version Queries', file: 'protocol-version-queries.test.mjs' },
    { name: 'System & Utility Queries', file: 'system-utility-queries.test.mjs' },
    { name: 'State Transitions', file: 'state-transitions.test.mjs' },
    { name: 'Proof Verification', file: 'proof-verification.test.mjs' },
    { name: 'Wrapper Methods Validation', file: 'wrapper-methods-test.mjs' },
    { name: 'Dual Mode Testing', file: 'dual-mode-testing.test.mjs' },
    { name: 'Quick Funded Setup', file: 'quick-funded-setup.test.mjs', requiresEnv: true }
];

// Test results
const results = {
    totalTests: 0,
    totalPassed: 0,
    totalFailed: 0,
    suites: [],
    startTime: Date.now()
};

// ANSI color codes
const colors = {
    reset: '\x1b[0m',
    bright: '\x1b[1m',
    green: '\x1b[32m',
    red: '\x1b[31m',
    yellow: '\x1b[33m',
    blue: '\x1b[34m',
    cyan: '\x1b[36m'
};

console.log(`${colors.bright}${colors.blue}========================================`);
console.log(`       WASM SDK Comprehensive Tests     `);
console.log(`========================================${colors.reset}\n`);

// Function to run a single test file
async function runTest(name, file, requiresEnv = false) {
    return new Promise((resolve) => {
        // Check for required environment variables
        if (requiresEnv && (!process.env.IDENTITY_ID || !process.env.MNEMONIC)) {
            console.log(`${colors.yellow}Skipping ${name} - requires IDENTITY_ID and MNEMONIC environment variables${colors.reset}`);
            resolve({
                name,
                passed: 0,
                failed: 0, 
                total: 0,
                skipped: true,
                duration: 0,
                exitCode: 0
            });
            return;
        }

        console.log(`${colors.cyan}Running ${name} tests...${colors.reset}`);
        
        const startTime = Date.now();
        const testPath = join(__dirname, file);
        
        let output = '';
        let errorOutput = '';
        
        const child = spawn('node', [testPath], {
            env: { ...process.env, NODE_NO_WARNINGS: '1' }
        });
        
        child.stdout.on('data', (data) => {
            output += data.toString();
        });
        
        child.stderr.on('data', (data) => {
            errorOutput += data.toString();
        });
        
        child.on('close', (code) => {
            const duration = Date.now() - startTime;
            
            // Parse test results from output
            const passedMatch = output.match(/(\d+) passed/);
            const failedMatch = output.match(/(\d+) failed/);
            const totalMatch = output.match(/(\d+) total/);
            
            const passed = passedMatch ? parseInt(passedMatch[1]) : 0;
            const failed = failedMatch ? parseInt(failedMatch[1]) : 0;
            const total = totalMatch ? parseInt(totalMatch[1]) : 0;
            
            // Extract individual test results
            const testLines = output.split('\n').filter(line => 
                line.includes('✅') || line.includes('❌')
            );
            
            const suite = {
                name,
                file,
                duration,
                passed,
                failed,
                total,
                exitCode: code,
                tests: testLines.map(line => {
                    const isPassing = line.includes('✅');
                    const testName = line.replace(/^.*[✅❌]\s*/, '').trim();
                    return { name: testName, passed: isPassing };
                }),
                errors: []
            };
            
            // Extract error messages
            const errorLines = output.split('\n').filter((line, index, arr) => {
                return line.includes('❌') && index + 1 < arr.length;
            });
            
            errorLines.forEach((line, index) => {
                const testName = line.replace(/^.*❌\s*/, '').trim();
                const nextLines = output.split('\n').slice(
                    output.split('\n').indexOf(line) + 1
                );
                const errorMessage = nextLines.find(l => l.trim().startsWith(''))?.trim() || '';
                if (errorMessage) {
                    suite.errors.push({ test: testName, error: errorMessage });
                }
            });
            
            // Check for panics or crashes
            if (errorOutput.includes('panicked at') || output.includes('panicked at')) {
                suite.hasPanic = true;
                suite.panicMessage = 'Test suite panicked - see output for details';
            }
            
            results.suites.push(suite);
            results.totalTests += total;
            results.totalPassed += passed;
            results.totalFailed += failed;
            
            // Print summary for this suite
            if (code === 0 && failed === 0) {
                console.log(`${colors.green}✓ ${name}: ${passed}/${total} tests passed${colors.reset} (${duration}ms)\n`);
            } else {
                console.log(`${colors.red}✗ ${name}: ${passed}/${total} tests passed, ${failed} failed${colors.reset} (${duration}ms)`);
                if (suite.hasPanic) {
                    console.log(`  ${colors.yellow}⚠ Suite panicked during execution${colors.reset}`);
                }
                console.log('');
            }
            
            resolve(suite);
        });
    });
}

// Run all tests sequentially
async function runAllTests() {
    for (const { name, file, requiresEnv } of testFiles) {
        try {
            await runTest(name, file, requiresEnv);
        } catch (error) {
            console.error(`${colors.red}Error running ${name}: ${error.message}${colors.reset}`);
            results.suites.push({
                name,
                file,
                error: error.message,
                passed: 0,
                failed: 1,
                total: 1
            });
            results.totalFailed += 1;
            results.totalTests += 1;
        }
    }
    
    results.endTime = Date.now();
    results.totalDuration = results.endTime - results.startTime;
    
    // Print overall summary
    console.log(`${colors.bright}${colors.blue}========================================`);
    console.log(`              Test Summary              `);
    console.log(`========================================${colors.reset}\n`);
    
    console.log(`Total Test Suites: ${results.suites.length}`);
    console.log(`Total Tests: ${results.totalTests}`);
    console.log(`${colors.green}Passed: ${results.totalPassed}${colors.reset}`);
    console.log(`${colors.red}Failed: ${results.totalFailed}${colors.reset}`);
    console.log(`Time: ${(results.totalDuration / 1000).toFixed(2)}s\n`);
    
    // Show failed tests
    if (results.totalFailed > 0) {
        console.log(`${colors.red}${colors.bright}Failed Tests:${colors.reset}`);
        results.suites.forEach(suite => {
            if (suite.failed > 0 || suite.hasPanic) {
                console.log(`\n${colors.yellow}${suite.name}:${colors.reset}`);
                suite.tests.filter(t => !t.passed).forEach(test => {
                    console.log(`  ${colors.red}✗ ${test.name}${colors.reset}`);
                });
                if (suite.hasPanic) {
                    console.log(`  ${colors.yellow}⚠ ${suite.panicMessage}${colors.reset}`);
                }
            }
        });
    }
    
    // Generate detailed report
    generateReport(results);
    
    // Exit with appropriate code
    process.exit(results.totalFailed > 0 ? 1 : 0);
}

// Generate detailed HTML report
function generateReport(results) {
    const reportPath = join(__dirname, 'test-report.html');
    
    const html = `
<!DOCTYPE html>
<html>
<head>
    <title>WASM SDK Test Report</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 20px; background: #f5f5f5; }
        .container { max-width: 1200px; margin: 0 auto; background: white; padding: 20px; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }
        h1 { color: #333; border-bottom: 2px solid #4CAF50; padding-bottom: 10px; }
        h2 { color: #555; margin-top: 30px; }
        .summary { display: flex; gap: 20px; margin: 20px 0; }
        .summary-item { flex: 1; padding: 15px; border-radius: 5px; text-align: center; }
        .summary-item h3 { margin: 0 0 10px 0; }
        .summary-item .number { font-size: 2em; font-weight: bold; }
        .passed { background: #e8f5e9; color: #2e7d32; }
        .failed { background: #ffebee; color: #c62828; }
        .total { background: #e3f2fd; color: #1565c0; }
        .suite { margin: 20px 0; border: 1px solid #ddd; border-radius: 5px; overflow: hidden; }
        .suite-header { background: #f0f0f0; padding: 15px; font-weight: bold; cursor: pointer; }
        .suite-header.passed { border-left: 5px solid #4CAF50; }
        .suite-header.failed { border-left: 5px solid #f44336; }
        .suite-body { padding: 15px; display: none; }
        .suite-body.show { display: block; }
        .test { padding: 5px 0; }
        .test.passed::before { content: "✓ "; color: #4CAF50; }
        .test.failed::before { content: "✗ "; color: #f44336; }
        .error { color: #d32f2f; margin-left: 20px; font-style: italic; }
        .panic { background: #fff3cd; color: #856404; padding: 10px; margin: 10px 0; border-radius: 5px; }
        .timestamp { color: #666; font-size: 0.9em; }
    </style>
    <script>
        function toggleSuite(id) {
            const body = document.getElementById(id);
            body.classList.toggle('show');
        }
    </script>
</head>
<body>
    <div class="container">
        <h1>WASM SDK Test Report</h1>
        <p class="timestamp">Generated: ${new Date().toLocaleString()}</p>
        
        <div class="summary">
            <div class="summary-item total">
                <h3>Total Tests</h3>
                <div class="number">${results.totalTests}</div>
            </div>
            <div class="summary-item passed">
                <h3>Passed</h3>
                <div class="number">${results.totalPassed}</div>
            </div>
            <div class="summary-item failed">
                <h3>Failed</h3>
                <div class="number">${results.totalFailed}</div>
            </div>
        </div>
        
        <h2>Test Suites</h2>
        ${results.suites.map((suite, index) => `
            <div class="suite">
                <div class="suite-header ${suite.failed === 0 && !suite.hasPanic ? 'passed' : 'failed'}" 
                     onclick="toggleSuite('suite-${index}')">
                    ${suite.name} - ${suite.passed}/${suite.total} passed (${suite.duration}ms)
                    ${suite.hasPanic ? '<span style="float: right;">⚠ Panic</span>' : ''}
                </div>
                <div class="suite-body" id="suite-${index}">
                    ${suite.hasPanic ? `<div class="panic">${suite.panicMessage}</div>` : ''}
                    ${suite.tests.map(test => `
                        <div class="test ${test.passed ? 'passed' : 'failed'}">
                            ${test.name}
                        </div>
                    `).join('')}
                </div>
            </div>
        `).join('')}
        
        <h2>Execution Details</h2>
        <p>Total execution time: ${(results.totalDuration / 1000).toFixed(2)} seconds</p>
        <p>Test files executed: ${results.suites.length}</p>
    </div>
</body>
</html>
    `;
    
    writeFileSync(reportPath, html);
    console.log(`\n${colors.cyan}Detailed report saved to: ${reportPath}${colors.reset}`);
}

// Run the tests
runAllTests().catch(error => {
    console.error(`${colors.red}Fatal error: ${error.message}${colors.reset}`);
    process.exit(1);
});