const Dash = require('dash');

const clone = require('lodash.clone');

const fundWallet = require('@dashevo/wallet-lib/src/utils/fundWallet');

const getDAPISeeds = require('./getDAPISeeds');

/**
 * Create and fund DashJS client
 *
 * @returns {Promise<Client>}
 */
async function createClientWithFundedWallet(amount) {
  const seeds = getDAPISeeds();

  let walletOptions = {
    waitForInstantLockTimeout: 120000,
  };

  if (process.env.SKIP_SYNC_BEFORE_HEIGHT) {
    walletOptions.unsafeOptions = {
      skipSynchronizationBeforeHeight: process.env.SKIP_SYNC_BEFORE_HEIGHT,
    };
  }

  const clientOpts = {
    seeds,
    network: process.env.NETWORK,
    wallet: walletOptions,
  };

  const faucetClient = new Dash.Client({
    ...clientOpts,
    wallet: {
      ...walletOptions,
      privateKey: process.env.FAUCET_PRIVATE_KEY,
    },
  });

  walletOptions = clone(walletOptions);

  const client = new Dash.Client({
    ...clientOpts,
    wallet: walletOptions,
  });

  await fundWallet(faucetClient.wallet, client.wallet, amount);

  await faucetClient.disconnect();

  return client;
}

module.exports = createClientWithFundedWallet;
