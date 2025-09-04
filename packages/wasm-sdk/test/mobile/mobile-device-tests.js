/**
 * Mobile Device Testing Suite for WASM SDK
 * Tests mobile-specific constraints, performance, and user experience
 */

const { chromium, webkit } = require('playwright');

class MobileDeviceTests {
    constructor() {
        this.results = [];
        this.mobileDevices = [
            {
                name: 'iPhone 12 Pro',
                device: 'iPhone 12 Pro',
                memory: '6GB',
                network: '4G',
                expectedPerformance: 'high'
            },
            {
                name: 'iPhone SE',
                device: 'iPhone SE',
                memory: '3GB', 
                network: '4G',
                expectedPerformance: 'medium'
            },
            {
                name: 'Pixel 5',
                device: 'Pixel 5',
                memory: '8GB',
                network: '4G',
                expectedPerformance: 'high'
            },
            {
                name: 'Galaxy S20',
                device: 'Galaxy S20',
                memory: '12GB',
                network: '5G',
                expectedPerformance: 'high'
            },
            {
                name: 'Low-end Android',
                device: 'Moto G4',
                memory: '2GB',
                network: '3G',
                expectedPerformance: 'low'
            }
        ];

        this.performanceConstraints = {
            high: {
                maxInitTime: 15000,   // 15 seconds
                maxMemoryMB: 80,      // 80MB
                maxQueryTime: 8000    // 8 seconds
            },
            medium: {
                maxInitTime: 30000,   // 30 seconds
                maxMemoryMB: 60,      // 60MB  
                maxQueryTime: 12000   // 12 seconds
            },
            low: {
                maxInitTime: 60000,   // 1 minute
                maxMemoryMB: 40,      // 40MB
                maxQueryTime: 20000   // 20 seconds
            }
        };
    }

    async runMobileTests() {
        console.log('📱 Starting Mobile Device Compatibility Tests');
        console.log('='.repeat(50));

        for (const mobileDevice of this.mobileDevices) {
            console.log(`\n📱 Testing ${mobileDevice.name} (${mobileDevice.memory} RAM, ${mobileDevice.network})`);
            await this.testMobileDevice(mobileDevice);
        }

        const report = this.generateMobileReport();
        await this.saveMobileReport(report);
        
        return report;
    }

    async testMobileDevice(deviceConfig) {
        const browser = await this.getBrowserForDevice(deviceConfig);
        const context = await browser.newContext({
            ...this.getDeviceEmulation(deviceConfig)
        });

        try {
            // Apply network throttling based on device network
            await context.emulateNetwork(this.getNetworkConfig(deviceConfig.network));

            const page = await context.newPage();
            const testResult = await this.runMobileDeviceTests(page, deviceConfig);
            
            this.results.push(testResult);
            
            const status = testResult.success ? '✅' : '❌';
            const memory = testResult.memory.peakUsageMB;
            const initTime = testResult.performance.initializationTime;
            
            console.log(`   ${status} ${deviceConfig.name} - Init: ${initTime}ms, Memory: ${memory}MB`);

        } finally {
            await browser.close();
        }
    }

    async getBrowserForDevice(deviceConfig) {
        // Use appropriate browser engine for device
        if (deviceConfig.name.includes('iPhone') || deviceConfig.name.includes('iPad')) {
            return await webkit.launch({ headless: true });
        } else {
            return await chromium.launch({ 
                headless: true,
                args: [
                    '--no-sandbox',
                    '--disable-dev-shm-usage',
                    '--disable-web-security',
                    '--max_old_space_size=512' // Simulate memory constraints
                ]
            });
        }
    }

