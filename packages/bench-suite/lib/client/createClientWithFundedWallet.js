const Dash = require('dash');

const clone = require('lodash.clone');

const fundWallet = require('@dashevo/wallet-lib/src/utils/fundWallet');

/**
 * Create and fund DashJS client
 *
 * @param {number} amount
 * @param {Object} config
 * @param {{host: string, httpPort: string, grpcPort: string}[]} config.seeds
 * @param {string} config.network
 * @param {string} config.faucetPrivateKey
 * @param {number} [config.skipSyncBeforeHeight]
 *
 * @returns {Promise<Client>}
 */
async function createClientWithFundedWallet(amount, config) {
  let walletOptions = {
    waitForInstantLockTimeout: 120000,
  };

  if (config.skipSyncBeforeHeight) {
    walletOptions.unsafeOptions = {
      skipSynchronizationBeforeHeight: config.skipSyncBeforeHeight,
    };
  }

  const clientOpts = {
    seeds: config.seeds,
    network: config.network,
    wallet: walletOptions,
  };

  const faucetClient = new Dash.Client({
    ...clientOpts,
    wallet: {
      ...walletOptions,
      privateKey: config.faucetPrivateKey,
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
