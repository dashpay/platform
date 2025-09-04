/**
 * Performance Regression Detection for WASM SDK
 * Compares current performance against historical baselines
 */

const fs = require('fs').promises;
const path = require('path');
const LoadTimeBenchmarks = require('./load-time-benchmarks');
const MemoryBenchmarks = require('./memory-benchmarks');

class RegressionDetector {
    constructor() {
        this.baselineDir = path.join(__dirname, 'baselines');
        this.reportsDir = path.join(__dirname, 'reports');
        this.regressionThresholds = {
            loadTime: {
                warning: 1.2,  // 20% slower triggers warning
                critical: 1.5  // 50% slower triggers critical
            },
            memory: {
                warning: 1.15, // 15% more memory triggers warning  
                critical: 1.3  // 30% more memory triggers critical
            },
            bundleSize: {
                warning: 1.1,  // 10% larger bundle triggers warning
                critical: 1.25 // 25% larger bundle triggers critical
            }
        };
    }

    async detectRegressions() {
        console.log('🔍 Starting Performance Regression Detection');
        console.log('='.repeat(50));

        // Run current benchmarks
        console.log('\n📊 Running current performance benchmarks...');
        const currentResults = await this.runCurrentBenchmarks();

        // Load baseline for comparison
        console.log('\n📋 Loading baseline performance data...');
        const baseline = await this.loadBaseline();

        if (!baseline) {
            console.log('⚠️  No baseline found - establishing current run as baseline');
            await this.saveAsBaseline(currentResults);
            return {
                type: 'baseline-established',
                baseline: currentResults,
                regressions: [],
                recommendations: ['Baseline established - future runs will compare against this performance']
            };
        }

        // Compare performance
        console.log('\n🔍 Analyzing performance changes...');
        const regressions = this.analyzeRegressions(baseline, currentResults);

        // Generate regression report
        const report = {
            testRun: {
                timestamp: new Date().toISOString(),
                baselineVersion: baseline.version || 'unknown',
                currentVersion: currentResults.version || 'current',
                testType: 'Regression Detection'
            },
            baseline: this.extractBaselineMetrics(baseline),
            current: this.extractCurrentMetrics(currentResults),
            regressions: regressions,
            summary: this.generateRegressionSummary(regressions),
            recommendations: this.generateRegressionRecommendations(regressions)
        };

        await this.saveRegressionReport(report);

        // Check if we should update baseline
        if (this.shouldUpdateBaseline(regressions)) {
            console.log('\n✨ Performance improved - updating baseline');
            await this.saveAsBaseline(currentResults);
        }

        return report;
    }

    async runCurrentBenchmarks() {
        const results = {
            version: 'current',
            timestamp: new Date().toISOString(),
            loadTime: null,
            memory: null,
            bundleSize: null
        };

        try {
            // Run load time benchmarks
            const loadBenchmarks = new LoadTimeBenchmarks();
            results.loadTime = await loadBenchmarks.runAllBenchmarks();
        } catch (error) {
            console.error('Load time benchmarks failed:', error.message);
            results.loadTime = { error: error.message };
        }

        try {
            // Run memory benchmarks
            const memBenchmarks = new MemoryBenchmarks();
            results.memory = await memBenchmarks.runMemoryBenchmarks();
        } catch (error) {
            console.error('Memory benchmarks failed:', error.message);
            results.memory = { error: error.message };
        }

        try {
            // Check bundle size
            results.bundleSize = await this.measureBundleSize();
        } catch (error) {
            console.error('Bundle size check failed:', error.message);
            results.bundleSize = { error: error.message };
        }

        return results;
    }