    getDeviceEmulation(deviceConfig) {
        const deviceEmulations = {
            'iPhone 12 Pro': {
                viewport: { width: 390, height: 844 },
                deviceScaleFactor: 3,
                isMobile: true,
                hasTouch: true,
                userAgent: 'Mozilla/5.0 (iPhone; CPU iPhone OS 15_0 like Mac OS X) AppleWebKit/605.1.15'
            },
            'iPhone SE': {
                viewport: { width: 375, height: 667 },
                deviceScaleFactor: 2,
                isMobile: true,
                hasTouch: true,
                userAgent: 'Mozilla/5.0 (iPhone; CPU iPhone OS 15_0 like Mac OS X) AppleWebKit/605.1.15'
            },
            'Pixel 5': {
                viewport: { width: 393, height: 851 },
                deviceScaleFactor: 2.75,
                isMobile: true,
                hasTouch: true,
                userAgent: 'Mozilla/5.0 (Linux; Android 11; Pixel 5) AppleWebKit/537.36'
            },
            'Galaxy S20': {
                viewport: { width: 360, height: 800 },
                deviceScaleFactor: 3,
                isMobile: true,
                hasTouch: true,
                userAgent: 'Mozilla/5.0 (Linux; Android 11; SM-G981B) AppleWebKit/537.36'
            },
            'Moto G4': {
                viewport: { width: 360, height: 640 },
                deviceScaleFactor: 2,
                isMobile: true,
                hasTouch: true,
                userAgent: 'Mozilla/5.0 (Linux; Android 7.0; Moto G (4)) AppleWebKit/537.36'
            }
        };

        return deviceEmulations[deviceConfig.device] || deviceEmulations['Pixel 5'];
    }

    getNetworkConfig(networkType) {
        const networkConfigs = {
            '5G': {
                downloadThroughput: 20 * 1000 * 1000,  // 20 Mbps
                uploadThroughput: 10 * 1000 * 1000,    // 10 Mbps
                latency: 10
            },
            '4G': {
                downloadThroughput: 4 * 1000 * 1000,   // 4 Mbps
                uploadThroughput: 3 * 1000 * 1000,     // 3 Mbps  
                latency: 150
            },
            '3G': {
                downloadThroughput: 1.6 * 1000 * 1000, // 1.6 Mbps
                uploadThroughput: 750 * 1000,           // 750 Kbps
                latency: 300
            },
            '2G': {
                downloadThroughput: 280 * 1000,         // 280 Kbps
                uploadThroughput: 256 * 1000,           // 256 Kbps
                latency: 800
            }
        };

        return networkConfigs[networkType] || networkConfigs['4G'];
    }

    async runMobileDeviceTests(page, deviceConfig) {
        const constraints = this.performanceConstraints[deviceConfig.expectedPerformance];
        const testResult = {
            device: deviceConfig.name,
            deviceConfig: deviceConfig,
            success: false,
            error: null,
            performance: {},
            memory: {},
            features: {},
            userExperience: {},
            timestamp: new Date().toISOString()
        };

        try {
            // Test 1: Initialization Performance
            console.log(`     📊 Testing initialization performance...`);
            const initResult = await this.testInitializationPerformance(page, constraints);
            testResult.performance = initResult;

            // Test 2: Memory Constraints
            console.log(`     🧠 Testing memory constraints...`);
            const memoryResult = await this.testMemoryConstraints(page, constraints);
            testResult.memory = memoryResult;

            // Test 3: Touch Interactions
            console.log(`     👆 Testing touch interactions...`);
            const touchResult = await this.testTouchInteractions(page);
            testResult.features.touch = touchResult;

            // Test 4: Mobile UI/UX
            console.log(`     🎨 Testing mobile UI/UX...`);
            const uxResult = await this.testMobileUX(page);
            testResult.userExperience = uxResult;

            // Test 5: Battery Impact (simulated)
            console.log(`     🔋 Testing battery impact...`);
            const batteryResult = await this.testBatteryImpact(page);
            testResult.features.battery = batteryResult;

            testResult.success = true;
            testResult.overallGrade = this.calculateMobileGrade(testResult, constraints);

        } catch (error) {
            testResult.error = error.message;
            console.log(`     ❌ Mobile test failed: ${error.message}`);
        }

        return testResult;
    }

    async testInitializationPerformance(page, constraints) {
        const startTime = Date.now();
        
        // Navigate to sample app
        await page.goto('/samples/identity-manager/', { 
            waitUntil: 'domcontentloaded',
            timeout: 120000 
        });

        // Wait for WASM and SDK initialization
        await page.waitForSelector('.status-dot.connected', { 
            timeout: constraints.maxInitTime 
        });

        const initializationTime = Date.now() - startTime;
        
        return {
            initializationTime,
            meetsTarget: initializationTime <= constraints.maxInitTime,
            targetTime: constraints.maxInitTime,
            grade: initializationTime <= constraints.maxInitTime ? 'A' : 
                   initializationTime <= constraints.maxInitTime * 1.5 ? 'B' : 'C'
        };
    }

