const Dash = require('dash');

const fundWallet = require('@dashevo/wallet-lib/src/utils/fundWallet');

const {
  contractId: dpnsContractId,
} = require('@dashevo/dpns-contract/lib/systemIds');

const getDAPISeeds = require('./getDAPISeeds');

const createFaucetClient = require('./createFaucetClient');

/**
 * Create and fund DashJS client
 * @param {string} [HDPrivateKey]
 *
 * @returns {Promise<Client>}
 */
async function createClientWithFundedWallet(HDPrivateKey = undefined) {
  const seeds = getDAPISeeds();

  const clientOpts = {
    // seeds,
    dapiAddresses: [
      '34.210.237.116',
      '54.69.65.231',
      // '54.185.90.95',
      // '54.186.234.0',
      // '35.87.212.139',
      // '34.212.52.44',
      '34.217.47.197',
      '34.220.79.131',
      '18.237.212.176',
      '54.188.17.188',
      '34.210.1.159',
    ],
    network: process.env.NETWORK,
    apps: {
      dpns: {
        contractId: dpnsContractId,
      },
    },
  };

  const faucetClient = createFaucetClient();

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

  const amount = 40000;

  await fundWallet(faucetClient.wallet, client.wallet, amount);

  return client;
}

module.exports = createClientWithFundedWallet;
