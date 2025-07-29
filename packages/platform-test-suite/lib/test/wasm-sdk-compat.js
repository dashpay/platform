/**
 * Compatibility layer for tests that use platform.dpp directly
 * This allows tests to work with both legacy DPP and new wasm-sdk delegation
 */

/**
 * Create a DPP compatibility shim for wasm-sdk
 * @param {Platform} platform - Platform instance
 * @returns {Object} DPP-like interface
 */
function createDPPCompat(platform) {
  // If legacy DPP is available, use it
  if (platform.dpp) {
    return platform.dpp;
  }

  // Otherwise, create a compatibility layer
  return {
    identity: {
      createInstantAssetLockProof: (instantLock, transaction, outputIndex) => {
        // Create a mock instant asset lock proof that matches DPP format
        return {
          type: 0, // Instant lock type
          instantLock: Buffer.isBuffer(instantLock) ? instantLock : Buffer.from(instantLock),
          transaction: Buffer.isBuffer(transaction) ? transaction : Buffer.from(transaction),
          outputIndex: outputIndex
        };
      },
      
      createChainAssetLockProof: (coreChainLockedHeight, outPoint) => {
        // Create a mock chain asset lock proof that matches DPP format
        return {
          type: 1, // Chain lock type
          coreChainLockedHeight: coreChainLockedHeight,
          outPoint: Buffer.isBuffer(outPoint) ? outPoint : Buffer.from(outPoint)
        };
      },
      
      createIdentityTopUpTransition: async (assetLockProof, outputPrivateKey, identity) => {
        return platform.identities.utils.createIdentityTopUpTransition(
          assetLockProof,
          outputPrivateKey,
          identity
        );
      },
      
      createFromBuffer: async (buffer) => {
        // This would need to be implemented based on wasm-sdk's response format
        throw new Error('createFromBuffer not yet implemented for wasm-sdk compatibility');
      }
    },
    
    dataContract: {
      createFromBuffer: async (buffer) => {
        // This would need to be implemented based on wasm-sdk's response format
        throw new Error('createFromBuffer not yet implemented for wasm-sdk compatibility');
      }
    },
    
    document: {
      createFromBuffer: async (buffer) => {
        // This would need to be implemented based on wasm-sdk's response format
        throw new Error('createFromBuffer not yet implemented for wasm-sdk compatibility');
      }
    }
  };
}

/**
 * Patch client to add DPP compatibility
 * @param {Client} client - Dash client instance
 * @returns {Client} Patched client
 */
function patchClientForTests(client) {
  // Add DPP compatibility if not already present
  if (!client.platform.dpp && client.platform.wasmSdk) {
    Object.defineProperty(client.platform, 'dpp', {
      get() {
        return createDPPCompat(this);
      },
      configurable: true
    });
  }
  
  return client;
}

module.exports = {
  createDPPCompat,
  patchClientForTests
};