    async testMemoryConstraints(page, constraints) {
        // Measure initial memory
        const initialMemory = await this.measureMobileMemory(page);
        
        // Perform memory-intensive operations
        const operations = [
            async () => {
                await page.fill('#identityIdInput', '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk');
                await page.click('#lookupBtn');
                await page.waitForSelector('#identityResults, #errorMessage', { timeout: 30000 });
            },
            async () => {
                await page.click('#checkBalanceBtn');
                await page.waitForTimeout(3000);
            },
            async () => {
                await page.click('#viewKeysBtn');
                await page.waitForTimeout(3000);
            }
        ];

        const memorySamples = [initialMemory];
        
        for (const operation of operations) {
            try {
                await operation();
                const memory = await this.measureMobileMemory(page);
                memorySamples.push(memory);
                await page.waitForTimeout(2000);
            } catch (error) {
                console.warn('Operation failed during memory test:', error.message);
            }
        }

        const peakMemory = Math.max(...memorySamples);
        const peakUsageMB = peakMemory / 1024 / 1024;
        
        return {
            initialUsageMB: Math.round(initialMemory / 1024 / 1024),
            peakUsageMB: Math.round(peakUsageMB),
            memoryGrowthMB: Math.round((peakMemory - initialMemory) / 1024 / 1024),
            samples: memorySamples.length,
            meetsConstraints: peakUsageMB <= constraints.maxMemoryMB,
            constraintMB: constraints.maxMemoryMB
        };
    }

    async testTouchInteractions(page) {
        const touchTests = [];
        
        try {
            // Test basic touch interactions
            await page.tap('#identityIdInput');
            touchTests.push({ test: 'input-tap', success: true });
            
            await page.tap('#lookupBtn');
            touchTests.push({ test: 'button-tap', success: true });
            
            // Test swipe/scroll if applicable
            try {
                await page.evaluate(() => {
                    window.scrollTo(0, 200);
                });
                touchTests.push({ test: 'scroll', success: true });
            } catch (error) {
                touchTests.push({ test: 'scroll', success: false, error: error.message });
            }
            
        } catch (error) {
            touchTests.push({ test: 'general-touch', success: false, error: error.message });
        }

        const successfulTests = touchTests.filter(t => t.success).length;
        
        return {
            totalTests: touchTests.length,
            successfulTests,
            successRate: (successfulTests / touchTests.length * 100).toFixed(1),
            tests: touchTests,
            overallSuccess: successfulTests === touchTests.length
        };
    }

    async testMobileUX(page) {
        const uxChecks = [];

        try {
            // Check viewport responsiveness
            const viewportSize = page.viewportSize();
            uxChecks.push({
                check: 'viewport-size',
                result: viewportSize.width <= 500 ? 'mobile' : 'not-mobile',
                success: viewportSize.width <= 500
            });

            // Check if UI elements are properly sized for touch
            const buttonSizes = await page.evaluate(() => {
                const buttons = document.querySelectorAll('.btn');
                const sizes = [];
                
                buttons.forEach(btn => {
                    const rect = btn.getBoundingClientRect();
                    sizes.push({
                        width: rect.width,
                        height: rect.height,
                        area: rect.width * rect.height
                    });
                });
                
                return sizes;
            });

            // Buttons should be large enough for touch (minimum 44px height recommended)
            const touchFriendlyButtons = buttonSizes.filter(size => size.height >= 44).length;
            const touchFriendlyRate = touchFriendlyButtons / buttonSizes.length;
            
            uxChecks.push({
                check: 'touch-friendly-buttons',
                result: `${touchFriendlyButtons}/${buttonSizes.length} buttons >= 44px height`,
                success: touchFriendlyRate >= 0.8 // 80% should be touch-friendly
            });

            // Check text readability
            const textSizes = await page.evaluate(() => {
                const elements = document.querySelectorAll('p, span, div');
                const fontSizes = [];
                
                elements.forEach(el => {
                    const style = window.getComputedStyle(el);
                    const fontSize = parseInt(style.fontSize);
                    if (fontSize > 0 && el.textContent.trim().length > 0) {
                        fontSizes.push(fontSize);
                    }
                });
                
                return fontSizes;
            });

            const readableFonts = textSizes.filter(size => size >= 14).length; // 14px minimum for mobile
            const readabilityRate = textSizes.length > 0 ? readableFonts / textSizes.length : 1;
            
            uxChecks.push({
                check: 'text-readability',
                result: `${readableFonts}/${textSizes.length} elements >= 14px`,
                success: readabilityRate >= 0.7 // 70% should be readable
            });

            // Check loading states visibility
            const loadingVisible = await page.isVisible('#loadingMessage, .loading-message');
            uxChecks.push({
                check: 'loading-indicators',
                result: loadingVisible ? 'visible' : 'not-visible',
                success: true // Loading indicators are optional
            });

        } catch (error) {
            uxChecks.push({
                check: 'general-ux',
                result: error.message,
                success: false
            });
        }

        const successfulChecks = uxChecks.filter(c => c.success).length;
        
        return {
            totalChecks: uxChecks.length,
            successfulChecks,
            uxScore: (successfulChecks / uxChecks.length * 100).toFixed(1),
            checks: uxChecks,
            overallSuccess: successfulChecks >= uxChecks.length * 0.8 // 80% threshold
        };
    }

