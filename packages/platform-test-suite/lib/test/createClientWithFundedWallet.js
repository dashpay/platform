import Dash from 'dash';

import fundWallet from '@dashevo/wallet-lib/src/utils/fundWallet.js';

import dpnsSystemIds from '@dashevo/dpns-contract/lib/systemIds.js';
const { contractId: dpnsContractId } = dpnsSystemIds;

import getDAPISeeds from './getDAPISeeds.js';

import createFaucetClient from './createFaucetClient.js';

let faucetClient;

/**
 * Create and fund DashJS client
 * @param {number} amount - amount of Duffs to fund wallet with
 * @param {string} [HDPrivateKey]
 * @returns {Promise<Client>}
 */
async function createClientWithFundedWallet(amount, HDPrivateKey = undefined) {
  const useFaucetWalletStorage = process.env.FAUCET_WALLET_USE_STORAGE === 'true';

  const dapiAddresses = (process.env.DAPI_ADDRESSES || '')
    .split(',')
    .map((address) => address.trim())
    .filter(Boolean);

  const clientOpts = {
    network: process.env.NETWORK,
    timeout: 25000,
    apps: {
      dpns: {
        contractId: dpnsContractId,
      },
    },
  };

  if (dapiAddresses.length > 0) {
    clientOpts.dapiAddresses = dapiAddresses;
  } else {
    clientOpts.seeds = getDAPISeeds();
  }

  if (!faucetClient || (faucetClient && useFaucetWalletStorage)) {
    faucetClient = createFaucetClient();
  }

  const walletOptions = {
    waitForInstantLockTimeout: 120000,
  };

  if (process.env.SKIP_SYNC_BEFORE_HEIGHT && HDPrivateKey) {
    walletOptions.unsafeOptions = {
      skipSynchronizationBeforeHeight: process.env.SKIP_SYNC_BEFORE_HEIGHT,
    };
  }

  if (HDPrivateKey) {
    walletOptions.HDPrivateKey = HDPrivateKey;
  } else {
    walletOptions.mnemonic = null;
  }

  const client = new Dash.Client({
    ...clientOpts,
    wallet: walletOptions,
  });

  await fundWallet(faucetClient.wallet, client.wallet, amount);

  if (useFaucetWalletStorage) {
    await faucetClient.disconnect();
  }

  return client;
}

export default createClientWithFundedWallet;
