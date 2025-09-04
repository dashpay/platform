/**
 * Resource management for WASM SDK
 * Handles WASM memory management and resource cleanup
 */

import { WasmOperationError, ErrorUtils } from './error-handler.js';

/**
 * Resource manager class for handling WASM objects and memory
 */
export class ResourceManager {
    /**
     * Create a resource manager
     */
    constructor() {
        this.resources = new Map();
        this.resourceCounter = 0;
        this.destroyed = false;
        
        // Bind cleanup to process events
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
     * Register a WASM resource for management
     * @param {*} resource - WASM resource to manage
     * @param {string} type - Resource type for debugging
     * @param {Function} cleanupFn - Custom cleanup function
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
     * Release a specific resource
     * @param {string} resourceId - Resource ID to release
     * @returns {boolean} True if resource was released
     */
    release(resourceId) {
        const resourceInfo = this.resources.get(resourceId);
        if (!resourceInfo) {
            return false;
        }
        
        try {
            // Call custom cleanup function if provided
            if (resourceInfo.cleanupFn && typeof resourceInfo.cleanupFn === 'function') {
                resourceInfo.cleanupFn(resourceInfo.resource);
            }
            
            // Call standard WASM cleanup methods if available
            if (resourceInfo.resource && typeof resourceInfo.resource.free === 'function') {
                resourceInfo.resource.free();
            } else if (resourceInfo.resource && typeof resourceInfo.resource.destroy === 'function') {
                resourceInfo.resource.destroy();
            } else if (resourceInfo.resource && typeof resourceInfo.resource.dispose === 'function') {
                resourceInfo.resource.dispose();
            }
        } catch (error) {
            // Log cleanup errors but don't throw - we still want to remove the resource
            console.warn(`Error cleaning up resource ${resourceId}:`, error);
        }
        
        this.resources.delete(resourceId);
        return true;
    }

    /**
     * Release all resources of a specific type
     * @param {string} type - Resource type to release
     * @returns {number} Number of resources released
     */
    releaseByType(type) {
        let released = 0;
        
        for (const [resourceId, resourceInfo] of this.resources.entries()) {
            if (resourceInfo.type === type) {
                if (this.release(resourceId)) {
                    released++;
                }
            }
        }
        
        return released;
    }

    /**
     * Get resource information
     * @param {string} resourceId - Resource ID
     * @returns {Object|null} Resource information or null if not found
     */
    getResourceInfo(resourceId) {
        const resourceInfo = this.resources.get(resourceId);
        if (!resourceInfo) {
            return null;
        }
        
        return {
            id: resourceId,
            type: resourceInfo.type,
            createdAt: resourceInfo.createdAt,
            lastAccessed: resourceInfo.lastAccessed,
            age: Date.now() - resourceInfo.createdAt,
            timeSinceAccess: Date.now() - resourceInfo.lastAccessed
        };
    }

    /**
     * Get statistics about managed resources
     * @returns {Object} Resource statistics
     */
    getStats() {
        const stats = {
            totalResources: this.resources.size,
            byType: {},
            oldestResource: null,
            newestResource: null,
            totalAge: 0
        };
        
        let oldestTime = Infinity;
        let newestTime = 0;
        
        for (const [resourceId, resourceInfo] of this.resources.entries()) {
            // Count by type
            stats.byType[resourceInfo.type] = (stats.byType[resourceInfo.type] || 0) + 1;
            
            // Track oldest and newest
            if (resourceInfo.createdAt < oldestTime) {
                oldestTime = resourceInfo.createdAt;
                stats.oldestResource = {
                    id: resourceId,
                    type: resourceInfo.type,
                    createdAt: resourceInfo.createdAt
                };
            }
            
            if (resourceInfo.createdAt > newestTime) {
                newestTime = resourceInfo.createdAt;
                stats.newestResource = {
                    id: resourceId,
                    type: resourceInfo.type,
                    createdAt: resourceInfo.createdAt
                };
            }
            
            stats.totalAge += Date.now() - resourceInfo.createdAt;
        }
        
        stats.averageAge = stats.totalResources > 0 ? stats.totalAge / stats.totalResources : 0;
        
        return stats;
    }

    /**
     * Clean up stale resources based on age or access time
     * @param {Object} options - Cleanup options
     * @param {number} options.maxAge - Maximum age in milliseconds
     * @param {number} options.maxIdleTime - Maximum idle time in milliseconds
     * @returns {number} Number of resources cleaned up
     */
    cleanup(options = {}) {
        const { maxAge = 300000, maxIdleTime = 60000 } = options; // 5 minutes max age, 1 minute max idle
        let cleanedUp = 0;
        const now = Date.now();
        
        for (const [resourceId, resourceInfo] of this.resources.entries()) {
            const age = now - resourceInfo.createdAt;
            const idleTime = now - resourceInfo.lastAccessed;
            
            if (age > maxAge || idleTime > maxIdleTime) {
                if (this.release(resourceId)) {
                    cleanedUp++;
                }
            }
        }
        
        return cleanedUp;
    }

    /**
     * Wrap a WASM operation to automatically manage its resources
     * @param {Function} operation - WASM operation to wrap
     * @param {string} operationType - Operation type for error context
     * @param {Object} options - Wrapping options
     * @returns {Function} Wrapped operation
     */
    wrapOperation(operation, operationType, options = {}) {
        const { autoRegister = true, cleanup = true } = options;
        
        return async (...args) => {
            const resourceIds = [];
            
            try {
                const result = await operation(...args);
                
                // Auto-register result if it looks like a WASM resource
                if (autoRegister && result && typeof result === 'object') {
                    if (typeof result.free === 'function' || 
                        typeof result.destroy === 'function' || 
                        typeof result.dispose === 'function') {
                        const resourceId = this.register(result, operationType);
                        resourceIds.push(resourceId);
                    }
                }
                
                return result;
            } catch (error) {
                // Clean up any resources that were created during the failed operation
                if (cleanup) {
                    resourceIds.forEach(id => this.release(id));
                }
                
                throw ErrorUtils.createAsyncErrorHandler(operationType)(error);
            }
        };
    }

    /**
     * Create a managed promise that cleans up resources on completion
     * @param {Promise} promise - Promise to manage
     * @param {string[]} resourceIds - Resource IDs to clean up
     * @returns {Promise} Managed promise
     */
    managedPromise(promise, resourceIds = []) {
        return promise.finally(() => {
            resourceIds.forEach(id => this.release(id));
        });
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
        let cleanedUp = 0;
        
        for (const resourceId of resourceIds) {
            if (this.release(resourceId)) {
                cleanedUp++;
            }
        }
        
        this.resources.clear();
        
        if (cleanedUp > 0) {
            console.debug(`ResourceManager destroyed, cleaned up ${cleanedUp} resources`);
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
     * Get all resource IDs
     * @returns {string[]} Array of resource IDs
     */
    getAllResourceIds() {
        return Array.from(this.resources.keys());
    }

    /**
     * Get resources by type
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
}

/**
 * Global resource manager instance
 * Can be used when a single global instance is sufficient
 */
export const globalResourceManager = new ResourceManager();

/**
 * Resource management utilities
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
                resource[method]();
            }
        };
    },

    /**
     * Auto-detect and create appropriate cleanup function
     * @param {*} resource - Resource to create cleanup for
     * @returns {Function|null} Cleanup function or null if none needed
     */
    detectCleanupFunction: (resource) => {
        if (!ResourceUtils.isWasmResource(resource)) {
            return null;
        }
        
        if (typeof resource.free === 'function') {
            return ResourceUtils.createCleanupFunction('free');
        } else if (typeof resource.destroy === 'function') {
            return ResourceUtils.createCleanupFunction('destroy');
        } else if (typeof resource.dispose === 'function') {
            return ResourceUtils.createCleanupFunction('dispose');
        }
        
        return null;
    }
};