    async measureBundleSize() {
        const pkgDir = path.join(__dirname, '../../pkg');
        
        const bundleInfo = {
            timestamp: new Date().toISOString(),
            files: {}
        };

        try {
            const files = await fs.readdir(pkgDir);
            
            for (const file of files) {
                const filePath = path.join(pkgDir, file);
                const stats = await fs.stat(filePath);
                
                if (stats.isFile()) {
                    bundleInfo.files[file] = {
                        size: stats.size,
                        sizeMB: stats.size / 1024 / 1024,
                        sizeKB: stats.size / 1024
                    };
                }
            }

            // Calculate totals
            bundleInfo.total = {
                files: Object.keys(bundleInfo.files).length,
                totalSize: Object.values(bundleInfo.files).reduce((sum, file) => sum + file.size, 0),
                wasmSize: bundleInfo.files['dash_wasm_sdk_bg.wasm']?.size || 0,
                jsSize: bundleInfo.files['dash_wasm_sdk.js']?.size || 0
            };

            bundleInfo.total.totalSizeMB = bundleInfo.total.totalSize / 1024 / 1024;
            bundleInfo.total.wasmSizeMB = bundleInfo.total.wasmSize / 1024 / 1024;
            bundleInfo.total.jsSizeKB = bundleInfo.total.jsSize / 1024;

        } catch (error) {
            bundleInfo.error = error.message;
        }

        return bundleInfo;
    }

    async loadBaseline() {
        try {
            const baselinePath = path.join(this.baselineDir, 'performance-baseline.json');
            const baselineData = await fs.readFile(baselinePath, 'utf8');
            return JSON.parse(baselineData);
        } catch (error) {
            console.log('No existing baseline found');
            return null;
        }
    }

    async saveAsBaseline(results) {
        await fs.mkdir(this.baselineDir, { recursive: true });
        
        const baselinePath = path.join(this.baselineDir, 'performance-baseline.json');
        const backupPath = path.join(this.baselineDir, `baseline-backup-${Date.now()}.json`);
        
        // Backup existing baseline if it exists
        try {
            await fs.access(baselinePath);
            await fs.copyFile(baselinePath, backupPath);
        } catch (error) {
            // No existing baseline to backup
        }
        
        await fs.writeFile(baselinePath, JSON.stringify(results, null, 2));
        console.log(`📊 Baseline saved: ${baselinePath}`);
    }