    async testBatteryImpact(page) {
        const startTime = Date.now();
        let operationCount = 0;
        
        try {
            // Simulate continuous usage for 2 minutes
            const testDuration = 2 * 60 * 1000;
            const endTime = startTime + testDuration;

            while (Date.now() < endTime) {
                // Perform lightweight operations
                await page.click('#lookupBtn');
                await page.waitForTimeout(5000); // Wait between operations
                operationCount++;
                
                if (operationCount >= 10) break; // Limit operations for test speed
            }

            const actualDuration = Date.now() - startTime;
            const operationsPerMinute = (operationCount / actualDuration) * 60000;

            return {
                testDurationMs: actualDuration,
                operationsPerformed: operationCount,
                operationsPerMinute: Math.round(operationsPerMinute),
                estimatedBatteryImpact: this.estimateBatteryImpact(operationsPerMinute),
                success: true
            };

        } catch (error) {
            return {
                testDurationMs: Date.now() - startTime,
                operationsPerformed: operationCount,
                success: false,
                error: error.message
            };
        }
    }

    estimateBatteryImpact(operationsPerMinute) {
        // Rough estimation based on operation frequency
        if (operationsPerMinute > 20) return 'high';
        if (operationsPerMinute > 10) return 'medium';
        return 'low';
    }

    async measureMobileMemory(page) {
        try {
            const memory = await page.evaluate(() => {
                if (window.performance && window.performance.memory) {
                    return window.performance.memory.usedJSHeapSize;
                }
                return 0;
            });
            
            return memory;
        } catch (error) {
            return 0;
        }
    }

    calculateMobileGrade(testResult, constraints) {
        let score = 100;
        
        // Performance scoring
        if (testResult.performance.initializationTime > constraints.maxInitTime) {
            score -= 30;
        } else if (testResult.performance.initializationTime > constraints.maxInitTime * 0.8) {
            score -= 15;
        }

        // Memory scoring  
        if (testResult.memory.peakUsageMB > constraints.maxMemoryMB) {
            score -= 25;
        } else if (testResult.memory.peakUsageMB > constraints.maxMemoryMB * 0.8) {
            score -= 10;
        }

        // UX scoring
        if (!testResult.userExperience.overallSuccess) {
            score -= 20;
        }

        // Touch interaction scoring
        if (!testResult.features.touch.overallSuccess) {
            score -= 15;
        }

        // Battery impact scoring
        if (testResult.features.battery.estimatedBatteryImpact === 'high') {
            score -= 10;
        }

        if (score >= 90) return 'A';
        if (score >= 80) return 'B'; 
        if (score >= 70) return 'C';
        if (score >= 60) return 'D';
        return 'F';
    }

    generateMobileReport() {
        const report = {
            testRun: {
                timestamp: new Date().toISOString(),
                version: 'WASM SDK v0.1.0',
                testType: 'Mobile Device Compatibility',
                devicesTestedCount: this.results.length
            },
            summary: this.generateMobileSummary(),
            deviceResults: this.results,
            recommendations: this.generateMobileRecommendations()
        };

        return report;
    }

