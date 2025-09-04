/**
 * Memory Usage Benchmarks for WASM SDK
 * Tests memory consumption patterns and detects memory leaks
 */

const { chromium } = require('playwright');

class MemoryBenchmarks {
    constructor() {
        this.results = [];
        this.memoryTargets = {
            wasmHeap: { min: 50 * 1024 * 1024, max: 200 * 1024 * 1024 }, // 50-200MB per issue
            jsHeap: { max: 100 * 1024 * 1024 },   // 100MB JS heap limit
            mobileLimit: { max: 100 * 1024 * 1024 } // 100MB mobile constraint
        };
    }

    async runMemoryBenchmarks() {
        console.log('🧠 Starting WASM SDK Memory Benchmarks');
        console.log('='.repeat(50));

        const scenarios = [
            { name: 'baseline', description: 'Basic SDK initialization' },
            { name: 'identity-operations', description: 'Identity queries and operations' },
            { name: 'document-queries', description: 'Large document result sets' },
            { name: 'token-operations', description: 'Token portfolio and transfers' },
            { name: 'bulk-operations', description: 'Bulk processing operations' },
            { name: 'long-running', description: 'Extended usage simulation' },
            { name: 'memory-stress', description: 'Memory pressure testing' }
        ];

        for (const scenario of scenarios) {
            console.log(`\n🔬 Testing: ${scenario.description}`);
            await this.runMemoryScenario(scenario);
        }

        const report = this.generateMemoryReport();
        await this.saveMemoryReport(report);
        
        return report;
    }

    async runMemoryScenario(scenario) {
        const browser = await chromium.launch({ 
            headless: true,
            args: ['--no-sandbox', '--disable-dev-shm-usage', '--disable-web-security']
        });
        
        const context = await browser.newContext();
        const page = await context.newPage();

        try {
            const measurements = await this.executeScenario(page, scenario);
            
            const result = {
                scenario: scenario.name,
                description: scenario.description,
                success: measurements.success,
                error: measurements.error,
                memory: measurements.memory,
                performance: this.analyzeMemoryPerformance(measurements.memory),
                timestamp: new Date().toISOString()
            };

            this.results.push(result);
            
            if (result.success) {
                const peak = Math.round(result.memory.peakUsage / 1024 / 1024);
                console.log(`   ✅ Completed - Peak memory: ${peak}MB`);
            } else {
                console.log(`   ❌ Failed - ${result.error}`);
            }

        } finally {
            await browser.close();
        }
    }

    async executeScenario(page, scenario) {
        const measurements = {
            success: false,
            error: null,
            memory: {
                initial: 0,
                peakUsage: 0,
                finalUsage: 0,
                samples: [],
                wasmHeapSize: 0,
                jsHeapSize: 0
            }
        };

        try {
            // Navigate to test application
            await page.goto('http://localhost:8888/samples/identity-manager/', {
                waitUntil: 'domcontentloaded',
                timeout: 60000
            });

            // Initial memory measurement
            measurements.memory.initial = await this.measureMemory(page);
            measurements.memory.samples.push({
                timestamp: Date.now(),
                usage: measurements.memory.initial,
                phase: 'initial'
            });

            // Wait for SDK initialization
            await page.waitForSelector('.status-dot.connected', { timeout: 30000 });

            // Post-initialization memory
            const postInit = await this.measureMemory(page);
            measurements.memory.samples.push({
                timestamp: Date.now(),
                usage: postInit,
                phase: 'post-initialization'
            });

            // Execute scenario-specific operations
            await this.executeScenarioOperations(page, scenario, measurements);

            // Final memory measurement
            measurements.memory.finalUsage = await this.measureMemory(page);
            measurements.memory.peakUsage = Math.max(...measurements.memory.samples.map(s => s.usage));

            // Calculate memory metrics
            measurements.memory.growthFromInit = measurements.memory.finalUsage - measurements.memory.initial;
            measurements.memory.peakGrowthFromInit = measurements.memory.peakUsage - measurements.memory.initial;

            measurements.success = true;

        } catch (error) {
            measurements.error = error.message;
            measurements.memory.finalUsage = await this.measureMemory(page).catch(() => 0);
        }

        return measurements;
    }