    analyzeRegressions(baseline, current) {
        const regressions = [];

        // Analyze load time regressions
        if (baseline.loadTime && current.loadTime && baseline.loadTime.summary && current.loadTime.summary) {
            const baselineAvg = baseline.loadTime.summary.averageLoadTimeMs;
            const currentAvg = current.loadTime.summary.averageLoadTimeMs;
            
            if (baselineAvg && currentAvg) {
                const ratio = currentAvg / baselineAvg;
                
                if (ratio >= this.regressionThresholds.loadTime.critical) {
                    regressions.push({
                        type: 'load-time',
                        severity: 'critical',
                        metric: 'Average Load Time',
                        baseline: `${baselineAvg}ms`,
                        current: `${currentAvg}ms`,
                        change: `+${((ratio - 1) * 100).toFixed(1)}%`,
                        impact: 'User experience severely degraded'
                    });
                } else if (ratio >= this.regressionThresholds.loadTime.warning) {
                    regressions.push({
                        type: 'load-time',
                        severity: 'warning',
                        metric: 'Average Load Time',
                        baseline: `${baselineAvg}ms`,
                        current: `${currentAvg}ms`,
                        change: `+${((ratio - 1) * 100).toFixed(1)}%`,
                        impact: 'Noticeable performance degradation'
                    });
                } else if (ratio < 0.9) {
                    regressions.push({
                        type: 'load-time',
                        severity: 'improvement',
                        metric: 'Average Load Time',
                        baseline: `${baselineAvg}ms`,
                        current: `${currentAvg}ms`,
                        change: `${((ratio - 1) * 100).toFixed(1)}%`,
                        impact: 'Performance improvement detected'
                    });
                }
            }
        }

        // Analyze memory regressions
        if (baseline.memory && current.memory && baseline.memory.summary && current.memory.summary) {
            const baselineMem = baseline.memory.summary.peakMemory.averageMB;
            const currentMem = current.memory.summary.peakMemory.averageMB;
            
            if (baselineMem && currentMem) {
                const ratio = currentMem / baselineMem;
                
                if (ratio >= this.regressionThresholds.memory.critical) {
                    regressions.push({
                        type: 'memory',
                        severity: 'critical',
                        metric: 'Peak Memory Usage',
                        baseline: `${baselineMem}MB`,
                        current: `${currentMem}MB`,
                        change: `+${((ratio - 1) * 100).toFixed(1)}%`,
                        impact: 'Memory usage exceeds acceptable limits'
                    });
                } else if (ratio >= this.regressionThresholds.memory.warning) {
                    regressions.push({
                        type: 'memory',
                        severity: 'warning',
                        metric: 'Peak Memory Usage',
                        baseline: `${baselineMem}MB`,
                        current: `${currentMem}MB`,
                        change: `+${((ratio - 1) * 100).toFixed(1)}%`,
                        impact: 'Memory usage increased significantly'
                    });
                }
            }
        }

        // Analyze bundle size regressions
        if (baseline.bundleSize && current.bundleSize && 
            baseline.bundleSize.total && current.bundleSize.total) {
            const baselineSize = baseline.bundleSize.total.totalSizeMB;
            const currentSize = current.bundleSize.total.totalSizeMB;
            
            if (baselineSize && currentSize) {
                const ratio = currentSize / baselineSize;
                
                if (ratio >= this.regressionThresholds.bundleSize.critical) {
                    regressions.push({
                        type: 'bundle-size',
                        severity: 'critical',
                        metric: 'Total Bundle Size',
                        baseline: `${baselineSize.toFixed(2)}MB`,
                        current: `${currentSize.toFixed(2)}MB`,
                        change: `+${((ratio - 1) * 100).toFixed(1)}%`,
                        impact: 'Bundle size increase will significantly impact load times'
                    });
                } else if (ratio >= this.regressionThresholds.bundleSize.warning) {
                    regressions.push({
                        type: 'bundle-size',
                        severity: 'warning',
                        metric: 'Total Bundle Size',
                        baseline: `${baselineSize.toFixed(2)}MB`,
                        current: `${currentSize.toFixed(2)}MB`,
                        change: `+${((ratio - 1) * 100).toFixed(1)}%`,
                        impact: 'Bundle size increase may impact load performance'
                    });
                }
            }
        }

        return regressions;
    }

    extractBaselineMetrics(baseline) {
        return {
            loadTime: baseline.loadTime?.summary?.averageLoadTimeMs || 'N/A',
            memory: baseline.memory?.summary?.peakMemory?.averageMB || 'N/A',
            bundleSize: baseline.bundleSize?.total?.totalSizeMB?.toFixed(2) || 'N/A',
            timestamp: baseline.timestamp
        };
    }

    extractCurrentMetrics(current) {
        return {
            loadTime: current.loadTime?.summary?.averageLoadTimeMs || 'N/A',
            memory: current.memory?.summary?.peakMemory?.averageMB || 'N/A',
            bundleSize: current.bundleSize?.total?.totalSizeMB?.toFixed(2) || 'N/A',
            timestamp: current.timestamp
        };
    }

    generateRegressionSummary(regressions) {
        const summary = {
            total: regressions.length,
            critical: regressions.filter(r => r.severity === 'critical').length,
            warnings: regressions.filter(r => r.severity === 'warning').length,
            improvements: regressions.filter(r => r.severity === 'improvement').length,
            overallStatus: 'pass'
        };

        if (summary.critical > 0) {
            summary.overallStatus = 'critical';
        } else if (summary.warnings > 0) {
            summary.overallStatus = 'warning';
        } else if (summary.improvements > 0) {
            summary.overallStatus = 'improved';
        }

        return summary;
    }