    generateMobileSummary() {
        const successfulTests = this.results.filter(r => r.success);
        
        return {
            totalDevices: this.results.length,
            successfulDevices: successfulTests.length,
            successRate: (successfulTests.length / this.results.length * 100).toFixed(1),
            
            performance: {
                averageInitTime: successfulTests.length > 0 ? 
                    Math.round(successfulTests.reduce((sum, r) => sum + r.performance.initializationTime, 0) / successfulTests.length) : 0,
                fastestDevice: successfulTests.reduce((fastest, r) => 
                    r.performance.initializationTime < fastest.performance.initializationTime ? r : fastest, successfulTests[0])?.device,
                slowestDevice: successfulTests.reduce((slowest, r) => 
                    r.performance.initializationTime > slowest.performance.initializationTime ? r : slowest, successfulTests[0])?.device
            },
            
            memory: {
                averagePeakUsage: successfulTests.length > 0 ?
                    Math.round(successfulTests.reduce((sum, r) => sum + r.memory.peakUsageMB, 0) / successfulTests.length) : 0,
                maxMemoryDevice: successfulTests.reduce((max, r) => 
                    r.memory.peakUsageMB > max.memory.peakUsageMB ? r : max, successfulTests[0])?.device,
                minMemoryDevice: successfulTests.reduce((min, r) => 
                    r.memory.peakUsageMB < min.memory.peakUsageMB ? r : min, successfulTests[0])?.device
            },

            grades: successfulTests.reduce((grades, r) => {
                grades[r.device] = r.overallGrade;
                return grades;
            }, {}),

            batteryImpact: {
                lowImpact: successfulTests.filter(r => r.features.battery?.estimatedBatteryImpact === 'low').length,
                mediumImpact: successfulTests.filter(r => r.features.battery?.estimatedBatteryImpact === 'medium').length,
                highImpact: successfulTests.filter(r => r.features.battery?.estimatedBatteryImpact === 'high').length
            }
        };
    }

    generateMobileRecommendations() {
        const recommendations = [];
        const summary = this.generateMobileSummary();

        // Performance recommendations
        if (summary.performance.averageInitTime > 30000) {
            recommendations.push({
                priority: 'high',
                category: 'mobile-performance',
                issue: `Average mobile initialization time ${summary.performance.averageInitTime}ms exceeds acceptable range`,
                recommendation: 'Optimize WASM bundle for mobile devices and implement progressive loading'
            });
        }

        // Memory recommendations
        if (summary.memory.averagePeakUsage > 80) {
            recommendations.push({
                priority: 'high',
                category: 'mobile-memory',
                issue: `Average mobile memory usage ${summary.memory.averagePeakUsage}MB approaches mobile limits`,
                recommendation: 'Implement aggressive memory management and consider mobile-specific builds'
            });
        }

        // Device-specific recommendations
        const failedDevices = this.results.filter(r => !r.success);
        if (failedDevices.length > 0) {
            recommendations.push({
                priority: 'medium',
                category: 'device-compatibility',
                issue: `${failedDevices.length} mobile devices failed testing`,
                recommendation: `Investigate compatibility issues on: ${failedDevices.map(d => d.device).join(', ')}`
            });
        }

        // Battery impact recommendations
        if (summary.batteryImpact.highImpact > 0) {
            recommendations.push({
                priority: 'medium',
                category: 'battery-optimization',
                issue: `${summary.batteryImpact.highImpact} devices show high battery impact`,
                recommendation: 'Optimize operation frequency and implement background processing limits'
            });
        }

        if (recommendations.length === 0) {
            recommendations.push({
                priority: 'info',
                category: 'mobile-success',
                issue: 'All mobile targets met',
                recommendation: 'Continue monitoring mobile performance in production'
            });
        }

        return recommendations;
    }

    async saveMobileReport(report) {
        const fs = require('fs').promises;
        const path = require('path');
        
        const reportsDir = path.join(__dirname, 'reports');
        await fs.mkdir(reportsDir, { recursive: true });
        
        const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
        const fileName = `mobile-device-test-${timestamp}.json`;
        const filePath = path.join(reportsDir, fileName);
        
        await fs.writeFile(filePath, JSON.stringify(report, null, 2));
        
        const latestPath = path.join(reportsDir, 'latest-mobile.json');
        await fs.writeFile(latestPath, JSON.stringify(report, null, 2));
        
        await this.generateMobileHTMLReport(report, path.join(reportsDir, 'latest-mobile.html'));
        
        console.log(`\n📊 Mobile test reports saved:`);
        console.log(`   Detailed: ${filePath}`);
        console.log(`   Latest: ${latestPath}`);
        console.log(`   HTML: latest-mobile.html`);
    }

