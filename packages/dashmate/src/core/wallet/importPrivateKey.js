/**
 * Import private key into wallet
 *
 * @typedef {importPrivateKey}
 * @param {CoreService} coreService
 * @param {string} privateKey
 * @param {string?} walletName
 * @return {Promise<void>}
 */
async function importPrivateKey(coreService, privateKey, walletName = null) {

  return coreService.getRpcClient().importPrivKey(privateKey, {wallet: walletName});
}

module.exports = importPrivateKey;
