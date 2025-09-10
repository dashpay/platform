/**
 * Credit Usage Tracker
 * Monitors and reports on testnet credit consumption across all funded tests
 */

const fs = require('fs');
const path = require('path');

class CreditTracker {
    constructor(options = {}) {
        this.debug = options.debug || false;
        this.logFile = options.logFile || path.join(__dirname, '../logs/credit-usage.log');
        this.network = options.network || 'testnet';
        
        // Usage limits and monitoring
        this.dailyLimit = options.dailyLimit || 20000000000; // 20 DASH in satoshis
        this.testSuiteLimit = options.testSuiteLimit || 5000000000; // 5 DASH per test suite
        this.operationLimit = options.operationLimit || 500000000; // 0.5 DASH per operation
        
        // Current session tracking
        this.currentSession = {
            startTime: Date.now(),
            sessionId: this.generateSessionId(),
            totalUsage: 0,
            operations: [],
            errors: [],
            warnings: []
        };

        // Historical tracking
        this.dailyUsage = this.loadDailyUsage();
        
        this.initializeLogging();
    }

    /**
     * Generate unique session ID
     */
    generateSessionId() {
        const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
        const random = Math.random().toString(36).substring(2, 8);
        return `session-${timestamp}-${random}`;
    }

    /**
     * Initialize logging directory and files
     */
    initializeLogging() {
        const logDir = path.dirname(this.logFile);
        if (!fs.existsSync(logDir)) {
            fs.mkdirSync(logDir, { recursive: true });
        }

        // Create log file if it doesn't exist
        if (!fs.existsSync(this.logFile)) {
            this.writeLog('INFO', 'Credit tracking initialized');
        }
    }

    /**
     * Record a funding operation
     */
    recordOperation(operation) {
        const timestamp = Date.now();
        const operationRecord = {
            timestamp,
            sessionId: this.currentSession.sessionId,
            type: operation.type, // 'identity-creation', 'identity-topup', 'document-creation', etc.
            identityId: operation.identityId,
            amount: operation.amount, // Credits consumed
            satoshis: operation.satoshis, // Actual satoshis spent
            txId: operation.txId,
            testName: operation.testName,
            success: operation.success !== false,
            error: operation.error || null,
            metadata: operation.metadata || {}
        };

        this.currentSession.operations.push(operationRecord);
        this.currentSession.totalUsage += operationRecord.satoshis || 0;

        // Update daily usage
        const today = new Date().toISOString().split('T')[0];
        if (!this.dailyUsage[today]) {
            this.dailyUsage[today] = 0;
        }
        this.dailyUsage[today] += operationRecord.satoshis || 0;

        // Check limits
        this.checkLimits(operationRecord);

        // Write to log
        this.writeLog('OPERATION', JSON.stringify(operationRecord));

        if (this.debug) {
            console.log(`📊 Credit tracked: ${operation.type} - ${operationRecord.amount} credits (${operationRecord.satoshis} satoshis)`);
        }

        return operationRecord;
    }

    /**
     * Check usage against safety limits
     */
    checkLimits(operation) {
        const today = new Date().toISOString().split('T')[0];
        const todaysUsage = this.dailyUsage[today] || 0;

        // Daily limit check
        if (todaysUsage > this.dailyLimit) {
            this.recordWarning(`Daily limit exceeded: ${todaysUsage}/${this.dailyLimit} satoshis`, operation);
        }

        // Test suite limit check
        if (this.currentSession.totalUsage > this.testSuiteLimit) {
            this.recordWarning(`Test suite limit exceeded: ${this.currentSession.totalUsage}/${this.testSuiteLimit} satoshis`, operation);
        }

        // Operation limit check
        const operationAmount = operation.satoshis || 0;
        if (operationAmount > this.operationLimit) {
            this.recordWarning(`Operation limit exceeded: ${operationAmount}/${this.operationLimit} satoshis`, operation);
        }

        // Rate limiting check
        const recentOperations = this.currentSession.operations.filter(
            op => Date.now() - op.timestamp < 60000 // Last minute
        ).length;

        if (recentOperations > 10) {
            this.recordWarning(`High operation rate: ${recentOperations} operations in last minute`, operation);
        }
    }

    /**
     * Record a warning
     */
    recordWarning(message, operation = null) {
        const warning = {
            timestamp: Date.now(),
            sessionId: this.currentSession.sessionId,
            message,
            operation: operation || null
        };

        this.currentSession.warnings.push(warning);
        this.writeLog('WARNING', `${message} | Operation: ${JSON.stringify(operation)}`);

        if (this.debug) {
            console.warn(`⚠️ Credit Warning: ${message}`);
        }
    }

    /**
     * Record an error
     */
    recordError(message, operation = null) {
        const error = {
            timestamp: Date.now(),
            sessionId: this.currentSession.sessionId,
            message,
            operation: operation || null
        };

        this.currentSession.errors.push(error);
        this.writeLog('ERROR', `${message} | Operation: ${JSON.stringify(operation)}`);

        if (this.debug) {
            console.error(`❌ Credit Error: ${message}`);
        }
    }

