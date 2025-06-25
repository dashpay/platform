// Example: Using ES modules with wasm-drive-verify

// Option 1: Import only what you need (best for bundle size)
import { verifyFullIdentityByIdentityId } from 'wasm-drive-verify/identity';

async function verifyIdentity() {
  const proof = new Uint8Array([/* ... */]);
  const identityId = new Uint8Array([/* ... */]);
  const platformVersion = 1;
  
  const result = await verifyFullIdentityByIdentityId(proof, identityId, platformVersion);
  console.log('Identity verification result:', result);
}

// Option 2: Dynamic imports for code splitting
async function verifyDocumentOnDemand() {
  // This module will only be loaded when this function is called
  const { verifyProof } = await import('wasm-drive-verify/document');
  
  const proof = new Uint8Array([/* ... */]);
  const contractId = new Uint8Array([/* ... */]);
  const documentTypeName = 'myDocument';
  const query = { /* ... */ };
  const platformVersion = 1;
  
  const result = await verifyProof(proof, contractId, documentTypeName, query, platformVersion);
  console.log('Document verification result:', result);
}

// Option 3: Import multiple functions from a module
import { 
  verifyTokenBalanceForIdentityId,
  verifyTokenInfoForIdentityId 
} from 'wasm-drive-verify/tokens';

async function checkTokenInfo() {
  const proof = new Uint8Array([/* ... */]);
  const contractId = new Uint8Array([/* ... */]);
  const identityId = new Uint8Array([/* ... */]);
  const platformVersion = 1;
  
  const [balance, info] = await Promise.all([
    verifyTokenBalanceForIdentityId(proof, contractId, identityId, platformVersion),
    verifyTokenInfoForIdentityId(proof, contractId, identityId, platformVersion)
  ]);
  
  console.log('Token balance:', balance);
  console.log('Token info:', info);
}

// Example: Bundle size comparison
// Old way (imports everything):
// import * as wasmDriveVerify from 'wasm-drive-verify';
// Bundle size: ~2.5MB

// New way (imports only identity module):
// import { verifyFullIdentityByIdentityId } from 'wasm-drive-verify/identity';
// Bundle size: ~400KB (84% reduction!)

// Usage in different bundlers:

// Webpack configuration
export const webpackConfig = {
  experiments: {
    asyncWebAssembly: true,
  },
  module: {
    rules: [
      {
        test: /\.wasm$/,
        type: 'webassembly/async',
      },
    ],
  },
};

// Vite configuration
export const viteConfig = {
  optimizeDeps: {
    exclude: ['wasm-drive-verify'],
  },
};

// Next.js configuration
export const nextConfig = {
  webpack: (config) => {
    config.experiments = {
      ...config.experiments,
      asyncWebAssembly: true,
    };
    return config;
  },
};