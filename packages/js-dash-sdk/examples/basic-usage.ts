import { createSDK } from '../src';

async function main() {
  // Initialize SDK
  const sdk = createSDK({
    network: 'testnet'
  });

  console.log('Initializing SDK...');
  await sdk.initialize();
  console.log('SDK initialized!');

  // Example: Fetch an identity
  const identityId = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31iV'; // Example ID
  
  try {
    console.log(`\nFetching identity ${identityId}...`);
    const identity = await sdk.identities.get(identityId);
    
    if (identity) {
      console.log('Identity found:');
      console.log(`  ID: ${identity.id}`);
      console.log(`  Balance: ${identity.balance}`);
      console.log(`  Revision: ${identity.revision}`);
      console.log(`  Public Keys: ${identity.publicKeys.length}`);
    } else {
      console.log('Identity not found');
    }
  } catch (error) {
    console.error('Error fetching identity:', error);
  }

  // Example: Resolve a DPNS name
  try {
    console.log('\nResolving DPNS name "alice"...');
    const name = await sdk.names.resolve('alice');
    
    if (name) {
      console.log('Name found:');
      console.log(`  Label: ${name.label}`);
      console.log(`  Owner: ${name.ownerId}`);
      console.log(`  Records:`, name.records);
    } else {
      console.log('Name not found');
    }
  } catch (error) {
    console.error('Error resolving name:', error);
  }

  // Example: Query documents
  const dpnsContractId = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31iV'; // DPNS contract
  
  try {
    console.log('\nQuerying DPNS domains...');
    const domains = await sdk.documents.query({
      dataContractId: dpnsContractId,
      type: 'domain',
      limit: 5,
      orderBy: [['normalizedLabel', 'asc']]
    });
    
    console.log(`Found ${domains.length} domains`);
    domains.forEach((doc, i) => {
      console.log(`  ${i + 1}. ${doc.data.label}`);
    });
  } catch (error) {
    console.error('Error querying documents:', error);
  }

  // Clean up
  sdk.destroy();
  console.log('\nSDK cleaned up');
}

// Run the example
main().catch(console.error);