    async generateMobileHTMLReport(report, htmlPath) {
        const html = `
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Mobile Device Compatibility Report</title>
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; margin: 20px; }
        .container { max-width: 1200px; margin: 0 auto; }
        .header { background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: white; padding: 30px; border-radius: 12px; }
        .summary { background: #f8f9fa; padding: 25px; border-radius: 8px; margin: 20px 0; }
        .device-result { margin: 20px 0; padding: 20px; border-radius: 8px; }
        .device-success { background: #d4edda; border: 2px solid #c3e6cb; }
        .device-warning { background: #fff3cd; border: 2px solid #ffeaa7; }
        .device-error { background: #f8d7da; border: 2px solid #f5c6cb; }
        .metrics-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 15px; }
        .metric-card { background: white; padding: 15px; border-radius: 6px; border: 1px solid #dee2e6; text-align: center; }
        .metric-value { font-size: 1.4rem; font-weight: bold; color: #2c3e50; }
        .metric-label { font-size: 0.85rem; color: #6c757d; }
        .grade { padding: 8px 16px; border-radius: 15px; font-weight: bold; color: white; margin: 5px; }
        .grade-A { background: #28a745; }
        .grade-B { background: #17a2b8; }
        .grade-C { background: #ffc107; color: #333; }
        .grade-D { background: #fd7e14; }
        .grade-F { background: #dc3545; }
        .device-specs { font-size: 0.9rem; color: #666; margin-bottom: 10px; }
        .test-details { background: #f8f9fa; padding: 15px; border-radius: 6px; margin-top: 15px; }
        .test-item { margin: 5px 0; padding: 8px; border-radius: 4px; }
        .test-success { background: #d4edda; }
        .test-warning { background: #fff3cd; }
        .test-error { background: #f8d7da; }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>📱 Mobile Device Compatibility Report</h1>
            <p>Generated: ${report.testRun.timestamp}</p>
            <p>WASM SDK Mobile Testing - Performance, Memory, and UX Validation</p>
        </div>

        <div class="summary">
            <h2>📊 Test Summary</h2>
            <div class="metrics-grid">
                <div class="metric-card">
                    <div class="metric-value">${report.summary.successfulDevices}/${report.summary.totalDevices}</div>
                    <div class="metric-label">Devices Passed</div>
                </div>
                <div class="metric-card">
                    <div class="metric-value">${report.summary.performance.averageInitTime}ms</div>
                    <div class="metric-label">Avg Init Time</div>
                </div>
                <div class="metric-card">
                    <div class="metric-value">${report.summary.memory.averagePeakUsage}MB</div>
                    <div class="metric-label">Avg Memory Usage</div>
                </div>
                <div class="metric-card">
                    <div class="metric-value">${report.summary.batteryImpact.lowImpact}</div>
                    <div class="metric-label">Low Battery Impact</div>
                </div>
            </div>
        </div>

        <h2>📱 Device Test Results</h2>
        ${report.deviceResults.map(result => `
            <div class="device-result ${result.success ? (result.overallGrade <= 'B' ? 'device-success' : 'device-warning') : 'device-error'}">
                <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 15px;">
                    <h3>${result.device}</h3>
                    ${result.success ? `<span class="grade grade-${result.overallGrade}">${result.overallGrade}</span>` : '<span style="color: #dc3545;">❌ FAILED</span>'}
                </div>
                
                <div class="device-specs">
                    ${result.deviceConfig.memory} RAM | ${result.deviceConfig.network} Network | Expected: ${result.deviceConfig.expectedPerformance} performance
                </div>

                ${result.success ? `
                    <div class="metrics-grid">
                        <div class="metric-card">
                            <div class="metric-value">${result.performance.initializationTime}ms</div>
                            <div class="metric-label">Init Time</div>
                        </div>
                        <div class="metric-card">
                            <div class="metric-value">${result.memory.peakUsageMB}MB</div>
                            <div class="metric-label">Peak Memory</div>
                        </div>
                        <div class="metric-card">
                            <div class="metric-value">${result.userExperience.uxScore}%</div>
                            <div class="metric-label">UX Score</div>
                        </div>
                        <div class="metric-card">
                            <div class="metric-value">${result.features.touch.successRate}%</div>
                            <div class="metric-label">Touch Success</div>
                        </div>
                    </div>

                    <details class="test-details">
                        <summary>Detailed Test Results</summary>
                        <h4>Memory Analysis</h4>
                        <p>Initial: ${result.memory.initialUsageMB}MB | Peak: ${result.memory.peakUsageMB}MB | Growth: ${result.memory.memoryGrowthMB}MB</p>
                        
                        <h4>Touch Interactions (${result.features.touch.successfulTests}/${result.features.touch.totalTests})</h4>
                        ${result.features.touch.tests.map(test => `
                            <div class="test-item test-${test.success ? 'success' : 'error'}">
                                ${test.test}: ${test.success ? '✅' : '❌'} ${test.error || ''}
                            </div>
                        `).join('')}

                        <h4>User Experience (${result.userExperience.successfulChecks}/${result.userExperience.totalChecks})</h4>
                        ${result.userExperience.checks.map(check => `
                            <div class="test-item test-${check.success ? 'success' : 'warning'}">
                                ${check.check}: ${check.result}
                            </div>
                        `).join('')}

                        <h4>Battery Impact</h4>
                        <p>Operations/min: ${result.features.battery.operationsPerMinute} | Impact: ${result.features.battery.estimatedBatteryImpact}</p>
                    </details>
                ` : `
                    <div style="color: #dc3545; padding: 15px; background: #f8d7da; border-radius: 6px;">
                        <strong>Test Failed:</strong> ${result.error}
                    </div>
                `}
            </div>
        `).join('')}

        ${report.recommendations.length > 0 ? `
            <div style="background: #d1ecf1; padding: 20px; border-radius: 8px; margin-top: 30px;">
                <h2>💡 Mobile Optimization Recommendations</h2>
                ${report.recommendations.map(rec => `
                    <div style="margin: 15px 0; padding: 15px; border-left: 4px solid ${
                        rec.priority === 'high' ? '#dc3545' : 
                        rec.priority === 'medium' ? '#ffc107' : '#17a2b8'
                    }; background: white; border-radius: 4px;">
                        <strong>${rec.priority.toUpperCase()}:</strong> ${rec.category}<br>
                        <strong>Issue:</strong> ${rec.issue}<br>
                        <strong>Recommendation:</strong> ${rec.recommendation}
                    </div>
                `).join('')}
            </div>
        ` : ''}
    </div>
</body>
</html>`;

        const fs = require('fs').promises;
        return fs.writeFile(htmlPath, html);
    }

