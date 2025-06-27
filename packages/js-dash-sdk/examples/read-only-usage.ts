/**
 * Example: Read-only usage of Dash SDK for websites
 * 
 * This demonstrates how to use the SDK for pulling and displaying data
 * without any write operations or wallet requirements.
 */

import { createSDK } from '../src';

async function readOnlyExample() {
  console.log('Dash SDK Read-Only Example');
  console.log('==========================\n');

  // 1. Initialize SDK without wallet
  const sdk = createSDK({
    network: 'testnet',
    // No wallet configuration needed for read-only operations
  });

  await sdk.initialize();
  console.log('✓ SDK initialized for read-only access\n');

  // 2. Fetch Platform State
  console.log('Platform State:');
  const provider = sdk.getContextProvider();
  
  const [blockHeight, blockTime, coreHeight, version] = await Promise.all([
    provider.getLatestPlatformBlockHeight(),
    provider.getLatestPlatformBlockTime(),
    provider.getLatestPlatformCoreChainLockedHeight(),
    provider.getLatestPlatformVersion()
  ]);

  console.log(`  Block Height: ${blockHeight}`);
  console.log(`  Block Time: ${new Date(blockTime).toISOString()}`);
  console.log(`  Core Chain Height: ${coreHeight}`);
  console.log(`  Platform Version: ${version}\n`);

  // 3. Look up an Identity (read-only)
  console.log('Identity Lookup:');
  try {
    // Example identity ID (replace with a real one)
    const identityId = 'GWRvAjq5wuz8mNyB8NwS2cjBi7CmRiPPZmtFnPuTUzNp';
    const identity = await sdk.identities.get(identityId);
    
    if (identity) {
      console.log(`  ID: ${identity.id}`);
      console.log(`  Balance: ${identity.balance} credits`);
      console.log(`  Public Keys: ${identity.publicKeys.length}`);
      console.log(`  Revision: ${identity.revision}\n`);
    }
  } catch (error) {
    console.log('  (Example identity not found)\n');
  }

  // 4. Resolve DPNS Names (read-only)
  console.log('DPNS Name Resolution:');
  try {
    const name = await sdk.names.resolve('alice');
    if (name) {
      console.log(`  Name: ${name.label}`);
      console.log(`  Owner: ${name.ownerId}`);
      console.log(`  Linked Identity: ${name.records?.dashUniqueIdentityId || 'Not set'}\n`);
    }
  } catch (error) {
    console.log('  (Name not found)\n');
  }

  // 5. Search for DPNS Names
  console.log('DPNS Name Search:');
  const searchResults = await sdk.names.search('a', { limit: 5 });
  console.log(`  Found ${searchResults.length} names starting with 'a'`);
  searchResults.forEach(name => {
    console.log(`    - ${name.label}.${name.normalizedParentDomainName}`);
  });
  console.log();

  // 6. Fetch Data Contract (read-only)
  console.log('Data Contract Fetch:');
  const dpnsContractId = sdk.names.getDPNSContractId();
  const contract = await sdk.contracts.get(dpnsContractId);
  
  if (contract) {
    console.log(`  Contract ID: ${contract.id}`);
    console.log(`  Owner: ${contract.ownerId}`);
    console.log(`  Version: ${contract.version}`);
    console.log(`  Document Types: ${Object.keys(contract.documents || {}).join(', ')}\n`);
  }

  // 7. Query Documents (read-only)
  console.log('Document Query:');
  const documents = await sdk.documents.query({
    dataContractId: dpnsContractId,
    type: 'domain',
    where: [
      ['normalizedParentDomainName', '==', 'dash']
    ],
    orderBy: [['normalizedLabel', 'asc']],
    limit: 10
  });

  console.log(`  Found ${documents.length} domain documents`);
  documents.slice(0, 3).forEach(doc => {
    console.log(`    - ${doc.data.normalizedLabel}.${doc.data.normalizedParentDomainName}`);
  });
  if (documents.length > 3) {
    console.log(`    ... and ${documents.length - 3} more`);
  }
  console.log();

  // 8. Get Contract History (read-only)
  console.log('Contract History:');
  const history = await sdk.contracts.getHistory(dpnsContractId);
  console.log(`  Contract has ${history.length} versions`);
  history.slice(0, 3).forEach((version, index) => {
    console.log(`    Version ${index + 1}: ${version.version}`);
  });
  console.log();

  // 9. Advanced Document Queries
  console.log('Advanced Queries:');
  
  // Query with multiple conditions
  const complexQuery = await sdk.documents.query({
    dataContractId: dpnsContractId,
    type: 'domain',
    where: [
      ['normalizedParentDomainName', '==', 'dash'],
      ['records.dashUniqueIdentityId', 'exists']
    ],
    limit: 5
  });
  
  console.log(`  Domains with linked identities: ${complexQuery.length}`);

  // 10. Using the Web Service Provider for Quorum Keys
  if (provider.getCapabilities?.().includes('quorum_keys')) {
    console.log('\nQuorum Information:');
    try {
      const quorumKeys = await provider.getQuorumKeys();
      console.log(`  Active Quorums: ${quorumKeys.size}`);
      
      // Display first few quorums
      let count = 0;
      for (const [hash, info] of quorumKeys) {
        if (count++ >= 3) break;
        console.log(`    - ${hash.substring(0, 16)}...`);
      }
      if (quorumKeys.size > 3) {
        console.log(`    ... and ${quorumKeys.size - 3} more`);
      }
    } catch (error) {
      console.log('  (Quorum service not available)');
    }
  }
}

