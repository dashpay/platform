import Dash from 'dash';
import dpnsSystemIds from '@dashevo/dpns-contract/lib/systemIds.js';
const { contractId } = dpnsSystemIds;

import getDAPISeeds from './getDAPISeeds.js';

function createClientWithoutWallet() {
  const dapiAddresses = (process.env.DAPI_ADDRESSES || '')
    .split(',')
    .map((address) => address.trim())
    .filter(Boolean);

  return new Dash.Client({
    ...(dapiAddresses.length > 0
      ? { dapiAddresses }
      : { seeds: getDAPISeeds() }),
    network: process.env.NETWORK,
    timeout: 25000,
    apps: {
      dpns: {
        contractId,
      },
    },
  });
}

export default createClientWithoutWallet;