    /**
     * Get current session statistics
     */
    getSessionStats() {
        const sessionDuration = Date.now() - this.currentSession.startTime;
        const today = new Date().toISOString().split('T')[0];
        const todaysUsage = this.dailyUsage[today] || 0;

        return {
            sessionId: this.currentSession.sessionId,
            duration: sessionDuration,
            totalOperations: this.currentSession.operations.length,
            totalUsage: this.currentSession.totalUsage,
            todaysUsage,
            dailyLimit: this.dailyLimit,
            dailyUsagePercentage: (todaysUsage / this.dailyLimit * 100).toFixed(1),
            warnings: this.currentSession.warnings.length,
            errors: this.currentSession.errors.length,
            averageOperationCost: this.currentSession.operations.length > 0 
                ? (this.currentSession.totalUsage / this.currentSession.operations.length).toFixed(0)
                : 0,
            network: this.network
        };
    }

    /**
     * Generate usage report
     */
    generateUsageReport() {
        const stats = this.getSessionStats();
        const report = {
            summary: stats,
            operations: this.currentSession.operations,
            warnings: this.currentSession.warnings,
            errors: this.currentSession.errors,
            dailyHistory: this.dailyUsage,
            generatedAt: new Date().toISOString()
        };

        return report;
    }

    /**
     * Export usage report to file
     */
    exportUsageReport(filename = null) {
        if (!filename) {
            const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
            filename = path.join(path.dirname(this.logFile), `usage-report-${timestamp}.json`);
        }

        const report = this.generateUsageReport();
        fs.writeFileSync(filename, JSON.stringify(report, null, 2));

        if (this.debug) {
            console.log(`📊 Usage report exported: ${filename}`);
        }

        return filename;
    }

    /**
     * Load daily usage history
     */
    loadDailyUsage() {
        const dailyUsageFile = path.join(path.dirname(this.logFile), 'daily-usage.json');
        
        try {
            if (fs.existsSync(dailyUsageFile)) {
                const content = fs.readFileSync(dailyUsageFile, 'utf8');
                return JSON.parse(content);
            }
        } catch (error) {
            console.warn(`⚠️ Failed to load daily usage: ${error.message}`);
        }

        return {};
    }

    /**
     * Save daily usage history
     */
    saveDailyUsage() {
        const dailyUsageFile = path.join(path.dirname(this.logFile), 'daily-usage.json');
        
        try {
            fs.writeFileSync(dailyUsageFile, JSON.stringify(this.dailyUsage, null, 2));
        } catch (error) {
            console.warn(`⚠️ Failed to save daily usage: ${error.message}`);
        }
    }

    /**
     * Write to log file
     */
    writeLog(level, message) {
        const timestamp = new Date().toISOString();
        const logEntry = `[${timestamp}] [${level}] [${this.currentSession.sessionId}] ${message}\n`;
        
        try {
            fs.appendFileSync(this.logFile, logEntry);
        } catch (error) {
            console.warn(`⚠️ Failed to write log: ${error.message}`);
        }
    }

    /**
     * Cleanup and finalize session
     */
    async finalize() {
        const stats = this.getSessionStats();
        
        if (this.debug) {
            console.log('📊 Final Credit Usage Summary:');
            console.log(`   Session Duration: ${(stats.duration / 1000).toFixed(1)}s`);
            console.log(`   Total Operations: ${stats.totalOperations}`);
            console.log(`   Total Usage: ${stats.totalUsage} satoshis (${(stats.totalUsage / 1e8).toFixed(6)} DASH)`);
            console.log(`   Today's Usage: ${stats.todaysUsage} satoshis (${stats.dailyUsagePercentage}% of daily limit)`);
            console.log(`   Warnings: ${stats.warnings}`);
            console.log(`   Errors: ${stats.errors}`);
        }

        // Save daily usage
        this.saveDailyUsage();

        // Write session summary to log
        this.writeLog('SESSION_END', JSON.stringify(stats));

        // Export detailed report
        const reportFile = this.exportUsageReport();
        
        return {
            stats,
            reportFile,
            success: stats.errors === 0
        };
    }

    /**
     * Emergency shutdown with full report
     */
    emergencyShutdown(reason) {
        console.error(`🚨 EMERGENCY SHUTDOWN: ${reason}`);
        
        this.recordError(`Emergency shutdown: ${reason}`);
        
        const report = this.generateUsageReport();
        const emergencyFile = path.join(
            path.dirname(this.logFile), 
            `EMERGENCY-REPORT-${Date.now()}.json`
        );
        
        fs.writeFileSync(emergencyFile, JSON.stringify(report, null, 2));
        
        console.error(`🚨 Emergency report saved: ${emergencyFile}`);
        
        throw new Error(`Emergency shutdown: ${reason}`);
    }
}

module.exports = CreditTracker;