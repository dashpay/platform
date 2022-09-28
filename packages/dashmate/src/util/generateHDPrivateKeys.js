const { Wallet } = require('@dashevo/wallet-lib');

/**
 * Get identity HDPrivateKey for network
 *
 * @typedef {generateHDPrivateKeys}
 *
 * @param {string} network
 * @param {number[]} keyIndexes
 *
 * @returns {Promise<{
 *   hdPrivateKey: HDPrivateKey,
 *   derivedPrivateKeys: HDPrivateKey[],
 * }>}
 */
async function generateHDPrivateKeys(network, keyIndexes = [0]) {
  const wallet = new Wallet({ network, offlineMode: true });
  const account = await wallet.getAccount();

  const derivedPrivateKeys = [];
  keyIndexes.forEach((keyIndex) => {
    const derivedPrivateKey = account.identities.getIdentityHDKeyByIndex(0, keyIndex);

    derivedPrivateKeys.push(derivedPrivateKey);
  });

  const hdPrivateKey = wallet.exportWallet('HDPrivateKey');

  await wallet.disconnect();

  return {
    hdPrivateKey,
    derivedPrivateKeys,
  };
}

module.exports = generateHDPrivateKeys;