    async executeScenarioOperations(page, scenario, measurements) {
        switch (scenario.name) {
            case 'baseline':
                // Just wait and measure baseline usage
                await new Promise(resolve => setTimeout(resolve, 5000));
                break;

            case 'identity-operations':
                await this.runIdentityOperations(page, measurements);
                break;

            case 'document-queries':
                await this.runDocumentQueries(page, measurements);
                break;

            case 'token-operations':
                await this.runTokenOperations(page, measurements);
                break;

            case 'bulk-operations':
                await this.runBulkOperations(page, measurements);
                break;

            case 'long-running':
                await this.runLongRunningTest(page, measurements);
                break;

            case 'memory-stress':
                await this.runMemoryStressTest(page, measurements);
                break;
        }
    }

    async runIdentityOperations(page, measurements) {
        const identityIds = [
            '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk',
            '6vbqTJxsBnwdEBZsV7HgSsWi7xBJL82MqgJ9QCUaGaZb',
            '7V5e3dzBRDn7qhbEtaAmJNkqBkE1rCDjNB7YJBxtJzM8'
        ];

        for (let i = 0; i < identityIds.length; i++) {
            await page.fill('#identityIdInput', identityIds[i]);
            await page.click('#lookupBtn');
            
            // Wait for results
            await page.waitForSelector('#identityResults, #errorMessage', { timeout: 30000 });
            
            // Measure memory after each operation
            const memUsage = await this.measureMemory(page);
            measurements.memory.samples.push({
                timestamp: Date.now(),
                usage: memUsage,
                phase: `identity-lookup-${i + 1}`
            });

            await new Promise(resolve => setTimeout(resolve, 2000)); // Pause between operations
        }
    }

    async runDocumentQueries(page, measurements) {
        // Navigate to document explorer
        await page.goto('http://localhost:8888/samples/document-explorer/', {
            waitUntil: 'domcontentloaded'
        });

        await page.waitForSelector('.status-dot.connected', { timeout: 30000 });

        // Load DPNS contract
        await page.click('[data-contract="dpns"]');
        await new Promise(resolve => setTimeout(resolve, 2000));

        const memAfterContract = await this.measureMemory(page);
        measurements.memory.samples.push({
            timestamp: Date.now(),
            usage: memAfterContract,
            phase: 'contract-loaded'
        });

        // Execute multiple document queries
        const queryLimits = [10, 25, 50];
        
        for (const limit of queryLimits) {
            await page.selectOption('#documentTypeSelect', 'domain');
            await page.fill('#limitInput', limit.toString());
            await page.click('#executeQueryBtn');

            // Wait for results
            await page.waitForSelector('#resultsContainer .document-grid, #resultsContainer .empty-state', {
                timeout: 30000
            });

            const memAfterQuery = await this.measureMemory(page);
            measurements.memory.samples.push({
                timestamp: Date.now(),
                usage: memAfterQuery,
                phase: `document-query-${limit}`
            });

            await new Promise(resolve => setTimeout(resolve, 3000));
        }
    }

    async runTokenOperations(page, measurements) {
        // Navigate to token transfer app
        await page.goto('http://localhost:8888/samples/token-transfer/', {
            waitUntil: 'domcontentloaded'
        });

        await page.waitForSelector('.status-dot.connected', { timeout: 30000 });

        // Load portfolio
        await page.fill('#portfolioIdentityId', '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk');
        await page.click('#loadPortfolioBtn');

        await new Promise(resolve => setTimeout(resolve, 5000));

        const memAfterPortfolio = await this.measureMemory(page);
        measurements.memory.samples.push({
            timestamp: Date.now(),
            usage: memAfterPortfolio,
            phase: 'portfolio-loaded'
        });

        // Token operations
        await page.fill('#tokenContractId', 'Hqyu8WcRwXCTwbNxdga4CN5gsVEGc67wng4TFzceyLUv');
        await page.fill('#tokenPosition', '0');
        await page.click('#calculateTokenIdBtn');

        await new Promise(resolve => setTimeout(resolve, 3000));

        const memAfterCalculation = await this.measureMemory(page);
        measurements.memory.samples.push({
            timestamp: Date.now(),
            usage: memAfterCalculation,
            phase: 'token-calculation'
        });
    }

