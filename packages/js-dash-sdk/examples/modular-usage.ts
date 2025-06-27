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
    }

    // Work with names
    const name = await names.resolve('alice');
    if (name) {
      console.log(`Name owner: ${name.ownerId}`);
    }
  } catch (error) {
    console.error('Error:', error);
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