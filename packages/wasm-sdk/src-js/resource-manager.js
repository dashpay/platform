/**
 * Simplified Resource management for WASM SDK
 * Focuses on explicit resource tracking with guaranteed cleanup
 */

import { WasmOperationError } from './error-handler.js';

/**
 * Simplified resource manager class for handling WASM objects and memory
 * Removed complex auto-registration and wrapping features
 */
export class ResourceManager {
    /**
     * Create a resource manager
     */
    constructor() {
        this.resources = new Map();
        this.resourceCounter = 0;
        this.destroyed = false;
        
        // Bind cleanup to process events for Node.js
        if (typeof process !== 'undefined') {
            process.on('exit', () => this.destroy());
            process.on('SIGINT', () => this.destroy());
            process.on('SIGTERM', () => this.destroy());
        }
        
        // Bind cleanup to window events for browser
        if (typeof window !== 'undefined') {
            window.addEventListener('beforeunload', () => this.destroy());
            window.addEventListener('unload', () => this.destroy());
        }
    }

    /**
     * Register a WASM resource for explicit management
     * @param {*} resource - WASM resource to manage
     * @param {string} type - Resource type for debugging
     * @param {Function} cleanupFn - Custom cleanup function (optional)
     * @returns {string} Resource ID for tracking
     */
    register(resource, type = 'unknown', cleanupFn = null) {
        if (this.destroyed) {
            throw new WasmOperationError(
                'Cannot register resources on destroyed ResourceManager',
                'register_resource'
            );
        }

        const resourceId = `resource_${++this.resourceCounter}`;
        
        this.resources.set(resourceId, {
            resource,
            type,
            cleanupFn,
            createdAt: Date.now(),
            lastAccessed: Date.now()
        });
        
        return resourceId;
    }

    /**
     * Get a managed resource
     * @param {string} resourceId - Resource ID
     * @returns {*} The managed resource
     */
    get(resourceId) {
        if (this.destroyed) {
            throw new WasmOperationError(
                'Cannot access resources on destroyed ResourceManager',
                'get_resource'
            );
        }

        const resourceInfo = this.resources.get(resourceId);
        if (!resourceInfo) {
            throw new WasmOperationError(
                `Resource '${resourceId}' not found`,
                'get_resource',
                { resourceId }
            );
        }
        
        resourceInfo.lastAccessed = Date.now();
        return resourceInfo.resource;
    }

    /**
     * Release a specific resource with proper error handling
     * @param {string} resourceId - Resource ID to release
     * @returns {boolean} True if resource was successfully released
     */
    release(resourceId) {
        const resourceInfo = this.resources.get(resourceId);
        if (!resourceInfo) {
            return false;
        }
        
        let cleanupSuccess = false;
        let cleanupError = null;
        
        try {
            // Call custom cleanup function if provided
            if (resourceInfo.cleanupFn && typeof resourceInfo.cleanupFn === 'function') {
                resourceInfo.cleanupFn(resourceInfo.resource);
                cleanupSuccess = true;
            } else {
                // Try standard WASM cleanup methods in order of preference
                cleanupSuccess = this._performStandardCleanup(resourceInfo.resource);
            }
        } catch (error) {
            cleanupError = error;
            // Track cleanup failure but don't throw - still remove from tracking
        }
        
        // Always remove from tracking to prevent retry loops
        this.resources.delete(resourceId);
        
        // Log cleanup failures for monitoring
        if (!cleanupSuccess && cleanupError) {
            console.error(`Resource cleanup failed for ${resourceId}:`, cleanupError.message);
            // Could add metrics collection here
        }
        
        return cleanupSuccess;
    }

    /**
     * Release all resources of a specific type
     * @param {string} type - Resource type to release
     * @returns {Object} Cleanup results { total: number, successful: number, failed: number }
     */
    releaseByType(type) {
        const results = { total: 0, successful: 0, failed: 0 };
        
        for (const [resourceId, resourceInfo] of this.resources.entries()) {
            if (resourceInfo.type === type) {
                results.total++;
                if (this.release(resourceId)) {
                    results.successful++;
                } else {
                    results.failed++;
                }
            }
        }
        
        return results;
    }

