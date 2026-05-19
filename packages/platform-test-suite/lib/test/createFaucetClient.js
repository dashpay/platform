import Dash from 'dash';
import dpnsSystemIds from '@dashevo/dpns-contract/lib/systemIds.js';
const { contractId } = dpnsSystemIds;

import getDAPISeeds from './getDAPISeeds.js';

let storageAdapter;

if (typeof window === 'undefined') {
  const { NodeForage } = await import('nodeforage');
  storageAdapter = new NodeForage({
    dir: process.env.FAUCET_WALLET_STORAGE_DIR || process.cwd(),
    name: `faucet-wallet-${process.env.FAUCET_ADDRESS}`,
  });
} else {
  storageAdapter = (await import('localforage')).default;
}

let faucetClient;

function createFaucetClient() {
  const dapiAddresses = (process.env.DAPI_ADDRESSES || '')
    .split(',')
    .map((address) => address.trim())
    .filter(Boolean);

  const clientOpts = {
    ...(dapiAddresses.length > 0
      ? { dapiAddresses }
      : { seeds: getDAPISeeds() }),
    network: process.env.NETWORK,
    apps: {
      dpns: {
        contractId,
      },
    },
  };

  const walletOptions = {
    privateKey: process.env.FAUCET_PRIVATE_KEY,
  };

  if (process.env.FAUCET_WALLET_USE_STORAGE === 'true') {
    walletOptions.adapter = storageAdapter;
  }

  if (process.env.SKIP_SYNC_BEFORE_HEIGHT) {
    walletOptions.unsafeOptions = {
      skipSynchronizationBeforeHeight: process.env.SKIP_SYNC_BEFORE_HEIGHT,
    };
  }

  faucetClient = new Dash.Client({
    ...clientOpts,
    wallet: walletOptions,
  });

  return faucetClient;
}

export default createFaucetClient;
