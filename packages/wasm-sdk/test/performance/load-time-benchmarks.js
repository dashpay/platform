/**
 * Load Time Benchmarks for WASM SDK
 * Tests load performance across different network conditions and devices
 */

const { chromium } = require('playwright');

class LoadTimeBenchmarks {
    constructor() {
        this.results = [];
        this.networkConditions = {
            local: { offline: false, downloadThroughput: -1, uploadThroughput: -1, latency: 0 },
            wifi: { offline: false, downloadThroughput: 30 * 1000 * 1000, uploadThroughput: 15 * 1000 * 1000, latency: 20 },
            '4g': { offline: false, downloadThroughput: 4 * 1000 * 1000, uploadThroughput: 3 * 1000 * 1000, latency: 150 },
            '3g': { offline: false, downloadThroughput: 1.6 * 1000 * 1000, uploadThroughput: 750 * 1000, latency: 300 },
            '2g': { offline: false, downloadThroughput: 280 * 1000, uploadThroughput: 256 * 1000, latency: 800 }
        };
        
        this.performanceTargets = {
            local: { target: 1000, max: 3000 },      // Sub-second to 3s
            wifi: { target: 3000, max: 8000 },       // 3-8 seconds  
            '4g': { target: 10000, max: 30000 },     // 10-30 seconds (per issue requirements)
            '3g': { target: 60000, max: 300000 },    // 1-5 minutes (per issue requirements)
            '2g': { target: 120000, max: 600000 }    // 2-10 minutes
        };
    }

    async runAllBenchmarks() {
        console.log('🚀 Starting WASM SDK Load Time Benchmarks');
        console.log('='.repeat(50));

        for (const [networkName, networkConfig] of Object.entries(this.networkConditions)) {
            await this.benchmarkNetwork(networkName, networkConfig);
        }

        const report = this.generateReport();
        await this.saveReport(report);
        
        return report;
    }

    async benchmarkNetwork(networkName, networkConfig) {
        console.log(`\n📡 Testing ${networkName.toUpperCase()} network conditions`);
        console.log(`   Download: ${this.formatThroughput(networkConfig.downloadThroughput)}`);
        console.log(`   Upload: ${this.formatThroughput(networkConfig.uploadThroughput)}`);
        console.log(`   Latency: ${networkConfig.latency}ms`);

        const browser = await chromium.launch({ headless: true });
        const context = await browser.newContext();
        
        try {
            // Emulate network conditions
            await context.emulateNetwork(networkConfig);

            // Run multiple iterations for statistical significance
            const iterations = networkName === 'local' ? 5 : 3;
            const networkResults = [];

            for (let i = 0; i < iterations; i++) {
                console.log(`   Iteration ${i + 1}/${iterations}...`);
                
                const page = await context.newPage();
                const result = await this.measureLoadTime(page, networkName, i + 1);
                networkResults.push(result);
                await page.close();

                // Brief pause between iterations
                await new Promise(resolve => setTimeout(resolve, 1000));
            }

            // Calculate statistics
            const stats = this.calculateNetworkStats(networkName, networkResults);
            this.results.push(stats);

            console.log(`   ✅ ${networkName} completed - Avg: ${stats.averageLoadTime}ms`);

        } finally {
            await browser.close();
        }
    }