    /**
     * Execute operation with explicit resource cleanup using try-finally
     * Simplified version without auto-registration complexity
     * @param {Function} operation - Operation to execute
     * @param {Array} managedResources - Resources to cleanup after operation
     * @returns {Promise<*>} Operation result
     */
    async executeWithCleanup(operation, managedResources = []) {
        const resources = [...managedResources]; // Copy to avoid mutation
        
        try {
            const result = await operation();
            
            // Only add to cleanup list if operation succeeded
            if (this._isWasmResource(result)) {
                const resourceId = this.register(result, 'operation_result');
                resources.push(resourceId);
            }
            
            return result;
        } finally {
            // ALWAYS cleanup registered resources, even on errors
            resources.forEach(resourceId => {
                if (typeof resourceId === 'string') {
                    this.release(resourceId);
                } else if (this._isWasmResource(resourceId)) {
                    // Direct resource cleanup
                    this._performStandardCleanup(resourceId);
                }
            });
        }
    }

    /**
     * Get basic resource statistics
     * @returns {Object} Resource statistics
     */
    getStats() {
        const stats = {
            totalResources: this.resources.size,
            byType: {},
            oldestResourceAge: 0,
            newestResourceAge: 0
        };
        
        let oldestTime = Infinity;
        let newestTime = 0;
        const now = Date.now();
        
        for (const [resourceId, resourceInfo] of this.resources.entries()) {
            // Count by type
            stats.byType[resourceInfo.type] = (stats.byType[resourceInfo.type] || 0) + 1;
            
            // Track ages
            if (resourceInfo.createdAt < oldestTime) {
                oldestTime = resourceInfo.createdAt;
                stats.oldestResourceAge = now - resourceInfo.createdAt;
            }
            
            if (resourceInfo.createdAt > newestTime) {
                newestTime = resourceInfo.createdAt;
                stats.newestResourceAge = now - resourceInfo.createdAt;
            }
        }
        
        return stats;
    }

    /**
     * Clean up stale resources based on age
     * @param {Object} options - Cleanup options
     * @param {number} options.maxAge - Maximum age in milliseconds (default: 10 minutes)
     * @returns {Object} Cleanup results
     */
    cleanup(options = {}) {
        const { maxAge = 600000 } = options; // 10 minutes default
        const now = Date.now();
        const results = { total: 0, cleaned: 0, failed: 0 };
        
        for (const [resourceId, resourceInfo] of this.resources.entries()) {
            const age = now - resourceInfo.createdAt;
            
            if (age > maxAge) {
                results.total++;
                if (this.release(resourceId)) {
                    results.cleaned++;
                } else {
                    results.failed++;
                }
            }
        }
        
        return results;
    }

    /**
     * Destroy the resource manager and clean up all resources
     */
    destroy() {
        if (this.destroyed) {
            return;
        }
        
        this.destroyed = true;
        
        // Release all managed resources
        const resourceIds = Array.from(this.resources.keys());
        const results = { total: resourceIds.length, successful: 0, failed: 0 };
        
        for (const resourceId of resourceIds) {
            if (this.release(resourceId)) {
                results.successful++;
            } else {
                results.failed++;
            }
        }
        
        this.resources.clear();
        
        if (results.total > 0) {
            console.debug(`ResourceManager destroyed: ${results.successful}/${results.total} resources cleaned successfully`);
            if (results.failed > 0) {
                console.warn(`ResourceManager: ${results.failed} resources failed to cleanup`);
            }
        }
    }

    /**
     * Check if the resource manager has been destroyed
     * @returns {boolean} True if destroyed
     */
    isDestroyed() {
        return this.destroyed;
    }

    /**
     * Get all resource IDs (for debugging)
     * @returns {string[]} Array of resource IDs
     */
    getAllResourceIds() {
        return Array.from(this.resources.keys());
    }

