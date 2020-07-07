const {
  PrivateKey,
} = require('@dashevo/dashcore-lib');

const Dash = require('dash');

const getDAPISeeds = require('./getDAPISeeds');

const fundAddress = require('./fundAddress');
const wait = require('../wait');

/**
 * Create and fund DashJS client
 * @param {string} [HDPrivateKey]
 *
 * @returns {Promise<Client>}
 */
async function createClientWithFundedWallet(HDPrivateKey = undefined) {
  const seeds = getDAPISeeds();

  // Prepare to fund wallet
  const faucetPrivateKey = PrivateKey.fromString(process.env.FAUCET_PRIVATE_KEY);
  const faucetAddress = faucetPrivateKey
    .toAddress(process.env.NETWORK)
    .toString();

  const walletOptions = {};

  if (HDPrivateKey) {
    walletOptions.HDPrivateKey = HDPrivateKey;
  }

  const dashClient = new Dash.Client({
    seeds,
    wallet: walletOptions,
    network: process.env.NETWORK,
    apps: {
      dpns: {
        contractId: process.env.DPNS_CONTRACT_ID,
      },
    },
  });

  const account = await dashClient.getWalletAccount();

  const { address: addressToFund } = account.getAddress();

  const amount = 40000;

  await fundAddress(
    dashClient.getDAPIClient(),
    faucetAddress,
    faucetPrivateKey,
    addressToFund,
    amount,
  );

  do {
    await wait(500);
  } while (account.getTotalBalance() < amount);

  return dashClient;
}

module.exports = createClientWithFundedWallet;