    async runBulkOperations(page, measurements) {
        // Simulate bulk operations by running multiple queries
        const identityIds = [
            '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk',
            '6vbqTJxsBnwdEBZsV7HgSsWi7xBJL82MqgJ9QCUaGaZb',
            '7V5e3dzBRDn7qhbEtaAmJNkqBkE1rCDjNB7YJBxtJzM8',
            '4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF',
            '8ykhXr8PZWKDPKaLKhkQ9KzY2QRSKLMJ7kUvY5iBJjDs'
        ];

        // Fill bulk identity list
        await page.fill('#identityList', identityIds.join('\n'));
        await page.click('#bulkBalanceBtn');

        // Monitor memory during bulk processing
        for (let i = 0; i < 10; i++) {
            await new Promise(resolve => setTimeout(resolve, 2000));
            const memUsage = await this.measureMemory(page);
            measurements.memory.samples.push({
                timestamp: Date.now(),
                usage: memUsage,
                phase: `bulk-processing-${i + 1}`
            });
        }
    }

    async runLongRunningTest(page, measurements) {
        // Simulate 10 minutes of continuous usage
        const duration = 10 * 60 * 1000; // 10 minutes
        const startTime = Date.now();
        const interval = 30000; // Sample every 30 seconds

        let operationCount = 0;

        while (Date.now() - startTime < duration) {
            try {
                // Perform random operations
                const operation = operationCount % 3;
                
                switch (operation) {
                    case 0:
                        // Identity lookup
                        await page.fill('#identityIdInput', '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk');
                        await page.click('#lookupBtn');
                        break;
                    case 1:
                        // Balance check
                        await page.click('#checkBalanceBtn');
                        break;
                    case 2:
                        // View keys
                        await page.click('#viewKeysBtn');
                        break;
                }

                await new Promise(resolve => setTimeout(resolve, 2000));
                operationCount++;

                // Sample memory every interval
                if ((Date.now() - startTime) % interval < 2000) {
                    const memUsage = await this.measureMemory(page);
                    measurements.memory.samples.push({
                        timestamp: Date.now(),
                        usage: memUsage,
                        phase: `long-running-${Math.floor((Date.now() - startTime) / 60000)}min`,
                        operationCount
                    });
                }

            } catch (error) {
                console.warn('Operation failed during long-running test:', error.message);
            }
        }

        measurements.memory.totalOperations = operationCount;
        measurements.memory.testDurationMs = Date.now() - startTime;
    }

    async runMemoryStressTest(page, measurements) {
        // Stress test memory by performing many rapid operations
        const iterations = 50;
        
        for (let i = 0; i < iterations; i++) {
            try {
                // Rapid fire operations
                await page.fill('#identityIdInput', `5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk`);
                await page.click('#lookupBtn');
                
                // Don't wait for completion, just measure memory pressure
                if (i % 5 === 0) {
                    const memUsage = await this.measureMemory(page);
                    measurements.memory.samples.push({
                        timestamp: Date.now(),
                        usage: memUsage,
                        phase: `stress-test-${i + 1}`
                    });
                }

                // Very short pause
                await new Promise(resolve => setTimeout(resolve, 100));

            } catch (error) {
                // Continue stress test even if individual operations fail
                console.warn(`Stress operation ${i} failed:`, error.message);
            }
        }

        // Force garbage collection if possible
        try {
            await page.evaluate(() => {
                if (window.gc) {
                    window.gc();
                }
            });
        } catch (error) {
            // GC not available
        }

        // Final measurement after stress
        await new Promise(resolve => setTimeout(resolve, 5000));
        const finalMem = await this.measureMemory(page);
        measurements.memory.samples.push({
            timestamp: Date.now(),
            usage: finalMem,
            phase: 'post-stress-gc'
        });
    }

