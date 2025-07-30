#!/usr/bin/env node

/**
 * Fetches real proof data from Dash testnet for integration testing
 * This script collects various types of proofs and saves them for use in tests
 */

const fs = require('fs').promises;
const path = require('path');

// Import Dash SDK
const Dash = require('dash');

// Known testnet contract IDs (from system contracts)
const TESTNET_CONTRACTS = {
  dpns: '7133734967411265855288437346261134676850487612170005227449438774554101671041',
  dashpay: '11820826580861527503515256915869415134572226289567404439933090029265983217778',
  withdrawals: 'ARaM5u3iibTGq2qVcB3sfWBwUq51zV39MVWKw6Enk5QD',
  featureFlags: 'G7c8S5JDw5FkJEGGeGKCMoHwGbvCNrQrtdbnMirALNV2',
  masternodeRewardShares: 'GfbnFqmNZ8ZJG2ZJDhAj8P7KRbYvqwUmgRhhgJ7bDfwJ'
};

async function main() {
  console.log('Fetching testnet proofs...');

  // Initialize client
  const client = new Dash.Client({
    network: 'testnet',
    wallet: {
      mnemonic: null // We don't need a wallet for queries
    }
  });

  const proofData = {
    timestamp: new Date().toISOString(),
    network: 'testnet',
    proofs: {}
  };

  try {
    // Fetch some known identities
    console.log('Fetching identity proofs...');
    
    // Try to get an identity by a known testnet identity ID
    try {
      const identityId = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec'; // Example testnet identity
      const identity = await client.platform.identities.get(identityId);
      
      if (identity) {
        proofData.proofs.identity = {
          description: 'Full identity by ID',
          identityId: identityId,
          proof: identity.getMetadata().getProof(),
          identity: identity.toJSON()
        };
      }
    } catch (e) {
      console.log('Could not fetch identity:', e.message);
    }

    // Fetch DPNS document proofs
    console.log('Fetching DPNS document proofs...');
    try {
      const dpnsDocuments = await client.platform.documents.get(
        'dpns.domain',
        {
          limit: 2,
        }
      );

      if (dpnsDocuments.length > 0) {
        proofData.proofs.dpnsDocuments = {
          description: 'DPNS domain documents',
          contractId: TESTNET_CONTRACTS.dpns,
          documentType: 'domain',
          proof: dpnsDocuments[0].getMetadata().getProof(),
          documents: dpnsDocuments.map(doc => doc.toJSON())
        };
      }
    } catch (e) {
      console.log('Could not fetch DPNS documents:', e.message);
    }

    // Fetch data contract
    console.log('Fetching data contract proofs...');
    try {
      const contract = await client.platform.contracts.get(TESTNET_CONTRACTS.dpns);
      
      if (contract) {
        proofData.proofs.dataContract = {
          description: 'DPNS data contract',
          contractId: TESTNET_CONTRACTS.dpns,
          proof: contract.getMetadata().getProof(),
          contract: contract.toJSON()
        };
      }
    } catch (e) {
      console.log('Could not fetch contract:', e.message);
    }

    // Write collected proofs to file
    const outputDir = path.join(__dirname, 'fixtures');
    await fs.mkdir(outputDir, { recursive: true });
    
    const outputFile = path.join(outputDir, 'testnet_proofs.json');
    await fs.writeFile(outputFile, JSON.stringify(proofData, null, 2));
    
    console.log(`Proofs saved to: ${outputFile}`);
    console.log(`Total proofs collected: ${Object.keys(proofData.proofs).length}`);

  } catch (error) {
    console.error('Error fetching proofs:', error);
  } finally {
    await client.disconnect();
  }
}

main().catch(console.error);