    async measureLoadTime(page, networkName, iteration) {
        const startTime = Date.now();
        let wasmInitTime = null;
        let sdkReadyTime = null;
        let firstQueryTime = null;

        // Capture performance events
        const performanceEvents = [];

        page.on('console', msg => {
            if (msg.text().includes('[Performance]')) {
                performanceEvents.push({
                    timestamp: Date.now(),
                    message: msg.text()
                });
            }
        });

        try {
            // Navigate to a sample application
            await page.goto('http://localhost:8888/samples/identity-manager/', {
                waitUntil: 'domcontentloaded',
                timeout: 600000 // 10 minutes max for slow networks
            });

            // Wait for WASM initialization
            await page.waitForFunction(() => {
                return window.performance && window.performance.mark;
            }, { timeout: 60000 });

            // Mark WASM init complete
            wasmInitTime = Date.now();

            // Wait for SDK ready status
            await page.waitForSelector('.status-dot.connected', { 
                timeout: 120000 // 2 minutes for SDK initialization
            });
            
            sdkReadyTime = Date.now();

            // Perform a test query to measure full functionality
            await page.fill('#identityIdInput', '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk');
            await page.click('#lookupBtn');

            // Wait for query results or error
            await page.waitForSelector('#identityResults, #errorMessage', {
                timeout: 60000
            });

            firstQueryTime = Date.now();

            const result = {
                networkCondition: networkName,
                iteration,
                timestamps: {
                    navigationStart: startTime,
                    wasmInitComplete: wasmInitTime,
                    sdkReady: sdkReadyTime,
                    firstQueryComplete: firstQueryTime
                },
                durations: {
                    navigation: wasmInitTime - startTime,
                    wasmInit: sdkReadyTime - wasmInitTime, 
                    totalLoadTime: sdkReadyTime - startTime,
                    firstQuery: firstQueryTime - sdkReadyTime,
                    endToEnd: firstQueryTime - startTime
                },
                success: true,
                error: null
            };

            // Capture memory usage
            const metrics = await page.evaluate(() => {
                if (window.performance && window.performance.memory) {
                    return {
                        usedJSHeapSize: window.performance.memory.usedJSHeapSize,
                        totalJSHeapSize: window.performance.memory.totalJSHeapSize,
                        jsHeapSizeLimit: window.performance.memory.jsHeapSizeLimit
                    };
                }
                return null;
            });

            if (metrics) {
                result.memory = metrics;
            }

            return result;

        } catch (error) {
            return {
                networkCondition: networkName,
                iteration,
                timestamps: { navigationStart: startTime },
                durations: { totalLoadTime: Date.now() - startTime },
                success: false,
                error: error.message,
                timeout: error.message.includes('timeout')
            };
        }
    }

    calculateNetworkStats(networkName, results) {
        const successfulResults = results.filter(r => r.success);
        const targets = this.performanceTargets[networkName];

        if (successfulResults.length === 0) {
            return {
                networkName,
                success: false,
                error: 'All iterations failed',
                results: results
            };
        }

        const loadTimes = successfulResults.map(r => r.durations.totalLoadTime);
        const queryTimes = successfulResults.map(r => r.durations.firstQuery);
        const memoryUsages = successfulResults
            .map(r => r.memory?.usedJSHeapSize)
            .filter(m => m !== undefined);

        const stats = {
            networkName,
            success: true,
            iterations: results.length,
            successfulIterations: successfulResults.length,
            successRate: (successfulResults.length / results.length * 100).toFixed(1),
            
            loadTime: {
                average: Math.round(loadTimes.reduce((a, b) => a + b, 0) / loadTimes.length),
                min: Math.min(...loadTimes),
                max: Math.max(...loadTimes),
                median: this.calculateMedian(loadTimes),
                target: targets.target,
                maxTarget: targets.max
            },
            
            queryTime: {
                average: Math.round(queryTimes.reduce((a, b) => a + b, 0) / queryTimes.length),
                min: Math.min(...queryTimes),
                max: Math.max(...queryTimes)
            },

            memory: memoryUsages.length > 0 ? {
                averageUsedHeap: Math.round(memoryUsages.reduce((a, b) => a + b, 0) / memoryUsages.length),
                minUsedHeap: Math.min(...memoryUsages),
                maxUsedHeap: Math.max(...memoryUsages),
                averageUsedHeapMB: Math.round(memoryUsages.reduce((a, b) => a + b, 0) / memoryUsages.length / 1024 / 1024)
            } : null,

            performance: {
                meetsTarget: loadTimes.every(t => t <= targets.target),
                meetsMaxTarget: loadTimes.every(t => t <= targets.max),
                grade: this.calculatePerformanceGrade(loadTimes, targets)
            },

            rawResults: results
        };

        return stats;
    }

    calculateMedian(values) {
        const sorted = [...values].sort((a, b) => a - b);
        const mid = Math.floor(sorted.length / 2);
        return sorted.length % 2 !== 0 ? sorted[mid] : (sorted[mid - 1] + sorted[mid]) / 2;
    }