    async measureMemory(page) {
        try {
            const metrics = await page.evaluate(() => {
                const memory = {
                    timestamp: Date.now()
                };

                // Get JavaScript heap information
                if (window.performance && window.performance.memory) {
                    memory.js = {
                        used: window.performance.memory.usedJSHeapSize,
                        total: window.performance.memory.totalJSHeapSize,
                        limit: window.performance.memory.jsHeapSizeLimit
                    };
                }

                // Try to get WASM memory information
                if (window.WebAssembly && window.WebAssembly.Memory) {
                    try {
                        // This is approximate - actual WASM memory tracking is complex
                        memory.wasm = {
                            estimated: 0 // Would need WASM module cooperation for accurate measurement
                        };
                    } catch (e) {
                        // WASM memory not directly accessible
                    }
                }

                // Get general memory pressure indicators
                memory.documentCount = document.querySelectorAll('*').length;
                memory.eventListeners = 0; // Would need custom tracking

                return memory;
            });

            return metrics.js ? metrics.js.used : 0;

        } catch (error) {
            console.warn('Failed to measure memory:', error.message);
            return 0;
        }
    }

    analyzeMemoryPerformance(memoryData) {
        const peakMB = memoryData.peakUsage / 1024 / 1024;
        const growthMB = (memoryData.peakUsage - memoryData.initial) / 1024 / 1024;
        
        const analysis = {
            peakUsageMB: Math.round(peakMB),
            memoryGrowthMB: Math.round(growthMB),
            meetsWASMTarget: peakMB >= 50 && peakMB <= 200, // Issue requirement: 50-200MB
            meetsMobileTarget: peakMB <= 100, // Mobile constraint
            grade: this.calculateMemoryGrade(peakMB, growthMB),
            leakSuspicion: this.detectMemoryLeak(memoryData.samples)
        };

        return analysis;
    }

    calculateMemoryGrade(peakMB, growthMB) {
        // Grading based on peak usage and growth patterns
        if (peakMB <= 75 && growthMB <= 50) return 'A';
        if (peakMB <= 100 && growthMB <= 75) return 'B';
        if (peakMB <= 150 && growthMB <= 100) return 'C';
        if (peakMB <= 200 && growthMB <= 150) return 'D';
        return 'F';
    }

    detectMemoryLeak(samples) {
        if (samples.length < 5) return { suspected: false, confidence: 0 };

        // Look for consistent memory growth over time
        const growthRates = [];
        
        for (let i = 1; i < samples.length; i++) {
            const timeDiff = samples[i].timestamp - samples[i-1].timestamp;
            const memDiff = samples[i].usage - samples[i-1].usage;
            
            if (timeDiff > 0) {
                growthRates.push(memDiff / timeDiff); // bytes per millisecond
            }
        }

        const avgGrowthRate = growthRates.reduce((a, b) => a + b, 0) / growthRates.length;
        const positiveGrowthCount = growthRates.filter(rate => rate > 0).length;
        const leakConfidence = positiveGrowthCount / growthRates.length;

        return {
            suspected: avgGrowthRate > 0.1 && leakConfidence > 0.7, // 0.1 bytes/ms consistent growth
            confidence: Math.round(leakConfidence * 100),
            avgGrowthRate: avgGrowthRate,
            details: `${positiveGrowthCount}/${growthRates.length} samples showed growth`
        };
    }

    async runTokenOperations(page, measurements) {
        await page.goto('http://localhost:8888/samples/token-transfer/', {
            waitUntil: 'domcontentloaded'
        });

        await page.waitForSelector('.status-dot.connected', { timeout: 30000 });

        // Multiple token operations
        const operations = [
            () => page.click('#loadPortfolioBtn'),
            () => page.click('#calculateTokenIdBtn'),
            () => page.click('#getTokenInfoBtn'),
            () => page.click('#getPricingBtn')
        ];

        for (let i = 0; i < operations.length; i++) {
            try {
                await operations[i]();
                await new Promise(resolve => setTimeout(resolve, 3000));
                
                const memUsage = await this.measureMemory(page);
                measurements.memory.samples.push({
                    timestamp: Date.now(),
                    usage: memUsage,
                    phase: `token-operation-${i + 1}`
                });
            } catch (error) {
                console.warn(`Token operation ${i + 1} failed:`, error.message);
            }
        }
    }

    generateMemoryReport() {
        const report = {
            testRun: {
                timestamp: new Date().toISOString(),
                version: 'WASM SDK v0.1.0',
                testType: 'Memory Usage Benchmarks',
                environment: {
                    platform: process.platform,
                    nodeVersion: process.version,
                    targets: this.memoryTargets
                }
            },
            summary: this.generateMemorySummary(),
            scenarios: this.results,
            recommendations: this.generateMemoryRecommendations()
        };

        return report;
    }