    /**
     * Get resources by type (for debugging)
     * @param {string} type - Resource type
     * @returns {string[]} Array of resource IDs of the specified type
     */
    getResourceIdsByType(type) {
        const resourceIds = [];
        
        for (const [resourceId, resourceInfo] of this.resources.entries()) {
            if (resourceInfo.type === type) {
                resourceIds.push(resourceId);
            }
        }
        
        return resourceIds;
    }

    // ========== Private Helper Methods ==========

    /**
     * Perform standard WASM cleanup methods
     * @private
     * @param {*} resource - Resource to cleanup
     * @returns {boolean} True if cleanup was successful
     */
    _performStandardCleanup(resource) {
        if (!resource || typeof resource !== 'object') {
            return false;
        }
        
        try {
            // Try cleanup methods in order of preference
            if (typeof resource.free === 'function') {
                resource.free();
                return true;
            } else if (typeof resource.destroy === 'function') {
                resource.destroy();
                return true;
            } else if (typeof resource.dispose === 'function') {
                resource.dispose();
                return true;
            }
            
            return false; // No cleanup method found
        } catch (error) {
            return false; // Cleanup failed
        }
    }

    /**
     * Check if an object looks like a WASM resource
     * @private
     * @param {*} obj - Object to check
     * @returns {boolean} True if it looks like a WASM resource
     */
    _isWasmResource(obj) {
        return obj && 
               typeof obj === 'object' && 
               (typeof obj.free === 'function' || 
                typeof obj.destroy === 'function' || 
                typeof obj.dispose === 'function');
    }
}

/**
 * Resource management utilities - simplified
 */
export const ResourceUtils = {
    /**
     * Check if an object looks like a WASM resource
     * @param {*} obj - Object to check
     * @returns {boolean} True if it looks like a WASM resource
     */
    isWasmResource: (obj) => {
        return obj && 
               typeof obj === 'object' && 
               (typeof obj.free === 'function' || 
                typeof obj.destroy === 'function' || 
                typeof obj.dispose === 'function');
    },

    /**
     * Create a cleanup function for common WASM objects
     * @param {string} method - Cleanup method name ('free', 'destroy', 'dispose')
     * @returns {Function} Cleanup function
     */
    createCleanupFunction: (method = 'free') => {
        return (resource) => {
            if (resource && typeof resource[method] === 'function') {
                try {
                    resource[method]();
                    return true;
                } catch (error) {
                    console.warn(`Cleanup method ${method} failed:`, error.message);
                    return false;
                }
            }
            return false;
        };
    },

    /**
     * Execute operation with guaranteed cleanup
     * @param {Function} operation - Operation to execute
     * @param {Array} resources - Resources to cleanup
     * @returns {Promise<*>} Operation result
     */
    withCleanup: async (operation, resources = []) => {
        try {
            return await operation();
        } finally {
            resources.forEach(resource => {
                if (ResourceUtils.isWasmResource(resource)) {
                    try {
                        if (resource.free) resource.free();
                        else if (resource.destroy) resource.destroy();
                        else if (resource.dispose) resource.dispose();
                    } catch (error) {
                        console.warn('Resource cleanup failed:', error);
                    }
                }
            });
        }
    }
};

/**
 * Add a compatibility method to ResourceManager for existing API usage
 */
ResourceManager.prototype.wrapOperation = function(operation, operationName, options = {}) {
    const { autoRegister = false } = options;
    
    return async (...args) => {
        const resources = [];
        
        try {
            const result = await operation(...args);
            
            // Auto-register result if requested and it's a WASM resource
            if (autoRegister && this._isWasmResource(result)) {
                const resourceId = this.register(result, operationName);
                resources.push(resourceId);
            }
            
            return result;
        } catch (error) {
            // Clean up any resources that were registered during the failed operation
            resources.forEach(resourceId => this.release(resourceId));
            throw error;
        }
    };
};