    calculatePerformanceGrade(loadTimes, targets) {
        const avgTime = loadTimes.reduce((a, b) => a + b, 0) / loadTimes.length;
        
        if (avgTime <= targets.target) return 'A';
        if (avgTime <= targets.target * 1.5) return 'B';
        if (avgTime <= targets.max) return 'C';
        if (avgTime <= targets.max * 1.5) return 'D';
        return 'F';
    }

    formatThroughput(throughput) {
        if (throughput === -1) return 'Unlimited';
        if (throughput >= 1000000) return `${(throughput / 1000000).toFixed(1)} Mbps`;
        if (throughput >= 1000) return `${(throughput / 1000).toFixed(1)} Kbps`;
        return `${throughput} bps`;
    }

    generateReport() {
        const report = {
            testRun: {
                timestamp: new Date().toISOString(),
                version: 'WASM SDK v0.1.0',
                testType: 'Load Time Benchmarks',
                environment: {
                    userAgent: 'Playwright Chromium',
                    platform: process.platform,
                    nodeVersion: process.version
                }
            },
            summary: this.generateSummary(),
            detailedResults: this.results,
            recommendations: this.generateRecommendations()
        };

        return report;
    }

    generateSummary() {
        const summary = {
            totalNetworks: this.results.length,
            overallSuccess: this.results.every(r => r.success),
            performanceGrades: {},
            memoryUsage: {
                averageAcrossNetworks: 0,
                maxObserved: 0,
                targetRange: '50-200MB (per issue requirements)'
            }
        };

        // Calculate summary statistics
        let totalAvgTime = 0;
        let totalMemory = 0;
        let memoryCount = 0;

        this.results.forEach(result => {
            if (result.success) {
                summary.performanceGrades[result.networkName] = result.performance.grade;
                totalAvgTime += result.loadTime.average;
                
                if (result.memory) {
                    totalMemory += result.memory.averageUsedHeapMB;
                    memoryCount++;
                    summary.memoryUsage.maxObserved = Math.max(
                        summary.memoryUsage.maxObserved, 
                        result.memory.maxUsedHeap / 1024 / 1024
                    );
                }
            }
        });

        summary.averageLoadTimeMs = Math.round(totalAvgTime / this.results.filter(r => r.success).length);
        
        if (memoryCount > 0) {
            summary.memoryUsage.averageAcrossNetworks = Math.round(totalMemory / memoryCount);
        }

        return summary;
    }

    generateRecommendations() {
        const recommendations = [];

        this.results.forEach(result => {
            if (!result.success) {
                recommendations.push({
                    priority: 'high',
                    network: result.networkName,
                    issue: 'Failed to load',
                    recommendation: 'Investigate initialization failures and timeout handling'
                });
                return;
            }

            const targets = this.performanceTargets[result.networkName];
            
            if (result.loadTime.average > targets.max) {
                recommendations.push({
                    priority: 'high',
                    network: result.networkName,
                    issue: `Load time ${result.loadTime.average}ms exceeds maximum target ${targets.max}ms`,
                    recommendation: 'Optimize WASM bundle size and initialization process'
                });
            } else if (result.loadTime.average > targets.target) {
                recommendations.push({
                    priority: 'medium',
                    network: result.networkName,
                    issue: `Load time ${result.loadTime.average}ms exceeds target ${targets.target}ms`,
                    recommendation: 'Consider bundle optimization and caching strategies'
                });
            }

            if (result.memory && result.memory.averageUsedHeapMB > 200) {
                recommendations.push({
                    priority: 'medium',
                    network: result.networkName,
                    issue: `Memory usage ${result.memory.averageUsedHeapMB}MB exceeds 200MB target`,
                    recommendation: 'Investigate memory leaks and optimize resource management'
                });
            }
        });

        if (recommendations.length === 0) {
            recommendations.push({
                priority: 'info',
                network: 'all',
                issue: 'All performance targets met',
                recommendation: 'Continue monitoring performance in production'
            });
        }

        return recommendations;
    }