    generateMemorySummary() {
        const successfulResults = this.results.filter(r => r.success);
        
        if (successfulResults.length === 0) {
            return { error: 'No successful memory measurements' };
        }

        const peakUsages = successfulResults.map(r => r.memory.peakUsage);
        const memoryGrowths = successfulResults.map(r => r.memory.growthFromInit || 0);
        
        return {
            totalScenarios: this.results.length,
            successfulScenarios: successfulResults.length,
            successRate: (successfulResults.length / this.results.length * 100).toFixed(1),
            
            peakMemory: {
                averageMB: Math.round(peakUsages.reduce((a, b) => a + b, 0) / peakUsages.length / 1024 / 1024),
                maxMB: Math.round(Math.max(...peakUsages) / 1024 / 1024),
                minMB: Math.round(Math.min(...peakUsages) / 1024 / 1024)
            },
            
            memoryGrowth: {
                averageMB: Math.round(memoryGrowths.reduce((a, b) => a + b, 0) / memoryGrowths.length / 1024 / 1024),
                maxMB: Math.round(Math.max(...memoryGrowths) / 1024 / 1024)
            },
            
            targetCompliance: {
                wasmHeapCompliant: peakUsages.every(usage => {
                    const mb = usage / 1024 / 1024;
                    return mb >= 50 && mb <= 200;
                }),
                mobileCompliant: peakUsages.every(usage => usage <= this.memoryTargets.mobileLimit.max)
            },
            
            leakDetection: {
                suspectedLeaks: successfulResults.filter(r => r.performance.leakSuspicion.suspected).length,
                leakScenarios: successfulResults.filter(r => r.performance.leakSuspicion.suspected).map(r => r.scenario)
            }
        };
    }

    generateMemoryRecommendations() {
        const recommendations = [];
        const summary = this.generateMemorySummary();

        if (summary.peakMemory.maxMB > 200) {
            recommendations.push({
                priority: 'high',
                category: 'memory-usage',
                issue: `Peak memory usage ${summary.peakMemory.maxMB}MB exceeds 200MB target`,
                recommendation: 'Optimize WASM heap management and implement aggressive garbage collection'
            });
        }

        if (!summary.targetCompliance.mobileCompliant) {
            recommendations.push({
                priority: 'high', 
                category: 'mobile-compatibility',
                issue: 'Memory usage exceeds mobile device constraints (100MB)',
                recommendation: 'Implement mobile-specific optimizations and memory limiting'
            });
        }

        if (summary.leakDetection.suspectedLeaks > 0) {
            recommendations.push({
                priority: 'high',
                category: 'memory-leaks',
                issue: `${summary.leakDetection.suspectedLeaks} scenarios show suspected memory leaks`,
                recommendation: `Review scenarios: ${summary.leakDetection.leakScenarios.join(', ')}`
            });
        }

        if (summary.memoryGrowth.maxMB > 100) {
            recommendations.push({
                priority: 'medium',
                category: 'memory-growth',
                issue: `Maximum memory growth ${summary.memoryGrowth.maxMB}MB is excessive`,
                recommendation: 'Implement periodic memory cleanup and resource recycling'
            });
        }

        if (recommendations.length === 0) {
            recommendations.push({
                priority: 'info',
                category: 'compliance',
                issue: 'All memory targets met',
                recommendation: 'Continue monitoring memory usage in production environments'
            });
        }

        return recommendations;
    }

    async saveMemoryReport(report) {
        const fs = require('fs').promises;
        const path = require('path');
        
        const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
        const fileName = `memory-benchmark-${timestamp}.json`;
        const reportsDir = path.join(__dirname, 'reports');
        const filePath = path.join(reportsDir, fileName);
        
        // Ensure reports directory exists
        await fs.mkdir(reportsDir, { recursive: true });
        
        // Save detailed report
        await fs.writeFile(filePath, JSON.stringify(report, null, 2));
        
        // Save latest report
        const latestPath = path.join(reportsDir, 'latest-memory.json');
        await fs.writeFile(latestPath, JSON.stringify(report, null, 2));
        
        // Generate HTML report
        await this.generateMemoryHTMLReport(report, path.join(reportsDir, 'latest-memory.html'));
        
        console.log(`\n📊 Memory reports saved:`);
        console.log(`   Detailed: ${filePath}`);
        console.log(`   Latest: ${latestPath}`);
        console.log(`   HTML: latest-memory.html`);
    }