    generateRegressionRecommendations(regressions) {
        const recommendations = [];

        const criticalRegressions = regressions.filter(r => r.severity === 'critical');
        const warningRegressions = regressions.filter(r => r.severity === 'warning');

        if (criticalRegressions.length > 0) {
            recommendations.push({
                priority: 'critical',
                category: 'immediate-action',
                issue: `${criticalRegressions.length} critical performance regressions detected`,
                recommendation: 'Stop deployment and investigate performance degradation immediately',
                details: criticalRegressions.map(r => `${r.metric}: ${r.change}`)
            });
        }

        if (warningRegressions.length > 0) {
            recommendations.push({
                priority: 'high',
                category: 'performance-monitoring',
                issue: `${warningRegressions.length} performance warnings detected`,
                recommendation: 'Monitor closely and investigate if trend continues',
                details: warningRegressions.map(r => `${r.metric}: ${r.change}`)
            });
        }

        // Bundle size specific recommendations
        const bundleRegressions = regressions.filter(r => r.type === 'bundle-size');
        if (bundleRegressions.length > 0) {
            recommendations.push({
                priority: 'medium',
                category: 'bundle-optimization',
                issue: 'Bundle size increased',
                recommendation: 'Review new dependencies and consider bundle splitting or compression improvements'
            });
        }

        // Memory specific recommendations  
        const memoryRegressions = regressions.filter(r => r.type === 'memory');
        if (memoryRegressions.length > 0) {
            recommendations.push({
                priority: 'high',
                category: 'memory-optimization',
                issue: 'Memory usage increased',
                recommendation: 'Investigate memory leaks, optimize resource management, and review WASM heap usage'
            });
        }

        if (recommendations.length === 0) {
            recommendations.push({
                priority: 'info',
                category: 'maintenance',
                issue: 'No regressions detected',
                recommendation: 'Continue regular performance monitoring and baseline updates'
            });
        }

        return recommendations;
    }

    shouldUpdateBaseline(regressions) {
        // Update baseline if there are significant improvements and no regressions
        const improvements = regressions.filter(r => r.severity === 'improvement').length;
        const negativeRegressions = regressions.filter(r => r.severity === 'warning' || r.severity === 'critical').length;
        
        return improvements > 0 && negativeRegressions === 0;
    }

    async saveRegressionReport(report) {
        await fs.mkdir(this.reportsDir, { recursive: true });
        
        const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
        const fileName = `regression-report-${timestamp}.json`;
        const filePath = path.join(this.reportsDir, fileName);
        
        // Save detailed report
        await fs.writeFile(filePath, JSON.stringify(report, null, 2));
        
        // Save latest report
        const latestPath = path.join(this.reportsDir, 'latest-regression.json');
        await fs.writeFile(latestPath, JSON.stringify(report, null, 2));
        
        // Generate HTML report
        await this.generateRegressionHTMLReport(report);
        
        console.log(`\n📊 Regression reports saved:`);
        console.log(`   Detailed: ${filePath}`);
        console.log(`   Latest: ${latestPath}`);
    }