    async saveReport(report) {
        const fs = require('fs').promises;
        const path = require('path');
        
        const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
        const fileName = `load-time-benchmark-${timestamp}.json`;
        const filePath = path.join(__dirname, 'reports', fileName);
        
        // Ensure reports directory exists
        await fs.mkdir(path.dirname(filePath), { recursive: true });
        
        // Save detailed report
        await fs.writeFile(filePath, JSON.stringify(report, null, 2));
        
        // Save latest report
        const latestPath = path.join(__dirname, 'reports', 'latest-load-time.json');
        await fs.writeFile(latestPath, JSON.stringify(report, null, 2));
        
        // Generate HTML report
        await this.generateHTMLReport(report, path.join(__dirname, 'reports', 'latest-load-time.html'));
        
        console.log(`\n📊 Reports saved:`);
        console.log(`   Detailed: ${filePath}`);
        console.log(`   Latest: ${latestPath}`);
        console.log(`   HTML: latest-load-time.html`);
    }

    async generateHTMLReport(report, htmlPath) {
        const html = `
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>WASM SDK Load Time Benchmark Report</title>
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; margin: 40px; }
        .header { background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: white; padding: 30px; border-radius: 12px; }
        .summary { background: #f8f9fa; padding: 25px; border-radius: 8px; margin: 20px 0; }
        .network-result { margin: 20px 0; padding: 20px; border: 2px solid #e9ecef; border-radius: 8px; }
        .network-result.success { border-color: #28a745; }
        .network-result.warning { border-color: #ffc107; }
        .network-result.error { border-color: #dc3545; }
        .metrics-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 15px; margin: 15px 0; }
        .metric-item { background: white; padding: 15px; border-radius: 6px; border: 1px solid #dee2e6; text-align: center; }
        .metric-value { font-size: 1.5rem; font-weight: bold; color: #2c3e50; }
        .metric-label { font-size: 0.9rem; color: #6c757d; }
        .grade { padding: 10px 20px; border-radius: 20px; font-weight: bold; color: white; }
        .grade-A { background: #28a745; }
        .grade-B { background: #17a2b8; }
        .grade-C { background: #ffc107; color: #333; }
        .grade-D { background: #fd7e14; }
        .grade-F { background: #dc3545; }
        .recommendations { background: #fff3cd; padding: 20px; border-radius: 8px; border: 2px solid #ffeaa7; }
        pre { background: #2c3e50; color: #ecf0f1; padding: 15px; border-radius: 6px; overflow-x: auto; font-size: 12px; }
    </style>
</head>
<body>
    <div class="header">
        <h1>🚀 WASM SDK Load Time Benchmark Report</h1>
        <p>Generated: ${report.testRun.timestamp}</p>
        <p>Test Environment: ${report.testRun.environment.platform} | ${report.testRun.environment.userAgent}</p>
    </div>

    <div class="summary">
        <h2>📊 Summary</h2>
        <div class="metrics-grid">
            <div class="metric-item">
                <div class="metric-value">${report.summary.totalNetworks}</div>
                <div class="metric-label">Networks Tested</div>
            </div>
            <div class="metric-item">
                <div class="metric-value">${report.summary.averageLoadTimeMs}ms</div>
                <div class="metric-label">Average Load Time</div>
            </div>
            <div class="metric-item">
                <div class="metric-value">${report.summary.memoryUsage.averageAcrossNetworks}MB</div>
                <div class="metric-label">Average Memory</div>
            </div>
            <div class="metric-item">
                <div class="metric-value">${report.summary.overallSuccess ? '✅' : '❌'}</div>
                <div class="metric-label">Overall Success</div>
            </div>
        </div>
    </div>

    ${report.detailedResults.map(result => `
        <div class="network-result ${result.success ? (result.performance.grade <= 'B' ? 'success' : 'warning') : 'error'}">
            <h3>📡 ${result.networkName.toUpperCase()} Network</h3>
            ${result.success ? `
                <div class="metrics-grid">
                    <div class="metric-item">
                        <div class="metric-value">${result.loadTime.average}ms</div>
                        <div class="metric-label">Average Load Time</div>
                    </div>
                    <div class="metric-item">
                        <div class="metric-value">${result.loadTime.min}-${result.loadTime.max}ms</div>
                        <div class="metric-label">Range</div>
                    </div>
                    <div class="metric-item">
                        <div class="metric-value">${result.queryTime.average}ms</div>
                        <div class="metric-label">Query Time</div>
                    </div>
                    <div class="metric-item">
                        <div class="metric-value grade grade-${result.performance.grade}">${result.performance.grade}</div>
                        <div class="metric-label">Performance Grade</div>
                    </div>
                </div>
                ${result.memory ? `
                    <p><strong>Memory Usage:</strong> ${result.memory.averageUsedHeapMB}MB average, ${Math.round(result.memory.maxUsedHeap / 1024 / 1024)}MB peak</p>
                ` : ''}
                <p><strong>Target:</strong> ${result.loadTime.target}ms (${result.performance.meetsTarget ? '✅ Met' : '❌ Missed'})</p>
            ` : `
                <p style="color: #dc3545;"><strong>❌ Failed:</strong> ${result.error}</p>
            `}
        </div>
    `).join('')}

    ${report.recommendations.length > 0 ? `
        <div class="recommendations">
            <h2>💡 Recommendations</h2>
            ${report.recommendations.map(rec => `
                <div style="margin: 10px 0; padding: 10px; border-left: 4px solid ${
                    rec.priority === 'high' ? '#dc3545' : 
                    rec.priority === 'medium' ? '#ffc107' : '#17a2b8'
                };">
                    <strong>${rec.priority.toUpperCase()}:</strong> ${rec.network} - ${rec.issue}<br>
                    <em>Recommendation:</em> ${rec.recommendation}
                </div>
            `).join('')}
        </div>
    ` : ''}

    <details style="margin-top: 30px;">
        <summary><h2>🔧 Raw Test Data</h2></summary>
        <pre>${JSON.stringify(report, null, 2)}</pre>
    </details>
</body>
</html>`;

        const fs = require('fs').promises;
        await fs.writeFile(htmlPath, html);
    }