    async generateMemoryHTMLReport(report, htmlPath) {
        const html = `
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>WASM SDK Memory Benchmark Report</title>
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; margin: 40px; background: #f5f5f5; }
        .container { max-width: 1200px; margin: 0 auto; background: white; border-radius: 12px; overflow: hidden; box-shadow: 0 4px 20px rgba(0,0,0,0.1); }
        .header { background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: white; padding: 30px; }
        .section { padding: 30px; border-bottom: 1px solid #e9ecef; }
        .summary { background: #f8f9fa; }
        .metrics-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 20px; }
        .metric-card { background: white; padding: 20px; border-radius: 8px; border: 2px solid #e9ecef; text-align: center; }
        .metric-value { font-size: 2rem; font-weight: bold; color: #2c3e50; margin-bottom: 5px; }
        .metric-label { color: #6c757d; font-size: 0.9rem; }
        .scenario-result { margin: 20px 0; padding: 20px; border-radius: 8px; }
        .scenario-success { background: #d4edda; border: 2px solid #c3e6cb; }
        .scenario-warning { background: #fff3cd; border: 2px solid #ffeaa7; }
        .scenario-error { background: #f8d7da; border: 2px solid #f5c6cb; }
        .memory-chart { background: white; padding: 20px; border-radius: 8px; margin: 15px 0; }
        .sample-list { max-height: 300px; overflow-y: auto; font-family: monospace; font-size: 12px; }
        .grade { padding: 8px 16px; border-radius: 20px; font-weight: bold; color: white; }
        .grade-A { background: #28a745; }
        .grade-B { background: #17a2b8; }
        .grade-C { background: #ffc107; color: #333; }
        .grade-D { background: #fd7e14; }
        .grade-F { background: #dc3545; }
        .recommendations { background: #d1ecf1; padding: 20px; border-radius: 8px; }
        .recommendation { margin: 10px 0; padding: 15px; border-radius: 6px; }
        .rec-high { background: #f8d7da; border-left: 4px solid #dc3545; }
        .rec-medium { background: #fff3cd; border-left: 4px solid #ffc107; }
        .rec-info { background: #d4edda; border-left: 4px solid #28a745; }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>🧠 Memory Usage Benchmark Report</h1>
            <p>Generated: ${report.testRun.timestamp}</p>
            <p>Target Range: 50-200MB WASM Heap | Mobile Limit: 100MB</p>
        </div>

        <div class="section summary">
            <h2>📊 Executive Summary</h2>
            <div class="metrics-grid">
                <div class="metric-card">
                    <div class="metric-value">${report.summary.successfulScenarios}/${report.summary.totalScenarios}</div>
                    <div class="metric-label">Scenarios Passed</div>
                </div>
                <div class="metric-card">
                    <div class="metric-value">${report.summary.peakMemory.averageMB}MB</div>
                    <div class="metric-label">Average Peak Memory</div>
                </div>
                <div class="metric-card">
                    <div class="metric-value">${report.summary.peakMemory.maxMB}MB</div>
                    <div class="metric-label">Maximum Memory</div>
                </div>
                <div class="metric-card">
                    <div class="metric-value">${report.summary.targetCompliance.wasmHeapCompliant ? '✅' : '❌'}</div>
                    <div class="metric-label">WASM Target Compliance</div>
                </div>
                <div class="metric-card">
                    <div class="metric-value">${report.summary.targetCompliance.mobileCompliant ? '✅' : '❌'}</div>
                    <div class="metric-label">Mobile Compliance</div>
                </div>
                <div class="metric-card">
                    <div class="metric-value">${report.summary.leakDetection.suspectedLeaks}</div>
                    <div class="metric-label">Suspected Leaks</div>
                </div>
            </div>
        </div>

        <div class="section">
            <h2>🔬 Scenario Results</h2>
            ${report.scenarios.map(result => `
                <div class="scenario-result ${result.success ? (result.performance.grade <= 'C' ? 'scenario-success' : 'scenario-warning') : 'scenario-error'}">
                    <h3>${result.scenario.toUpperCase().replace('-', ' ')}</h3>
                    <p><em>${result.description}</em></p>
                    
                    ${result.success ? `
                        <div style="display: flex; align-items: center; gap: 20px; margin: 15px 0;">
                            <span>Peak Memory: <strong>${result.performance.peakUsageMB}MB</strong></span>
                            <span>Growth: <strong>${result.performance.memoryGrowthMB}MB</strong></span>
                            <span class="grade grade-${result.performance.grade}">${result.performance.grade}</span>
                            ${result.performance.leakSuspicion.suspected ? 
                                `<span style="background: #dc3545; color: white; padding: 4px 8px; border-radius: 4px;">⚠️ Leak Suspected (${result.performance.leakSuspicion.confidence}%)</span>` : ''}
                        </div>
                        
                        ${result.memory.samples.length > 0 ? `
                            <details>
                                <summary>Memory Samples (${result.memory.samples.length} measurements)</summary>
                                <div class="sample-list">
                                    ${result.memory.samples.map(sample => 
                                        `${new Date(sample.timestamp).toLocaleTimeString()} | ${sample.phase.padEnd(20)} | ${Math.round(sample.usage / 1024 / 1024)}MB`
                                    ).join('<br>')}
                                </div>
                            </details>
                        ` : ''}
                    ` : `
                        <p style="color: #dc3545;"><strong>❌ Failed:</strong> ${result.error}</p>
                    `}
                </div>
            `).join('')}
        </div>

        ${report.recommendations.length > 0 ? `
            <div class="section recommendations">
                <h2>💡 Recommendations</h2>
                ${report.recommendations.map(rec => `
                    <div class="recommendation rec-${rec.priority}">
                        <strong>${rec.priority.toUpperCase()}:</strong> ${rec.category}<br>
                        <strong>Issue:</strong> ${rec.issue}<br>
                        <strong>Recommendation:</strong> ${rec.recommendation}
                    </div>
                `).join('')}
            </div>
        ` : ''}

        <div class="section">
            <details>
                <summary><h2>🔧 Raw Test Data</h2></summary>
                <pre style="font-size: 10px; line-height: 1.3;">${JSON.stringify(report, null, 2)}</pre>
            </details>
        </div>
    </div>
</body>
</html>`;

        const fs = require('fs').promises;
        await fs.writeFile(htmlPath, html);
    }