    async generateRegressionHTMLReport(report) {
        const html = `
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>Performance Regression Report</title>
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; margin: 40px; background: #f5f5f5; }
        .container { max-width: 1000px; margin: 0 auto; background: white; border-radius: 12px; overflow: hidden; box-shadow: 0 4px 20px rgba(0,0,0,0.1); }
        .header { background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: white; padding: 30px; }
        .status-${report.summary.overallStatus} { border-left: 8px solid ${
            report.summary.overallStatus === 'critical' ? '#dc3545' :
            report.summary.overallStatus === 'warning' ? '#ffc107' :
            report.summary.overallStatus === 'improved' ? '#28a745' : '#17a2b8'
        }; }
        .section { padding: 30px; border-bottom: 1px solid #e9ecef; }
        .metrics-comparison { display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 20px; }
        .metric-card { background: #f8f9fa; padding: 20px; border-radius: 8px; text-align: center; }
        .metric-value { font-size: 1.5rem; font-weight: bold; margin-bottom: 5px; }
        .metric-baseline { color: #6c757d; }
        .metric-current { color: #2c3e50; }
        .metric-change { font-weight: bold; }
        .change-positive { color: #dc3545; }
        .change-negative { color: #28a745; }
        .change-neutral { color: #6c757d; }
        .regression-item { margin: 15px 0; padding: 20px; border-radius: 8px; }
        .regression-critical { background: #f8d7da; border: 2px solid #f5c6cb; }
        .regression-warning { background: #fff3cd; border: 2px solid #ffeaa7; }
        .regression-improvement { background: #d4edda; border: 2px solid #c3e6cb; }
        .severity-badge { padding: 6px 12px; border-radius: 12px; font-size: 12px; font-weight: bold; color: white; }
        .severity-critical { background: #dc3545; }
        .severity-warning { background: #ffc107; color: #333; }
        .severity-improvement { background: #28a745; }
    </style>
</head>
<body>
    <div class="container status-${report.summary.overallStatus}">
        <div class="header">
            <h1>🔍 Performance Regression Report</h1>
            <p>Generated: ${report.testRun.timestamp}</p>
            <p>Baseline: ${report.testRun.baselineVersion} → Current: ${report.testRun.currentVersion}</p>
            <h2 style="margin-top: 20px;">
                Overall Status: ${
                    report.summary.overallStatus === 'critical' ? '🚨 CRITICAL REGRESSIONS' :
                    report.summary.overallStatus === 'warning' ? '⚠️ PERFORMANCE WARNINGS' :
                    report.summary.overallStatus === 'improved' ? '✅ PERFORMANCE IMPROVED' :
                    '✅ NO REGRESSIONS'
                }
            </h2>
        </div>

        <div class="section">
            <h2>📊 Performance Comparison</h2>
            <div class="metrics-comparison">
                <div class="metric-card">
                    <div class="metric-value metric-baseline">Baseline</div>
                    <div>Load Time: ${report.baseline.loadTime}ms</div>
                    <div>Memory: ${report.baseline.memory}MB</div>
                    <div>Bundle: ${report.baseline.bundleSize}MB</div>
                </div>
                <div class="metric-card">
                    <div class="metric-value metric-current">Current</div>
                    <div>Load Time: ${report.current.loadTime}ms</div>
                    <div>Memory: ${report.current.memory}MB</div>
                    <div>Bundle: ${report.current.bundleSize}MB</div>
                </div>
                <div class="metric-card">
                    <div class="metric-value metric-change">Change</div>
                    ${this.formatChangeMetrics(report.baseline, report.current)}
                </div>
            </div>
        </div>

        ${report.regressions.length > 0 ? `
            <div class="section">
                <h2>🚨 Detected Changes</h2>
                ${report.regressions.map(regression => `
                    <div class="regression-item regression-${regression.severity}">
                        <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 10px;">
                            <h4>${regression.metric}</h4>
                            <span class="severity-badge severity-${regression.severity}">${regression.severity.toUpperCase()}</span>
                        </div>
                        <p><strong>Change:</strong> ${regression.baseline} → ${regression.current} (${regression.change})</p>
                        <p><strong>Impact:</strong> ${regression.impact}</p>
                    </div>
                `).join('')}
            </div>
        ` : ''}

        ${report.recommendations.length > 0 ? `
            <div class="section">
                <h2>💡 Recommendations</h2>
                ${report.recommendations.map(rec => `
                    <div class="regression-item regression-${rec.priority === 'critical' ? 'critical' : 'warning'}">
                        <h4>${rec.category.toUpperCase().replace('-', ' ')}</h4>
                        <p><strong>${rec.priority.toUpperCase()}:</strong> ${rec.issue}</p>
                        <p><strong>Action:</strong> ${rec.recommendation}</p>
                        ${rec.details ? `<p><strong>Details:</strong> ${rec.details.join(', ')}</p>` : ''}
                    </div>
                `).join('')}
            </div>
        ` : ''}
    </div>
</body>
</html>`;

        const htmlPath = path.join(this.reportsDir, 'latest-regression.html');
        await fs.writeFile(htmlPath, html);
    }

