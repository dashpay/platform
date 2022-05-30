const Dash = require('dash');

let storageAdapter;

if (typeof window === 'undefined') {
  // eslint-disable-next-line global-require
  const { NodeForage } = require('nodeforage');
  storageAdapter = new NodeForage({ name: `../../db/faucet-wallet-${process.env.FAUCET_ADDRESS}` });
} else {
  // eslint-disable-next-line global-require
  storageAdapter = require('localforage');
}

const { contractId } = require('@dashevo/dpns-contract/lib/systemIds');

const getDAPISeeds = require('./getDAPISeeds');

let faucetClient;

function createFaucetClient() {
  const seeds = getDAPISeeds();

  const clientOpts = {
    seeds,
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

  if (process.env.FAUCET_WALLET_USE_STORAGE === "true") {
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

module.exports = createFaucetClient;
