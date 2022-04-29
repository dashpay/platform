const Dash = require('dash');

const fundWallet = require('@dashevo/wallet-lib/src/utils/fundWallet');

const {
  contractId: dpnsContractId,
} = require('@dashevo/dpns-contract/lib/systemIds');

const getDAPISeeds = require('./getDAPISeeds');

const createFaucetClient = require('./createFaucetClient');

let faucetClient;

/**
 * Create and fund DashJS client
 * @param {string} [HDPrivateKey]
 * @param {number} [amount] - amount of Duffs to fund wallet with
 * @returns {Promise<Client>}
 */
async function createClientWithFundedWallet(HDPrivateKey = undefined, amount = 40000) {
  const useFaucetWalletStorage = process.env.FAUCET_WALLET_USE_STORAGE === "true"
  const seeds = getDAPISeeds();

  const clientOpts = {
    seeds,
    network: process.env.NETWORK,
    apps: {
      dpns: {
        contractId: dpnsContractId,
      },
    },
  };

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
    await faucetClient.wallet.disconnect();
  }

  return client;
}

module.exports = createClientWithFundedWallet;
