const {
  PrivateKey,
} = require('@dashevo/dashcore-lib');

const Dash = require('dash');

const fundAddress = require('./fundAddress');
const wait = require('../util/wait');

/**
 *  * Create and fund DashJS client
 *
 * @typedef {createClientWithFundedWallet}
 * @param {string} network
 * @param {string} faucetPrivateKeyString
 * @param {string} [seed]
 * @return {Promise<Client>}
 */
async function createClientWithFundedWallet(network, faucetPrivateKeyString, seed = undefined) {
  // Prepare to fund wallet
  const faucetPrivateKey = PrivateKey.fromString(faucetPrivateKeyString);
  const faucetAddress = faucetPrivateKey
    .toAddress(network)
    .toString();

  const options = {
    wallet: {
      mnemonic: null,
    },
    network,
  };

  if (seed) {
    options.seeds = [seed];
  }

  const dashClient = new Dash.Client(options);

  const account = await dashClient.getWalletAccount();

  const { address: addressToFund } = account.getAddress();

  const amount = 40000;

  await fundAddress(
    dashClient.getDAPIClient(),
    network,
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