    // CLI interface
    static async runCLI() {
        const benchmarks = new LoadTimeBenchmarks();
        
        try {
            // Start web server for testing
            const { spawn } = require('child_process');
            const server = spawn('python3', ['-m', 'http.server', '8888'], {
                cwd: '../../../',
                detached: true,
                stdio: 'ignore'
            });

            // Wait for server to start
            await new Promise(resolve => setTimeout(resolve, 3000));

            const report = await benchmarks.runAllBenchmarks();
            
            // Display summary
            console.log('\n📊 BENCHMARK SUMMARY');
            console.log('='.repeat(50));
            console.log(`Networks Tested: ${report.summary.totalNetworks}`);
            console.log(`Average Load Time: ${report.summary.averageLoadTimeMs}ms`);
            console.log(`Average Memory: ${report.summary.memoryUsage.averageAcrossNetworks}MB`);
            console.log(`Overall Success: ${report.summary.overallSuccess ? '✅' : '❌'}`);
            
            console.log('\n🎯 PERFORMANCE GRADES:');
            report.detailedResults.forEach(result => {
                if (result.success) {
                    console.log(`  ${result.networkName.toUpperCase().padEnd(6)} | Grade ${result.performance.grade} | ${result.loadTime.average}ms avg | ${result.memory?.averageUsedHeapMB || 'N/A'}MB`);
                } else {
                    console.log(`  ${result.networkName.toUpperCase().padEnd(6)} | FAILED | ${result.error}`);
                }
            });

            if (report.recommendations.length > 0) {
                console.log('\n💡 RECOMMENDATIONS:');
                report.recommendations.forEach(rec => {
                    console.log(`  ${rec.priority.toUpperCase()}: ${rec.recommendation}`);
                });
            }

            // Kill server
            process.kill(server.pid);
            
            return report;

        } catch (error) {
            console.error('❌ Benchmark failed:', error.message);
            process.exit(1);
        }
    }
}

module.exports = LoadTimeBenchmarks;

// CLI execution
if (require.main === module) {
    LoadTimeBenchmarks.runCLI().then(() => {
        console.log('\n✅ Load time benchmarks completed successfully');
        process.exit(0);
    }).catch(error => {
        console.error('❌ Benchmarks failed:', error);
        process.exit(1);
    });
}