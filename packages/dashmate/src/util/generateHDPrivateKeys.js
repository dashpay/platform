const { Wallet } = require('@dashevo/wallet-lib');

/**
 * Get identity HDPrivateKey for network
 *
 * @typedef {generateHDPrivateKeys}
 *
 * @param {string} network
 *
 * @returns {Promise<HDPrivateKey>}
 */
async function generateHDPrivateKeys(network) {
  const wallet = new Wallet({ network, offlineMode: true });
  const account = await wallet.getAccount();

  const derivedPrivateKey = account.identities.getIdentityHDKeyByIndex(0, 0);
  const hdPrivateKey = wallet.exportWallet('HDPrivateKey');

  await wallet.disconnect();

  return {
    hdPrivateKey,
    derivedPrivateKey,
  };
}

module.exports = generateHDPrivateKeys;