    formatChangeMetrics(baseline, current) {
        const changes = [];
        
        if (baseline.loadTime !== 'N/A' && current.loadTime !== 'N/A') {
            const change = ((current.loadTime - baseline.loadTime) / baseline.loadTime * 100).toFixed(1);
            const className = change > 0 ? 'change-positive' : change < 0 ? 'change-negative' : 'change-neutral';
            changes.push(`<div class="${className}">Load: ${change > 0 ? '+' : ''}${change}%</div>`);
        }
        
        if (baseline.memory !== 'N/A' && current.memory !== 'N/A') {
            const change = ((current.memory - baseline.memory) / baseline.memory * 100).toFixed(1);
            const className = change > 0 ? 'change-positive' : change < 0 ? 'change-negative' : 'change-neutral';
            changes.push(`<div class="${className}">Memory: ${change > 0 ? '+' : ''}${change}%</div>`);
        }

        if (baseline.bundleSize !== 'N/A' && current.bundleSize !== 'N/A') {
            const change = ((current.bundleSize - baseline.bundleSize) / baseline.bundleSize * 100).toFixed(1);
            const className = change > 0 ? 'change-positive' : change < 0 ? 'change-negative' : 'change-neutral';
            changes.push(`<div class="${className}">Bundle: ${change > 0 ? '+' : ''}${change}%</div>`);
        }

        return changes.join('');
    }

    // CI Integration
    static async runCI() {
        const detector = new RegressionDetector();
        
        try {
            const report = await detector.detectRegressions();
            
            // CI-friendly output
            console.log('\n📊 REGRESSION DETECTION SUMMARY');
            console.log('='.repeat(50));
            console.log(`Overall Status: ${report.summary?.overallStatus || report.type}`);
            
            if (report.regressions) {
                console.log(`Total Changes: ${report.summary.total}`);
                console.log(`Critical: ${report.summary.critical}`);
                console.log(`Warnings: ${report.summary.warnings}`);
                console.log(`Improvements: ${report.summary.improvements}`);
                
                // Print critical regressions
                const critical = report.regressions.filter(r => r.severity === 'critical');
                if (critical.length > 0) {
                    console.log('\n🚨 CRITICAL REGRESSIONS:');
                    critical.forEach(reg => {
                        console.log(`  ❌ ${reg.metric}: ${reg.change} (${reg.baseline} → ${reg.current})`);
                    });
                    
                    // Exit with error code for CI failure
                    process.exit(1);
                }
                
                // Print warnings
                const warnings = report.regressions.filter(r => r.severity === 'warning');
                if (warnings.length > 0) {
                    console.log('\n⚠️  PERFORMANCE WARNINGS:');
                    warnings.forEach(reg => {
                        console.log(`  ⚠️  ${reg.metric}: ${reg.change} (${reg.baseline} → ${reg.current})`);
                    });
                }
                
                // Print improvements
                const improvements = report.regressions.filter(r => r.severity === 'improvement');
                if (improvements.length > 0) {
                    console.log('\n✅ PERFORMANCE IMPROVEMENTS:');
                    improvements.forEach(reg => {
                        console.log(`  ✅ ${reg.metric}: ${reg.change} (${reg.baseline} → ${reg.current})`);
                    });
                }
            }
            
            console.log('\n✅ Regression detection completed');
            return report;

        } catch (error) {
            console.error('❌ Regression detection failed:', error.message);
            process.exit(1);
        }
    }
}

module.exports = RegressionDetector;

// CLI execution
if (require.main === module) {
    RegressionDetector.runCI().then(() => {
        process.exit(0);
    }).catch(error => {
        console.error('❌ Regression detection failed:', error);
        process.exit(1);
    });
}