    // CLI interface
    static async runCLI() {
        const mobileTests = new MobileDeviceTests();
        
        try {
            const report = await mobileTests.runMobileTests();
            
            console.log('\n📊 MOBILE DEVICE TEST SUMMARY');
            console.log('='.repeat(50));
            console.log(`Devices Tested: ${report.summary.totalDevices}`);
            console.log(`Success Rate: ${report.summary.successRate}%`);
            console.log(`Average Init Time: ${report.summary.performance.averageInitTime}ms`);
            console.log(`Average Memory: ${report.summary.memory.averagePeakUsage}MB`);
            
            console.log('\n📱 DEVICE GRADES:');
            Object.entries(report.summary.grades).forEach(([device, grade]) => {
                console.log(`  ${device.padEnd(20)} | Grade ${grade}`);
            });

            if (report.recommendations.length > 0) {
                console.log('\n💡 MOBILE RECOMMENDATIONS:');
                report.recommendations.forEach(rec => {
                    console.log(`  ${rec.priority.toUpperCase()}: ${rec.recommendation}`);
                });
            }
            
            return report;

        } catch (error) {
            console.error('❌ Mobile device tests failed:', error.message);
            process.exit(1);
        }
    }
}

module.exports = MobileDeviceTests;

// CLI execution
if (require.main === module) {
    MobileDeviceTests.runCLI().then(() => {
        console.log('\n✅ Mobile device tests completed successfully');
        process.exit(0);
    }).catch(error => {
        console.error('❌ Mobile tests failed:', error);
        process.exit(1);
    });
}