// Example: Building a simple data browser
async function dataBrowserExample() {
  console.log('\n\nData Browser Example');
  console.log('====================\n');

  const sdk = createSDK({ network: 'testnet' });
  await sdk.initialize();

  // Function to browse any data contract
  async function browseContract(contractId: string, documentType?: string) {
    const contract = await sdk.contracts.get(contractId);
    if (!contract) {
      console.log('Contract not found');
      return;
    }

    console.log(`Contract: ${contract.id}`);
    console.log(`Document Types: ${Object.keys(contract.documents || {}).join(', ')}`);

    if (documentType && contract.documents?.[documentType]) {
      // Query specific document type
      const docs = await sdk.documents.query({
        dataContractId: contractId,
        type: documentType,
        limit: 5
      });

      console.log(`\nFound ${docs.length} ${documentType} documents:`);
      docs.forEach((doc, i) => {
        console.log(`\n  Document ${i + 1}:`);
        console.log(`    ID: ${doc.id}`);
        console.log(`    Owner: ${doc.ownerId}`);
        console.log(`    Data: ${JSON.stringify(doc.data, null, 2)}`);
      });
    }
  }

  // Browse DPNS contract
  await browseContract(sdk.names.getDPNSContractId(), 'domain');
}

// Example: Building a monitoring dashboard
async function monitoringDashboard() {
  console.log('\n\nMonitoring Dashboard Example');
  console.log('============================\n');

  const sdk = createSDK({ network: 'testnet' });
  await sdk.initialize();

  // Function to get platform statistics
  async function getPlatformStats() {
    const provider = sdk.getContextProvider();
    
    const stats = {
      blockHeight: await provider.getLatestPlatformBlockHeight(),
      blockTime: await provider.getLatestPlatformBlockTime(),
      coreHeight: await provider.getLatestPlatformCoreChainLockedHeight(),
      version: await provider.getLatestPlatformVersion()
    };

    // Calculate block production rate
    const timePerBlock = await provider.getTimePerBlockMillis();
    const blocksPerHour = Math.round(3600000 / timePerBlock);

    return {
      ...stats,
      blocksPerHour,
      lastBlockAge: Date.now() - stats.blockTime
    };
  }

  // Display stats
  const stats = await getPlatformStats();
  console.log('Platform Statistics:');
  console.log(`  Current Height: ${stats.blockHeight}`);
  console.log(`  Blocks/Hour: ${stats.blocksPerHour}`);
  console.log(`  Last Block: ${Math.round(stats.lastBlockAge / 1000)}s ago`);
  console.log(`  Platform Version: ${stats.version}`);

  // Monitor for a few seconds (in real app, this would be continuous)
  console.log('\nMonitoring block production...');
  let lastHeight = stats.blockHeight;
  
  for (let i = 0; i < 3; i++) {
    await new Promise(resolve => setTimeout(resolve, 2000));
    const currentHeight = await sdk.getContextProvider().getLatestPlatformBlockHeight();
    
    if (currentHeight > lastHeight) {
      console.log(`  New block! Height: ${currentHeight} (+${currentHeight - lastHeight})`);
      lastHeight = currentHeight;
    } else {
      console.log('  Waiting for new block...');
    }
  }
}

// Run examples
async function main() {
  try {
    await readOnlyExample();
    await dataBrowserExample();
    await monitoringDashboard();
  } catch (error) {
    console.error('Example error:', error);
  }
}

main().catch(console.error);