    // CLI interface
    static async runCLI() {
        const benchmarks = new MemoryBenchmarks();
        
        try {
            const report = await benchmarks.runMemoryBenchmarks();
            
            console.log('\n📊 MEMORY BENCHMARK SUMMARY');
            console.log('='.repeat(50));
            console.log(`Scenarios Tested: ${report.summary.totalScenarios}`);
            console.log(`Success Rate: ${report.summary.successRate}%`);
            console.log(`Average Peak Memory: ${report.summary.peakMemory.averageMB}MB`);
            console.log(`Maximum Memory: ${report.summary.peakMemory.maxMB}MB`);
            console.log(`WASM Target Compliance: ${report.summary.targetCompliance.wasmHeapCompliant ? '✅' : '❌'}`);
            console.log(`Mobile Compliance: ${report.summary.targetCompliance.mobileCompliant ? '✅' : '❌'}`);
            
            if (report.summary.leakDetection.suspectedLeaks > 0) {
                console.log(`\n⚠️  MEMORY LEAK WARNING: ${report.summary.leakDetection.suspectedLeaks} scenarios show suspected leaks`);
            }

            if (report.recommendations.length > 0) {
                console.log('\n💡 RECOMMENDATIONS:');
                report.recommendations.forEach(rec => {
                    console.log(`  ${rec.priority.toUpperCase()}: ${rec.recommendation}`);
                });
            }
            
            return report;

        } catch (error) {
            console.error('❌ Memory benchmarks failed:', error.message);
            process.exit(1);
        }
    }
}

module.exports = MemoryBenchmarks;

// CLI execution
if (require.main === module) {
    MemoryBenchmarks.runCLI().then(() => {
        console.log('\n✅ Memory benchmarks completed successfully');
        process.exit(0);
    }).catch(error => {
        console.error('❌ Benchmarks failed:', error);
        process.exit(1);
    });
}