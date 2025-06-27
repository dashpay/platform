/**
 * Example: Using SDK modules individually for smaller bundle size
 */

// Import only what you need
import { SDK } from '../src/SDK';
import { CentralizedProvider } from '../src/core/CentralizedProvider';
import { IdentityModule } from '../src/modules/identities/IdentityModule';
import { NamesModule } from '../src/modules/names/NamesModule';

async function main() {
  // Create a minimal SDK instance
  const sdk = new SDK({
    network: 'testnet',
    contextProvider: new CentralizedProvider({
      url: 'https://platform-testnet.dash.org/api'
    })
  });

  console.log('Initializing minimal SDK...');
  await sdk.initialize();

  // Create only the modules you need
  const identities = new IdentityModule(sdk);
  const names = new NamesModule(sdk);

  // Use the modules
  try {
    // Work with identities
    const identityId = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31iV';
    const identity = await identities.get(identityId);
    
    if (identity) {
      console.log(`Identity balance: ${identity.balance}`);
    } else {
      console.log('Identity not found');
    }

    // Work with names
    const name = await names.resolve('alice');
    if (name) {
      console.log(`Name owner: ${name.ownerId}`);
    } else {
      console.log('Name not found');
    }
  } catch (error: any) {
    // Handle specific error types
    if (error.name === 'NetworkError') {
      console.error('Network error: Check your connection');
    } else if (error.name === 'NotFoundError') {
      console.error('Resource not found:', error.message);
    } else if (error.name === 'InitializationError') {
      console.error('Failed to initialize SDK:', error.message);
    } else {
      console.error('Unexpected error:', error.message || error);
    }
    
    // In production, you might want to:
    // - Send errors to monitoring service
    // - Show user-friendly error messages
    // - Implement retry logic for transient errors
    process.exit(1);
  }

  // Clean up
  sdk.destroy();
}

// Tree-shaking example: Import specific types only
import type { Identity, DPNSName } from '../src';

function processIdentity(identity: Identity): void {
  console.log(`Processing identity ${identity.id}`);
}

function processName(name: DPNSName): void {
  console.log(`Processing name ${name.label}`);
}

main().catch(console.error);